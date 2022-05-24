use crate::app::model::ModulePermissions;
use crate::app::wasm::{Runner, WasmError};
use anyhow::anyhow;
use async_trait::async_trait;
use k8s_openapi::api::core::v1::{ConfigMap, Container, Pod, PodSpec, Toleration};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use kube::api::PostParams;
use kube::{Api, ResourceExt};
use std::collections::BTreeMap;
use std::time::Duration;
use tokio::time::sleep;
use workflow_model::model::{Phase, PluginInvocation, PluginResult};

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
    #[tracing::instrument(name = "wasm.run", skip(self))]
    async fn run(
        &self,
        oci_image: &str,
        invocation: PluginInvocation,
        _perms: &Option<ModulePermissions>,
    ) -> anyhow::Result<PluginResult, WasmError> {
        let (config_maps, namespace): (Api<ConfigMap>, &str) = {
            let client = self.client.clone();
            match &self.namespace {
                Some(ns) => (Api::namespaced(client, ns), ns),
                None => (Api::default_namespaced(client), "default"),
            }
        };
        let input_json = serde_json::to_string(&invocation)
            .map_err(|e| WasmError::EnvironmentSetup(anyhow!(e)))?;
        let config_map = ConfigMap {
            metadata: ObjectMeta {
                namespace: Some(namespace.to_owned()),
                generate_name: Some(NAME_PREFIX.into()),
                ..Default::default()
            },
            data: Some(BTreeMap::from([("input.json".into(), input_json)])),
            ..Default::default()
        };
        let pp = PostParams::default();
        let config_map_name = match config_maps.create(&pp, &config_map).await {
            Ok(config_map) => {
                let name = config_map.name();
                tracing::debug!(?name, "Created ConfigMap");
                name
            }
            Err(why) => {
                return Err(WasmError::Invocation(
                    anyhow!(why).context("Creating ConfigMap"),
                ))
            }
        };
        let pods: Api<Pod> = {
            let client = self.client.clone();
            match &self.namespace {
                Some(ns) => Api::namespaced(client, ns),
                None => Api::default_namespaced(client),
            }
        };
        let pod = Pod {
            metadata: ObjectMeta {
                namespace: Some(namespace.to_owned()),
                name: Some(config_map_name.clone()),
                ..Default::default()
            },
            spec: Some(PodSpec {
                containers: vec![Container {
                    name: config_map_name.clone(),
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
        match pods.create(&pp, &pod).await {
            Ok(pod) => {
                let name = pod.name();
                tracing::debug!(?name, "Created Pod");
            }
            Err(why) => return Err(WasmError::Invocation(anyhow!(why).context("Creating Pod"))),
        };

        let mut tries = 0;
        let mut result: Option<String> = None;
        while tries < 100 {
            let config_map = config_maps
                .get(&config_map_name)
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
            sleep(Duration::from_millis(100)).await;
        }
        if result.is_none() {
            return Ok(PluginResult {
                phase: Phase::Failed,
                message: "Timeout: Result not received in time".into(),
                outputs: Default::default(),
            });
        }
        let result: PluginResult = serde_json::from_str(&result.unwrap())
            .map_err(|e| WasmError::Invocation(anyhow!(e)))?;
        Ok(result)
    }
}

const NAME_PREFIX: &str = "wasm-workflow-";
