use anyhow::anyhow;
use serde_json::json;
use std::slice::Iter;
use workflow_model::model::{Outputs, Parameter, Phase, PluginInvocation, PluginResult};

fn main() {
    workflow_model::plugin::main(Box::new(run));
}

fn run(invocation: PluginInvocation) -> anyhow::Result<PluginResult> {
    let req_params: RequestInfo = invocation.parameters.try_into()?;

    let mut req = http::request::Builder::new()
        .method(req_params.method)
        .uri(req_params.url);

    if req_params.content_type.is_some() {
        req = req.header("Content-Type".to_string(), req_params.content_type.unwrap())
    }

    let body: Option<bytes::Bytes> = req_params.body.map(|body| body.into());

    let req = req.body(body).unwrap();

    let mut res = wasi_experimental_http::request(req)
        .map_err(|err| anyhow!(err).context("Unspecified error during request"))?;
    let status_code = res.status_code;
    let content_type: Option<String> = match res.header_get("Content-Type".to_string()) {
        Ok(value) => Some(value),
        Err(_) => None,
    };
    let res = res.body_read_all()?;
    let res = std::str::from_utf8(&res)?;
    let res = json!(res);

    let mut parameters = vec![
        Parameter {
            name: "status_code".to_string(),
            value: json!(status_code.as_u16()),
        },
        Parameter {
            name: "body".to_string(),
            value: res,
        },
    ];

    if let Some(content_type) = content_type {
        parameters.push(Parameter {
            name: "content_type".to_string(),
            value: json!(content_type),
        })
    }

    Ok(PluginResult {
        phase: Phase::Succeeded,
        message: "Done".to_string(),
        outputs: Outputs {
            artifacts: vec![],
            parameters,
        },
    })
}

struct RequestInfo {
    url: String,
    method: http::Method,
    content_type: Option<String>,
    body: Option<String>,
}

impl RequestInfo {
    fn handle_url(mut parameters: Iter<Parameter>) -> anyhow::Result<String> {
        let url = match parameters.find(|param| param.name == "url") {
            Some(param) => param,
            None => return Err(anyhow!("Expected url parameter to be given")),
        };
        let url = match url.value.as_str() {
            Some(url) => url,
            None => return Err(anyhow!("Expected url parameter to be a string")),
        };
        Ok(url.to_owned())
    }

    fn handle_method(mut parameters: Iter<Parameter>) -> anyhow::Result<http::Method> {
        let method = match parameters.find(|param| param.name == "method") {
            Some(param) => param,
            None => return Ok(http::Method::GET),
        };
        let method: &str = match method.value.as_str() {
            Some(url) => url,
            None => return Err(anyhow!("Expected method parameter to be a string")),
        };
        Ok(http::Method::from_bytes(method.as_bytes())?)
    }

    fn handle_body(mut parameters: Iter<Parameter>) -> anyhow::Result<Option<String>> {
        let body = match parameters.find(|param| param.name == "body") {
            Some(param) => param,
            None => return Ok(None),
        };
        let body: &str = match body.value.as_str() {
            Some(body) => body,
            None => return Err(anyhow!("Expected body parameter to be a string")),
        };
        Ok(Some(body.to_owned()))
    }

    fn handle_content_type(mut parameters: Iter<Parameter>) -> anyhow::Result<Option<String>> {
        let content_type = match parameters.find(|param| param.name == "content_type") {
            Some(param) => param,
            None => return Ok(None),
        };
        let content_type = match content_type.value.as_str() {
            Some(content_type) => content_type,
            None => return Err(anyhow!("Expected content_type parameter to be a string")),
        };
        Ok(Some(content_type.to_string()))
    }
}

impl TryFrom<Vec<Parameter>> for RequestInfo {
    type Error = anyhow::Error;

    fn try_from(parameters: Vec<Parameter>) -> Result<Self, Self::Error> {
        let method = RequestInfo::handle_method(parameters.iter())?;
        let url = RequestInfo::handle_url(parameters.iter())?;
        let mut body = RequestInfo::handle_body(parameters.iter())?;
        let mut content_type = RequestInfo::handle_content_type(parameters.iter())?;

        if body.is_none() || content_type.is_none() {
            body = None;
            content_type = None;
        }

        Ok(RequestInfo {
            method,
            url,
            body,
            content_type,
        })
    }
}
