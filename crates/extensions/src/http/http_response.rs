use data_encoding::BASE64;
use serde::{Deserialize, Serialize};

use super::Header;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct HttpResponse {
    pub(crate) status: u16,
    pub(crate) headers: Vec<Header>,
    pub(crate) body: String,
}

impl HttpResponse {
    pub(crate) fn new(status: u16, headers: Vec<Header>, body: &[u8]) -> Self {
        Self {
            status,
            headers,
            body: BASE64.encode(body),
        }
    }

    pub(crate) fn decoded_body(&self) -> Result<Vec<u8>, String> {
        BASE64
            .decode(self.body.as_bytes())
            .map_err(|error| format!("the response body is not base64: {error}"))
    }
}
