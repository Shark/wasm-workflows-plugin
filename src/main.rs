use std::net::{IpAddr,SocketAddr};
use std::str::FromStr;
use anyhow::anyhow;
use tracing::Level;
use tracing_subscriber::{filter, FmtSubscriber, prelude::__tracing_subscriber_SubscriberExt};

pub mod app;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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

    let deps = app::dependencies::initialize().map_err(
        |err| anyhow!(err).context("Initializing dependencies failed")
    )?;
    let config = deps.get_config();
    let bind_ip = config.bind_ip.clone();
    let bind_port = config.bind_port.clone();
    tracing::debug!("Loaded Config: {:?}", config);

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
