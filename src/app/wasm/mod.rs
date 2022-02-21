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
) -> anyhow::Result<ExecuteTemplateResult> {
    let (mut store, wf) = setup(oci_image, insecure_oci_registries).await?;
    let node = wf.invoke(&mut store, invocation).await?;
    let mut outputs: Option<Outputs> = None;
    if node.parameters.len() > 0 {
        let mut out_params: Vec<Parameter> = Vec::new();
        for param in node.parameters.into_iter() {
            let parsed_value_json = serde_json::from_str(&param.value_json).map_err(|err| {
                anyhow::Error::new(err).context(format!("Failed to parse Wasm output parameter \"{:?}\"", param))
            })?;
            out_params.push(Parameter {
                name: param.name,
                value: parsed_value_json,
            })
        }
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
