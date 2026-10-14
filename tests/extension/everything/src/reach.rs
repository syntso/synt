use synt_extension_api::http::{self, Request};
use synt_extension_api::serde_json::{self, Value, json};
use synt_extension_api::{Error, call};

use crate::params::Params;

pub fn ping() -> Result<Value, Error> {
    let answer = call("core.ping", None, "{}")?;

    parse(&answer)
}

pub fn fetch(params: &Params) -> Result<Value, Error> {
    let method = params.optional("method").unwrap_or("GET");
    let mut request = Request::new(method, params.text("url")?).header("Accept", "text/plain");

    if let Some(secret) = params.optional("secret") {
        request = request.secret_header("Authorization", secret, "Bearer ");
    }

    if let Some(body) = params.optional("body") {
        request = request.body(body);
    }

    let response = http::send(request)?;

    Ok(json!({
        "status": response.status(),
        "body": response.text(),
        "echoed": response.header("x-echo-authorization"),
    }))
}

pub fn settings() -> Result<Value, Error> {
    parse(&synt_extension_api::settings())
}

fn parse(text: &str) -> Result<Value, Error> {
    serde_json::from_str(text).map_err(|error| Error::new("unexpected", error.to_string()))
}
