use anyhow::anyhow;
use qrcode::{render::unicode, QrCode};
wit_bindgen_rust::export!("../../../src/app/wasm/workflow.wit");

struct ModuleResult {
    pub message: String,
    pub parameters: Vec<Parameter>,
}

struct Parameter {
    pub name: String,
    pub value: serde_json::Value,
}

struct Workflow;

impl workflow::Workflow for Workflow {
    fn invoke(ctx: workflow::Invocation) -> workflow::Node {
        match Workflow::run(ctx) {
            Ok(result) => {
                let out_params: Vec<workflow::Parameter> = result
                    .parameters
                    .into_iter()
                    .map(|param| workflow::Parameter {
                        name: param.name,
                        value_json: param.value.to_string(),
                    })
                    .collect();
                workflow::Node {
                    phase: "Succeeded".to_string(),
                    message: result.message,
                    parameters: out_params,
                }
            }
            Err(err) => workflow::Node {
                phase: "Failed".to_string(),
                message: err.to_string(),
                parameters: vec![],
            },
        }
    }
}

impl Workflow {
    fn run(ctx: workflow::Invocation) -> anyhow::Result<ModuleResult> {
        let maybe_input_text = ctx
            .parameters
            .into_iter()
            .find(|param| param.name == "text")
            .and_then(|param| Some(param.value_json));
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
        return Ok(ModuleResult {
            message: "QR code successfully generated".to_string(),
            parameters: vec![Parameter {
                name: "qrcode".to_string(),
                value: image.into(),
            }],
        });
    }
}
