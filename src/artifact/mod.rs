use crate::config::AppConfig;
use crate::error::OrgaError;
use crate::models::{Artifact, ArtifactMeta};

pub mod git;

pub trait ArtifactStore {
    fn commit(&self, ticket_id: &str, name: &str, content: &[u8]) -> Result<ArtifactMeta, OrgaError>;
    fn get(&self, ticket_id: &str, name: &str) -> Result<Option<Artifact>, OrgaError>;
    fn list(&self, ticket_id: &str) -> Result<Vec<ArtifactMeta>, OrgaError>;
}

pub fn build_artifact_store(config: &AppConfig) -> Result<Box<dyn ArtifactStore>, OrgaError> {
    let artifact_cfg = config.artifact.as_ref().ok_or_else(|| {
        OrgaError::ConfigError("[artifact] section missing from config".into())
    })?;

    match artifact_cfg.backend.as_str() {
        "git" => {
            let git_cfg = artifact_cfg.git.as_ref().ok_or_else(|| {
                OrgaError::ConfigError("[artifact.git] section missing".into())
            })?;
            let auth = git::GitAuth {
                ssh_key: git_cfg.ssh_key.as_deref().map(|p| std::path::PathBuf::from(
                    orga_config_expand(p)
                )),
                ssh_passphrase: git_cfg.ssh_passphrase.clone(),
                http_username: git_cfg.http_username.clone(),
                http_password: git_cfg.http_password.clone(),
            };
            let store = git::GitArtifactStore::new(
                git_cfg.path.clone(),
                config.agent.name.clone(),
                git_cfg.remote.clone(),
                git_cfg.branch.clone().unwrap_or_else(|| "main".into()),
                auth,
            );
            Ok(Box::new(store))
        }
        other => Err(OrgaError::ConfigError(format!(
            "unsupported artifact backend '{other}'. Supported backends: git"
        ))),
    }
}

fn orga_config_expand(path: &str) -> String {
    use crate::config::expand_tilde;
    expand_tilde(path).to_string_lossy().into_owned()
}
