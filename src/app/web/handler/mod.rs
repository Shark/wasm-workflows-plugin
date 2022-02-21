use axum::extract::Extension;
use axum::http::{StatusCode};
use axum::Json;
use axum::response::{IntoResponse, Response};
use tracing::{debug, error};
use serde_json::json;
use crate::app::model::{ExecuteTemplateRequest, ExecuteTemplateResponse};
use crate::app::config::DynConfigProvider;

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

    match crate::app::wasm::run(&image, &request.workflow.metadata.name, insecure_oci_registries).await {
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
