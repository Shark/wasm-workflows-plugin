use std::io::BufWriter;
use anyhow::anyhow;

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
                let out_params: Vec<workflow::Parameter> = result.parameters.into_iter().map(
                    |param| workflow::Parameter {
                        name: param.name,
                        value_json: param.value.to_string(),
                    }
                ).collect();
                workflow::Node {
                    phase: "Succeeded".to_string(),
                    message: result.message,
                    parameters: out_params,
                }
            },
            Err(err) => workflow::Node {
                phase: "Failed".to_string(),
                message: err.to_string(),
                parameters: vec![],
            }
        }
    }
}

impl Workflow {
    fn run(ctx: workflow::Invocation) -> anyhow::Result<ModuleResult> {
        let maybe_input_text = ctx.parameters
            .into_iter()
            .find(|param| param.name == "text")
            .and_then(|param| Some(param.value_json));
        let input_text = match maybe_input_text {
            Some(text) => text,
            None => return Err(anyhow!("Expected parameter \"text\" to be present")),
        };
        let mut buf = BufWriter::new(Vec::new());
        let text_width = 24;
        ferris_says::say(input_text.as_bytes(), text_width, &mut buf)?;
        let bytes = buf.into_inner()?;
        let output_text = String::from_utf8(bytes)?;
        return Ok(ModuleResult {
            message: "Conversion successful".to_string(),
            parameters: vec![
                Parameter {
                    name: "text".to_string(),
                    value: output_text.into(),
                }
            ]
        })
    }
}
