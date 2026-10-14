use data_encoding::BASE64;
use serde::{Deserialize, Serialize};
use ureq::Body;
use ureq::http::Response;

use super::{Header, Revealed};

const LARGEST_BODY: u64 = 8 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct HttpResponse {
    pub(crate) status: u16,
    pub(crate) headers: Vec<Header>,
    pub(crate) body: String,
}

impl HttpResponse {
    pub(crate) fn received(
        mut response: Response<Body>,
        revealed: &[Revealed],
    ) -> Result<Self, String> {
        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .map(|(name, value)| Header {
                name: name.as_str().to_owned(),
                value: scrubbed_text(&String::from_utf8_lossy(value.as_bytes()), revealed),
            })
            .collect();

        let body = response
            .body_mut()
            .with_config()
            .limit(LARGEST_BODY)
            .read_to_vec()
            .map_err(|error| format!("the response body: {error}"))?;
        let body = revealed
            .iter()
            .fold(body, |body, secret| secret.scrub(&body));

        Ok(Self {
            status,
            headers,
            body: BASE64.encode(&body),
        })
    }

    pub(crate) fn decoded_body(&self) -> Result<Vec<u8>, String> {
        BASE64
            .decode(self.body.as_bytes())
            .map_err(|error| format!("the response body is not base64: {error}"))
    }

    pub(crate) fn body_size(&self) -> usize {
        self.decoded_body().map_or(0, |body| body.len())
    }
}

fn scrubbed_text(text: &str, revealed: &[Revealed]) -> String {
    revealed
        .iter()
        .fold(text.to_owned(), |text, secret| secret.scrub_text(&text))
}
