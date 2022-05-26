use crate::app::config::Config;
use crate::app::model::argo::ArtifactRepositoryConfig;
use anyhow::{anyhow, Context};
use k8s_openapi::api::core::v1::{ConfigMap, Secret};
use kube_client::config::{
    AuthInfo, Cluster, Context as KubeContext, KubeConfigOptions, Kubeconfig, NamedAuthInfo,
    NamedCluster, NamedContext,
};
use kube_client::Api;
use workflow_model::model::S3ArtifactRepositoryConfig;

async fn try_given_kubeconfig(config: &Config) -> anyhow::Result<Option<kube::Config>> {
    let k8s_api_ca_crt = config.k8s_api_ca_crt()?;
    if config.k8s_api_url.is_none()
        || k8s_api_ca_crt.is_none()
        || config.k8s_api_namespace.is_none()
        || config.k8s_api_token.is_none()
    {
        return Ok(None);
    }
    let k8s_api_ca_crt = k8s_api_ca_crt.unwrap();
    let k8s_api_url = config.k8s_api_url.as_ref().unwrap().to_owned();
    let k8s_api_namespace = config.k8s_api_namespace.as_ref().unwrap().to_owned();
    let k8s_api_token = config.k8s_api_token.as_ref().unwrap().to_owned();
    let kubeconfig = Kubeconfig {
        clusters: vec![NamedCluster {
            name: "k8s".into(),
            cluster: Cluster {
                server: k8s_api_url,
                insecure_skip_tls_verify: Some(false),
                certificate_authority: None,
                certificate_authority_data: Some(k8s_api_ca_crt),
                proxy_url: None,
                extensions: None,
            },
        }],
        auth_infos: vec![NamedAuthInfo {
            name: "k8s".into(),
            auth_info: AuthInfo {
                token: Some(k8s_api_token.into()),
                ..Default::default()
            },
        }],
        contexts: vec![NamedContext {
            name: "k8s".into(),
            context: KubeContext {
                cluster: "k8s".into(),
                user: "k8s".into(),
                namespace: Some(k8s_api_namespace),
                extensions: None,
            },
        }],
        ..Default::default()
    };
    let kubeconfig_options = KubeConfigOptions {
        context: Some("k8s".into()),
        cluster: Some("k8s".into()),
        user: Some("k8s".into()),
    };
    let config = kube::Config::from_custom_kubeconfig(kubeconfig, &kubeconfig_options)
        .await
        .map_err(|why_no_load| {
            tracing::error!(?why_no_load, "Failed to load kube client config from ENV");
            why_no_load
        })
        .map_err(|e| anyhow!(e))?;
    Ok(Some(config))
}

pub async fn create_kube_client(config: &Config) -> anyhow::Result<kube::Client> {
    let given_config = try_given_kubeconfig(config).await?;
    let config = match given_config {
        Some(config) => config,
        None => kube::Config::infer().await?,
    };
    kube::Client::try_from(config).map_err(|e| anyhow!(e))
}

pub async fn fetch_artifact_repository_config(
    client: &kube::Client,
    ns: Option<&str>,
    configmap_name: &str,
) -> anyhow::Result<S3ArtifactRepositoryConfig> {
    let config_maps: Api<ConfigMap> = {
        let client = client.clone();
        match ns {
            Some(ns) => Api::namespaced(client, ns),
            None => Api::default_namespaced(client),
        }
    };
    let secrets: Api<Secret> = {
        let client = client.clone();
        match ns {
            Some(ns) => Api::namespaced(client, ns),
            None => Api::default_namespaced(client),
        }
    };

    let config_map = config_maps
        .get(configmap_name)
        .await
        .context("getting ConfigMap")?;
    if config_map.data.is_none() {
        return Err(anyhow!("Did not find data in the ConfigMap"));
    }
    let config = config_map.data.unwrap();
    let config = config.get("artifactRepository");
    if config.is_none() {
        return Err(anyhow!(
            "Did not find artifactRepository key in ConfigMapData"
        ));
    }
    let config: ArtifactRepositoryConfig = serde_yaml::from_str(config.unwrap())?;
    let access_key_secret = secrets
        .get(&config.s3_config.access_key_secret.name)
        .await
        .context("getting access_key_secret")?;
    let secret_key_secret = secrets
        .get(&config.s3_config.secret_key_secret.name)
        .await
        .context("getting secret_key_secret")?;

    let access_key: String = match access_key_secret.data {
        Some(data) => {
            let key = &config.s3_config.access_key_secret.key;
            match data.get(key) {
                Some(data) => String::from_utf8(data.0.to_owned())
                    .context(format!("Parsing data in key {}", key))?,
                None => {
                    return Err(anyhow!(format!(
                        "Did not find the key {} in the access_key_secret",
                        key
                    )))
                }
            }
        }
        None => return Err(anyhow!("Did not find data in the access_key_secret")),
    };
    let secret_key: String = match secret_key_secret.data {
        Some(data) => {
            let key = &config.s3_config.secret_key_secret.key;
            match data.get(key) {
                Some(data) => String::from_utf8(data.0.to_owned())
                    .context(format!("Parsing data in key {}", key))?,
                None => {
                    return Err(anyhow!(format!(
                        "Did not find the key {} in the secret_key_secret",
                        key
                    )))
                }
            }
        }
        None => return Err(anyhow!("Did not find data in the secret_key_secret")),
    };

    let config = S3ArtifactRepositoryConfig {
        access_key,
        secret_key,
        bucket: config.s3_config.bucket,
        endpoint: config.s3_config.endpoint,
        region: config.s3_config.region,
        insecure: config.s3_config.insecure,
        path_style_endpoint: config.s3_config.path_style_endpoint,
    };
    Ok(config)
}
