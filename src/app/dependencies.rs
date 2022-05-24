use crate::app::config::{Config, Mode};
use crate::app::k8s;
use crate::app::wasm::distributed::DistributedRunner;
use crate::app::wasm::local::{cache, LocalRunner};
use crate::app::wasm::Runner;
use anyhow::anyhow;
use clap::Parser;
use std::sync::Arc;

pub trait DependencyProvider {
    fn get_config(&self) -> &Config;
    fn get_runner(&self) -> Box<dyn Runner + Send + Sync>;
}

pub type DynDependencyProvider = Arc<dyn DependencyProvider + Send + Sync>;

struct RuntimeDependencyProvider {
    config: Config,
    client: Option<kube::Client>,
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

    let provider = RuntimeDependencyProvider { config, client };
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
}
