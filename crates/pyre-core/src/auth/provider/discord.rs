use axum::{
    extract::{Query, State},
    response::{IntoResponse, Redirect},
};
use axum_login::AuthSession;
use garde::Validate;
use oauth2::{basic::BasicClient, AuthUrl, ClientId, ClientSecret, RedirectUrl, Scope, TokenUrl};
use pyre_axum_csrf::{cookie::CsrfCookie, token::CsrfToken};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::{
    auth::{
        error::Error,
        provider::{ConfiguredClient, Provider, ProviderKind},
        session::SessionBackend,
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

pub async fn redirect(
    token: CsrfCookie,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, Error> {
    let csrf = token.get()?;

    // TODO: catpcha verify as middleware layer, verification happens automatic from cookies or headers in layer
    // Create Extractor, make a separate crate for pyre-axum-captcha

    let (auth_url, _csrf_token) = state
        .get_oauth_client(ProviderKind::Discord)
        .map_err(Error::ProviderNotFound)?
        .authorize_url(|| oauth2::CsrfToken::new((*csrf).clone()))
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
pub async fn auth(
    session: AuthSession<SessionBackend>,
    Query(query): Query<DiscordCallback>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, Error> {
    if !CsrfToken::validate_signature_only(&state.secret, &query.state)? {
        warn!("invalid CSRF token");
        return Err(Error::InvalidOAuthCsrf);
    }

    Provider::new_from_discord(&state, query.code)
        .await?
        .authenticate(session, state)
        .await?;

    Ok(Redirect::to("/"))
}
