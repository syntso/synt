use data_encoding::BASE64;
use serde::{Deserialize, Serialize};
use ureq::Body;
use ureq::http::Response;

use super::{Header, Revealed};
use crate::ExtensionError;

const LARGEST_BODY: u64 = 8 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct HttpResponse {
    status: u16,
    headers: Vec<Header>,
    body: String,
}

impl HttpResponse {
    pub(crate) fn received(
        mut response: Response<Body>,
        revealed: &[Revealed],
    ) -> Result<Self, ExtensionError> {
        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .map(|(name, value)| {
                let text = String::from_utf8_lossy(value.as_bytes());

                Header::new(name.as_str().to_owned(), Self::scrubbed(&text, revealed))
            })
            .collect();

        let body = response
            .body_mut()
            .with_config()
            .limit(LARGEST_BODY)
            .read_to_vec()
            .map_err(ExtensionError::ResponseBody)?;
        let body = revealed
            .iter()
            .fold(body, |body, secret| secret.scrub(&body));

        Ok(Self {
            status,
            headers,
            body: BASE64.encode(&body),
        })
    }

    pub(crate) fn status(&self) -> u16 {
        self.status
    }

    pub(crate) fn headers(&self) -> &[Header] {
        &self.headers
    }

    pub(crate) fn decoded_body(&self) -> Result<Vec<u8>, ExtensionError> {
        BASE64
            .decode(self.body.as_bytes())
            .map_err(ExtensionError::ResponseNotBase64)
    }

    pub(crate) fn body_size(&self) -> usize {
        self.decoded_body().map_or(0, |body| body.len())
    }

    fn scrubbed(text: &str, revealed: &[Revealed]) -> String {
        revealed
            .iter()
            .fold(text.to_owned(), |text, secret| secret.scrub_text(&text))
    }
}
