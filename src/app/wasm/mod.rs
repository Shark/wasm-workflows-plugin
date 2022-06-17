use crate::app::model::ModulePermissions;
use anyhow::Error;
use async_trait::async_trait;
use workflow_model::model::{PluginInvocation, PluginResult, S3ArtifactRepositoryConfig};

pub mod distributed;
pub mod local;

#[async_trait]
pub trait Runner {
    async fn run(
        &self,
        oci_image: &str,
        invocation: PluginInvocation,
        perms: &Option<ModulePermissions>,
        artifact_repo_config: Option<S3ArtifactRepositoryConfig>,
    ) -> anyhow::Result<PluginResult, WasmError>;
}

#[derive(Debug)]
pub enum WasmError {
    EnvironmentSetup(Error),
    Retrieve(Error),
    Precompile(Error),
    Invocation(Error),
    OutputProcessing(Error),
    Timeout(Error),
}
