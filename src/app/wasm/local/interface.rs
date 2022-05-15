use crate::app::model::ModulePermissions;
use anyhow::anyhow;
use std::io::Cursor;
use tracing::debug;
use wasi_common::pipe::WritePipe;
use wasi_experimental_http_wasmtime::{HttpCtx, HttpState};
use wasmtime::{Engine, Linker, Module, Store, TypedFunc};
use wasmtime_wasi::{WasiCtx, WasiCtxBuilder};
use workflow_model::host::WorkingDir;
use workflow_model::model::{Outputs, Phase, PluginInvocation, PluginResult};

pub trait WorkflowPlugin {
    fn run(&mut self, invocation: PluginInvocation) -> anyhow::Result<PluginResult>;
}

struct ModuleCtx {
    pub wasi: WasiCtx,
    pub http: HttpCtx,
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

fn setup_module(
    engine: &Engine,
    perms: &Option<ModulePermissions>,
    working_dir: &WorkingDir,
) -> anyhow::Result<(Linker<ModuleCtx>, Store<ModuleCtx>)> {
    let mut linker = Linker::new(engine);
    let _ = wasmtime_wasi::add_to_linker(&mut linker, |ctx: &mut ModuleCtx| &mut ctx.wasi)?;
    let preopen_working_dir =
        cap_std::fs::Dir::open_ambient_dir(working_dir.path(), cap_std::ambient_authority())?;
    let mut wasi = WasiCtxBuilder::new()
        .preopened_dir(
            preopen_working_dir,
            workflow_model::model::WORKING_DIR_PLUGIN_PATH,
        )?
        .build();
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

pub struct WASIModule {
    working_dir: WorkingDir,
    store: Option<Store<ModuleCtx>>,
    workflow: TypedFunc<(), ()>,
}

impl WASIModule {
    pub fn try_new(
        engine: &Engine,
        module: &Module,
        perms: &Option<ModulePermissions>,
    ) -> anyhow::Result<Self> {
        let working_dir = WorkingDir::try_new()?;
        let (mut linker, mut store) = setup_module(engine, perms, &working_dir)?;
        linker.module(&mut store, "", module)?;
        let workflow = linker
            .get_default(&mut store, "")?
            .typed::<(), (), _>(&store)?;

        Ok(Self::new(working_dir, store, workflow))
    }

    fn new(working_dir: WorkingDir, store: Store<ModuleCtx>, workflow: TypedFunc<(), ()>) -> Self {
        WASIModule {
            working_dir,
            workflow,
            store: Some(store),
        }
    }
}

impl WorkflowPlugin for WASIModule {
    fn run(&mut self, invocation: PluginInvocation) -> anyhow::Result<PluginResult> {
        self.working_dir.set_input(&invocation)?;
        let mut store = self.store.as_mut().expect("present store");
        let (stdout, stderr) = prepare_sys_output(&mut store.data_mut().wasi);

        match self.workflow.call(&mut store, ()) {
            Ok(_) => {
                self.store = None;
                let (stdout, stderr) = retrieve_sys_output(stdout, stderr)?;
                debug!(?stdout, ?stderr, "Module Output");
                self.working_dir.result()
            }
            Err(e) => {
                self.store = None;
                let (stdout, stderr) = retrieve_sys_output(stdout, stderr)?;
                debug!(?stdout, ?stderr, "Module Output");
                Ok(PluginResult {
                    phase: Phase::Failed,
                    message: e.to_string(),
                    outputs: Outputs::default(),
                })
            }
        }
    }
}
