use anyhow::anyhow;
use kube::CustomResource;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, warn};

const RAW_WORKFLOW_CONTAINER: &str = r#"
apiVersion: argoproj.io/v1alpha1
kind: Workflow
metadata:
  name: image-processor-container-
  namespace: argo-workflows
spec:
  entrypoint: process-all
  podGC:
    strategy: OnWorkflowCompletion
  templates:
  - name: process-all
    dag:
      tasks: []
  - name: process-image
    inputs:
      parameters:
      - name: effect
        value: desaturate
      artifacts:
      - name: input
        path: /input.jpg
        s3:
          key: IMG_5994.jpg
    container:
      image: 192.168.64.2:32000/container-image-processor:v5
      command: [image-processor, '--workflow-name={{ workflow.name }}']
      env:
      - name: OTEL_ENABLE
        value: '1'
      - name: OTEL_EXPORTER_JAEGER_AGENT_HOST
        value: jaeger-agent-svc.default.svc.cluster.local
      - name: OTEL_EXPORTER_JAEGER_AGENT_PORT
        value: '6831'
    outputs:
      artifacts:
      - name: output
        path: /output.jpg
"#;

const RAW_WORKFLOW_WASM: &str = r#"
apiVersion: argoproj.io/v1alpha1
kind: Workflow
metadata:
  name: image-processor-wasm-
  namespace: argo-workflows
spec:
  entrypoint: process-all
  templates:
  - name: process-all
    dag:
      tasks: []
  - name: process-image
    inputs:
      parameters:
      - name: effect
        value: desaturate
      artifacts:
      - name: input
        path: /input.jpg
        s3:
          key: IMG_5994.jpg
    plugin:
      wasm:
        module:
          # oci: ghcr.io/shark/wasm-workflows-plugin-image-processor:latest
          oci: 192.168.64.2:32000/image-processor:latest
"#;

pub struct SerializedWorkflow {
    pub name: String,
    pub namespace: String,
    pub yaml: String,
}

pub(crate) fn container_workflow(num_images: u16) -> SerializedWorkflow {
    let base: MyWorkflow = serde_yaml::from_str(RAW_WORKFLOW_CONTAINER).unwrap();
    return serialize_workflow(extend_workflow(base, num_images));
}

pub(crate) fn wasm_workflow(num_images: u16) -> SerializedWorkflow {
    let base: MyWorkflow = serde_yaml::from_str(RAW_WORKFLOW_WASM).unwrap();
    return serialize_workflow(extend_workflow(base, num_images));
}

fn extend_workflow(mut base: MyWorkflow, num_images: u16) -> MyWorkflow {
    let name_postfix = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(10)
        .map(char::from)
        .collect::<String>()
        .to_lowercase();
    base.metadata.name += &name_postfix;
    let mut template: &mut Template = base
        .spec
        .templates
        .iter_mut()
        .find(|t| t.name == "process-all")
        .expect("template is available");
    let new_steps = (0..num_images)
        .map(|n| TemplateRef {
            name: format!("process-{}", n),
            template: "process-image".into(),
            arguments: Arguments {
                parameters: Default::default(),
                artifacts: Default::default(),
                extra: Default::default(),
            },
            extra: Default::default(),
        })
        .collect();
    template.dag = Some(DAGTemplate {
        tasks: new_steps,
        extra: Default::default(),
    });
    return base;
}

fn serialize_workflow(wf: MyWorkflow) -> SerializedWorkflow {
    SerializedWorkflow {
        name: wf.metadata.name.clone(),
        namespace: wf.metadata.namespace.clone(),
        yaml: serde_yaml::to_string(&wf).unwrap(),
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct MyWorkflow {
    metadata: WorkflowMetadata,
    spec: MyWorkflowSpec,

    #[serde(flatten)]
    extra: HashMap<String, serde_yaml::Value>,
}

#[derive(Serialize, Deserialize, Debug)]
struct WorkflowMetadata {
    name: String,
    namespace: String,

    #[serde(flatten)]
    extra: HashMap<String, serde_yaml::Value>,
}

#[derive(Serialize, Deserialize, Debug)]
struct MyWorkflowSpec {
    templates: Vec<Template>,

    #[serde(flatten)]
    extra: HashMap<String, serde_yaml::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Inputs {
    artifacts: Vec<Artifact>,

    #[serde(flatten)]
    extra: HashMap<String, serde_yaml::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Artifact {
    name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    from: Option<String>,

    #[serde(flatten)]
    extra: HashMap<String, serde_yaml::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Template {
    name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    inputs: Option<TemplateInputs>,

    #[serde(skip_serializing_if = "Option::is_none")]
    dag: Option<DAGTemplate>,

    #[serde(flatten)]
    extra: HashMap<String, serde_yaml::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct TemplateInputs {
    parameters: Vec<Parameter>,
    artifacts: Vec<TemplateArtifact>,

    #[serde(flatten)]
    extra: HashMap<String, serde_yaml::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct TemplateArtifact {
    name: String,
    path: String,

    #[serde(flatten)]
    extra: HashMap<String, serde_yaml::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct DAGTemplate {
    tasks: Vec<TemplateRef>,

    #[serde(flatten)]
    extra: HashMap<String, serde_yaml::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct TemplateRef {
    name: String,
    template: String,
    arguments: Arguments,

    #[serde(flatten)]
    extra: HashMap<String, serde_yaml::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Arguments {
    parameters: Vec<Parameter>,
    artifacts: Vec<Artifact>,

    #[serde(flatten)]
    extra: HashMap<String, serde_yaml::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Parameter {
    name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,

    #[serde(flatten)]
    extra: HashMap<String, serde_yaml::Value>,
}

#[derive(CustomResource, Clone, Debug, Deserialize, Serialize, JsonSchema)]
#[kube(
    group = "argoproj.io",
    version = "v1alpha1",
    kind = "Workflow",
    namespaced,
    status = "WorkflowStatus"
)]
pub(crate) struct WorkflowSpec {}

#[derive(Deserialize, Serialize, Debug, Clone, JsonSchema)]
pub(crate) struct WorkflowStatus {
    progress: String,
    phase: String,
}

impl WorkflowStatus {
    pub fn result(&self) -> anyhow::Result<WorkflowResult> {
        match self.phase.as_str() {
            "Failed" => Ok(WorkflowResult::Failed),
            "Succeeded" => Ok(WorkflowResult::Succeeded),
            "Running" => {
                let progresses: Vec<&str> = self.progress.split("/").collect();
                if progresses.len() != 2 {
                    return Err(anyhow!(format!(
                        "Unexpected progress format: {}",
                        &self.progress
                    )));
                }
                // if e.g. 6/6 or 10/10
                if progresses.get(0).unwrap() == progresses.get(1).unwrap() {
                    return Ok(WorkflowResult::Succeeded);
                }
                debug!(progress = ?&self.progress, "Workflow is still partially running");
                Ok(WorkflowResult::Running)
            }
            other => Err(anyhow!(format!("Unexpected workflow phase: {}", other))),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum WorkflowResult {
    Succeeded,
    Failed,
    Running,
}

#[derive(Debug)]
pub struct WorkflowStats {
    pub name: String,
    pub result: WorkflowResult,
    pub total_time_taken: usize,
}
