use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use workflow_model::model::{Artifact, Parameter, Phase};

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
    pub extra: HashMap<String, serde_json::Value>,
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

#[derive(Serialize, Debug)]
#[allow(dead_code)]
pub struct ExecuteTemplateResult {
    pub phase: Phase,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<Outputs>,
}

impl ExecuteTemplateResult {
    pub(crate) fn from_plugin_result(src: workflow_model::model::PluginResult) -> Self {
        let outputs = match !src.outputs.parameters.is_empty() || !src.outputs.artifacts.is_empty()
        {
            true => {
                let parameters = Some(src.outputs.parameters);
                let artifacts = None;
                Some(Outputs {
                    artifacts,
                    parameters,
                })
            }
            false => None,
        };
        Self {
            phase: src.phase,
            message: src.message,
            outputs,
        }
    }
}
