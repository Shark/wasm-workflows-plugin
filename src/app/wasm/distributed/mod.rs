use crate::app::model::ModulePermissions;
use crate::app::wasm::{Runner, WasmError};
use anyhow::anyhow;
use async_trait::async_trait;
use futures::stream::{BoxStream, SelectAll};
use futures::TryStreamExt;
use futures::{stream, try_join, StreamExt};
use k8s_openapi::api::core::v1::{ConfigMap, Container, Pod, PodSpec, Toleration};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::api::PostParams;
use kube::{Api, ResourceExt};
use kube_core::params::{DeleteParams, ListParams};
use kube_runtime::{watcher, WatchStreamExt};
use std::collections::{BTreeMap, HashMap};
use std::time::Duration;
use tracing::{debug, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use workflow_model::model::{PluginInvocation, PluginResult, S3ArtifactRepositoryConfig};

pub struct DistributedRunner {
    client: kube::Client,
    namespace: Option<String>,
    config: Config,
}

impl DistributedRunner {
    pub fn new(client: kube::Client, namespace: Option<String>, config: Config) -> Self {
        DistributedRunner {
            client,
            namespace,
            config,
        }
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
            .create_config_map(&invocation, &artifact_repo_config, &Span::current())
            .await?;
        let pod_name = self.create_pod(&config_map_name, oci_image).await?;
        let result = self.wait_for_result(&config_map_name, &pod_name).await;
        try_join!(
            self.delete_pod(&pod_name),
            self.delete_config_map(&config_map_name)
        )?;
        return result;
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
                debug!(?name, "Created ConfigMap");
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
                debug!(name = ?pod.name(), "Created Pod");
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
    ) -> anyhow::Result<PluginResult, WasmError> {
        let config_maps: Api<ConfigMap> = self.api();
        let pods: Api<Pod> = self.api();
        let field_selector_pod = format!("metadata.name={}", pod_name);
        let field_selector_config_map = format!("metadata.name={}", config_map_name);
        let events = stream::select_all(vec![
            watcher(pods, ListParams::default().fields(&field_selector_pod))
                .applied_objects()
                .map_ok(|p| WatchEvent::Pod(Box::new(p)))
                .boxed(),
            watcher(
                config_maps,
                ListParams::default().fields(&field_selector_config_map),
            )
            .applied_objects()
            .map_ok(|c| WatchEvent::ConfigMap(Box::new(c)))
            .boxed(),
        ]);
        let result = match tokio::time::timeout(
            Duration::from_secs(self.config.wait_duration as u64),
            watch_events(events),
        )
        .await
        {
            Ok(result) => match result {
                Ok(result) => result,
                Err(why) => return Err(why),
            },
            Err(timeout_err) => return Err(WasmError::Timeout(timeout_err.into())),
        };
        serde_json::from_str(&result).map_err(|e| WasmError::Invocation(anyhow!(e)))
    }

    #[tracing::instrument(name = "wasm.delete_pod", level = "debug", skip(self))]
    async fn delete_pod(&self, pod_name: &str) -> SomeResult {
        let pods: Api<Pod> = self.api();
        match pods.delete(pod_name, &DeleteParams::default()).await {
            Ok(_) => Ok(()),
            Err(why) => Err(WasmError::Invocation(
                anyhow!(why).context("Deleting ConfigMap"),
            )),
        }
    }

    async fn delete_config_map(&self, config_map_name: &str) -> SomeResult {
        let config_maps: Api<ConfigMap> = self.api();
        match config_maps
            .delete(config_map_name, &DeleteParams::default())
            .await
        {
            Ok(_) => Ok(()),
            Err(why) => Err(WasmError::Invocation(
                anyhow!(why).context("Deleting ConfigMap"),
            )),
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

type SomeResult = anyhow::Result<(), WasmError>;

async fn watch_events(
    mut events: SelectAll<BoxStream<'_, Result<WatchEvent, kube_runtime::watcher::Error>>>,
) -> anyhow::Result<String, WasmError> {
    while let Some(e) = events
        .try_next()
        .await
        .map_err(|why| WasmError::Invocation(anyhow!(why)))?
    {
        match e {
            WatchEvent::ConfigMap(config_map) => {
                match config_map.data.unwrap_or_default().get("result.json") {
                    Some(result) => return Ok(result.clone()),
                    None => {
                        debug!("Result not (yet) present in ConfigMap");
                        continue;
                    }
                }
            }
            WatchEvent::Pod(pod) => {
                if let Some(status) = pod.status {
                    if let Some(phase) = status.phase {
                        debug!("Pod phase is {}", phase);
                        match phase.as_str() {
                            "Pending" => continue,
                            "Running" => continue,
                            "Succeeded" => {
                                debug!("Pod has succeeded");
                            }
                            "Failed" => {
                                return Err(WasmError::Invocation(anyhow!("Pod has failed")))
                            }
                            other => {
                                debug!("Unknown phase {}", other);
                                continue;
                            }
                        }
                    }
                }
            }
        }
    }
    Err(WasmError::Invocation(anyhow!(
        "Impossible state while watching events for Pod & ConfigMap"
    )))
}

enum WatchEvent {
    ConfigMap(Box<ConfigMap>),
    Pod(Box<Pod>),
}

pub struct Config {
    pub wait_duration: u16,
}

const DEFAULT_NAMESPACE: &str = "default";
const NAME_PREFIX: &str = "wasm-workflow-";
