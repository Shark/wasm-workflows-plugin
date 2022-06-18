use crate::chrono::{DateTime, Utc};
use crate::model::{WorkflowResult, WorkflowStats};
use anyhow::anyhow;
use clap::{value_parser, Arg, Command};
use futures::stream::BoxStream;
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::chrono;
use kube::runtime::{watcher, WatchStreamExt};
use kube_client::Api;
use kube_core::params::ListParams;
use model::{Mode, Workflow};
use std::io::ErrorKind;
use std::num::TryFromIntError;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;
use tokio::time;
use tracing::{debug, error, info, warn};
use tracing_subscriber::filter::EnvFilter;
use tracing_subscriber::prelude::*;

mod jaeger;
mod model;
mod store;

#[derive(Debug, Clone)]
struct Config {
    mode: Mode,
    num_parallel_images: u16,
    jaeger_grpc_endpoint: String,
    results_dir: PathBuf,
    num_workflows: u16,
    interval: Option<u16>,
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
                .value_parser(["wasm-local", "wasm-distributed", "container"])
                .required(true),
        )
        .arg(
            Arg::new("parallel-images")
                .long("parallel-images")
                .takes_value(true)
                .value_parser(value_parser!(u16).range(1..))
                .default_value("1"),
        )
        .arg(
            Arg::new("jaeger-grpc-endpoint")
                .long("jaeger-grpc-endpoint")
                .takes_value(true),
        )
        .arg(
            Arg::new("results-dir")
                .long("results-dir")
                .takes_value(true)
                .default_value("results"),
        )
        .arg(
            Arg::new("num-workflows")
                .long("num-workflows")
                .takes_value(true)
                .value_parser(value_parser!(u16).range(1..))
                .default_value("1"),
        )
        .arg(
            Arg::new("interval")
                .long("interval")
                .takes_value(true)
                .value_parser(value_parser!(u16)),
        )
        .get_matches();

    let config = {
        let num_parallel_images = *matches.get_one::<u16>("parallel-images").unwrap();
        let mode = match matches.get_one::<String>("mode").unwrap().as_str() {
            "wasm-local" => Mode::WasmLocal,
            "wasm-distributed" => Mode::WasmDistributed,
            "container" => Mode::Container,
            mode => panic!("Unexpected mode {}", mode),
        };
        let jaeger_grpc_endpoint = matches.get_one::<String>("jaeger-grpc-endpoint").unwrap();
        let results_dir: PathBuf = {
            let results_dir = matches.get_one::<String>("results-dir").unwrap();
            let results_dir = PathBuf::from(results_dir);
            let results_dir = std::env::current_dir()
                .expect("get working dir")
                .join(results_dir);
            if let Err(why) = tokio::fs::metadata(&results_dir).await {
                if why.kind() != ErrorKind::NotFound {
                    panic!("Error checking if results dir exists: {}", why);
                }
                tokio::fs::create_dir_all(&results_dir)
                    .await
                    .expect("create results dir")
            }
            results_dir
        };
        let num_workflows = *matches.get_one::<u16>("num-workflows").unwrap();
        let interval = matches.get_one::<u16>("interval").cloned();
        Config {
            mode,
            num_parallel_images,
            results_dir,
            jaeger_grpc_endpoint: jaeger_grpc_endpoint.to_owned(),
            num_workflows,
            interval,
        }
    };
    info!(?config, "Config");
    let k8s = kube::Client::try_from(kube::Config::infer().await.expect("infer kubeconfig"))
        .expect("create k8s client");
    let (tx, rx) = tokio::sync::mpsc::channel(8);
    let mut store = store::Store::new(rx, config.results_dir.clone());

    let store_handle = tokio::spawn(async move { store.run().await });

    let mut num_dispatched_workflows = 0;
    let generator_handle = tokio::spawn(async move {
        let mut interval: Option<time::Interval> = match config.interval {
            Some(interval) => Some(time::interval(Duration::from_secs(interval.into()))),
            None => None,
        };
        loop {
            let k8s = k8s.clone();
            let tx = tx.clone();
            let config = config.clone();
            let num_workflows = config.num_workflows;
            match interval {
                Some(ref mut interval) => {
                    let _ = interval.tick().await;
                    info!("Spawning new workflow (no {})", num_dispatched_workflows);
                    spawn_run(config, k8s, tx);
                }
                None => {
                    info!("Spawning new workflow (no {})", num_dispatched_workflows);
                    spawn_run(config, k8s, tx).await.expect("able to join");
                }
            }
            num_dispatched_workflows += 1;
            if num_dispatched_workflows >= num_workflows {
                info!("Enough Workflows!");
                break;
            }
        }
    });

    futures::future::join_all(vec![store_handle, generator_handle]).await;
}

fn spawn_run(config: Config, k8s: kube::Client, tx: Sender<WorkflowStats>) -> JoinHandle<()> {
    tokio::spawn(async move {
        match run(&config, k8s).await {
            Ok(stats) => {
                if let Err(why) = tx.send(stats).await {
                    error!(?why, "Error sending result to store");
                }
            }
            Err(why) => {
                error!("Error running benchmark: {:?}", why)
            }
        }
    })
}

async fn run(config: &Config, k8s: kube::Client) -> anyhow::Result<WorkflowStats> {
    let workflow = match config.mode {
        Mode::WasmLocal => model::wasm_local_workflow(config.num_parallel_images),
        Mode::WasmDistributed => model::wasm_distributed_workflow(config.num_parallel_images),
        Mode::Container => model::container_workflow(config.num_parallel_images),
    };
    exec_kubectl(&workflow.yaml).await?;
    let api: Api<Workflow> = Api::namespaced(k8s, &workflow.namespace);

    let field_selector = format!("metadata.name={}", workflow.name);
    let items = watcher(api, ListParams::default().fields(&field_selector))
        .applied_objects()
        .boxed();
    let (workflow_result, total_time_taken_sec, finished_at) =
        time::timeout(Duration::from_secs(300), process_stream(items))
            .await
            .map_err(|why| anyhow!(why).context("Waiting for update on Workflow"))?
            .map_err(|why| anyhow!(why).context("Checking for Workflow result"))?;

    let invocation_stats = jaeger::find_durations(
        &config.jaeger_grpc_endpoint,
        config.mode.clone(),
        &workflow.name,
    )
    .await?;
    let workflow_stats = WorkflowStats {
        workflow_name: workflow.name,
        result: workflow_result,
        mode: config.mode.clone(),
        num_parallel_images: config.num_parallel_images,
        finished_at,
        total_time_taken_sec,
        invocation_stats,
    };
    Ok(workflow_stats)
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

async fn process_stream(
    mut items: BoxStream<'_, Result<Workflow, kube_runtime::watcher::Error>>,
) -> anyhow::Result<(WorkflowResult, usize, chrono::DateTime<Utc>)> {
    let mut started_at: Option<DateTime<Utc>> = None;
    while let Some(p) = items.try_next().await? {
        if let Some(status) = p.status {
            let result: WorkflowResult = status
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
            match result {
                WorkflowResult::Succeeded | WorkflowResult::Failed => {
                    return Ok((result, total_time_taken, Utc::now()))
                }
                WorkflowResult::Running => continue,
            }
        } else {
            debug!("Workflow does not have a status")
        }
    }
    Err(anyhow!("Unable to retrieve result"))
}
