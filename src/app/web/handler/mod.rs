use axum::http::StatusCode;
use axum::Json;
use axum::response::IntoResponse;
use tracing::debug;

mod model;

pub async fn execute_template(Json(payload): Json<model::ExecuteTemplatePayload>) -> impl IntoResponse {
    debug!("Payload: {:?}", payload);

    (StatusCode::OK, "Got it")
}
