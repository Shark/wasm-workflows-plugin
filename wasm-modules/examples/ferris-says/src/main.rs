use anyhow::anyhow;
use std::io::BufWriter;
use workflow_model::model::{Outputs, Parameter, Phase, PluginInvocation, PluginResult};
use workflow_model::plugin::ArtifactManager;

fn main() {
    workflow_model::plugin::main(Box::new(run));
}

fn run(invocation: PluginInvocation, _artifacts: ArtifactManager) -> anyhow::Result<PluginResult> {
    let maybe_input_text = invocation
        .parameters
        .iter()
        .find(|param| param.name == "text")
        .and_then(|param| param.value.as_str());
    let input_text = match maybe_input_text {
        Some(text) => text,
        None => return Err(anyhow!("Expected parameter \"text\" to be present")),
    };
    let mut buf = BufWriter::new(Vec::new());
    let text_width = 24;
    ferris_says::say(input_text.as_bytes(), text_width, &mut buf)?;
    let bytes = buf.into_inner()?;
    let output_text = String::from_utf8(bytes)?;
    Ok(PluginResult {
        phase: Phase::Succeeded,
        message: format!("Success, have {} parameters", invocation.parameters.len()),
        outputs: Outputs {
            artifacts: vec![],
            parameters: vec![Parameter {
                name: "text".to_string(),
                value: output_text.into(),
            }],
        },
    })
}
