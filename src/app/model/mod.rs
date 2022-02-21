use std::collections::HashMap;
use serde::{Serialize, Deserialize};

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

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct Outputs {
    pub artifacts: Option<Vec<Artifact>>,
    pub parameters: Option<Vec<Parameter>>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct Artifact {
    pub name: String,
    pub path: String,
    pub s3: Option<S3Artifact>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct S3Artifact {
    pub key: String,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct Parameter {
    pub name: String,
    pub value: String,
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
}
