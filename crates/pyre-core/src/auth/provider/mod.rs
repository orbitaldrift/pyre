use axum_login::AuthSession;
use discord::DiscordProvider;
use garde::Validate;
use oauth2::{basic::BasicClient, AuthorizationCode, EndpointNotSet, EndpointSet, TokenResponse};
use serde::{Deserialize, Serialize};
use tracing::warn;

use super::{
    error::Error,
    session::{Credentials, SessionBackend},
    user::User,
};
use crate::{
    db::{self, Dao},
    svc::state::AppState,
};

pub mod discord;

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
    pub id: i32,

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

        let discord_provider = http
            .get("https://discordapp.com/api/users/@me")
            .bearer_auth(token.access_token().secret())
            .send()
            .await
            .map_err(|e| Error::DiscordToken(e.to_string()))?
            .json::<DiscordProvider>()
            .await
            .map_err(|e| Error::DiscordBody(e.to_string()))?;

        let provider: Provider = discord_provider.into();
        provider
            .validate()
            .map_err(|e| Error::InvalidProvider(e.to_string()))?;

        Ok(provider)
    }

    pub async fn authenticate(
        &mut self,
        mut session: AuthSession<SessionBackend>,
        state: AppState,
    ) -> Result<(), Error> {
        // TODO: move to different fn called login that takes provider and authenticates/logs in.
        if let Some(user) = session
            .authenticate(Credentials {
                provider: self.clone(),
            })
            .await?
        {
            session.login(&user).await?;
        } else {
            let tx = state.sql_pool.begin().await.map_err(db::Error::Sqlx)?;

            let mut user: User = self.clone().into();
            user.create(state.sql_pool.clone())
                .await
                .inspect_err(|e| warn!(%e, "failed to create user"))
                .map_err(|_| Error::UserExists)?;

            self.user_id = user.id;
            self.create(state.sql_pool.clone())
                .await
                .inspect_err(|e| warn!(%e, "failed to create provider"))?;

            session.login(&user).await?;

            tx.commit().await.map_err(db::Error::Sqlx)?;
        }

        Ok(())
    }
}
