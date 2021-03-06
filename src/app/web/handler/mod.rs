use crate::app::dependencies::DynDependencyProvider;
use crate::app::model::ModuleSource::OCI;
use crate::app::model::{ExecuteTemplateRequest, ExecuteTemplateResponse, ExecuteTemplateResult};
use crate::app::wasm::WasmError;
use axum::extract::Extension;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use axum_macros::debug_handler;
use tracing::{debug, error, Instrument};
use workflow_model::model::{ArtifactRef, Parameter, Phase, PluginInvocation};

#[debug_handler]
#[tracing::instrument(name = "request.execute_template", fields(workflow_name=&request.workflow.metadata.name.as_str()), skip(deps))]
pub async fn execute_template(
    Json(request): Json<ExecuteTemplateRequest>,
    Extension(deps): Extension<DynDependencyProvider>,
) -> Result<Json<ExecuteTemplateResponse>, AppError> {
    debug!("Request: {:?}", request);

    let (module_source, permissions, plugin_options_map) = match request.template.plugin.wasm {
        Some(config) => (config.module, config.permissions, config.extra),
        None => return Ok(ExecuteTemplateResponse { node: None }.into()),
    };

    let plugin_options: Vec<Parameter> = plugin_options_map
        .into_iter()
        .map(|(name, value)| Parameter { name, value })
        .collect();

    let OCI(image) = module_source;

    let mut in_params: Vec<Parameter> = Vec::new();
    if let Some(params) = request.template.inputs.parameters {
        in_params = params;
    }

    let mut in_artifacts: Vec<ArtifactRef> = Vec::new();
    if let Some(artifacts) = request.template.inputs.artifacts {
        in_artifacts = artifacts;
    }

    let invocation = PluginInvocation {
        workflow_name: request.workflow.metadata.name,
        parameters: in_params,
        artifacts: in_artifacts,
        plugin_options,
    };

    // Spawn the module runner in a new tokio thread
    // Note: Added this because without the thread, the program would block when using
    // wasi-experimental-http in the module. But only on the second module run. This probably had
    // something to do with reqwest and connection pooling, but the thread resolved the problem
    // as well and was an easier solution.
    // TODO as this changed from spawn_blocking to spawn, this might be a problem again!
    let span = tracing::info_span!("wasm");
    let result = tokio::task::spawn(
        async move {
            let runner = deps.get_runner();
            let artifact_repo_config = deps.get_artifact_repository_config();
            runner
                .run(&image, invocation, &permissions, artifact_repo_config)
                .await
        }
        .instrument(span),
    )
    .await
    .expect("able to join runner task");

    match result {
        Ok(result) => {
            let result = ExecuteTemplateResult::from_plugin_result(result);
            let response = ExecuteTemplateResponse { node: Some(result) };
            debug!(?response, "Send Response");
            Ok(response.into())
        }
        Err(err) => {
            error!(?err, "Send Error");
            Err(err.into())
        }
    }
}

pub enum AppError {
    ModuleExecution(WasmError),
}

impl From<WasmError> for AppError {
    fn from(inner: WasmError) -> Self {
        AppError::ModuleExecution(inner)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::ModuleExecution(WasmError::EnvironmentSetup(_err)) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Wasm environment is not set up correctly",
            ),
            AppError::ModuleExecution(WasmError::Invocation(_err)) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Wasm module invocation failed",
            ),
            AppError::ModuleExecution(WasmError::OutputProcessing(_err)) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Wasm module output processing failed",
            ),
            AppError::ModuleExecution(WasmError::Retrieve(_err)) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Wasm module could not be retrieved",
            ),
            AppError::ModuleExecution(WasmError::Precompile(_)) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Wasm module could not be precompiled",
            ),
            AppError::ModuleExecution(WasmError::Timeout(_)) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Wasm module did not report result in time (timeout)",
            ),
        };

        let response = Json(ExecuteTemplateResponse {
            node: Some(ExecuteTemplateResult {
                phase: Phase::Failed,
                message: error_message.to_string(),
                outputs: None,
            }),
        });

        (status, response).into_response()
    }
}
