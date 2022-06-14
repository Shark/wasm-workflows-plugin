extern crate photon_rs as photon;

use anyhow::{anyhow, Context};
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;
use tracing::info_span;
use workflow_model::model::{
    ArtifactRef, Outputs, Phase, PluginInvocation, PluginResult, WORKING_DIR_PLUGIN_PATH,
};
use workflow_model::plugin::ArtifactManager;

#[cfg(target_os = "wasi")]
fn main() {
    workflow_model::plugin::main(Box::new(run));
}

#[cfg(not(target_os = "wasi"))]
mod cli;

#[cfg(not(target_os = "wasi"))]
fn main() {
    let invocation = cli::initialize();
    let artifact_manager = ArtifactManager::new_with_base_path(std::env::current_dir().unwrap());
    let result = info_span!("processor.run").in_scope(|| run(invocation, artifact_manager));
    match result {
        Ok(_) => {}
        Err(why) => {
            panic!("Error occurred: {:?}", why);
        }
    }
    opentelemetry::global::shutdown_tracer_provider();
}

fn run(
    invocation: PluginInvocation,
    artifact_manager: ArtifactManager,
) -> anyhow::Result<PluginResult> {
    let input_artifact = invocation
        .artifacts
        .iter()
        .find(|artifact| artifact.name == "input");
    if input_artifact.is_none() {
        return Err(anyhow!("Artifact 'input' not present but required"));
    }
    let input_artifact = input_artifact.unwrap();

    let watermark_artifact = invocation
        .artifacts
        .iter()
        .find(|artifact| artifact.name == "watermark");

    let img_bytes = fs::read(artifact_manager.input_artifact_path(input_artifact))
        .context("Reading input artifact")?;
    let mut img =
        photon::native::open_image_from_bytes(&img_bytes).context("Converting input to image")?;

    // Apply effect
    let effect_param = invocation
        .parameters
        .iter()
        .find(|param| param.name == "effect");
    if let Some(effect_param) = effect_param {
        let effect: Option<Effect> = match effect_param.value.as_str() {
            Some(str) => Some(Effect::from_str(str).context("Parsing effect")?),
            None => None,
        };
        if let Some(e) = effect {
            photon::colour_spaces::hsl(&mut img, &e.to_string(), 0.2_f32);
        }
    }

    // Apply watermark
    if let Some(watermark_artifact) = watermark_artifact {
        let watermark_bytes = fs::read(artifact_manager.input_artifact_path(watermark_artifact))
            .context("Reading watermark artifact")?;
        let watermark = photon::native::open_image_from_bytes(&watermark_bytes)
            .context("Converting watermark to image")?;
        let x = img.get_width() - watermark.get_width();
        let y = img.get_height() - watermark.get_height();
        photon::multiple::watermark(&mut img, &watermark, x, y);
    }

    let output = ArtifactRef {
        name: "output".to_string(),
        path: "".to_string(),
        s3: None,
    };
    let output_path = format!(
        "{}.jpg",
        artifact_manager
            .output_artifact_path(&output)
            .to_str()
            .unwrap()
    );
    photon::native::save_image(img, &output_path).context("Saving output artifact")?;

    let artifact_path = artifact_manager.output_artifact_path(&output);
    fs::rename(output_path, artifact_path).context("Moving output artifact")?;

    Ok(PluginResult {
        phase: Phase::Succeeded,
        message: "Image processed".to_string(),
        outputs: Outputs {
            artifacts: vec![output],
            ..Default::default()
        },
    })
}

enum Effect {
    Saturate,
    Desaturate,
    ShiftHue,
    Darken,
    Lighten,
}

impl FromStr for Effect {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "saturate" => Ok(Self::Saturate),
            "desaturate" => Ok(Self::Desaturate),
            "shift_hue" => Ok(Self::ShiftHue),
            "darken" => Ok(Self::Darken),
            "lighten" => Ok(Self::Lighten),
            _ => Err(anyhow!(format!("{} is not a valid effect", s))),
        }
    }
}

impl ToString for Effect {
    fn to_string(&self) -> String {
        match self {
            Self::Saturate => "saturate".into(),
            Self::Desaturate => "desaturate".into(),
            Self::ShiftHue => "shift_hue".into(),
            Self::Darken => "darken".into(),
            Self::Lighten => "lighten".into(),
        }
    }
}
