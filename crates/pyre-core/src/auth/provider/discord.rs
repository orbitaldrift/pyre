use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
};
use axum_login::AuthSession;
use garde::Validate;
use oauth2::{basic::BasicClient, AuthUrl, ClientId, ClientSecret, RedirectUrl, Scope, TokenUrl};
use serde::{Deserialize, Serialize};
use tower_sessions::Session;
use tracing::warn;

use crate::{
    auth::{
        error::Error,
        provider::{ConfiguredClient, Provider, ProviderKind},
        session::SessionBackend,
        CSRF_SESSION_KEY,
    },
    svc::state::AppState,
};

#[derive(Debug, Deserialize)]
pub struct DiscordCallback {
    pub code: String,
    pub state: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiscordProvider {
    pub id: String,

    pub avatar: String,
    pub username: String,
    pub email: String,

    pub discriminator: String,
}

impl From<DiscordProvider> for Provider {
    fn from(value: DiscordProvider) -> Provider {
        let avatar = format!(
            "https://cdn.discordapp.com/avatars/{}/{}.webp?size=256",
            value.id,
            value.avatar.as_str()
        );

        Provider {
            id: 0,
            user_id: 0,
            external_id: value.id,
            provider_kind: ProviderKind::Discord,
            username: format!("{}#{}", value.username, value.discriminator),
            avatar: Some(avatar),
            email: Some(value.email),
            created_at: chrono::Utc::now(),
        }
    }
}

#[allow(clippy::ignored_unit_patterns)]
#[derive(Validate, Debug, Clone, Serialize, Deserialize)]
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

impl Default for Config {
    fn default() -> Self {
        Self {
            client_id: "1363014531705471097".to_string(),
            client_secret: ".discord.key".to_string(),
            auth_url: "https://discord.com/api/oauth2/authorize".to_string(),
            token_url: "https://discord.com/api/oauth2/token".to_string(),
            redirect_url: "https://127.0.0.1:4433/auth/discord/callback".to_string(),
            scopes: vec!["identify".to_string(), "email".to_string()],
        }
    }
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

pub async fn redirect(
    session: Session,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, Error> {
    let csrf = oauth2::CsrfToken::new_random();

    let (auth_url, csrf) = state
        .get_oauth_client(ProviderKind::Discord)
        .map_err(Error::ProviderNotFound)?
        .authorize_url(|| csrf)
        .add_scopes(
            state
                .config
                .discord
                .scopes
                .iter()
                .map(|s| Scope::new(s.clone())),
        )
        .add_extra_param("prompt", "none")
        .url();

    session.insert(CSRF_SESSION_KEY, csrf.secret()).await?;

    Ok(Redirect::to(auth_url.as_ref()))
}

pub async fn auth(
    auth_session: AuthSession<SessionBackend>,
    session: Session,
    Query(query): Query<DiscordCallback>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, Error> {
    let csrf = session.get::<String>(CSRF_SESSION_KEY).await?.unwrap();
    csrf.as_bytes()
        .eq(query.state.as_bytes())
        .then(|| {
            warn!("CSRF token mismatch");
        })
        .ok_or(Error::InvalidOAuthCsrf)?;

    Provider::new_from_discord(&state, query.code)
        .await?
        .authenticate(auth_session, state)
        .await?;

    Ok(Redirect::to("/"))
}
