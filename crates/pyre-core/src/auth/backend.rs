use std::collections::HashMap;

use async_trait::async_trait;
use axum_login::{AuthUser, AuthnBackend, UserId};
use serde::Deserialize;
use tracing::info;

#[derive(Debug, Clone)]
pub struct User {
    pub id: i64,
    pub pw_hash: Vec<u8>,
}

impl AuthUser for User {
    type Id = i64;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn session_auth_hash(&self) -> &[u8] {
        &self.pw_hash
    }
}

#[derive(Clone, Default)]
pub struct Backend {
    pub users: HashMap<i64, User>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Credentials {
    user_id: i64,
}

#[async_trait]
impl AuthnBackend for Backend {
    type User = User;
    type Credentials = Credentials;
    type Error = std::convert::Infallible;

    async fn authenticate(
        &self,
        Credentials { user_id }: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        let user = self.users.get(&user_id);

        info!("user: {:?}", user);

        Ok(self.users.get(&user_id).cloned())
    }

    async fn get_user(&self, user_id: &UserId<Self>) -> Result<Option<Self::User>, Self::Error> {
        Ok(self.users.get(user_id).cloned())
    }
}
