use crate::app::config::Config;
use anyhow::anyhow;
use kube_client::config::{
    AuthInfo, Cluster, Context, KubeConfigOptions, Kubeconfig, NamedAuthInfo, NamedCluster,
    NamedContext,
};

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
            context: Context {
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
