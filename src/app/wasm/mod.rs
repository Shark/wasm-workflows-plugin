use std::path::PathBuf;
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder};
use wasmtime::{Engine, Linker, Module, Store};
use crate::app::model::{ExecuteTemplateResult, Template, Workflow};

// https://github.com/bytecodealliance/wit-bindgen/pull/126
wit_bindgen_wasmtime::import!({paths: ["./src/app/wasm/workflow.wit"], async: ["invoke"]});

pub async fn run(module_path: PathBuf, _template: &Template, workflow: &Workflow) -> anyhow::Result<ExecuteTemplateResult> {
    let (mut store, wf) = setup(module_path).await?;
    let invocation = workflow::Invocation {
        workflowname: &workflow.metadata.name,
    };
    let node = wf.invoke(&mut store, invocation).await?;
    Ok(ExecuteTemplateResult {
        phase: node.phase,
        message: node.message,
    })
}

async fn setup(module_path: PathBuf) -> anyhow::Result<(
    Store<(WasiCtx, workflow::WorkflowData)>,
    workflow::Workflow<(WasiCtx, workflow::WorkflowData)>
)> {
    // Setup wasmtime runtime
    let mut config = wasmtime::Config::new();
    let config = config.async_support(true);
    let engine = Engine::new(config)?;
    let module = Module::from_file(&engine, module_path)?;
    let mut linker = Linker::new(&engine);
    let _ = wasmtime_wasi::add_to_linker(&mut linker, |(wasi, _plugin_data)| wasi)?;
    let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args()?.build();
    let mut store = Store::new(&engine, (wasi, workflow::WorkflowData {}));

    let (wf, _instance) = workflow::Workflow::instantiate(&mut store, &module, &mut linker, |(_wasi, plugin_data)| {
        plugin_data
    }).await?;

    Ok((store, wf))
}
