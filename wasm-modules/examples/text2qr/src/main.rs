use anyhow::anyhow;
use qrcode::{render::unicode, QrCode};
use workflow_model::model::{Outputs, Parameter, Phase, PluginInvocation, PluginResult};

fn main() {
    workflow_model::plugin::main(Box::new(run));
}

fn run(invocation: PluginInvocation) -> anyhow::Result<PluginResult> {
    let maybe_input_text = invocation
        .parameters
        .iter()
        .find(|param| param.name == "text")
        .and_then(|param| param.value.as_str());
    let input_text = match maybe_input_text {
        Some(text) => text,
        None => return Err(anyhow!("Expected parameter \"text\" to be present")),
    };
    let code = QrCode::new(input_text).unwrap();
    let image = code
        .render::<unicode::Dense1x2>()
        .dark_color(unicode::Dense1x2::Light)
        .light_color(unicode::Dense1x2::Dark)
        .build();
    Ok(PluginResult {
        phase: Phase::Succeeded,
        message: "QR code successfully generated".to_string(),
        outputs: Outputs {
            artifacts: vec![],
            parameters: vec![Parameter {
                name: "qrcode".to_string(),
                value: image.into(),
            }],
        },
    })
}
