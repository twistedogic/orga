use rig_core::providers::{anthropic, openai};

use crate::config::LlmConfig;
use crate::error::OrgaError;

pub enum LlmClient {
    Anthropic(anthropic::Client),
    OpenAi(openai::CompletionsClient),
}

pub fn build_llm_client(cfg: &LlmConfig) -> Result<LlmClient, OrgaError> {
    match cfg.provider.as_str() {
        "anthropic" => {
            let mut builder = anthropic::Client::builder().api_key(cfg.api_key.clone());
            if let Some(ref ep) = cfg.endpoint {
                builder = builder.base_url(ep);
            }
            let client = builder.build().map_err(|e| {
                OrgaError::ConfigError(format!("failed to build Anthropic client: {e}"))
            })?;
            Ok(LlmClient::Anthropic(client))
        }
        "openai" => {
            let mut builder = openai::CompletionsClient::builder().api_key(cfg.api_key.clone());
            if let Some(ref ep) = cfg.endpoint {
                builder = builder.base_url(ep);
            }
            let client = builder.build().map_err(|e| {
                OrgaError::ConfigError(format!("failed to build OpenAI client: {e}"))
            })?;
            Ok(LlmClient::OpenAi(client))
        }
        other => Err(OrgaError::ConfigError(format!(
            "[llm] unsupported provider '{other}'"
        ))),
    }
}
