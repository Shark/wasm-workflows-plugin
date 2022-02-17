use std::net::SocketAddr;
use tracing::Level;
use tracing_subscriber::{filter, FmtSubscriber, prelude::__tracing_subscriber_SubscriberExt};

pub mod app;

#[tokio::main]
async fn main() {
    let filter = filter::EnvFilter::try_from_default_env().unwrap_or(
        filter::EnvFilter::default()
            .add_directive(filter::LevelFilter::INFO.into())
            .add_directive("tower_http=debug".parse().expect("parse directive"))
            .add_directive("wasm_workflow_executor=debug".parse().expect("parse directive")),
    );
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::TRACE)
        .finish()
        .with(filter);

    tracing::subscriber::set_global_default(subscriber).expect("set default subscriber");

    let app = app::web::router::routes();

    // TODO support customize binding addr & port
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .expect("start server");
}
