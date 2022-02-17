use std::collections::HashMap;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct ExecuteTemplatePayload {
    template: Template,
    workflow: Workflow,
}

#[derive(Deserialize, Debug)]
struct Template {
    inputs: Inputs,
    metadata: HashMap<String, String>,
    name: String,
    outputs: Outputs,
    plugin: Plugin,
}

#[derive(Deserialize, Debug)]
struct Inputs {
    artifacts: Option<Vec<Artifact>>,
    parameters: Option<Vec<Parameter>>,
}

#[derive(Deserialize, Debug)]
struct Outputs {
    artifacts: Option<Vec<Artifact>>,
    parameters: Option<Vec<Parameter>>,
}

#[derive(Deserialize, Debug)]
struct Artifact {
    name: String,
    path: String,
    s3: Option<S3Artifact>,
}

#[derive(Deserialize, Debug)]
struct S3Artifact {
    key: String,
}

#[derive(Deserialize, Debug)]
struct Parameter {
    name: String,
    value: String,
}

#[derive(Deserialize, Debug)]
struct Plugin {
    wasm: HashMap<String, String>,
}

#[derive(Deserialize, Debug)]
struct Workflow {
    metadata: WorkflowMetadata,
}

#[derive(Deserialize, Debug)]
struct WorkflowMetadata {
    name: String,
}
