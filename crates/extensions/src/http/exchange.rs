use std::time::Duration;

use ureq::http::{HeaderName, HeaderValue, Method, Request, Response};
use ureq::{Agent, AsSendBody, Body, RequestExt};

use super::{Destination, Header, HttpRequest, HttpResponse, Revealed, SecretSource};
use crate::manifest::Capabilities;
use crate::tls::trusted_tls;

const METHODS: [&str; 7] = ["GET", "HEAD", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"];
const RESERVED_HEADERS: [&str; 6] = [
    "host",
    "content-length",
    "transfer-encoding",
    "connection",
    "upgrade",
    "proxy-authorization",
];
const LARGEST_BODY: u64 = 8 * 1024 * 1024;
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const LONGEST_TIMEOUT_MS: u32 = 60_000;
const MOST_HEADERS: usize = 64;

pub(crate) struct Exchange {
    agent: Agent,
}

impl Exchange {
    pub(crate) fn new() -> Self {
        let agent = Agent::config_builder()
            .tls_config(trusted_tls())
            .max_redirects(0)
            .http_status_as_error(false)
            .proxy(None)
            .build()
            .new_agent();

        Self { agent }
    }

    pub(crate) fn send(
        &self,
        request: &HttpRequest,
        capabilities: &Capabilities,
        secrets: Option<&dyn SecretSource>,
    ) -> Result<HttpResponse, String> {
        let method = method(&request.method)?;
        let destination = reachable(&request.url, capabilities)?;
        check_headers(request)?;

        let revealed = reveal(request, &destination, capabilities, secrets)?;
        let built = build(request, method, &revealed)?;
        let timeout = timeout(request.timeout_ms)?;

        let body = request.decoded_body()?;
        let response = match body {
            Some(body) => run(&self.agent, built.body(body), timeout),
            None => run(&self.agent, built.body(()), timeout),
        }
        .map_err(|error| format!("{}: {error}", request.url))?;

        answer(response, &revealed)
    }
}

fn method(method: &str) -> Result<Method, String> {
    let upper = method.to_ascii_uppercase();

    if !METHODS.contains(&upper.as_str()) {
        return Err(format!("{method} is not one of {}", METHODS.join(", ")));
    }

    Method::from_bytes(upper.as_bytes()).map_err(|error| error.to_string())
}

fn reachable(url: &str, capabilities: &Capabilities) -> Result<Destination, String> {
    let destination = Destination::parse(url)?;

    if !destination.is_listed_in(capabilities) {
        return Err(format!(
            "{} is not listed in capabilities.http",
            destination.listed()
        ));
    }

    Ok(destination)
}

fn check_headers(request: &HttpRequest) -> Result<(), String> {
    if request.headers.len() + request.secrets.len() > MOST_HEADERS {
        return Err(format!("a request carries at most {MOST_HEADERS} headers"));
    }

    let names = request
        .headers
        .iter()
        .map(|header| &header.name)
        .chain(request.secrets.iter().map(|secret| &secret.name));

    for name in names {
        let lower = name.to_ascii_lowercase();

        if RESERVED_HEADERS.contains(&lower.as_str()) {
            return Err(format!("the core sets the {name} header itself"));
        }
    }

    let secret_names: Vec<String> = request
        .secrets
        .iter()
        .map(|secret| secret.name.to_ascii_lowercase())
        .collect();

    match request
        .headers
        .iter()
        .find(|header| secret_names.contains(&header.name.to_ascii_lowercase()))
    {
        Some(header) => Err(format!(
            "the {} header is filled from a secret and cannot be set as well",
            header.name
        )),
        None => Ok(()),
    }
}

fn reveal(
    request: &HttpRequest,
    destination: &Destination,
    capabilities: &Capabilities,
    secrets: Option<&dyn SecretSource>,
) -> Result<Vec<Revealed>, String> {
    if request.secrets.is_empty() {
        return Ok(Vec::new());
    }

    let source = secrets.ok_or("the core keeps no secrets to attach")?;

    if !destination.keeps_secrets_private() {
        return Err(format!(
            "secrets go only over https or to this computer, not to http://{}",
            destination.listed()
        ));
    }

    request
        .secrets
        .iter()
        .map(|attached| {
            if !destination.may_carry(&attached.secret, capabilities) {
                return Err(format!(
                    "capabilities.secrets does not let secret://{} go to {}",
                    attached.secret,
                    destination.listed()
                ));
            }

            let value = source
                .reveal(&attached.secret)?
                .ok_or_else(|| format!("there is no secret secret://{}", attached.secret))?;

            Ok(Revealed::new(attached, value))
        })
        .collect()
}

fn build(
    request: &HttpRequest,
    method: Method,
    revealed: &[Revealed],
) -> Result<ureq::http::request::Builder, String> {
    let mut builder = Request::builder().method(method).uri(&request.url);

    for header in &request.headers {
        let name = header_name(&header.name)?;
        let value = HeaderValue::from_str(&header.value)
            .map_err(|_| format!("the {} header has a value that cannot be sent", header.name))?;

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

fn run(
    agent: &Agent,
    request: Result<Request<impl AsSendBody>, ureq::http::Error>,
    timeout: Duration,
) -> Result<Response<Body>, String> {
    let request = request.map_err(|error| error.to_string())?;

    request
        .with_agent(agent)
        .configure()
        .timeout_global(Some(timeout))
        .run()
        .map_err(|error| error.to_string())
}

fn answer(mut response: Response<Body>, revealed: &[Revealed]) -> Result<HttpResponse, String> {
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

    Ok(HttpResponse::new(status, headers, &body))
}

fn scrubbed_text(text: &str, revealed: &[Revealed]) -> String {
    revealed
        .iter()
        .fold(text.to_owned(), |text, secret| secret.scrub_text(&text))
}

fn header_name(name: &str) -> Result<HeaderName, String> {
    HeaderName::from_bytes(name.as_bytes()).map_err(|_| format!("{name:?} is not a header name"))
}

fn timeout(milliseconds: Option<u32>) -> Result<Duration, String> {
    match milliseconds {
        None => Ok(DEFAULT_TIMEOUT),
        Some(milliseconds) if (1..=LONGEST_TIMEOUT_MS).contains(&milliseconds) => {
            Ok(Duration::from_millis(u64::from(milliseconds)))
        }
        Some(milliseconds) => Err(format!(
            "timeout_ms is 1 to {LONGEST_TIMEOUT_MS}, not {milliseconds}"
        )),
    }
}
