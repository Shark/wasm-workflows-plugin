use crate::app::model::ModulePermissions;
use crate::app::wasm::{Runner, WasmError};
use anyhow::anyhow;
use async_trait::async_trait;
use k8s_openapi::api::core::v1::{ConfigMap, Container, Pod, PodSpec, Toleration};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::api::PostParams;
use kube::{Api, ResourceExt};
use kube_core::params::DeleteParams;
use std::collections::{BTreeMap, HashMap};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use workflow_model::model::{Phase, PluginInvocation, PluginResult, S3ArtifactRepositoryConfig};

pub struct DistributedRunner {
    client: kube::Client,
    namespace: Option<String>,
}

impl DistributedRunner {
    pub fn new(client: kube::Client, namespace: Option<String>) -> Self {
        DistributedRunner { client, namespace }
    }
}

#[async_trait]
impl Runner for DistributedRunner {
    #[tracing::instrument(
        name = "wasm.run_distributed",
        ret,
        err(Debug),
        skip(self, artifact_repo_config)
    )]
    async fn run(
        &self,
        oci_image: &str,
        invocation: PluginInvocation,
        _perms: &Option<ModulePermissions>,
        artifact_repo_config: Option<S3ArtifactRepositoryConfig>,
    ) -> anyhow::Result<PluginResult, WasmError> {
        let config_map_name = self
            .create_config_map(
                &invocation,
                &artifact_repo_config,
                &tracing::Span::current(),
            )
            .await?;
        let pod_name = self.create_pod(&config_map_name, oci_image).await?;
        let result = match self.wait_for_result(&config_map_name, &pod_name).await {
            Ok(result) => {
                self.delete_pod(&pod_name).await?;
                result
            }
            Err(why) => {
                self.delete_pod(&pod_name).await?;
                return Err(why);
            }
        };
        if result.is_none() {
            return Ok(PluginResult {
                phase: Phase::Failed,
                message: "Timeout: Result not received in time".into(),
                outputs: Default::default(),
            });
        }
        Ok(result.unwrap())
    }
}

