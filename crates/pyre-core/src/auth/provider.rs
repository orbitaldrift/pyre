use discord::DiscordUser;
use garde::Validate;
use oauth2::{basic::BasicClient, AuthorizationCode, EndpointNotSet, EndpointSet, TokenResponse};
use serde::{Deserialize, Serialize};

use super::error::Error;
use crate::{db::Dao, svc::state::AppState};

pub type ConfiguredClient =
    BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>;

#[derive(
    sqlx::Type, strum::Display, Copy, Hash, Debug, Clone, Eq, PartialEq, Serialize, Deserialize,
)]
#[sqlx(type_name = "provider_kind", rename_all = "lowercase")]
pub enum ProviderKind {
    Discord,
}

#[allow(clippy::struct_field_names)]
#[derive(Validate, Debug, Clone, Serialize, Deserialize)]
pub struct Provider {
    #[garde(skip)]
    id: i32,

    #[garde(skip)]
    pub user_id: i32,

    #[garde(ascii, length(min = 1, max = 255))]
    pub external_id: String,

    #[garde(skip)]
    pub provider_kind: ProviderKind,

    #[garde(ascii, length(min = 1, max = 32))]
    pub username: String,

    #[garde(url)]
    pub avatar: Option<String>,

    #[garde(email)]
    pub email: Option<String>,

    #[garde(skip)]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[async_trait::async_trait]
impl Dao for Provider {
    type Id = (ProviderKind, String);

    type Dal = sqlx::PgPool;

    async fn get(dal: Self::Dal, id: Self::Id) -> Result<Option<Self>, crate::db::Error> {
        let mut conn = dal.acquire().await?;

        Ok(sqlx::query_as!(
            Provider,
            r#"
            SELECT 
                p.id, 
                p.user_id, 
                p.external_id, 
                p.kind AS "provider_kind!: ProviderKind", 
                p.username, 
                p.created_at, 
                NULL AS "avatar?: String", 
                NULL AS "email?: String"
            FROM providers p
            WHERE p.kind = $1 AND p.external_id = $2
            "#,
            id.0 as ProviderKind,
            id.1
        )
        .fetch_optional(conn.as_mut())
        .await?)
    }

    async fn delete(_dal: Self::Dal, _id: Self::Id) -> Result<(), crate::db::Error> {
        todo!()
    }

    async fn create(&mut self, dal: Self::Dal) -> Result<(), crate::db::Error> {
        let mut conn = dal.acquire().await?;

        let q = sqlx::query!(
            r#"
            INSERT INTO providers (user_id, external_id, kind, username, created_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            "#,
            &self.user_id,
            &self.external_id,
            self.provider_kind as ProviderKind,
            &self.username,
            &self.created_at
        )
        .fetch_one(conn.as_mut())
        .await?;

        self.id = q.id;
        Ok(())
    }

    async fn update(&self, _dal: Self::Dal) -> Result<Self::Id, crate::db::Error> {
        todo!()
    }
}

impl Provider {
    /// Creates a new provider instance from an `OAuth2` exchange.
    ///
    /// # Panics
    /// If the `OAuth2` client is not found in the state.
    pub async fn new_from_discord(state: &AppState, code: String) -> Result<Self, Error> {
        let oauth = state
            .get_oauth_client(ProviderKind::Discord)
            .map_err(Error::ProviderNotFound)?;

        let http = state.http_client.clone();

        let token = oauth
            .exchange_code(AuthorizationCode::new(code))
            .request_async(&http)
            .await
            .map_err(|e| Error::OAuth2TokenRequest(e.to_string()))?;

        let discord_user = http
            .get("https://discordapp.com/api/users/@me")
            .bearer_auth(token.access_token().secret())
            .send()
            .await
            .map_err(|e| Error::DiscordToken(e.to_string()))?
            .json::<DiscordUser>()
            .await
            .map_err(|e| Error::DiscordBody(e.to_string()))?;

        let provider = discord_user.into_provider();
        provider
            .validate()
            .map_err(|e| Error::InvalidProvider(e.to_string()))?;

        Ok(provider)
    }
}

pub(crate) mod discord {
    use axum::{
        extract::{Query, State},
        response::{IntoResponse, Redirect},
    };
    use axum_login::AuthSession;
    use garde::Validate;
    use oauth2::{
        basic::BasicClient,
        AuthUrl,
        ClientId,
        ClientSecret,
        RedirectUrl,
        Scope,
        TokenUrl,
    };
    use pyre_axum_csrf::token::CsrfToken;
    use serde::{Deserialize, Serialize};
    use tracing::warn;

    use super::{ConfiguredClient, Provider, ProviderKind};
    use crate::{
        auth::{
            error::Error,
            session::{Credentials, SessionBackend},
            user::User,
        },
        db::{self, Dao},
        svc::state::AppState,
    };

