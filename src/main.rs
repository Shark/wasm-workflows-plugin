use crate::app::tracing as app_tracing;
use anyhow::anyhow;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use tokio::signal;

pub mod app;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let deps = app::dependencies::initialize()
        .map_err(|err| anyhow!(err).context("Initializing dependencies failed"))?;
    let config = deps.get_config();

    app_tracing::setup(config.debug, config.enable_telemetry).expect("setup tracing");
    tracing::debug!(?config, "Loaded Config");

    let bind_ip = config.bind_ip.clone();
    let bind_port = config.bind_port;

    let app = app::web::router::routes(deps);

    let ip_addr = IpAddr::from_str(&bind_ip).map_err(|err| {
        anyhow::Error::new(err).context(format!("Failed to parse IP \"{}\"", bind_ip))
    })?;
    let addr = SocketAddr::new(ip_addr, bind_port);
    tracing::info!("Listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(graceful_shutdown())
        .await
        .expect("start server");

    Ok(())
}

// From https://github.com/tokio-rs/axum/blob/616a43a/examples/graceful-shutdown/src/main.rs
async fn graceful_shutdown() {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::warn!("Signal received, shutting down gracefully");
}
