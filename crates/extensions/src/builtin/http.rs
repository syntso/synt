use std::sync::{Arc, OnceLock};

use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use super::blocking::blocking;
use super::guard::calling_extension;
use crate::http::{Exchange, HttpRequest, HttpResponse};
use crate::{ExtensionError, Extensions};

const READING_METHODS: [&str; 3] = ["GET", "HEAD", "OPTIONS"];

#[derive(Clone)]
pub(crate) struct ExtensionHttp {
    extensions: Extensions,
    exchange: Arc<OnceLock<Exchange>>,
}

impl ExtensionHttp {
    pub(crate) fn new(extensions: Extensions) -> Self {
        Self {
            extensions,
            exchange: Arc::new(OnceLock::new()),
        }
    }
}

impl Action for ExtensionHttp {
    const ID: &'static str = "extension.http";
    const TITLE: &'static str = "Send an HTTP request for an extension to a host it lists";
    const RISK: Risk = Risk::Read;

    type Params = HttpRequest;
    type Output = HttpResponse;

    fn risk(&self, _target: Option<&ResourceRef>, params: &HttpRequest) -> Risk {
        let method = params.method.to_ascii_uppercase();

        if READING_METHODS.contains(&method.as_str()) {
            return Risk::Read;
        }

        Risk::Write
    }

    fn recorded_params(&self, params: &Value) -> Value {
        let headers: Vec<&Value> = list(params, "headers")
            .iter()
            .filter_map(|header| header.get("name"))
            .collect();
        let secrets: Vec<Value> = list(params, "secrets")
            .iter()
            .map(|secret| json!({ "name": secret.get("name"), "secret": secret.get("secret") }))
            .collect();
        let body_bytes = params
            .get("body")
            .and_then(Value::as_str)
            .map(|body| body.len() / 4 * 3);

        json!({
            "method": params.get("method"),
            "url": params.get("url"),
            "headers": headers,
            "secrets": secrets,
            "body_bytes": body_bytes,
            "timeout_ms": params.get("timeout_ms"),
        })
    }

    fn summary(&self, output: &HttpResponse) -> Value {
        json!({ "status": output.status, "body_bytes": output.body.len() / 4 * 3 })
    }

    async fn run(
        &self,
        context: &Context,
        params: HttpRequest,
    ) -> Result<HttpResponse, ActionError> {
        let extension = calling_extension(context)?;
        let loaded = self
            .extensions
            .loaded(&extension)
            .map_err(|error| ActionError::failed(error.to_string()))?;

        let capabilities = loaded.guest.manifest().capabilities.clone();
        let secrets = self.extensions.services().secrets.get().cloned();
        let exchange = Arc::clone(&self.exchange);

        blocking(move || {
            exchange
                .get_or_init(Exchange::new)
                .send(&params, &capabilities, secrets.as_deref())
                .map_err(ExtensionError::Refused)
        })
        .await
    }
}

fn list<'v>(params: &'v Value, field: &str) -> &'v [Value] {
    params
        .get(field)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}
