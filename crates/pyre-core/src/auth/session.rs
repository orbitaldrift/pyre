use std::sync::Arc;

use async_trait::async_trait;
use axum_login::{AuthnBackend, UserId};
use garde::Validate;
use serde::Deserialize;

use super::{
    error::Error,
    provider::{self, Provider},
    user::User,
};
use crate::{db::Dao, svc::state::AppState};

#[derive(Clone)]
pub struct SessionBackend {
    state: Arc<AppState>,
}

impl SessionBackend {
    pub fn new(state: Arc<AppState>) -> Self {
        Self { state }
    }
}

#[derive(Validate, Debug, Clone, Deserialize)]
pub struct Credentials {
    #[garde(skip)]
    pub provider: provider::Provider,
}

#[async_trait]
impl AuthnBackend for SessionBackend {
    type User = User;
    type Credentials = Credentials;
    type Error = Error;

    async fn authenticate(
        &self,
        Credentials { provider }: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        let provider = Provider::get(
            self.state.sql_pool.clone(),
            (provider.provider_kind, provider.external_id),
        )
        .await?;

        match provider {
            Some(provider) => Ok(User::get(self.state.sql_pool.clone(), provider.user_id).await?),
            None => Ok(None),
        }
    }

    async fn get_user(&self, user_id: &UserId<Self>) -> Result<Option<Self::User>, Self::Error> {
        Ok(User::get(self.state.sql_pool.clone(), *user_id).await?)
    }
}