    #[derive(Debug, Deserialize)]
    pub struct DiscordCallback {
        pub code: String,
        pub state: String,
    }

    #[derive(Debug, Clone, Deserialize)]
    pub struct DiscordUser {
        pub id: String,

        pub avatar: String,
        pub username: String,
        pub email: String,

        pub discriminator: String,
    }

    impl DiscordUser {
        pub fn into_provider(self) -> super::Provider {
            let avatar = format!(
                "https://cdn.discordapp.com/avatars/{}/{}.webp?size=256",
                self.id,
                self.avatar.as_str()
            );

            super::Provider {
                id: 0,
                user_id: 0,
                external_id: self.id,
                provider_kind: super::ProviderKind::Discord,
                username: format!("{}#{}", self.username, self.discriminator),
                avatar: Some(avatar),
                email: Some(self.email),
                created_at: chrono::Utc::now(),
            }
        }
    }

    #[allow(clippy::ignored_unit_patterns)]
    #[derive(Validate, Debug, Clone, Default, Serialize, Deserialize)]
    pub struct Config {
        #[garde(ascii, length(min = 1))]
        pub client_id: String,

        #[garde(ascii, length(min = 1))]
        pub client_secret: String,

        #[garde(url)]
        pub auth_url: String,

        #[garde(url)]
        pub token_url: String,

        #[garde(url)]
        pub redirect_url: String,

        #[garde(length(min = 1), inner(ascii, length(min = 1)))]
        pub scopes: Vec<String>,
    }

    /// Discord `OAuth2` client
    ///
    /// # Panics
    /// If configuration contains invalid values.
    pub async fn new_client(cfg: Config) -> ConfiguredClient {
        cfg.validate().expect("Invalid Discord config");

        let secret = tokio::fs::read(cfg.client_secret.clone())
            .await
            .unwrap_or_else(|_| panic!("failed to read secret from {}", cfg.client_secret));

        assert!(!secret.is_empty(), "discord secret must not be empty file");

        BasicClient::new(ClientId::new(cfg.client_id))
            .set_client_secret(ClientSecret::new(
                String::from_utf8_lossy(&secret).to_string(),
            ))
            .set_auth_uri(AuthUrl::new(cfg.auth_url).unwrap())
            .set_token_uri(TokenUrl::new(cfg.token_url).unwrap())
            .set_redirect_uri(RedirectUrl::new(cfg.redirect_url).unwrap())
    }

    pub async fn auth_discord(
        token: CsrfToken,
        State(state): State<AppState>,
    ) -> Result<impl IntoResponse, Error> {
        let csrf = token.get().map_err(|e| Error::CsrfNotSet(e.to_string()))?;

        // TODO: catpcha verify as middleware layer, verification happens automatic from cookies or headers in layer
        // also provide Extractor, make a separate crate

        let (auth_url, _csrf_token) = state
            .get_oauth_client(ProviderKind::Discord)
            .map_err(Error::ProviderNotFound)?
            .authorize_url(|| oauth2::CsrfToken::new(csrf))
            .add_scopes(
                state
                    .config
                    .discord
                    .scopes
                    .iter()
                    .map(|s| Scope::new(s.clone())),
            )
            .url();

        Ok(Redirect::to(auth_url.as_ref()))
    }

    #[axum::debug_handler]
    pub async fn auth_discord_auth(
        mut session: AuthSession<SessionBackend>,
        token: CsrfToken,
        Query(query): Query<DiscordCallback>,
        State(state): State<AppState>,
    ) -> Result<impl IntoResponse, Error> {
        let _csrf = token.get().map_err(|e| Error::CsrfNotSet(e.to_string()))?;

        // TODO: csrf verify from query.state against csrf, constant time, supported by csrftoken type instead of get->verify

        let mut provider = Provider::new_from_discord(&state, query.code).await?;

        // TODO: move to different fn called login that takes provider and authenticates/logs in.
        if let Some(user) = session
            .authenticate(Credentials {
                provider: provider.clone(),
            })
            .await?
        {
            session.login(&user).await?;
        } else {
            let tx = state.sql_pool.begin().await.map_err(db::Error::Sqlx)?;

            let mut user: User = provider.clone().into();
            user.create(state.sql_pool.clone())
                .await
                .inspect_err(|e| warn!(%e, "failed to create user"))
                .map_err(|_| Error::UserExists)?;

            provider.user_id = user.id;
            provider
                .create(state.sql_pool.clone())
                .await
                .inspect_err(|e| warn!(%e, "failed to create provider"))?;

            session.login(&user).await?;

            tx.commit().await.map_err(db::Error::Sqlx)?;
        }

        Ok(Redirect::to("/"))
    }
}
