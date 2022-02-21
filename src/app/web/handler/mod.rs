use axum::extract::Extension;
use axum::http::{StatusCode};
use axum::Json;
use axum::response::{IntoResponse, Response};
use tracing::{debug, error};
use serde_json::json;
use crate::app::model::{ExecuteTemplateRequest, ExecuteTemplateResponse, Parameter};
use crate::app::config::DynConfigProvider;
use crate::app::wasm;

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
            Err(ModuleExecutionError::Generic.into())
        },
    }
}

pub enum AppError {
    ModuleExecution(ModuleExecutionError),
}

impl From<ModuleExecutionError> for AppError {
    fn from(inner: ModuleExecutionError) -> Self {
        AppError::ModuleExecution(inner)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::ModuleExecution(ModuleExecutionError::Generic) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Module Execution Failed")
            }
        };

        let body = Json(json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

pub enum ModuleExecutionError {
    Generic
}

struct Param {
    name: String,
    value: String,
}
