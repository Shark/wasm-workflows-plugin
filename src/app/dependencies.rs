use std::path::PathBuf;
use std::sync::Arc;
use clap::Parser;
use wasmtime::Engine;
use crate::app::config::Config;
use crate::app::wasm::{self, cache, cache::ModuleCache};

pub trait DependencyProvider {
    fn get_config(&self) -> &Config;
    fn get_wasm_engine(&self) -> &wasmtime::Engine;
    fn get_module_cache(&self) -> &(dyn ModuleCache + Send + Sync);
}

pub type DynDependencyProvider = Arc<dyn DependencyProvider + Send + Sync>;

struct RuntimeDependencyProvider {
    config: Config,
    wasm_engine: wasmtime::Engine,
    module_cache: Box<dyn ModuleCache + Send + Sync>,
}

pub fn initialize() -> anyhow::Result<DynDependencyProvider> {
    let config = Config::parse();
    let wasm_engine = wasm::setup_engine()?;
    let module_cache: Box<dyn ModuleCache + Send + Sync> = match config.fs_cache_dir.to_owned() {
        Some(dir) => Box::new(cache::new_fs_cache(PathBuf::from(dir))),
        None => Box::new(cache::new_nop_cache()),
    };
    Ok(Arc::new(RuntimeDependencyProvider {
        config,
        wasm_engine,
        module_cache,
    }))
}

impl DependencyProvider for RuntimeDependencyProvider {
    fn get_config(&self) -> &Config {
        &self.config
    }

    fn get_wasm_engine(&self) -> &Engine {
        &self.wasm_engine
    }

    fn get_module_cache(&self) -> &(dyn ModuleCache + Send + Sync) {
        &self.module_cache
    }
}

// This is to make the compiler happy so that we can return a trait object from get_module_cache
impl<W: ModuleCache + ?Sized> ModuleCache for Box<W> {
    #[inline]
    fn get(&self, image: &str) -> anyhow::Result<Option<Vec<u8>>> {
        (**self).get(image)
    }

    #[inline]
    fn put(&self, image: &str, data: &[u8]) -> anyhow::Result<()> {
        (**self).put(image, data)
    }

    #[inline]
    fn purge(&self, max_size_mib: u64) -> anyhow::Result<()> {
        (**self).purge(max_size_mib)
    }
}
