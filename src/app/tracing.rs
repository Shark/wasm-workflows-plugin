use crate::app::config::LogLevel;
use anyhow::anyhow;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::filter::dynamic_filter_fn;
use tracing_subscriber::{
    filter::{EnvFilter, LevelFilter},
    prelude::*,
};

pub fn setup(log_level: &LogLevel, enable_telemetry: bool) -> anyhow::Result<()> {
    let filter = EnvFilter::from_default_env().add_directive(LevelFilter::INFO.into());

    let filter = match log_level {
        LogLevel::Debug => filter
            .add_directive("tower_http=debug".parse().expect("parse directive"))
            .add_directive(
                "wasm_workflows_plugin=debug"
                    .parse()
                    .expect("parse directive"),
            )
            .add_directive("workflow_model=debug".parse().expect("parse directive")),
        LogLevel::Trace => filter
            .add_directive("tower_http=debug".parse().expect("parse directive"))
            .add_directive(
                "wasm_workflows_plugin=trace"
                    .parse()
                    .expect("parse directive"),
            )
            .add_directive("workflow_model=trace".parse().expect("parse directive")),
        _ => filter,
    };

    let telemetry: Option<OpenTelemetryLayer<_, _>> = match enable_telemetry {
        true => Some(telemetry()?),
        false => None,
    };

    tracing_subscriber::registry()
        .with(telemetry)
        .with(
            tracing_subscriber::fmt::layer()
                .compact()
                .with_filter(dynamic_filter_fn(move |metadata, ctx| {
                    filter.enabled(metadata, ctx.clone())
                })),
        )
        .init();

    Ok(())
}

fn telemetry() -> anyhow::Result<
    tracing_opentelemetry::OpenTelemetryLayer<
        tracing_subscriber::Registry,
        opentelemetry::sdk::trace::Tracer,
    >,
> {
    let tracer = opentelemetry_jaeger::new_pipeline()
        .with_service_name("wasm-workflows-plugin")
        .install_simple()
        .map_err(|err| anyhow!(err).context("opentelemetry_jaeger setup failed"))?;

    // Create a tracing layer with the configured tracer
    Ok(tracing_opentelemetry::layer().with_tracer(tracer))
}
