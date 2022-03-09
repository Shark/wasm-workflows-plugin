use std::str::FromStr;

pub async fn fetch_oci_image(name: &str, allowed_insecure: Vec<String>) -> anyhow::Result<Vec<u8>> {
    // Implementation kind-of based upon https://github.com/wasmCloud/wasmcloud-otp/blob/f6ae5c50a3c3cb2d0b923b6d92641c4b7f1d1d73/host_core/native/hostcore_wasmcloud_native/src/oci.rs
    // TODO add caching
    // TODO add auth support
    let img = oci_distribution::Reference::from_str(name)?;
    let auth = oci_distribution::secrets::RegistryAuth::Anonymous;
    let protocol = oci_distribution::client::ClientProtocol::HttpsExcept(allowed_insecure.to_vec());
    let config = oci_distribution::client::ClientConfig {
        protocol,
        ..Default::default()
    };
    let mut oci_client = oci_distribution::Client::new(config);
    // TODO add pull timeout
    let img_data = oci_client
        .pull(
            &img,
            &auth,
            vec![
                "application/vnd.module.wasm.content.layer.v1+wasm",
                "application/vnd.wasm.content.layer.v1+wasm",
                "application/vnd.oci.image.layer.v1.tar",
            ],
        )
        .await?;
    let content = img_data
        .layers
        .iter()
        .flat_map(|l| l.data.clone())
        .collect::<Vec<_>>();

    Ok(content)
}
