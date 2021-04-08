use thiserror::Error;

#[derive(Debug, Error)]
pub enum OrgaError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("unauthorized: {0}")]
    Unauthorized(String),

    #[error("rate limited by backend, try again later")]
    RateLimited,

    #[error("network error: {0}")]
    NetworkError(String),

    #[error("backend error: {0}")]
    BackendError(String),

    #[error("config error: {0}")]
    ConfigError(String),
}

impl From<reqwest::Error> for OrgaError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() || e.is_connect() {
            OrgaError::NetworkError(e.to_string())
        } else {
            OrgaError::BackendError(e.to_string())
        }
    }
}

impl From<rusqlite::Error> for OrgaError {
    fn from(e: rusqlite::Error) -> Self {
        OrgaError::BackendError(format!("sqlite: {e}"))
    }
}
