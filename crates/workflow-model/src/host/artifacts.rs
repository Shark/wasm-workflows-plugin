use crate::host::WorkingDir;
use crate::model::{
    ArtifactRef, S3Artifact, S3ArtifactRepositoryConfig, INPUT_ARTIFACTS_PATH,
    OUTPUT_ARTIFACTS_PATH,
};
use anyhow::{anyhow, Context};
use awscreds::Credentials;
use rand::Rng;
use s3::{Bucket, Region};

pub struct ArtifactManager {
    bucket: Bucket,
}

impl ArtifactManager {
    pub fn try_new(config: S3ArtifactRepositoryConfig) -> anyhow::Result<Self> {
        let credentials = Credentials::new(
            Some(&config.access_key),
            Some(&config.secret_key),
            None,
            None,
            None,
        )?;
        let protocol = match &config.insecure {
            true => "http",
            false => "https",
        };
        let endpoint = format!("{}://{}", protocol, config.endpoint);
        let region = Region::Custom {
            region: config.region,
            endpoint,
        };
        let bucket = Bucket::new(&config.bucket, region, credentials)?;
        let bucket = match config.path_style_endpoint {
            true => bucket.with_path_style(),
            false => bucket,
        };
        Ok(Self { bucket })
    }

    #[tracing::instrument(name = "artifact.download", level = "debug", skip(self))]
    pub async fn download(&self, wd: &WorkingDir, artifact: &ArtifactRef) -> anyhow::Result<()> {
        let s3_key: &str = match &artifact.s3 {
            Some(s3) => s3.key.as_ref(),
            None => {
                return Err(anyhow!(format!(
                    "Expected artifact {} to be on s3",
                    &artifact.name
                )))
            }
        };
        let path = wd.path().join(INPUT_ARTIFACTS_PATH);
        let path = path.join(artifact.working_dir_path());
        let mut output_file = tokio::fs::File::create(&path).await.context(format!(
            "Creating file for input artifact {} at {:?}",
            &artifact.name, &path
        ))?;
        tracing::debug!("Downloading {} to {:?}", s3_key, &path);
        let status_code = self
            .bucket
            .get_object_stream(s3_key, &mut output_file)
            .await
            .context(format!(
                "Getting object for input artifact {} at {:?}",
                &artifact.name, &path
            ))?;
        match status_code {
            200 => Ok(()),
            code => Err(anyhow!(format!("Unexpected status code {} != 200", code))),
        }
    }

    #[tracing::instrument(name = "artifact.upload", level = "debug", skip(self))]
    pub async fn upload(
        &self,
        wd: &WorkingDir,
        workflow_name: &str,
        artifact: &ArtifactRef,
    ) -> anyhow::Result<ArtifactRef> {
        let path = wd.path().join(OUTPUT_ARTIFACTS_PATH);
        let path = path.join(artifact.working_dir_path());
        let mut input_file = tokio::fs::File::open(&path).await.context(format!(
            "Opening file for output artifact {} at {:?}",
            &artifact.name, &path
        ))?;
        let key = format!(
            "{}/{}-{}/{}",
            workflow_name,
            workflow_name,
            artifact_random_number(),
            &artifact.name
        );
        let status_code = self
            .bucket
            .put_object_stream(&mut input_file, &key)
            .await
            .context("Putting object for output artifact {} at key {}")?;
        match status_code {
            200 => {
                let mut artifact = artifact.clone();
                artifact.s3 = Some(S3Artifact {
                    key: key.to_owned(),
                });
                Ok(artifact)
            }
            code => Err(anyhow!(format!("Unexpected status code {} != 200", code))),
        }
    }
}

// https://www.reddit.com/r/learnrust/comments/lnewid/comment/go021w2
fn artifact_random_number() -> i64 {
    let num_digits = 10;
    let p = 10i64.pow(num_digits - 1);
    rand::thread_rng().gen_range(1000000000..10 * p)
}
