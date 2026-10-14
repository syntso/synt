use std::sync::{Arc, OnceLock};

use data_encoding::BASE64;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use super::blocking::blocking;
use super::guard::calling_extension;
use crate::http::{Exchange, HttpRequest, HttpResponse};
use crate::{ExtensionError, Extensions};

const READING_METHODS: [&str; 3] = ["GET", "HEAD", "OPTIONS"];
const HIDDEN: &str = "?";

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
            .map(decoded_length);

        json!({
            "method": params.get("method"),
            "url": params.get("url").and_then(Value::as_str).map(recorded_url),
            "headers": headers,
            "secrets": secrets,
            "body_bytes": body_bytes,
            "timeout_ms": params.get("timeout_ms"),
        })
    }

    fn summary(&self, output: &HttpResponse) -> Value {
        json!({ "status": output.status, "body_bytes": decoded_length(&output.body) })
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
                .send(
                    &params,
                    &capabilities,
                    secrets
                        .as_deref()
                        .map(|source| (source, extension.as_str())),
                )
                .map_err(ExtensionError::Refused)
        })
        .await
    }
}

fn decoded_length(base64: &str) -> usize {
    BASE64
        .decode(base64.as_bytes())
        .map_or(0, |bytes| bytes.len())
}

fn list<'v>(params: &'v Value, field: &str) -> &'v [Value] {
    params
        .get(field)
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

fn recorded_url(url: &str) -> String {
    let without_fragment = url.split('#').next().unwrap_or_default();
    let (place, query) = match without_fragment.split_once('?') {
        Some((place, query)) => (place, Some(query)),
        None => (without_fragment, None),
    };

    let place = without_user(place);

    let Some(query) = query else {
        return place;
    };

    let hidden: Vec<String> = query
        .split('&')
        .map(|pair| match pair.split_once('=') {
            Some((name, _)) => format!("{name}={HIDDEN}"),
            None => HIDDEN.to_owned(),
        })
        .collect();

    format!("{place}?{}", hidden.join("&"))
}

fn without_user(place: &str) -> String {
    let Some((scheme, rest)) = place.split_once("://") else {
        return place.to_owned();
    };

    let (authority, path) = rest.split_at(rest.find('/').unwrap_or(rest.len()));

    match authority.rsplit_once('@') {
        Some((_, host)) => format!("{scheme}://{HIDDEN}@{host}{path}"),
        None => place.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::recorded_url;

    #[test]
    fn query_values_are_hidden_and_their_names_kept() {
        assert_eq!(
            recorded_url(
                "https://api.example.com/v1/search?q=ann%40example.com&api_key=abc123&flag"
            ),
            "https://api.example.com/v1/search?q=?&api_key=?&?"
        );
    }

    #[test]
    fn a_plain_url_is_kept_and_a_fragment_dropped() {
        assert_eq!(
            recorded_url("https://api.example.com:8443/v1/health"),
            "https://api.example.com:8443/v1/health"
        );
        assert_eq!(
            recorded_url("https://api.example.com/page#token=abc"),
            "https://api.example.com/page"
        );
    }

    #[test]
    fn credentials_in_the_address_are_hidden() {
        assert_eq!(
            recorded_url("https://ann:hunter2@api.example.com/v1?x=1"),
            "https://?@api.example.com/v1?x=?"
        );
    }
}
