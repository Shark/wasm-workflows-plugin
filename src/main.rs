use std::net::{IpAddr,SocketAddr};
use std::str::FromStr;
use anyhow::anyhow;
use crate::app::tracing as app_tracing;

pub mod app;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let deps = app::dependencies::initialize().map_err(
        |err| anyhow!(err).context("Initializing dependencies failed")
    )?;
    let config = deps.get_config();

    app_tracing::setup(config.debug, config.enable_telemetry).expect("setup tracing");
    tracing::debug!(?config, "Loaded Config");

    let bind_ip = config.bind_ip.clone();
    let bind_port = config.bind_port.clone();

    let app = app::web::router::routes(deps);

    let ip_addr = IpAddr::from_str(&bind_ip)
        .map_err(|err|
            anyhow::Error::new(err).context(format!("Failed to parse IP \"{}\"", bind_ip))
        )?;
    let addr = SocketAddr::new(ip_addr, bind_port);
    tracing::info!("Listening on {}", addr);
    // TODO Add signal handler
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .expect("start server");

    Ok(())
}
