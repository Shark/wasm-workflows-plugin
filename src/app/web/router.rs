use crate::app::dependencies::DynDependencyProvider;
use crate::app::web::handler;
use axum::http::{header, HeaderValue};
use axum::routing::{get, post};
use axum::{AddExtensionLayer, Router};
use tower::limit::concurrency::ConcurrencyLimitLayer;
use tower_http::set_header::SetRequestHeaderLayer;
use tower_http::trace::TraceLayer;

pub fn routes(deps: DynDependencyProvider) -> axum::Router {
    let num_concurrent_requests = deps.get_config().num_concurrent_requests();
    let template_execute = post(handler::execute_template)
        // TODO remove SetRequestHeaderLayer once Argo sends correct Content-Type header
        .layer(AddExtensionLayer::new(deps))
        .layer(SetRequestHeaderLayer::if_not_present(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        ))
        .layer(ConcurrencyLimitLayer::new(num_concurrent_requests.into()))
        .layer(TraceLayer::new_for_http());

    Router::new()
        .route("/healthz", get(|| async { "ok\n" }))
        .route("/api/v1/template.execute", template_execute)
}
