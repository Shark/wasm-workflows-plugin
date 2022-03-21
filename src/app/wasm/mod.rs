use crate::app::model::{ExecuteTemplateResult, ModulePermissions, PluginInvocation};
use crate::app::wasm::cache::ModuleCache;
use crate::app::wasm::interface::{WASIModule, WITModule, WorkflowPlugin};
use anyhow::{anyhow, Error};
pub use interface::workflow::{Invocation, ParameterParam};
use tracing::{debug, info_span};
use wasmtime::{Engine, Module};

pub mod cache;
mod image;
mod interface;

pub struct Runner<'a> {
    engine: &'a wasmtime::Engine,
    cache: &'a (dyn ModuleCache + Send + Sync),
    insecure_oci_registries: &'a [&'a str],
}

impl<'a> Runner<'a> {
    pub fn new(
        engine: &'a wasmtime::Engine,
        cache: &'a (dyn ModuleCache + Send + Sync),
        insecure_oci_registries: &'a [&'a str],
    ) -> Self {
        Runner {
            engine,
            cache,
            insecure_oci_registries,
        }
    }
}

impl<'a> Runner<'a> {
    #[tracing::instrument(name = "wasm.run", skip(self))]
    pub async fn run(
        &self,
        oci_image: &str,
        invocation: PluginInvocation,
        perms: &Option<ModulePermissions>,
    ) -> anyhow::Result<ExecuteTemplateResult, WasmError> {
        let mut module: Option<Vec<u8>> = self.cache.get(oci_image).map_err(|err| {
            WasmError::EnvironmentSetup(anyhow!(err).context("Checking Wasm module cache failed"))
        })?;
        if module.is_none() {
            let pulled_mod = pull(oci_image, self.insecure_oci_registries)
                .await
                .map_err(|err| {
                    WasmError::Retrieve(anyhow!(err).context("Wasm module retrieve failed"))
                })?;
            let precompiled_mod =
                tracing::trace_span!("engine.precompile_module").in_scope(|| {
                    self.engine.precompile_module(&pulled_mod).map_err(|err| {
                        WasmError::Precompile(
                            anyhow!(err).context("Wasm module precompilation failed"),
                        )
                    })
                })?;
            let _ = self.cache.put(oci_image, &precompiled_mod).map_err(|err| {
                WasmError::Retrieve(anyhow!(err).context("Storing Wasm module in cache failed"))
            })?;
            module = Some(precompiled_mod);
        }

        let module =
            unsafe { Module::deserialize(self.engine, module.unwrap()) }.map_err(|err| {
                WasmError::EnvironmentSetup(anyhow!(err).context("Deserializing module failed"))
            })?;

        // First try to instantiate the module as WIT and fall back to WASI in case of an error
        let mut plugin: Box<dyn WorkflowPlugin + Send> =
            match WITModule::try_new(self.engine, &module, perms)
                .await
                .map_err(|err| {
                    WasmError::EnvironmentSetup(anyhow!(err).context("Creating WIT module failed"))
                }) {
                Ok(wit) => Box::new(wit),
                Err(err) => {
                    debug!(?err, "Error instantiating module as WIT");
                    match WASIModule::try_new(self.engine, &module)
                        .await
                        .map_err(|err| {
                            WasmError::EnvironmentSetup(
                                anyhow!(err).context("Creating WASI module failed"),
                            )
                        }) {
                        Ok(wasi) => Box::new(wasi),
                        Err(e) => return Err(e),
                    }
                }
            };
        let result = info_span!("wasm.execute_mod")
            .in_scope(|| plugin.run(invocation))
            .await
            .map_err(|err| {
                WasmError::Invocation(anyhow!(err).context("Wasm module invocation failed"))
            })?;
        Ok(result)
    }
}

#[tracing::instrument(name = "wasm.oci_pull", skip(insecure_oci_registries))]
async fn pull<'a>(
    oci_image_name: &str,
    insecure_oci_registries: &'a [&'a str],
) -> anyhow::Result<Vec<u8>> {
    // Pull module image, put into Vec<u8>
    image::fetch_oci_image(oci_image_name, insecure_oci_registries)
        .await
        .map_err(|err| anyhow!(err).context("Could not fetch Wasm OCI image"))
}

pub fn setup_engine() -> anyhow::Result<wasmtime::Engine> {
    let mut config = wasmtime::Config::new();
    let config = config.async_support(true);
    Engine::new(config)
}

#[derive(Debug)]
pub enum WasmError {
    EnvironmentSetup(Error),
    Retrieve(Error),
    Precompile(Error),
    Invocation(Error),
    OutputProcessing(Error),
}
