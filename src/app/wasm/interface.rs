use crate::app::model::{
    ExecuteTemplateResult, ModulePermissions, Outputs, Parameter, Phase, PluginInvocation,
};
use anyhow::anyhow;
use async_trait::async_trait;
use std::io::{BufReader, Cursor};
use tracing::debug;
use wasi_common::pipe::WritePipe;
use wasi_experimental_http_wasmtime::{HttpCtx, HttpState};
use wasmtime::{Engine, Linker, Module, Store, TypedFunc};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder};
use workflow::{Workflow, WorkflowData};

// https://github.com/bytecodealliance/wit-bindgen/pull/126
wit_bindgen_wasmtime::import!({paths: ["./src/app/wasm/workflow.wit"], async: ["invoke"]});

#[async_trait]
pub trait WorkflowPlugin {
    async fn run(&mut self, invocation: PluginInvocation) -> anyhow::Result<ExecuteTemplateResult>;
}

pub struct WITModule {
    store: Store<ModuleCtx>,
    workflow: workflow::Workflow<ModuleCtx>,
}

struct ModuleCtx {
    pub wasi: WasiCtx,
    pub http: HttpCtx,
    pub plugin_data: WorkflowData,
}

impl WITModule {
    pub async fn try_new(
        engine: &Engine,
        module: &Module,
        perms: &Option<ModulePermissions>,
    ) -> anyhow::Result<Self> {
        let mut http_allowed_hosts: Option<Vec<String>> = None;
        let mut http_max_concurrent_requests: Option<u32> = None;
        if let Some(the_perms) = perms {
            if let Some(http_perms) = &the_perms.http {
                http_allowed_hosts = Some(http_perms.allowed_hosts.to_owned());
                http_max_concurrent_requests = Some(http_perms.max_concurrent_requests)
            }
        }
        debug!(
            ?http_allowed_hosts,
            ?http_max_concurrent_requests,
            "WASI HTTP Settings"
        );

        let mut linker = Linker::new(engine);
        let _ = wasmtime_wasi::add_to_linker(&mut linker, |ctx: &mut ModuleCtx| -> &mut WasiCtx {
            &mut ctx.wasi
        })?;
        let wasi = WasiCtxBuilder::new().build();
        let mut store = Store::new(
            engine,
            ModuleCtx {
                wasi,
                http: HttpCtx {
                    allowed_hosts: http_allowed_hosts,
                    max_concurrent_requests: http_max_concurrent_requests,
                },
                plugin_data: WorkflowData {},
            },
        );

        let http = HttpState::new()?;
        http.add_to_linker(&mut linker, |ctx: &ModuleCtx| -> &HttpCtx { &ctx.http })?;

        let (wf, _instance) = Workflow::instantiate(
            &mut store,
            module,
            &mut linker,
            |ctx: &mut ModuleCtx| -> &mut WorkflowData { &mut ctx.plugin_data },
        )
        .await
        .map_err(|err| {
            anyhow!(err).context("Instantiating Wasm module with Workflow interface type failed")
        })?;

        Ok(Self::new(store, wf))
    }

    fn new(store: Store<ModuleCtx>, workflow: workflow::Workflow<ModuleCtx>) -> Self {
        WITModule { store, workflow }
    }
}

#[async_trait]
impl WorkflowPlugin for WITModule {
    async fn run(&mut self, invocation: PluginInvocation) -> anyhow::Result<ExecuteTemplateResult> {
        let serialized_invocation: SerializedInvocation = invocation.into();
        let stored_wasm_invocation: StoredWasmInvocation = (&serialized_invocation).into();
        let node = self
            .workflow
            .invoke(
                &mut self.store,
                workflow::Invocation {
                    workflow_name: stored_wasm_invocation.workflow_name,
                    plugin_options: &stored_wasm_invocation.plugin_options,
                    parameters: &stored_wasm_invocation.parameters,
                },
            )
            .await
            .map_err(|err| anyhow!(err).context("Invoking module failed"))?;
        let outputs: Option<Outputs> = match node.parameters.len() {
            0 => None,
            _ => {
                let out_params = node
                    .parameters
                    .into_iter()
                    .map(|param| -> anyhow::Result<Parameter> {
                        let parsed_value_json =
                            serde_json::from_str(&param.value_json).map_err(|err| {
                                anyhow::Error::new(err).context(format!(
                                    "Failed to parse output parameter \"{:?}\"",
                                    param
                                ))
                            })?;
                        Ok(Parameter {
                            name: param.name,
                            value: parsed_value_json,
                        })
                    })
                    .collect::<anyhow::Result<Vec<_>>>()
                    .map_err(|err| anyhow!(err).context("Error processing Wasm result node"))?;

                Some(Outputs {
                    artifacts: None,
                    parameters: Some(out_params),
                })
            }
        };
        let phase = match node.phase.parse::<Phase>() {
            Ok(phase) => phase,
            Err(_) => {
                return Err(anyhow::Error::msg(format!(
                    "Unable to parse \"{}\" as phase in result",
                    node.phase
                )))
            }
        };

        Ok(ExecuteTemplateResult {
            phase,
            message: node.message,
            outputs,
        })
    }
}

