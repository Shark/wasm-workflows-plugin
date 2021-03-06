use crate::app::model::ModulePermissions;
use crate::app::wasm::local::cache::ModuleCache;
use crate::app::wasm::local::interface::{WASIModule, WorkflowPlugin};
use crate::app::wasm::{Runner, WasmError};
use anyhow::anyhow;
use async_trait::async_trait;
use tracing::debug_span;
use wasmtime::{Engine, Module};
use workflow_model::model::{PluginInvocation, PluginResult, S3ArtifactRepositoryConfig};

pub mod cache;
mod image;
mod interface;

pub struct LocalRunner {
    cache: Box<dyn ModuleCache + Send + Sync>,
    insecure_oci_registries: Vec<String>,
}

impl LocalRunner {
    pub fn new(
        cache: Box<dyn ModuleCache + Send + Sync>,
        insecure_oci_registries: Vec<String>,
    ) -> Self {
        LocalRunner {
            cache,
            insecure_oci_registries,
        }
    }
}

#[async_trait]
impl Runner for LocalRunner {
    #[tracing::instrument(
        name = "wasm.run_local",
        ret,
        err(Debug),
        skip(self, artifact_repo_config)
    )]
    async fn run(
        &self,
        oci_image: &str,
        invocation: PluginInvocation,
        perms: &Option<ModulePermissions>,
        artifact_repo_config: Option<S3ArtifactRepositoryConfig>,
    ) -> anyhow::Result<PluginResult, WasmError> {
        let engine = setup_engine().map_err(WasmError::EnvironmentSetup)?;
        let mut module: Option<Vec<u8>> = self.cache.get(oci_image).map_err(|err| {
            WasmError::EnvironmentSetup(anyhow!(err).context("Checking Wasm module cache failed"))
        })?;
        if module.is_none() {
            let insecure_oci_registries: Vec<&str> = self
                .insecure_oci_registries
                .iter()
                .map(|i| i.as_str())
                .collect();
            let pulled_mod: Vec<u8> =
                pull(oci_image, &insecure_oci_registries)
                    .await
                    .map_err(|err| {
                        WasmError::Retrieve(anyhow!(err).context("Wasm module retrieve failed"))
                    })?;
            let precompiled_mod = debug_span!("engine.precompile_module").in_scope(|| {
                engine.precompile_module(&pulled_mod).map_err(|err| {
                    WasmError::Precompile(anyhow!(err).context("Wasm module precompilation failed"))
                })
            })?;
            let _ = self.cache.put(oci_image, &precompiled_mod).map_err(|err| {
                WasmError::Retrieve(anyhow!(err).context("Storing Wasm module in cache failed"))
            })?;
            module = Some(precompiled_mod);
        }

        let module = debug_span!("engine.deserialize_mod").in_scope(|| {
            unsafe { Module::deserialize(&engine, module.unwrap()) }.map_err(|err| {
                WasmError::EnvironmentSetup(anyhow!(err).context("Deserializing module failed"))
            })
        })?;

        // First try to instantiate the module as WIT and fall back to WASI in case of an error
        let mut plugin: Box<dyn WorkflowPlugin + Send> =
            match WASIModule::try_new(&engine, &module, perms, artifact_repo_config)
                .await
                .map_err(|err| {
                    WasmError::EnvironmentSetup(anyhow!(err).context("Creating WASI module failed"))
                }) {
                Ok(wasi) => Box::new(wasi),
                Err(e) => return Err(e),
            };
        let result = plugin.run(invocation).await.map_err(|err| {
            WasmError::Invocation(anyhow!(err).context("Wasm module invocation failed"))
        })?;
        Ok(result)
    }
}

#[tracing::instrument(name = "wasm.oci_pull", level = "debug", skip(insecure_oci_registries))]
async fn pull<'a>(
    oci_image_name: &str,
    insecure_oci_registries: &'a [&'a str],
) -> anyhow::Result<Vec<u8>> {
    // Pull module image, put into Vec<u8>
    image::fetch_oci_image(oci_image_name, insecure_oci_registries)
        .await
        .map_err(|err| anyhow!(err).context("Could not fetch Wasm OCI image"))
}

pub fn setup_engine() -> anyhow::Result<Engine> {
    let mut config = wasmtime::Config::new();
    config.async_support(true);
    Engine::new(&config)
}
