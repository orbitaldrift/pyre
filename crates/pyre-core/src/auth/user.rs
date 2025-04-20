use axum_login::AuthUser;
use const_hex::ToHexExt;
use garde::Validate;
use rand::RngCore;
use serde::{Deserialize, Serialize};

use super::provider::Provider;
use crate::db::Dao;

#[derive(Validate, Debug, Clone, Serialize, Deserialize)]
pub struct User {
    #[garde(skip)]
    pub id: i32,

    #[garde(url)]
    pub avatar: String,

    #[garde(ascii, length(min = 1, max = 32))]
    pub name: String,

    #[garde(email)]
    pub email: String,

    #[garde(ascii, length(min = 1, max = 64))]
    pub auth_hash: String,

    #[garde(skip)]
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<Provider> for User {
    fn from(provider: Provider) -> Self {
        let mut auth = [0u8; 32];
        rand::rng().fill_bytes(&mut auth);

        Self {
            id: provider.user_id,
            avatar: provider.avatar.unwrap_or_default(),
            name: provider.username,
            email: provider.email.unwrap_or_default(),
            auth_hash: blake3::hash(&auth).as_bytes().encode_hex(),
            created_at: provider.created_at,
        }
    }
}

#[async_trait::async_trait]
impl Dao for User {
    type Id = i32;

    type Dal = sqlx::PgPool;

    async fn get(dal: Self::Dal, id: Self::Id) -> Result<Option<Self>, crate::db::Error> {
        let mut conn = dal.acquire().await?;

        Ok(sqlx::query_as!(
            User,
            r#"
            SELECT 
                u.id, 
                u.avatar, 
                u.name, 
                u.email, 
                u.auth_hash, 
                u.created_at
            FROM users u
            WHERE u.id = $1
            "#,
            id
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
            INSERT INTO users (avatar, name, email, auth_hash)
            VALUES ($1, $2, $3, $4)
            RETURNING id
            "#,
            &self.avatar,
            &self.name,
            &self.email,
            &self.auth_hash
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

impl AuthUser for User {
    type Id = i32;

    fn id(&self) -> Self::Id {
        self.id
    }

    fn session_auth_hash(&self) -> &[u8] {
        self.auth_hash.as_bytes()
    }
}
