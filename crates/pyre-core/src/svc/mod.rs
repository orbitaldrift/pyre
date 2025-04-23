use garde::Validate;
use serde::{Deserialize, Serialize};

pub mod limiter;
pub mod middleware;
pub mod server;
pub mod state;

#[derive(Validate, Debug, Clone, Serialize, Deserialize)]
pub struct SessionConfig {
    #[garde(range(min = 1, max = 30))]
    pub session_days: i64,
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self { session_days: 7 }
    }
}
