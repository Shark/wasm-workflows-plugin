use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::str::FromStr;

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct ExecuteTemplateRequest {
    pub template: Template,
    pub workflow: Workflow,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct Template {
    pub inputs: Inputs,
    pub metadata: HashMap<String, String>,
    pub name: String,
    pub outputs: Outputs,
    pub plugin: Plugin,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct Inputs {
    pub artifacts: Option<Vec<Artifact>>,
    pub parameters: Option<Vec<Parameter>>,
}

#[derive(Serialize, Deserialize, Debug)]
#[allow(dead_code)]
pub struct Outputs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifacts: Option<Vec<Artifact>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Vec<Parameter>>,
}

#[derive(Serialize, Deserialize, Debug)]
#[allow(dead_code)]
pub struct Artifact {
    pub name: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub s3: Option<S3Artifact>,
}

#[derive(Serialize, Deserialize, Debug)]
#[allow(dead_code)]
pub struct S3Artifact {
    pub key: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[allow(dead_code)]
pub struct Parameter {
    pub name: String,
    pub value: serde_json::Value,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct Plugin {
    pub wasm: Option<WasmPluginConfig>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct WasmPluginConfig {
    pub module: ModuleSource,
    pub permissions: Option<ModulePermissions>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct ModulePermissions {
    pub http: Option<HTTPPermissions>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct HTTPPermissions {
    pub allowed_hosts: Vec<String>,
    // TODO this should be easier to accomplish
    #[serde(default = "default_max_concurrent_requests")]
    pub max_concurrent_requests: u32,
}

fn default_max_concurrent_requests() -> u32 {
    8
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub enum ModuleSource {
    #[serde(rename = "oci")]
    OCI(String),
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct Workflow {
    pub metadata: WorkflowMetadata,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct WorkflowMetadata {
    pub name: String,
}

#[derive(Serialize, Debug)]
#[allow(dead_code)]
pub struct ExecuteTemplateResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub node: Option<ExecuteTemplateResult>,
}

#[derive(Serialize, Deserialize, Debug)]
#[allow(dead_code)]
pub struct ExecuteTemplateResult {
    pub phase: Phase,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<Outputs>,
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

/// PluginInvocation is a single Wasm module invocation
#[derive(Serialize, Deserialize, Debug)]
pub struct PluginInvocation {
    pub workflow_name: String,
    pub plugin_options: Vec<Parameter>,
    pub parameters: Vec<Parameter>,
}
