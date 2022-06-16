use super::model::{
    PluginInvocation, PluginResult, INPUT_FILE_NAME, RESULT_FILE_NAME, WORKING_DIR_PLUGIN_PATH,
};
use crate::model::{ArtifactRef, Phase, INPUT_ARTIFACTS_PATH, OUTPUT_ARTIFACTS_PATH};
use anyhow::anyhow;
use std::fs::File;
use std::path::PathBuf;
use std::process::exit;

struct PluginManager {}

impl PluginManager {
    pub fn new() -> Self {
        PluginManager {}
    }

    pub fn invocation(&self) -> anyhow::Result<PluginInvocation> {
        let input_file = File::open(PathBuf::from(WORKING_DIR_PLUGIN_PATH).join(INPUT_FILE_NAME))?;
        let invocation: PluginInvocation = serde_json::from_reader(input_file)?;
        Ok(invocation)
    }

    pub fn set_result(&self, result: &PluginResult) -> anyhow::Result<()> {
        let output_file =
            File::create(PathBuf::from(WORKING_DIR_PLUGIN_PATH).join(RESULT_FILE_NAME))?;
        serde_json::to_writer(output_file, result)?;
        Ok(())
    }
}

pub struct ArtifactManager {
    base_path: PathBuf,
}

impl ArtifactManager {
    pub fn new() -> ArtifactManager {
        ArtifactManager {
            base_path: PathBuf::from(WORKING_DIR_PLUGIN_PATH),
        }
    }

    pub fn new_with_base_path(base_path: PathBuf) -> ArtifactManager {
        ArtifactManager { base_path }
    }
}

impl ArtifactManager {
    pub fn input_artifact_path(&self, artifact: &ArtifactRef) -> PathBuf {
        self.base_path
            .join(INPUT_ARTIFACTS_PATH)
            .join(artifact.working_dir_path())
    }

    pub fn open_input_artifact(&self, artifact: &ArtifactRef) -> anyhow::Result<File> {
        let path = self.input_artifact_path(artifact);
        File::open(&path).map_err(|why| {
            anyhow!(why).context(format!(
                "Opening file for input artifact {} at {:?}",
                &artifact.name, &path
            ))
        })
    }

    pub fn output_artifact_path(&self, artifact: &ArtifactRef) -> PathBuf {
        self.base_path
            .join(OUTPUT_ARTIFACTS_PATH)
            .join(artifact.working_dir_path())
    }

    pub fn open_output_artifact(&self, artifact: &ArtifactRef) -> anyhow::Result<File> {
        let path = self.output_artifact_path(artifact);
        File::open(&path).map_err(|why| {
            anyhow!(why).context(format!(
                "Opening file for output artifact {} at {:?}",
                &artifact.name, &path
            ))
        })
    }
}

pub fn main(
    plugin: Box<dyn FnOnce(PluginInvocation, ArtifactManager) -> anyhow::Result<PluginResult>>,
) {
    match wrapper(plugin) {
        Ok(_) => (),
        Err(why) => {
            eprintln!("{}", why);
            exit(1)
        }
    }
}

fn wrapper(
    plugin: Box<dyn FnOnce(PluginInvocation, ArtifactManager) -> anyhow::Result<PluginResult>>,
) -> anyhow::Result<()> {
    let plugin_manager = PluginManager::new();
    let invocation = plugin_manager.invocation()?;
    let artifact_manager = ArtifactManager::new();
    match plugin(invocation, artifact_manager) {
        Ok(result) => {
            plugin_manager.set_result(&result)?;
            Ok(())
        }
        Err(why) => {
            let error_result = PluginResult {
                phase: Phase::Failed,
                message: format!("{:#}", why),
                outputs: Default::default(),
            };
            plugin_manager.set_result(&error_result)?;
            Ok(())
        }
    }
}
