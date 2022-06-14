use anyhow::anyhow;
use clap::Parser;
use opentelemetry_jaeger::PipelineBuilder;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::prelude::*;
use workflow_model::model::{ArtifactRef, Parameter, PluginInvocation};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Config {
    #[clap(long = "watermark")]
    pub watermark: bool,

    #[clap(long = "effect")]
    pub effect: Option<String>,

    #[clap(long = "enable-telemetry", env = "OTEL_ENABLE")]
    pub enable_telemetry: bool,
}

fn pipeline() -> PipelineBuilder {
    opentelemetry_jaeger::new_pipeline().with_service_name("image-processor")
}

fn init_telemetry() -> anyhow::Result<()> {
    let tracer = pipeline()
        .install_simple()
        .map_err(|err| anyhow!(err).context("opentelemetry_jaeger setup failed"))?;

    tracing_subscriber::registry()
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .with(tracing_subscriber::fmt::layer().compact())
        .init();

    tracing::debug!(pipeline = ?pipeline(), "Jaeger Pipeline");

    Ok(())
}

pub(crate) fn initialize() -> PluginInvocation {
    let config = Config::parse();
    if config.enable_telemetry {
        init_telemetry().expect("initialize telemetry")
    }
    let mut invocation = PluginInvocation {
        workflow_name: "dummy".to_string(),
        plugin_options: vec![],
        parameters: vec![],
        artifacts: vec![ArtifactRef {
            name: "input".to_string(),
            path: "input".to_string(),
            s3: None,
        }],
    };
    if config.watermark {
        invocation.artifacts.push(ArtifactRef {
            name: "watermark".to_string(),
            path: "watermark".to_string(),
            s3: None,
        })
    }
    if let Some(effect) = config.effect {
        invocation.parameters.push(Parameter {
            name: "effect".to_string(),
            value: serde_json::Value::String(effect),
        })
    }
    invocation
}
