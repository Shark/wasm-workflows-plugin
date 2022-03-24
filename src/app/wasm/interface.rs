use crate::app::model::{
    ExecuteTemplateResult, ModulePermissions, Outputs, Parameter, Phase, PluginInvocation,
};
use anyhow::anyhow;
use std::io::{BufReader, Cursor};
use tracing::debug;
use wasi_common::pipe::{ReadPipe, WritePipe};
use wasi_experimental_http_wasmtime::{HttpCtx, HttpState};
use wasmtime::{Engine, Linker, Module, Store, TypedFunc};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder};
use workflow::{Workflow, WorkflowData};

// https://github.com/bytecodealliance/wit-bindgen/pull/126
wit_bindgen_wasmtime::import!({paths: ["./src/app/wasm/workflow.wit"]});

pub trait WorkflowPlugin {
    fn run(&mut self, invocation: PluginInvocation) -> anyhow::Result<ExecuteTemplateResult>;
}

pub struct WITModule {
    store: Option<Store<ModuleCtx>>,
    workflow: workflow::Workflow<ModuleCtx>,
}

struct ModuleCtx {
    pub wasi: WasiCtx,
    pub http: HttpCtx,
    pub plugin_data: WorkflowData,
}

impl ModuleCtx {
    fn set_stdin(&mut self, content: Vec<u8>) {
        let stdin = ReadPipe::new(BufReader::new(Cursor::new(content)));
        self.wasi.set_stdin(Box::new(stdin));
    }
}

fn http_ctx_from_perms(perms: &Option<ModulePermissions>) -> HttpCtx {
    let mut http_allowed_hosts: Option<Vec<String>> = None;
    let mut http_max_concurrent_requests: Option<u32> = None;
    if let Some(the_perms) = perms {
        if let Some(http_perms) = &the_perms.http {
            http_allowed_hosts = Some(http_perms.allowed_hosts.to_owned());
            http_max_concurrent_requests = Some(http_perms.max_concurrent_requests)
        }
    }
    HttpCtx {
        allowed_hosts: http_allowed_hosts,
        max_concurrent_requests: http_max_concurrent_requests,
    }
}

fn common_setup_module(
    engine: &Engine,
    perms: &Option<ModulePermissions>,
) -> anyhow::Result<(Linker<ModuleCtx>, Store<ModuleCtx>)> {
    let mut linker = Linker::new(engine);
    let _ = wasmtime_wasi::add_to_linker(&mut linker, |ctx: &mut ModuleCtx| &mut ctx.wasi)?;
    let mut wasi = WasiCtxBuilder::new().build();
    let stdout = WritePipe::new_in_memory();
    let stderr = WritePipe::new_in_memory();
    wasi.set_stdout(Box::new(stdout));
    wasi.set_stderr(Box::new(stderr));
    let http_ctx = http_ctx_from_perms(perms);
    debug!(
        allowed_hosts = ?http_ctx.allowed_hosts,
        max_concurrent_requests = ?http_ctx.max_concurrent_requests,
        "WASI HTTP Settings"
    );
    let http = HttpState::new()?;
    http.add_to_linker(&mut linker, |ctx: &ModuleCtx| -> &HttpCtx { &ctx.http })?;
    let store = Store::new(
        engine,
        ModuleCtx {
            wasi,
            http: http_ctx,
            plugin_data: WorkflowData {},
        },
    );
    Ok((linker, store))
}

type SysOutput = (WritePipe<Cursor<Vec<u8>>>, WritePipe<Cursor<Vec<u8>>>);

fn prepare_sys_output(wasi: &mut WasiCtx) -> SysOutput {
    let stdout = WritePipe::new_in_memory();
    let stderr = WritePipe::new_in_memory();
    wasi.set_stdout(Box::new(stdout.clone()));
    wasi.set_stderr(Box::new(stderr.clone()));
    (stdout, stderr)
}

