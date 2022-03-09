use anyhow::{anyhow, Error};
use tracing::info_span;
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder};
use wasmtime::{Engine, Linker, Module, Store};
pub use workflow::{Invocation, ParameterParam};
use crate::app::model::{ExecuteTemplateResult, Outputs, Parameter};
use crate::app::wasm::cache::ModuleCache;
// https://github.com/bytecodealliance/wit-bindgen/pull/126
wit_bindgen_wasmtime::import!({paths: ["./src/app/wasm/workflow.wit"], async: ["invoke"]});

mod image;
pub mod cache;

#[tracing::instrument(name = "wasm.run", skip(engine, cache))]
pub async fn run(
    engine: &wasmtime::Engine,
    cache: &'_ (dyn ModuleCache + Send + Sync),
    oci_image: &str,
    invocation: workflow::Invocation<'_>,
    insecure_oci_registries: Vec<String>,
) -> anyhow::Result<ExecuteTemplateResult, WasmError> {
    let mut module: Option<Vec<u8>> = cache.get(oci_image).map_err(
        |err| WasmError::EnvironmentSetup(anyhow!(err).context("Checking Wasm module cache failed"))
    )?;
    if module.is_none() {
        let pulled_mod = pull(oci_image, insecure_oci_registries).await.map_err(
            |err| WasmError::Retrieve(anyhow!(err).context("Wasm module retrieve failed"))
        )?;
        let precompiled_mod = tracing::trace_span!("engine.precompile_module").in_scope(|| {
            engine.precompile_module(&pulled_mod).map_err(
                |err| WasmError::Precompile(anyhow!(err).context("Wasm module precompilation failed"))
            )
        })?;
        let _ = cache.put(oci_image, &precompiled_mod).map_err(
            |err| WasmError::Retrieve(anyhow!(err).context("Storing Wasm module in cache failed"))
        )?;
        module = Some(precompiled_mod);
    }

    let (mut store, wf) = setup(engine, module.unwrap()).await.map_err(
        |err| WasmError::EnvironmentSetup(anyhow!(err).context("Wasm module invocation failed"))
    )?;
    let node = info_span!("wasm.execute_mod")
        .in_scope(|| wf.invoke(&mut store, invocation))
        .await
        .map_err(
            |err| WasmError::Invocation(anyhow!(err).context("Wasm module invocation failed"))
        )?;
    let outputs: Option<Outputs> = match node.parameters.len() {
        0 => None,
        _ => {
            let out_params = node.parameters.into_iter().map(|param| -> anyhow::Result<Parameter> {
                let parsed_value_json = serde_json::from_str(&param.value_json).map_err(
                    |err| anyhow::Error::new(err).context(format!("Failed to parse output parameter \"{:?}\"", param))
                )?;
                Ok(Parameter {
                    name: param.name,
                    value: parsed_value_json,
                })
            }).collect::<anyhow::Result<Vec<_>>>().map_err(
                |err| WasmError::OutputProcessing(anyhow!(err).context("Error processing Wasm result node"))
            )?;

            Some(Outputs {
                artifacts: None,
                parameters: Some(out_params),
            })
        }
    };

    Ok(ExecuteTemplateResult {
        phase: node.phase,
        message: node.message,
        outputs,
    })
}

#[tracing::instrument(name = "wasm.oci_pull", skip(insecure_oci_registries))]
async fn pull(oci_image_name: &str, insecure_oci_registries: Vec<String>) -> anyhow::Result<Vec<u8>> {
    // Pull module image, put into Vec<u8>
    image::fetch_oci_image(oci_image_name,insecure_oci_registries)
        .await
        .map_err(|err| anyhow!(err).context("Could not fetch Wasm OCI image"))
}

pub fn setup_engine() -> anyhow::Result<wasmtime::Engine> {
    let mut config = wasmtime::Config::new();
    let config = config.async_support(true);
    Engine::new(config)
}

#[tracing::instrument(name = "wasm.setup", skip(engine, module))]
async fn setup(engine: &wasmtime::Engine,
               module: Vec<u8>) -> anyhow::Result<(
    Store<(WasiCtx, workflow::WorkflowData)>,
    workflow::Workflow<(WasiCtx, workflow::WorkflowData)>
)> {
    let module = unsafe { Module::deserialize(engine, module) }?;
    let mut linker = Linker::new(engine);
    let _ = wasmtime_wasi::add_to_linker(&mut linker, |(wasi, _plugin_data)| wasi)?;
    // TODO Remove stdio & args
    let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args()?.build();
    let mut store = Store::new(engine, (wasi, workflow::WorkflowData {}));

    let (wf, _instance) = workflow::Workflow::instantiate(&mut store, &module, &mut linker, |(_wasi, plugin_data)| {
        plugin_data
    }).await.map_err(|err| anyhow!(err).context("Instantiating Wasm module with Workflow interface type failed"))?;

    Ok((store, wf))
}

#[derive(Debug)]
pub enum WasmError {
    EnvironmentSetup(Error),
    Retrieve(Error),
    Precompile(Error),
    Invocation(Error),
    OutputProcessing(Error),
}
