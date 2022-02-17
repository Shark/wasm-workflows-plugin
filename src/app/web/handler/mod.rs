use std::path::PathBuf;
use axum::http::{StatusCode};
use axum::Json;
use axum::response::{IntoResponse, Response};
use tracing::{debug, error};
use serde_json::json;
use crate::app::model::{ExecuteTemplatePayload, ExecuteTemplateResult};

pub async fn execute_template(Json(payload): Json<ExecuteTemplatePayload>) -> Result<Json<ExecuteTemplateResult>, AppError> {
    debug!("Payload: {:?}", payload);

    match crate::app::wasm::run(PathBuf::from("demo_mod.wasm"), &payload.template, &payload.workflow).await {
        Ok(result) => Ok(result.into()),
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
