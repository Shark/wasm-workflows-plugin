use crate::chrono::{DateTime, Utc};
use crate::model::{WorkflowResult, WorkflowStats};
use anyhow::anyhow;
use clap::{value_parser, Arg, Command};
use futures::stream::BoxStream;
use futures::{pin_mut, Stream, StreamExt, TryStreamExt};
use k8s_openapi::chrono;
use kube::runtime::{watcher, WatchStreamExt};
use kube_client::Api;
use kube_core::params::ListParams;
use kube_core::GroupVersionKind;
use model::Workflow;
use std::error::Error;
use std::io::Write;
use std::num::TryFromIntError;
use std::process::{ExitStatus, Stdio};
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tracing::{debug, error, info, warn};
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::prelude::*;

mod model;

#[derive(Debug)]
enum Mode {
    Wasm,
    Container,
}

#[derive(Debug)]
struct Config {
    mode: Mode,
    num_parallel_images: u16,
}

#[tokio::main]
async fn main() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or(EnvFilter::default().add_directive("info".parse().unwrap()));
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().compact())
        .with(filter)
        .init();

    let matches = Command::new("runner")
        .about("runs wasm-workflows-plugin benchmarks")
        .version("0.0.1")
        .arg(
            Arg::new("mode")
                .long("mode")
                .takes_value(true)
                .value_parser(["wasm", "container"])
                .required(true),
        )
        .arg(
            Arg::new("parallel-images")
                .long("parallel-images")
                .takes_value(true)
                .value_parser(value_parser!(u16).range(1..))
                .default_value("1"),
        )
        .get_matches();

    let config = {
        let num_parallel_images = *matches.get_one::<u16>("parallel-images").unwrap();
        let mode = match matches.get_one::<String>("mode").unwrap().as_str() {
            "wasm" => Mode::Wasm,
            "container" => Mode::Container,
            mode => panic!("Unexpected mode {}", mode),
        };
        Config {
            mode,
            num_parallel_images,
        }
    };
    info!(?config, "Config");
    let k8s = kube::Client::try_from(kube::Config::infer().await.expect("infer kubeconfig"))
        .expect("create k8s client");
    match run(&config, k8s).await {
        Ok(stats) => {
            info!(?stats, "Workflow Stats");
        }
        Err(why) => {
            error!("Error running benchmark: {:?}", why)
        }
    }
}

async fn exec_kubectl(definition: &str) -> anyhow::Result<()> {
    debug!("Executing \"kubectl apply -f-\"");
    let mut child = tokio::process::Command::new("kubectl")
        .arg("apply")
        .arg("-f-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| anyhow!(e).context("preparing to spawn kubectl"))?;
    {
        let mut stdin = child.stdin.take().unwrap();
        stdin
            .write_all(definition.as_bytes())
            .await
            .expect("write to stdin")
    }
    let output = child
        .wait_with_output()
        .await
        .map_err(|e| anyhow!(e).context("running kubectl"))?;
    if !output.status.success() {
        let stdout = String::from_utf8(output.stdout)
            .map_err(|why| anyhow!(why).context("reading stdout"))?;
        let stderr = String::from_utf8(output.stderr)
            .map_err(|why| anyhow!(why).context("reading stderr"))?;
        warn!(?stdout, ?stderr, "kubectl errored");
        return Err(anyhow!(format!(
            "Expected kubectl to exit with code 0, but got {}",
            output.status
        )));
    }
    Ok(())
}

async fn run(config: &Config, k8s: kube::Client) -> anyhow::Result<WorkflowStats> {
    let workflow = match config.mode {
        Mode::Wasm => model::wasm_workflow(config.num_parallel_images),
        Mode::Container => model::container_workflow(config.num_parallel_images),
    };
    exec_kubectl(&workflow.yaml).await?;
    let api: Api<Workflow> = Api::namespaced(k8s, &workflow.namespace);

    let field_selector = format!("metadata.name={}", workflow.name);
    let mut items = watcher(api, ListParams::default().fields(&field_selector))
        .applied_objects()
        .boxed();
    match tokio::time::timeout(
        Duration::from_secs(300),
        process_stream(&workflow.name, items),
    )
    .await
    {
        Ok(result) => result,
        Err(why) => Err(anyhow!(why).context("Waiting for update on Workflow")),
    }
}

async fn process_stream(
    name: &str,
    mut items: BoxStream<'_, Result<Workflow, kube_runtime::watcher::Error>>,
) -> anyhow::Result<WorkflowStats> {
    let mut started_at: Option<DateTime<Utc>> = None;
    while let Some(p) = items.try_next().await? {
        if let Some(status) = p.status {
            let result: model::WorkflowResult = status
                .result()
                .map_err(|why| anyhow!(why).context("Retrieve workflow result"))?;
            if started_at.is_none() && result == WorkflowResult::Running {
                started_at = Some(Utc::now());
            }
            let started_at = started_at.unwrap_or(Utc::now());
            let total_time_taken: usize = (Utc::now() - started_at)
                .num_seconds()
                .try_into()
                .map_err(|why: TryFromIntError| {
                    anyhow!(why).context("Tried to fit time_taken into an usize")
                })?;
            let stats = WorkflowStats {
                name: name.to_owned(),
                result: result.to_owned(),
                total_time_taken,
            };
            match result {
                WorkflowResult::Succeeded => return Ok(stats),
                WorkflowResult::Failed => return Ok(stats),
                WorkflowResult::Running => continue,
            }
        } else {
            debug!("Workflow does not have a status")
        }
    }
    Err(anyhow!("Unable to retrieve result"))
}
