use super::model::{
    PluginInvocation, PluginResult, INPUT_FILE_NAME, RESULT_FILE_NAME, WORKING_DIR_PLUGIN_PATH,
};
use crate::model::Phase;
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

pub fn main(plugin: Box<dyn FnOnce(PluginInvocation) -> anyhow::Result<PluginResult>>) {
    match wrapper(plugin) {
        Ok(_) => (),
        Err(why) => {
            eprintln!("{}", why);
            exit(1)
        }
    }
}

fn wrapper(
    plugin: Box<dyn FnOnce(PluginInvocation) -> anyhow::Result<PluginResult>>,
) -> anyhow::Result<()> {
    let plugin_manager = PluginManager::new();
    let invocation = plugin_manager.invocation()?;
    match plugin(invocation) {
        Ok(result) => {
            plugin_manager.set_result(&result)?;
            Ok(())
        }
        Err(why) => {
            let error_result = PluginResult {
                phase: Phase::Failed,
                message: why.to_string(),
                outputs: Default::default(),
            };
            plugin_manager.set_result(&error_result)?;
            Ok(())
        }
    }
}
