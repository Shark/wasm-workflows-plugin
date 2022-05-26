use crate::app::config::{Config, Mode};
use crate::app::k8s;
use crate::app::wasm::distributed::DistributedRunner;
use crate::app::wasm::local::{cache, LocalRunner};
use crate::app::wasm::Runner;
use anyhow::{anyhow, Context};
use clap::Parser;
use std::sync::Arc;
use workflow_model::model::S3ArtifactRepositoryConfig;

pub trait DependencyProvider {
    fn get_config(&self) -> &Config;
    fn get_runner(&self) -> Box<dyn Runner + Send + Sync>;
    fn get_artifact_repository_config(&self) -> Option<S3ArtifactRepositoryConfig>;
}

pub type DynDependencyProvider = Arc<dyn DependencyProvider + Send + Sync>;

struct RuntimeDependencyProvider {
    config: Config,
    client: Option<kube::Client>,
    artifact_repository_config: Option<S3ArtifactRepositoryConfig>,
}

pub async fn initialize() -> anyhow::Result<DynDependencyProvider> {
    let config = Config::parse();
    let client = match k8s::create_kube_client(&config).await {
        Ok(client) => Some(client),
        Err(why) => {
            if config.mode() == Mode::Distributed {
                let why =
                    anyhow!(why).context("Kube client is required because mode is distributed");
                return Err(why);
            }
            None
        }
    };
    let artifact_repository_config = match &config.argo_controller_configmap {
        Some(name) => match &client {
            Some(client) => {
                let config = k8s::fetch_artifact_repository_config(
                    client,
                    config.plugin_namespace.as_deref(),
                    name,
                )
                .await
                .context("Fetching artifact repository config")?;
                Some(config)
            }
            None => None,
        },
        None => None,
    };

    let provider = RuntimeDependencyProvider {
        config,
        client,
        artifact_repository_config,
    };
    Ok(Arc::new(provider))
}

impl DependencyProvider for RuntimeDependencyProvider {
    fn get_config(&self) -> &Config {
        &self.config
    }

    fn get_runner(&self) -> Box<dyn Runner + Send + Sync> {
        match self.config.mode() {
            Mode::Local => {
                let insecure_oci_registries: Vec<String> =
                    self.config.insecure_oci_registries.to_owned();
                let cache = cache::create_module_cache(&self.config.fs_cache_dir);
                let runner = LocalRunner::new(cache, insecure_oci_registries);
                Box::new(runner)
            }
            Mode::Distributed => {
                let client = self.client.as_ref().unwrap().clone();
                let namespace = self.config.plugin_namespace.to_owned();
                let runner = DistributedRunner::new(client, namespace);
                Box::new(runner)
            }
        }
    }

    fn get_artifact_repository_config(&self) -> Option<S3ArtifactRepositoryConfig> {
        self.artifact_repository_config
            .as_ref()
            .map(|cfg| cfg.to_owned())
    }
}
