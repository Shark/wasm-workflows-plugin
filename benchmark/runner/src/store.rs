use crate::WorkflowStats;
use anyhow::anyhow;
use std::io::ErrorKind;
use std::path::PathBuf;
use tokio::fs::{File, OpenOptions};
use tokio::io::AsyncWriteExt;

pub struct Store {
    rx: tokio::sync::mpsc::Receiver<WorkflowStats>,
    results_dir: PathBuf,
}

impl Store {
    pub fn new(rx: tokio::sync::mpsc::Receiver<WorkflowStats>, results_dir: PathBuf) -> Self {
        Store { rx, results_dir }
    }

    pub async fn run(&mut self) {
        while let Some(stats) = self.rx.recv().await {
            tracing::debug!(?stats, "Got stats");
            if let Err(why) = self.write_total_workflow_time(&stats).await {
                tracing::error!(?why, "Writing total workflow time failed");
            }
            if let Err(why) = self.write_per_invocation_times(&stats).await {
                tracing::error!(?why, "Writing total workflow time failed");
            }
        }
    }

    async fn write_total_workflow_time(&self, stats: &WorkflowStats) -> anyhow::Result<()> {
        let filename = format!(
            "total_{}_{}.csv",
            stats.mode.to_string(),
            stats.num_parallel_images
        );
        let out_path = self.results_dir.join(&filename);
        let mut out_file: Option<File> = None;
        if let Err(why) = tokio::fs::metadata(&out_path).await {
            if why.kind() != ErrorKind::NotFound {
                return Err(anyhow!(why)
                    .context(format!("Checking if {} exists in results dir", &filename)));
            }
            let mut f = File::create(&out_path)
                .await
                .map_err(|why| anyhow!(why).context("Creating empty file"))?;
            f.write_all(TOTAL_WORKFLOW_TIME_HEADER.as_ref())
                .await
                .map_err(|why| anyhow!(why).context("Writing file header"))?;
            out_file = Some(f);
        }
        if out_file.is_none() {
            out_file = Some(
                OpenOptions::new()
                    .write(true)
                    .append(true)
                    .open(&out_path)
                    .await
                    .map_err(|why| anyhow!(why).context("Opening output file"))?,
            );
        }
        let mut out_file = out_file.unwrap();
        let output = format!(
            "{};{};{};{}\n",
            stats.finished_at.to_rfc3339(),
            stats.result.to_string(),
            stats.total_time_taken_sec,
            stats.workflow_name,
        );
        out_file
            .write_all(output.as_ref())
            .await
            .map_err(|why| anyhow!(why).context("Writing to file"))?;
        Ok(())
    }

    async fn write_per_invocation_times(&self, stats: &WorkflowStats) -> anyhow::Result<()> {
        let filename = format!(
            "one-invocation_{}_{}.csv",
            stats.mode.to_string(),
            stats.num_parallel_images
        );
        let out_path = self.results_dir.join(&filename);
        let mut out_file: Option<File> = None;
        if let Err(why) = tokio::fs::metadata(&out_path).await {
            if why.kind() != ErrorKind::NotFound {
                return Err(anyhow!(why)
                    .context(format!("Checking if {} exists in results dir", &filename)));
            }
            let mut f = File::create(&out_path)
                .await
                .map_err(|why| anyhow!(why).context("Creating empty file"))?;
            f.write_all(INVOCATION_STATS_HEADER.as_ref())
                .await
                .map_err(|why| anyhow!(why).context("Writing file header"))?;
            out_file = Some(f);
        }
        if out_file.is_none() {
            out_file = Some(
                OpenOptions::new()
                    .write(true)
                    .append(true)
                    .open(&out_path)
                    .await
                    .map_err(|why| anyhow!(why).context("Opening output file"))?,
            );
        }
        let mut out_file = out_file.unwrap();
        for invocation in &stats.invocation_stats {
            let output = format!(
                "{};{};{};{}\n",
                invocation.timestamp.to_rfc3339(),
                stats.result.to_string(),
                invocation.processing_ms,
                stats.workflow_name,
            );
            out_file
                .write_all(output.as_ref())
                .await
                .map_err(|why| anyhow!(why).context("Writing to file"))?;
        }
        Ok(())
    }
}

const TOTAL_WORKFLOW_TIME_HEADER: &str =
    "finished_at_utc;workflow_result;total_time_taken;workflow_name\n";
const INVOCATION_STATS_HEADER: &str = "time_utc;workflow_result;processing_ms;workflow_name\n";
