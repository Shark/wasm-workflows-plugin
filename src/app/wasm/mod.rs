use anyhow::{anyhow, Error};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder};
use wasmtime::{Engine, Linker, Module, Store};
use crate::app::model::{ExecuteTemplateResult, Outputs, Parameter};

// https://github.com/bytecodealliance/wit-bindgen/pull/126
wit_bindgen_wasmtime::import!({paths: ["./src/app/wasm/workflow.wit"], async: ["invoke"]});

mod image;
pub use workflow::Invocation;
pub use workflow::ParameterParam;

pub async fn run(
    oci_image: &str,
    invocation: workflow::Invocation<'_>,
    insecure_oci_registries: Vec<String>,
) -> anyhow::Result<ExecuteTemplateResult, WasmError> {
    let module = pull(oci_image, insecure_oci_registries).await.map_err(
        |err| WasmError::Retrieve(anyhow!(err).context("Wasm module retrieve failed"))
    )?;
    let (mut store, wf) = setup(module).await.map_err(
        |err| WasmError::EnvironmentSetup(anyhow!(err).context("Wasm module invocation failed"))
    )?;
    let node = wf.invoke(&mut store, invocation).await.map_err(
        |err| WasmError::Invocation(anyhow!(err).context("Wasm module invocation failed"))
    )?;
    let mut outputs: Option<Outputs> = None;
    if node.parameters.len() > 0 {
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

        outputs = Some(Outputs {
            artifacts: None,
            parameters: Some(out_params),
        });
    }
    Ok(ExecuteTemplateResult {
        phase: node.phase,
        message: node.message,
        outputs,
    })
}

async fn pull(oci_image_name: &str, insecure_oci_registries: Vec<String>) -> anyhow::Result<Vec<u8>> {
    // Pull module image, put into Vec<u8>
    image::fetch_oci_image(oci_image_name,insecure_oci_registries)
        .await
        .map_err(|err| anyhow!(err).context("Could not fetch Wasm OCI image"))
}

async fn setup(module: Vec<u8>) -> anyhow::Result<(
    Store<(WasiCtx, workflow::WorkflowData)>,
    workflow::Workflow<(WasiCtx, workflow::WorkflowData)>
)> {
    // Setup wasmtime runtime
    let mut config = wasmtime::Config::new();
    let config = config.async_support(true);
    let engine = Engine::new(config)?;
    let module = Module::from_binary(&engine, &module)?;
    let mut linker = Linker::new(&engine);
    let _ = wasmtime_wasi::add_to_linker(&mut linker, |(wasi, _plugin_data)| wasi)?;
    let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args()?.build();
    let mut store = Store::new(&engine, (wasi, workflow::WorkflowData {}));

    let (wf, _instance) = workflow::Workflow::instantiate(&mut store, &module, &mut linker, |(_wasi, plugin_data)| {
        plugin_data
    }).await.map_err(|err| anyhow!(err).context("Instantiating Wasm module with Workflow interface type failed"))?;

    Ok((store, wf))
}

#[derive(Debug)]
pub enum WasmError {
    EnvironmentSetup(Error),
    Retrieve(Error),
    Invocation(Error),
    OutputProcessing(Error),
}
