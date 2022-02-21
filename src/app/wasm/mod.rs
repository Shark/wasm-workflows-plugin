use wasmtime_wasi::{WasiCtx, WasiCtxBuilder};
use wasmtime::{Engine, Linker, Module, Store};
use crate::app::model::ExecuteTemplateResult;

// https://github.com/bytecodealliance/wit-bindgen/pull/126
wit_bindgen_wasmtime::import!({paths: ["./src/app/wasm/workflow.wit"], async: ["invoke"]});

mod image;

pub async fn run(
    oci_image: &str,
    workflow_name: &str,
    insecure_oci_registries: Vec<String>,
) -> anyhow::Result<ExecuteTemplateResult> {
    let (mut store, wf) = setup(oci_image, insecure_oci_registries).await?;
    let invocation = workflow::Invocation {
        workflowname: workflow_name,
    };
    let node = wf.invoke(&mut store, invocation).await?;
    Ok(ExecuteTemplateResult {
        phase: node.phase,
        message: node.message,
    })
}

async fn setup(oci_image_name: &str, insecure_oci_registries: Vec<String>) -> anyhow::Result<(
    Store<(WasiCtx, workflow::WorkflowData)>,
    workflow::Workflow<(WasiCtx, workflow::WorkflowData)>
)> {
    // Pull module image, put into Vec<u8>
    let module = image::fetch_oci_image(oci_image_name, insecure_oci_registries).await?;

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
    }).await?;

    Ok((store, wf))
}
