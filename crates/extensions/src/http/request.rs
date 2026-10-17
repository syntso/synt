use std::time::Duration;

use data_encoding::BASE64;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ureq::http::request::Builder;
use ureq::http::{HeaderName, HeaderValue, Method, Request};

use super::{Header, Revealed, SecretHeader};
use crate::error::HttpError;

const METHODS: [&str; 7] = ["GET", "HEAD", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"];
const READING_METHODS: [&str; 3] = ["GET", "HEAD", "OPTIONS"];
const RESERVED_HEADERS: [&str; 6] = [
    "host",
    "content-length",
    "transfer-encoding",
    "connection",
    "upgrade",
    "proxy-authorization",
];
const MOST_HEADERS: usize = 64;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const LONGEST_TIMEOUT_MS: u32 = 60_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub(crate) struct HttpRequest {
    method: String,
    url: String,

    #[serde(default)]
    headers: Vec<Header>,

    #[serde(default)]
    secrets: Vec<SecretHeader>,

    #[serde(default)]
    body: Option<String>,

    #[serde(default)]
    timeout_ms: Option<u32>,
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
    pub(crate) fn with_headers(self, headers: Vec<Header>, secrets: Vec<SecretHeader>) -> Self {
        Self {
            headers,
            secrets,
            ..self
        }
    }

    #[must_use]
    pub(crate) fn with_body(self, body: Option<&[u8]>) -> Self {
        Self {
            body: body.map(|bytes| BASE64.encode(bytes)),
            ..self
        }
    }

    #[must_use]
    pub(crate) fn with_timeout_ms(self, timeout_ms: Option<u32>) -> Self {
        Self { timeout_ms, ..self }
    }

    pub(crate) fn url(&self) -> &str {
        &self.url
    }

    pub(crate) fn secrets(&self) -> &[SecretHeader] {
        &self.secrets
    }

    pub(crate) fn only_reads(&self) -> bool {
        let method = self.method.to_ascii_uppercase();

        READING_METHODS.contains(&method.as_str())
    }

    pub(crate) fn checked_method(&self) -> Result<Method, HttpError> {
        let upper = self.method.to_ascii_uppercase();
        let refused = || HttpError::Method {
            method: self.method.clone(),
            allowed: METHODS.join(", "),
        };

        if !METHODS.contains(&upper.as_str()) {
            return Err(refused());
        }

        Method::from_bytes(upper.as_bytes()).map_err(|_| refused())
    }

    pub(crate) fn check_headers(&self) -> Result<(), HttpError> {
        if self.headers.len() + self.secrets.len() > MOST_HEADERS {
            return Err(HttpError::TooManyHeaders(MOST_HEADERS));
        }

        self.check_reserved()?;
        self.check_secret_clash()
    }

    pub(crate) fn timeout(&self) -> Result<Duration, HttpError> {
        match self.timeout_ms {
            None => Ok(DEFAULT_TIMEOUT),
            Some(milliseconds) if (1..=LONGEST_TIMEOUT_MS).contains(&milliseconds) => {
                Ok(Duration::from_millis(u64::from(milliseconds)))
            }
            Some(asked) => Err(HttpError::Timeout {
                longest: LONGEST_TIMEOUT_MS,
                asked,
            }),
        }
    }

    pub(crate) fn decoded_body(&self) -> Result<Option<Vec<u8>>, HttpError> {
        self.body
            .as_deref()
            .map(|body| {
                BASE64
                    .decode(body.as_bytes())
                    .map_err(HttpError::BodyNotBase64)
            })
            .transpose()
    }

    pub(crate) fn builder(
        &self,
        method: Method,
        revealed: &[Revealed],
    ) -> Result<Builder, HttpError> {
        let mut builder = Request::builder().method(method).uri(&self.url);

        for header in &self.headers {
            let value = HeaderValue::from_str(header.value())
                .map_err(|_| HttpError::HeaderValue(header.name().to_owned()))?;

            builder = builder.header(Self::header_name(header.name())?, value);
        }

        for secret in revealed {
            let text = secret.header_value()?;
            let mut value = HeaderValue::from_str(&text)
                .map_err(|_| HttpError::SecretHeaderValue(secret.header().to_owned()))?;

            value.set_sensitive(true);
            builder = builder.header(Self::header_name(secret.header())?, value);
        }

        Ok(builder)
    }

    fn check_reserved(&self) -> Result<(), HttpError> {
        let names = self
            .headers
            .iter()
            .map(Header::name)
            .chain(self.secrets.iter().map(SecretHeader::name));

        for name in names {
            let lower = name.to_ascii_lowercase();

            if RESERVED_HEADERS.contains(&lower.as_str()) {
                return Err(HttpError::ReservedHeader(name.to_owned()));
            }
        }

        Ok(())
    }

    fn check_secret_clash(&self) -> Result<(), HttpError> {
        let secret_names: Vec<String> = self
            .secrets
            .iter()
            .map(|secret| secret.name().to_ascii_lowercase())
            .collect();

        let clash = self
            .headers
            .iter()
            .find(|header| secret_names.contains(&header.name().to_ascii_lowercase()));

        match clash {
            Some(header) => Err(HttpError::SecretClash(header.name().to_owned())),
            None => Ok(()),
        }
    }

    fn header_name(name: &str) -> Result<HeaderName, HttpError> {
        HeaderName::from_bytes(name.as_bytes()).map_err(|_| HttpError::HeaderName(name.to_owned()))
    }
}
