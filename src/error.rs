use rig_core::completion::CompletionError;
use rig_core::http_client;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LlmErrorKind {
    Network,
    RateLimited,
    Auth,
    Parse,
    Backend,
    Other,
}

impl LlmErrorKind {
    pub fn as_str(self) -> &'static str {
        match self {
            LlmErrorKind::Network => "network",
            LlmErrorKind::RateLimited => "rate_limit",
            LlmErrorKind::Auth => "auth",
            LlmErrorKind::Parse => "parse",
            LlmErrorKind::Backend => "backend",
            LlmErrorKind::Other => "other",
        }
    }
}

impl std::fmt::Display for LlmErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

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

    #[error("llm error ({kind}): {message}")]
    LlmError { kind: LlmErrorKind, message: String },

    #[error("config error: {0}")]
    ConfigError(String),

    #[error("systemd is only supported on Linux")]
    SystemdNotLinux,

    #[error("root privileges are required for system-level service installation")]
    SystemdRootRequired,

    #[error("failed to write systemd unit file: {0}")]
    SystemdWriteFailed(String),
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

pub fn classify_completion_error(err: &CompletionError) -> (LlmErrorKind, String) {
    match err {
        CompletionError::HttpError(http_err) => classify_http_client_error(http_err),
        CompletionError::ResponseError(_) => (LlmErrorKind::Parse, err.to_string()),
        CompletionError::ProviderError(_) => (LlmErrorKind::Backend, err.to_string()),
        CompletionError::JsonError(_) => (LlmErrorKind::Parse, err.to_string()),
        CompletionError::UrlError(_) => (LlmErrorKind::Parse, err.to_string()),
        CompletionError::RequestError(_) => (LlmErrorKind::Other, err.to_string()),
    }
}

fn classify_http_client_error(http_err: &http_client::Error) -> (LlmErrorKind, String) {
    match http_err {
        http_client::Error::Instance(boxed) => {
            if let Some(req_err) = boxed.downcast_ref::<reqwest::Error>() {
                if req_err.is_timeout() || req_err.is_connect() {
                    return (LlmErrorKind::Network, req_err.to_string());
                }
                if let Some(status) = req_err.status() {
                    return (status_to_kind(status.as_u16()), req_err.to_string());
                }
            }
            (LlmErrorKind::Other, http_err.to_string())
        }
        http_client::Error::InvalidStatusCode(status) => {
            (status_to_kind(status.as_u16()), http_err.to_string())
        }
        http_client::Error::InvalidStatusCodeWithMessage(status, _) => {
            (status_to_kind(status.as_u16()), http_err.to_string())
        }
        _ => (LlmErrorKind::Other, http_err.to_string()),
    }
}

fn status_to_kind(status: u16) -> LlmErrorKind {
    match status {
        429 => LlmErrorKind::RateLimited,
        401 | 403 => LlmErrorKind::Auth,
        400..=499 => LlmErrorKind::Parse,
        500..=599 => LlmErrorKind::Backend,
        _ => LlmErrorKind::Other,
    }
}

impl From<CompletionError> for OrgaError {
    fn from(err: CompletionError) -> Self {
        let (kind, message) = classify_completion_error(&err);
        OrgaError::LlmError { kind, message }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rig_core::http_client;

    #[test]
    fn llm_error_display_contains_kind_and_message() {
        let e = OrgaError::LlmError {
            kind: LlmErrorKind::RateLimited,
            message: "rate limited".into(),
        };
        let s = e.to_string();
        assert!(s.contains("rate_limit"), "got: {s}");
        assert!(s.contains("rate limited"), "got: {s}");
    }

    #[test]
    fn llm_error_kind_as_str_uses_snake_case_label() {
        assert_eq!(LlmErrorKind::Network.as_str(), "network");
        assert_eq!(LlmErrorKind::RateLimited.as_str(), "rate_limit");
        assert_eq!(LlmErrorKind::Auth.as_str(), "auth");
        assert_eq!(LlmErrorKind::Parse.as_str(), "parse");
        assert_eq!(LlmErrorKind::Backend.as_str(), "backend");
        assert_eq!(LlmErrorKind::Other.as_str(), "other");
    }

    #[test]
    fn classify_response_error_is_parse() {
        let err = CompletionError::ResponseError("bad shape".into());
        let (kind, _) = classify_completion_error(&err);
        assert_eq!(kind, LlmErrorKind::Parse);
    }

    #[test]
    fn classify_provider_error_is_backend() {
        let err = CompletionError::ProviderError("server blew up".into());
        let (kind, _) = classify_completion_error(&err);
        assert_eq!(kind, LlmErrorKind::Backend);
    }

    #[test]
    fn classify_invalid_status_code_uses_status() {
        let (kind, _) =
            classify_http_client_error(&http_client::Error::InvalidStatusCodeWithMessage(
                reqwest::StatusCode::TOO_MANY_REQUESTS,
                "rate limited".to_string(),
            ));
        assert_eq!(kind, LlmErrorKind::RateLimited);
    }

    #[test]
    fn from_completion_error_builds_llm_error_variant() {
        let err = CompletionError::HttpError(http_client::Error::InvalidStatusCode(
            reqwest::StatusCode::TOO_MANY_REQUESTS,
        ));
        let orga_err: OrgaError = err.into();
        match orga_err {
            OrgaError::LlmError { kind, .. } => assert_eq!(kind, LlmErrorKind::RateLimited),
            other => panic!("expected LlmError, got {other:?}"),
        }
    }
}