fn retrieve_sys_output(
    stdout: WritePipe<Cursor<Vec<u8>>>,
    stderr: WritePipe<Cursor<Vec<u8>>>,
) -> anyhow::Result<(String, String)> {
    let stdout: Vec<u8> = stdout
        .try_into_inner()
        .expect("sole remaining reference to WritePipe")
        .into_inner();
    let stdout =
        String::from_utf8(stdout).map_err(|err| anyhow!(err).context("Parsing stdout as UTF-8"))?;
    let stderr: Vec<u8> = stderr
        .try_into_inner()
        .expect("sole remaining reference to WritePipe")
        .into_inner();
    let stderr =
        String::from_utf8(stderr).map_err(|err| anyhow!(err).context("Parsing stderr as UTF-8"))?;
    Ok((stdout, stderr))
}

impl WITModule {
    pub fn try_new(
        engine: &Engine,
        module: &Module,
        perms: &Option<ModulePermissions>,
    ) -> anyhow::Result<Self> {
        let (mut linker, mut store) = common_setup_module(engine, perms)?;

        let (wf, _instance) = Workflow::instantiate(
            &mut store,
            module,
            &mut linker,
            |ctx: &mut ModuleCtx| -> &mut WorkflowData { &mut ctx.plugin_data },
        )
        .map_err(|err| {
            anyhow!(err).context("Instantiating Wasm module with Workflow interface type failed")
        })?;

        Ok(Self::new(store, wf))
    }

    fn new(store: Store<ModuleCtx>, workflow: workflow::Workflow<ModuleCtx>) -> Self {
        WITModule {
            store: Some(store),
            workflow,
        }
    }
}

impl WorkflowPlugin for WITModule {
    fn run(&mut self, invocation: PluginInvocation) -> anyhow::Result<ExecuteTemplateResult> {
        let serialized_invocation: SerializedInvocation = invocation.into();
        let stored_wasm_invocation: StoredWasmInvocation = (&serialized_invocation).into();
        let store = self.store.as_mut().expect("present store");
        let (stdout, stderr) = prepare_sys_output(&mut store.data_mut().wasi);
        let node = self
            .workflow
            .invoke(
                store,
                workflow::Invocation {
                    workflow_name: stored_wasm_invocation.workflow_name,
                    plugin_options: &stored_wasm_invocation.plugin_options,
                    parameters: &stored_wasm_invocation.parameters,
                },
            )
            .map_err(|err| anyhow!(err).context("Invoking module failed"))?;
        self.store = None;
        let (stdout, stderr) = retrieve_sys_output(stdout, stderr)?;
        debug!(?stdout, ?stderr, "Module Output");
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
    store: Option<Store<ModuleCtx>>,
    workflow: TypedFunc<(), ()>,
}

impl WASIModule {
    pub fn try_new(
        engine: &Engine,
        module: &Module,
        perms: &Option<ModulePermissions>,
    ) -> anyhow::Result<Self> {
        let (mut linker, mut store) = common_setup_module(engine, perms)?;
        linker.module(&mut store, "", module)?;
        let workflow = linker
            .get_default(&mut store, "")?
            .typed::<(), (), _>(&store)?;

        Ok(Self::new(store, workflow))
    }

    fn new(store: Store<ModuleCtx>, workflow: TypedFunc<(), ()>) -> Self {
        WASIModule {
            store: Some(store),
            workflow,
        }
    }
}

impl WorkflowPlugin for WASIModule {
    fn run(&mut self, invocation: PluginInvocation) -> anyhow::Result<ExecuteTemplateResult> {
        let invocation = serde_json::to_string(&invocation)?.as_bytes().to_owned();
        let mut store = self.store.as_mut().expect("present store");
        store.data_mut().set_stdin(invocation);
        let (stdout, stderr) = prepare_sys_output(&mut store.data_mut().wasi);

        match self.workflow.call(&mut store, ()) {
            Ok(_) => {
                self.store = None;
                let (stdout, stderr) = retrieve_sys_output(stdout, stderr)?;
                debug!(?stdout, ?stderr, "Module Output");
                match serde_json::from_str(&stdout) {
                    Ok(result) => Ok(result),
                    Err(err) => {
                        debug!(?err, "Error parsing stdout as ExecuteTemplateResult");
                        Ok(ExecuteTemplateResult {
                            phase: Phase::Succeeded,
                            message: stdout,
                            outputs: None,
                        })
                    }
                }
            }
            Err(e) => {
                self.store = None;
                let (stdout, stderr) = retrieve_sys_output(stdout, stderr)?;
                debug!(?stdout, ?stderr, "Module Output");
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
