use super::model::{INPUT_FILE_NAME, RESULT_FILE_NAME};
use crate::model::{PluginInvocation, PluginResult, INPUT_ARTIFACTS_PATH, OUTPUT_ARTIFACTS_PATH};
use anyhow::Context;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::path::Path;
use tempfile::TempDir;

pub struct WorkingDir {
    temp_dir: TempDir,
}

pub mod artifacts;

impl WorkingDir {
    pub async fn try_new() -> anyhow::Result<Self> {
        // Create working directory as TempDir
        let temp_dir = TempDir::new()?;
        tokio::fs::DirBuilder::new()
            .recursive(true)
            .create(temp_dir.path().join(INPUT_ARTIFACTS_PATH))
            .await
            .context("Creating input artifacts dir")?;
        tokio::fs::DirBuilder::new()
            .recursive(true)
            .create(temp_dir.path().join(OUTPUT_ARTIFACTS_PATH))
            .await
            .context("Creating output artifacts dir")?;

        Ok(Self { temp_dir })
    }

    pub fn set_input(&self, invocation: &PluginInvocation) -> anyhow::Result<()> {
        let input_file = {
            let path = self.temp_dir.path().join(INPUT_FILE_NAME);
            File::create(path)?
        };
        serde_json::to_writer(input_file, invocation)?;
        Ok(())
    }

    pub fn path(&self) -> &Path {
        self.temp_dir.path()
    }

    pub fn result(&self) -> anyhow::Result<PluginResult> {
        let result_file = {
            let path = self.temp_dir.path().join(RESULT_FILE_NAME);
            File::open(path)?
        };
        let plugin_result: PluginResult = serde_json::from_reader(result_file)?;
        Ok(plugin_result)
    }
}

impl Debug for WorkingDir {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let dir = self.temp_dir.path().to_str().unwrap_or("?");
        f.debug_struct("WorkingDir").field("dir", &dir).finish()
    }
}
