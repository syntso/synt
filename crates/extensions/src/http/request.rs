use std::time::Duration;

use data_encoding::BASE64;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ureq::http::request::Builder;
use ureq::http::{HeaderName, HeaderValue, Method, Request};

use super::{Header, Revealed, SecretHeader};

const METHODS: [&str; 7] = ["GET", "HEAD", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"];
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

    pub(crate) fn checked_method(&self) -> Result<Method, String> {
        let upper = self.method.to_ascii_uppercase();

        if !METHODS.contains(&upper.as_str()) {
            return Err(format!(
                "{} is not one of {}",
                self.method,
                METHODS.join(", ")
            ));
        }

        Method::from_bytes(upper.as_bytes()).map_err(|error| error.to_string())
    }

    pub(crate) fn check_headers(&self) -> Result<(), String> {
        if self.headers.len() + self.secrets.len() > MOST_HEADERS {
            return Err(format!("a request carries at most {MOST_HEADERS} headers"));
        }

        self.check_reserved()?;
        self.check_secret_clash()
    }

    pub(crate) fn timeout(&self) -> Result<Duration, String> {
        match self.timeout_ms {
            None => Ok(DEFAULT_TIMEOUT),
            Some(milliseconds) if (1..=LONGEST_TIMEOUT_MS).contains(&milliseconds) => {
                Ok(Duration::from_millis(u64::from(milliseconds)))
            }
            Some(milliseconds) => Err(format!(
                "timeout_ms is 1 to {LONGEST_TIMEOUT_MS}, not {milliseconds}"
            )),
        }
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

    pub(crate) fn builder(&self, method: Method, revealed: &[Revealed]) -> Result<Builder, String> {
        let mut builder = Request::builder().method(method).uri(&self.url);

        for header in &self.headers {
            let name = header_name(&header.name)?;
            let value = HeaderValue::from_str(&header.value).map_err(|_| {
                format!("the {} header has a value that cannot be sent", header.name)
            })?;

            builder = builder.header(name, value);
        }

        for secret in revealed {
            let text = secret.header_value()?;
            let mut value = HeaderValue::from_str(&text)
                .map_err(|_| format!("the {} header cannot carry this secret", secret.header()))?;
            value.set_sensitive(true);

            builder = builder.header(header_name(secret.header())?, value);
        }

        Ok(builder)
    }

    fn check_reserved(&self) -> Result<(), String> {
        let names = self
            .headers
            .iter()
            .map(|header| &header.name)
            .chain(self.secrets.iter().map(|secret| &secret.name));

        for name in names {
            let lower = name.to_ascii_lowercase();

            if RESERVED_HEADERS.contains(&lower.as_str()) {
                return Err(format!("the core sets the {name} header itself"));
            }
        }

        Ok(())
    }

    fn check_secret_clash(&self) -> Result<(), String> {
        let secret_names: Vec<String> = self
            .secrets
            .iter()
            .map(|secret| secret.name.to_ascii_lowercase())
            .collect();

        let clash = self
            .headers
            .iter()
            .find(|header| secret_names.contains(&header.name.to_ascii_lowercase()));

        match clash {
            Some(header) => Err(format!(
                "the {} header is filled from a secret and cannot be set as well",
                header.name
            )),
            None => Ok(()),
        }
    }
}

fn header_name(name: &str) -> Result<HeaderName, String> {
    HeaderName::from_bytes(name.as_bytes()).map_err(|_| format!("{name:?} is not a header name"))
}
