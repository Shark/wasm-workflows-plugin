wit_bindgen_rust::export!("../src/app/wasm/workflow.wit");

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
        let mut in_params: Vec<Parameter> = Vec::new();
        for ctx_param in ctx.parameters {
            let parsed_value_json = serde_json::from_str(&ctx_param.value_json).map_err(
                |err| anyhow::Error::new(err).context(format!("Failed parsing \"{}\"", ctx_param.value_json))
            )?;
            in_params.push(Parameter {
                name: ctx_param.name,
                value: parsed_value_json,
            });
        }
        return Ok(ModuleResult {
            message: format!("Hello from Wasm OCI Registry! Got {} input params.", in_params.len()),
            parameters: vec![
                Parameter {
                    name: "wasm-out-param".to_string(),
                    value: serde_json::Number::from(2342).into(),
                }
            ]
        })
    }
}
