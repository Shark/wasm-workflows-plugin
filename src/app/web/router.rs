use axum::http::{header, HeaderValue};
use axum::{AddExtensionLayer, Router};
use axum::routing::{get, post};
use tower_http::trace::TraceLayer;
use tower_http::set_header::SetRequestHeaderLayer;
use crate::app::config::{DynConfigProvider};
use crate::app::web::handler;

pub fn routes(config: DynConfigProvider) -> axum::Router {
    let template_execute = post(handler::execute_template)
        .layer(TraceLayer::new_for_http())
        // TODO remove SetRequestHeaderLayer once Argo sends correct Content-Type header
        .layer(SetRequestHeaderLayer::if_not_present(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        ))
        .layer(AddExtensionLayer::new(config));

    Router::new()
        .route("/healthz", get(|| async { "ok\n" }))
        .route("/api/v1/template.execute", template_execute)
}
