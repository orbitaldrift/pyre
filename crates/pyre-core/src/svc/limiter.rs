use std::sync::Arc;

use axum::extract::Request;
use axum_login::{AuthSession, AuthUser, AuthnBackend};
use governor::{clock::QuantaInstant, middleware::NoOpMiddleware};
use tower_governor::{
    governor::{GovernorConfig, GovernorConfigBuilder},
    key_extractor::{KeyExtractor, SmartIpKeyExtractor},
    GovernorError,
};

use super::server::HttpConfig;

/// A [`KeyExtractor`] that tries to get the user ID from the session before falling back to getting a key
/// from `SmartIpKeyExtractor`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserIdKeyExtractor<SB: AuthnBackend>(std::marker::PhantomData<SB>);

impl<SB: AuthnBackend + 'static> UserIdKeyExtractor<SB> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<SB: AuthnBackend + 'static> KeyExtractor for UserIdKeyExtractor<SB> {
    type Key = String;

    fn name(&self) -> &'static str {
        "user ID"
    }

    fn extract<T>(&self, req: &Request<T>) -> Result<Self::Key, GovernorError> {
        let session = req.extensions().get::<AuthSession<SB>>();
        if let Some(session) = session {
            if let Some(user) = &session.user {
                return Ok(user.id().to_string());
            }
        }

        let smart_ip_extractor = SmartIpKeyExtractor;
        Ok(smart_ip_extractor.extract(req)?.to_string())
    }

    fn key_name(&self, key: &Self::Key) -> Option<String> {
        Some(key.to_string())
    }
}

pub fn setup<T>(
    cfg: &HttpConfig,
    extractor: T,
) -> Arc<GovernorConfig<T, NoOpMiddleware<QuantaInstant>>>
where
    T: KeyExtractor,
    <T as KeyExtractor>::Key: Send + Sync + 'static,
{
    let extractor_name = extractor.name().to_string();
    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .const_per_second(cfg.limiter_period)
            .key_extractor(extractor)
            .finish()
            .unwrap(),
    );

    let governor_limiter = governor_conf.limiter().clone();

    // Spawn tokio task that runs every x seconds to retain only recently hit keys
    let interval = std::time::Duration::from_secs(cfg.limiter_retain_interval);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(interval).await;
            tracing::info!(
                gauge.governor = governor_limiter.len(),
                extractor = extractor_name,
            );
            governor_limiter.retain_recent();
        }
    });

    governor_conf
}
