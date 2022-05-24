use serde::Deserialize;
use std::fmt::Debug;

#[derive(Deserialize, PartialEq, Debug)]
pub struct ArtifactRepositoryConfig {
    #[serde(rename = "s3")]
    s3_config: S3Config,
}

#[derive(Deserialize, PartialEq, Debug)]
struct S3Config {
    #[serde(rename = "accessKeySecret")]
    access_key_secret: SecretRef,
    #[serde(rename = "secretKeySecret")]
    secret_key_secret: SecretRef,
    insecure: bool,
    bucket: String,
    endpoint: String,
    region: String,
}

#[derive(Deserialize, PartialEq, Debug)]
struct SecretRef {
    name: String,
    key: String,
}

#[cfg(test)]
mod tests {
    use crate::app::model::argo::{ArtifactRepositoryConfig, S3Config, SecretRef};
    use indoc::indoc;
    use std::fmt::Debug;

    #[test]
    fn test_deserialize() {
        let yaml = indoc! {"
        archiveLogs: true
        s3:
          accessKeySecret:
            name: argo-workflows-s3
            key: access_key
          secretKeySecret:
            name: argo-workflows-s3
            key: secret_key
          insecure: true
          bucket: argo-workflows
          endpoint: minio.minio.svc.cluster.local:9000
          region: eu-central-1
    "};

        let expected = ArtifactRepositoryConfig {
            s3_config: S3Config {
                access_key_secret: SecretRef {
                    name: "argo-workflows-s3".to_string(),
                    key: "access_key".to_string(),
                },
                secret_key_secret: SecretRef {
                    name: "argo-workflows-s3".to_string(),
                    key: "secret_key".to_string(),
                },
                insecure: true,
                bucket: "argo-workflows".to_string(),
                endpoint: "minio.minio.svc.cluster.local:9000".to_string(),
                region: "eu-central-1".to_string(),
            },
        };
        test_de(yaml, &expected);
    }

    // https://github.com/dtolnay/serde-yaml/blob/master/tests/test_de.rs
    fn test_de<T>(yaml: &str, expected: &T)
    where
        T: serde::de::DeserializeOwned + PartialEq + Debug,
    {
        let deserialized: T = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(*expected, deserialized);

        serde_yaml::from_str::<serde_yaml::Value>(yaml).unwrap();
        serde_yaml::from_str::<serde::de::IgnoredAny>(yaml).unwrap();
    }
}
