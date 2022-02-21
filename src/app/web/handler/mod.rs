use axum::extract::Extension;
use axum::http::{StatusCode};
use axum::Json;
use axum::response::{IntoResponse, Response};
use tracing::{debug, error};
use crate::app::model::{ExecuteTemplateRequest, ExecuteTemplateResponse, ExecuteTemplateResult, Parameter, PHASE_FAILED};
use crate::app::config::DynConfigProvider;
use crate::app::wasm;
use crate::app::wasm::WasmError;

pub async fn execute_template(
    Json(request): Json<ExecuteTemplateRequest>,
    Extension(config_provider): Extension<DynConfigProvider>
) -> Result<Json<ExecuteTemplateResponse>, AppError> {
    debug!("Request: {:?}", request);
    let insecure_oci_registries = config_provider.get().insecure_oci_registries.clone();

    let image = match request.template.plugin.wasm {
        Some(config) => config.image,
        None => {
            return Ok(ExecuteTemplateResponse {
                node: None,
            }.into())
        }
    };

    let mut in_params: Vec<Parameter> = Vec::new();
    if let Some(params) = request.template.inputs.parameters {
        in_params = params;
    }

    // Some place to hold the JSON value representations
    let params: Vec<Param> = in_params.into_iter().map(
        |param| Param {
            name: param.name,
            value: param.value.to_string(),
        }
    ).collect();

    let out_params: Vec<wasm::workflow::ParameterParam> = params.iter().map(
        |param| wasm::workflow::ParameterParam {
            name: &param.name,
            value_json: &param.value,
        }
    ).collect();

    let invocation = wasm::workflow::Invocation {
        workflow_name: &request.workflow.metadata.name,
        parameters: &out_params,
        plugin_options: &Vec::new(), // TODO fill
    };

    match wasm::run(&image, invocation, insecure_oci_registries).await {
        Ok(result) => {
            let response = ExecuteTemplateResponse {
                node: Some(result),
            };
            debug!("Response: {:?}", response);
            Ok(response.into())
        },
        Err(err) => {
            error!("Error: {:?}", err);
            Err(err.into())
        },
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
            AppError::ModuleExecution(WasmError::EnvironmentSetup(_err)) =>
                (StatusCode::INTERNAL_SERVER_ERROR, "Wasm environment is not set up correctly"),
            AppError::ModuleExecution(WasmError::Invocation(_err)) =>
                (StatusCode::INTERNAL_SERVER_ERROR, "Wasm module invocation failed"),
            AppError::ModuleExecution(WasmError::OutputProcessing(_err)) =>
                (StatusCode::INTERNAL_SERVER_ERROR, "Wasm module output processing failed"),
            AppError::ModuleExecution(WasmError::Retrieve(_err)) =>
                (StatusCode::INTERNAL_SERVER_ERROR, "Wasm module could not be retrieved"),
        };

        let response = Json(ExecuteTemplateResponse {
            node: Some(ExecuteTemplateResult {
                phase: PHASE_FAILED.to_string(),
                message: error_message.to_string(),
                outputs: None
            }),
        });

        (status, response).into_response()
    }
}

struct Param {
    name: String,
    value: String,
}
