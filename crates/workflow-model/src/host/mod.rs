use super::model::{INPUT_FILE_NAME, RESULT_FILE_NAME};
use crate::model::{PluginInvocation, PluginResult};
use std::fs::File;
use std::path::Path;
use tempfile::TempDir;

pub struct WorkingDir {
    temp_dir: TempDir,
}

impl WorkingDir {
    pub fn try_new() -> anyhow::Result<Self> {
        // Create working directory as TempDir
        let temp_dir = TempDir::new()?;

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