impl DistributedRunner {
    #[tracing::instrument(
        name = "wasm.create_config_map",
        level = "debug",
        skip(self, artifact_repo_config)
    )]
    async fn create_config_map(
        &self,
        invocation: &PluginInvocation,
        artifact_repo_config: &Option<S3ArtifactRepositoryConfig>,
        parent_span: &Span,
    ) -> anyhow::Result<String, WasmError> {
        let config_maps: Api<ConfigMap> = self.api();
        let namespace = self.namespace();
        let input_json = serde_json::to_string(invocation)
            .map_err(|e| WasmError::EnvironmentSetup(anyhow!(e)))?;
        let mut data: BTreeMap<String, String> = BTreeMap::new();
        data.insert("input.json".into(), input_json);
        if let Some(artifact_repo_config) = artifact_repo_config {
            let artifact_repo_config_json = serde_json::to_string(&artifact_repo_config)
                .map_err(|e| WasmError::EnvironmentSetup(anyhow!(e)))?;
            data.insert(
                "artifact-repo-config.json".into(),
                artifact_repo_config_json,
            );
        }
        {
            let mut carrier: HashMap<String, String> = HashMap::new();
            let cx = parent_span.context();
            opentelemetry::global::get_text_map_propagator(|propagator| {
                propagator.inject_context(&cx, &mut carrier)
            });
            let carrier_json = serde_json::to_string(&carrier).map_err(|e| {
                WasmError::EnvironmentSetup(anyhow!(e).context("Encoding OpenTelemetry carrier"))
            })?;
            data.insert("opentelemetry.json".into(), carrier_json);
        }
        let config_map = ConfigMap {
            metadata: ObjectMeta {
                namespace: Some(namespace.to_owned()),
                generate_name: Some(NAME_PREFIX.into()),
                ..Default::default()
            },
            data: Some(data),
            ..Default::default()
        };
        let pp = PostParams::default();
        match config_maps.create(&pp, &config_map).await {
            Ok(config_map) => {
                let name = config_map.name();
                tracing::debug!(?name, "Created ConfigMap");
                Ok(name)
            }
            Err(why) => Err(WasmError::Invocation(
                anyhow!(why).context("Creating ConfigMap"),
            )),
        }
    }

    #[tracing::instrument(name = "wasm.create_pod", level = "debug", skip(self))]
    async fn create_pod(
        &self,
        config_map_name: &str,
        oci_image: &str,
    ) -> anyhow::Result<String, WasmError> {
        let pods: Api<Pod> = self.api();
        let pod = Pod {
            metadata: ObjectMeta {
                namespace: Some(self.namespace().to_owned()),
                name: Some(config_map_name.to_owned()),
                ..Default::default()
            },
            spec: Some(PodSpec {
                containers: vec![Container {
                    name: config_map_name.to_owned(),
                    image: Some(oci_image.to_owned()),
                    ..Default::default()
                }],
                node_selector: Some(BTreeMap::from([(
                    "kubernetes.io/arch".into(),
                    "wasm32-wasi".into(),
                )])),
                tolerations: Some(vec![
                    Toleration {
                        key: Some("kubernetes.io/arch".into()),
                        operator: Some("Equal".into()),
                        value: Some("wasm32-wasi".into()),
                        effect: Some("NoExecute".into()),
                        ..Default::default()
                    },
                    Toleration {
                        key: Some("kubernetes.io/arch".into()),
                        operator: Some("Equal".into()),
                        value: Some("wasm32-wasi".into()),
                        effect: Some("NoSchedule".into()),
                        ..Default::default()
                    },
                    Toleration {
                        key: Some("node.kubernetes.io/network-unavailable".into()),
                        operator: Some("Exists".into()),
                        effect: Some("NoSchedule".into()),
                        ..Default::default()
                    },
                ]),
                ..Default::default()
            }),
            ..Default::default()
        };
        let pp = PostParams::default();
        match pods.create(&pp, &pod).await {
            Ok(pod) => {
                tracing::debug!(name = ?pod.name(), "Created Pod");
                Ok(pod.name())
            }
            Err(why) => Err(WasmError::Invocation(anyhow!(why).context("Creating Pod"))),
        }
    }

    #[tracing::instrument(name = "wasm.wait_for_result", level = "debug", skip(self))]
    async fn wait_for_result(
        &self,
        config_map_name: &str,
        pod_name: &str,
    ) -> anyhow::Result<Option<PluginResult>, WasmError> {
        let config_maps: Api<ConfigMap> = self.api();
        let pods: Api<Pod> = self.api();
        let mut tries = 0;
        let mut result: Option<String> = None;
        let interval = 500; // ms
        while tries < 1000 {
            let pod = pods
                .get(pod_name)
                .await
                .map_err(|e| WasmError::Invocation(anyhow!(e).context("Polling Pod")))?;
            if let Some(status) = pod.status {
                if let Some(phase) = status.phase {
                    debug!("Pod phase is {}", phase);
                    match phase.as_str() {
                        "Pending" => {
                            if (tries / interval) > 10 {
                                // Fail when pending for more than 10s
                                return Err(WasmError::Invocation(anyhow!(
                                    "Pod pending for more than 10s, probably no Krustlet available"
                                )));
                            }
                            tries += 1;
                            sleep(Duration::from_millis(interval)).await;
                            continue;
                        }
                        "Running" => {
                            tries += 1;
                            sleep(Duration::from_millis(interval)).await;
                            continue;
                        }
                        "Succeeded" => {
                            debug!("Pod has succeeded");
                        }
                        "Failed" => return Err(WasmError::Invocation(anyhow!("Pod has failed"))),
                        _ => {
                            tries += 1;
                            sleep(Duration::from_millis(interval)).await;
                            continue;
                        }
                    }
                }
            }
            let config_map = config_maps
                .get(config_map_name)
                .await
                .map_err(|e| WasmError::Invocation(anyhow!(e).context("Polling ConfigMap")))?;
            match config_map.data.unwrap_or_default().get("result.json") {
                Some(this_result) => {
                    result = Some(this_result.clone());
                }
                None => tracing::debug!(?tries, "ConfigMap does not contain result"),
            }
            if result.is_some() {
                break;
            }
            tries += 1;
        }
        match &result {
            Some(str) => Ok(Some(
                serde_json::from_str(str).map_err(|e| WasmError::Invocation(anyhow!(e)))?,
            )),
            None => Ok(None),
        }
    }

    #[tracing::instrument(name = "wasm.delete_pod", level = "debug", skip(self))]
    async fn delete_pod(&self, pod_name: &str) -> anyhow::Result<(), WasmError> {
        let pods: Api<Pod> = self.api();
        match pods.delete(pod_name, &DeleteParams::default()).await {
            Ok(_) => Ok(()),
            Err(why) => Err(WasmError::Invocation(anyhow!(why).context("Deleting pod"))),
        }
    }

    fn api<T>(&self) -> Api<T>
    where
        <T as kube_core::Resource>::DynamicType: Default,
        T: k8s_openapi::Metadata<Ty = ObjectMeta>,
    {
        match &self.namespace {
            Some(ns) => Api::namespaced(self.client.to_owned(), ns),
            None => Api::default_namespaced(self.client.to_owned()),
        }
    }

    fn namespace(&self) -> &str {
        match &self.namespace {
            Some(ns) => ns,
            None => DEFAULT_NAMESPACE,
        }
    }
}

const DEFAULT_NAMESPACE: &str = "default";
const NAME_PREFIX: &str = "wasm-workflow-";
