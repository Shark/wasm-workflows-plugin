use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Serialize, Deserialize, Debug)]
#[allow(dead_code)]
pub struct Parameter {
    pub name: String,
    pub value: serde_json::Value,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct ArtifactRef {
    pub name: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s3: Option<S3Artifact>,
}

impl ArtifactRef {
    pub(crate) fn working_dir_path(&self) -> &str {
        if self.path.starts_with("/") {
            &self.path[1..]
        } else {
            &self.path
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[allow(dead_code)]
pub struct S3Artifact {
    pub key: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[allow(dead_code)]
pub struct PluginResult {
    pub phase: Phase,
    pub message: String,
    pub outputs: Outputs,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum Phase {
    Succeeded,
    Failed,
}

impl FromStr for Phase {
    type Err = ();

    fn from_str(s: &str) -> Result<Phase, ()> {
        match s {
            "Succeeded" => Ok(Phase::Succeeded),
            "Failed" => Ok(Phase::Failed),
            _ => Err(()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[allow(dead_code)]
pub struct Outputs {
    pub artifacts: Vec<ArtifactRef>,
    pub parameters: Vec<Parameter>,
}

/// PluginInvocation is a single Wasm plugin invocation
#[derive(Serialize, Deserialize, Debug)]
pub struct PluginInvocation {
    pub workflow_name: String,
    pub plugin_options: Vec<Parameter>,
    pub parameters: Vec<Parameter>,
    pub artifacts: Vec<ArtifactRef>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct S3ArtifactRepositoryConfig {
    pub access_key: String,
    pub secret_key: String,
    pub bucket: String,
    pub endpoint: String,
    pub region: String,
    pub insecure: bool,
    pub path_style_endpoint: bool,
}

pub const WORKING_DIR_PLUGIN_PATH: &str = "/work";
pub const INPUT_ARTIFACTS_PATH: &str = "artifacts-in";
pub const OUTPUT_ARTIFACTS_PATH: &str = "artifacts-out";
pub const INPUT_FILE_NAME: &str = "input.json";
pub const RESULT_FILE_NAME: &str = "result.json";