pub struct WASIModule {
    store: Option<Store<WasiCtx>>,
    workflow: TypedFunc<(), ()>,
}

impl WASIModule {
    pub async fn try_new(engine: &Engine, module: &Module) -> anyhow::Result<Self> {
        let mut linker = Linker::new(engine);
        let _ = wasmtime_wasi::add_to_linker(&mut linker, |s| s)?;
        let wasi = WasiCtxBuilder::new().build();
        let mut store = Store::new(engine, wasi);
        linker.module_async(&mut store, "", module).await?;
        let workflow = linker
            .get_default(&mut store, "")?
            .typed::<(), (), _>(&store)?;

        Ok(Self::new(store, workflow))
    }

    pub fn new(store: Store<WasiCtx>, workflow: TypedFunc<(), ()>) -> Self {
        WASIModule {
            store: Some(store),
            workflow,
        }
    }
}

#[async_trait]
impl WorkflowPlugin for WASIModule {
    async fn run(&mut self, invocation: PluginInvocation) -> anyhow::Result<ExecuteTemplateResult> {
        let invocation_json = serde_json::to_string(&invocation)?.as_bytes().to_owned();
        let invocation_read_pipe =
            wasi_common::pipe::ReadPipe::new(BufReader::new(Cursor::new(invocation_json)));

        let store = self.store.as_mut().expect("store is present");
        store.data_mut().set_stdin(Box::new(invocation_read_pipe));

        let stdout = WritePipe::new_in_memory();
        store.data_mut().set_stdout(Box::new(stdout.clone()));

        match self.workflow.call_async(store, ()).await {
            Ok(_) => {
                // Store must be dropped so that we can read from stdout
                self.store = None;
                let stdout_u8: Vec<u8> = stdout
                    .try_into_inner()
                    .expect("sole remaining reference to WritePipe")
                    .into_inner();
                let stdout = String::from_utf8(stdout_u8)
                    .map_err(|err| anyhow!(err).context("Parsing stdout as UTF-8"))?;
                debug!(?stdout, "Stdout");
                match serde_json::from_str(&stdout) {
                    Ok(result) => Ok(result),
                    Err(err) => {
                        debug!(?err, "Error parsing stdout as ExecuteTemplateResult");
                        return Ok(ExecuteTemplateResult {
                            phase: Phase::Succeeded,
                            message: stdout,
                            outputs: None,
                        });
                    }
                }
            }
            Err(e) => {
                self.store = None;
                let stdout_u8: Vec<u8> = stdout
                    .try_into_inner()
                    .expect("sole remaining reference to WritePipe")
                    .into_inner();
                let stdout = String::from_utf8(stdout_u8)
                    .map_err(|err| anyhow!(err).context("Parsing stdout as UTF-8"))?;
                debug!(?stdout, "Stdout");
                Ok(ExecuteTemplateResult {
                    phase: Phase::Failed,
                    message: e.to_string(),
                    outputs: None,
                })
            }
        }
    }
}

struct SerializedParameter {
    name: String,
    value_json: String,
}

impl<'a> From<Parameter> for SerializedParameter {
    fn from(parameter: Parameter) -> Self {
        SerializedParameter {
            name: parameter.name,
            value_json: parameter.value.to_string(),
        }
    }
}

struct SerializedInvocation {
    workflow_name: String,
    serialized_plugin_options: Vec<SerializedParameter>,
    serialized_parameters: Vec<SerializedParameter>,
}

impl<'a> From<PluginInvocation> for SerializedInvocation {
    fn from(invocation: PluginInvocation) -> Self {
        let serialized_plugin_options = invocation
            .plugin_options
            .into_iter()
            .map(From::from)
            .collect::<Vec<SerializedParameter>>();
        let serialized_parameters = invocation
            .parameters
            .into_iter()
            .map(From::from)
            .collect::<Vec<SerializedParameter>>();
        SerializedInvocation {
            workflow_name: invocation.workflow_name,
            serialized_plugin_options,
            serialized_parameters,
        }
    }
}

impl<'a> From<&'a SerializedParameter> for workflow::ParameterParam<'a> {
    fn from(serialized: &'a SerializedParameter) -> Self {
        workflow::ParameterParam {
            name: &serialized.name,
            value_json: &serialized.value_json,
        }
    }
}

struct StoredWasmInvocation<'a> {
    workflow_name: &'a str,
    plugin_options: Vec<workflow::ParameterParam<'a>>,
    parameters: Vec<workflow::ParameterParam<'a>>,
}

impl<'a> From<&'a SerializedInvocation> for StoredWasmInvocation<'a> {
    fn from(serialized: &'a SerializedInvocation) -> Self {
        StoredWasmInvocation {
            workflow_name: &serialized.workflow_name,
            plugin_options: serialized
                .serialized_plugin_options
                .iter()
                .map(From::from)
                .collect(),
            parameters: serialized
                .serialized_parameters
                .iter()
                .map(From::from)
                .collect(),
        }
    }
}
