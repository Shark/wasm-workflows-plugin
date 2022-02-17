use axum::Router;
use axum::routing::post;
use tower_http::trace::TraceLayer;

pub fn routes() -> axum::Router {
    Router::new()
        .route("/api/v1/template.execute", post(super::handler::execute_template))
        .layer(TraceLayer::new_for_http())
}
