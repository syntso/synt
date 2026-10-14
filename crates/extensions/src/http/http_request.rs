use data_encoding::BASE64;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Header, SecretHeader};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct HttpRequest {
    pub(crate) method: String,
    pub(crate) url: String,

    #[serde(default)]
    pub(crate) headers: Vec<Header>,

    #[serde(default)]
    pub(crate) secrets: Vec<SecretHeader>,

    #[serde(default)]
    pub(crate) body: Option<String>,

    #[serde(default)]
    pub(crate) timeout_ms: Option<u32>,
}

impl HttpRequest {
    pub(crate) fn new(method: String, url: String) -> Self {
        Self {
            method,
            url,
            headers: Vec::new(),
            secrets: Vec::new(),
            body: None,
            timeout_ms: None,
        }
    }

    #[must_use]
    pub(crate) fn body_bytes(mut self, body: Option<&[u8]>) -> Self {
        self.body = body.map(|bytes| BASE64.encode(bytes));
        self
    }

    pub(crate) fn decoded_body(&self) -> Result<Option<Vec<u8>>, String> {
        self.body
            .as_deref()
            .map(|body| {
                BASE64
                    .decode(body.as_bytes())
                    .map_err(|error| format!("the body is not base64: {error}"))
            })
            .transpose()
    }
}
