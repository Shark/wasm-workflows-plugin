use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use serde_json::Value;

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
    pub image: String,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
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
    pub phase: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<Outputs>,
}
