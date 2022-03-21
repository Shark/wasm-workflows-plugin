use anyhow::anyhow;
use serde_json::json;
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
                message: format!("{:?}", err.context("Error during module execution")),
                parameters: vec![],
            },
        }
    }
}

impl Workflow {
    fn run(ctx: workflow::Invocation) -> anyhow::Result<ModuleResult> {
        let url = ctx.parameters.into_iter().find(|param| param.name == "url");

        if url.is_none() {
            return Err(anyhow!(format!("Expected url parameter to be given")));
        }
        let url = url.unwrap();

        let url: serde_json::Value = serde_json::from_str(&url.value_json).map_err(|err| {
            anyhow!(err).context(format!("Failed parsing \"{}\"", url.value_json))
        })?;
        let url = url.as_str();
        if url.is_none() {
            return Err(anyhow!(format!("Expected url parameter to be a string")));
        }
        let url = url.unwrap();

        let req = http::request::Builder::new()
            .method(http::Method::GET)
            .uri(url)
            .body(Some(bytes::Bytes::from(&b""[..])))
            .unwrap();
        let mut res = wasi_experimental_http::request(req)
            .map_err(|err| anyhow!(err).context("Made HTTP request"))?;
        let res = res.body_read_all()?;
        let res = std::str::from_utf8(&res)?;
        let res = json!(res);

        return Ok(ModuleResult {
            message: "Done".to_string(),
            parameters: vec![Parameter {
                name: "response".to_string(),
                value: res,
            }],
        });
    }
}
