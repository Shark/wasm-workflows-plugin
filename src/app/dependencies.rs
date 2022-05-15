use crate::app::config::{Config, Mode};
use crate::app::wasm::local::{cache, LocalRunner};
use crate::app::wasm::Runner;
use clap::Parser;
use std::sync::Arc;

pub trait DependencyProvider {
    fn get_config(&self) -> &Config;
    fn get_runner(&self) -> Box<dyn Runner + Send + Sync>;
}

pub type DynDependencyProvider = Arc<dyn DependencyProvider + Send + Sync>;

struct RuntimeDependencyProvider {
    config: Config,
}

pub fn initialize() -> anyhow::Result<DynDependencyProvider> {
    let config = Config::parse();

    Ok(Arc::new(RuntimeDependencyProvider { config }))
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
                unimplemented!()
            }
        }
    }
}
