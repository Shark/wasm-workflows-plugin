use std::collections::HashMap;
use serde::{Serialize, Deserialize};

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct ExecuteTemplatePayload {
    pub template: Template,
    pub workflow: Workflow,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct Template {
    inputs: Inputs,
    metadata: HashMap<String, String>,
    name: String,
    outputs: Outputs,
    plugin: Plugin,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct Inputs {
    artifacts: Option<Vec<Artifact>>,
    parameters: Option<Vec<Parameter>>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct Outputs {
    artifacts: Option<Vec<Artifact>>,
    parameters: Option<Vec<Parameter>>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct Artifact {
    name: String,
    path: String,
    s3: Option<S3Artifact>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct S3Artifact {
    key: String,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct Parameter {
    name: String,
    value: String,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct Plugin {
    wasm: HashMap<String, String>,
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
pub struct ExecuteTemplateResult {
    pub phase: String,
    pub message: String,
}
