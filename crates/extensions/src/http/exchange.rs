use std::time::Duration;

use ureq::http::request::Builder;
use ureq::http::{Request, Response};
use ureq::unversioned::transport::DefaultConnector;
use ureq::{Agent, AsSendBody, Body, RequestExt};

use super::{
    CheckedResolver, Destination, HttpRequest, HttpResponse, Revealed, SecretAccess, trusted_tls,
};
use crate::manifest::Capabilities;

pub(crate) struct Exchange {
    agent: Agent,
}

impl Exchange {
    pub(crate) fn new() -> Self {
        let config = Agent::config_builder()
            .tls_config(trusted_tls())
            .max_redirects(0)
            .http_status_as_error(false)
            .proxy(None)
            .build();

        let agent = Agent::with_parts(config, DefaultConnector::new(), CheckedResolver::new());

        Self { agent }
    }

    pub(crate) fn send(
        &self,
        request: &HttpRequest,
        capabilities: &Capabilities,
        secrets: Option<&SecretAccess<'_>>,
    ) -> Result<HttpResponse, String> {
        let method = request.checked_method()?;
        let destination = Destination::listed_in(&request.url, capabilities)?;
        request.check_headers()?;

        let revealed = reveal(request, &destination, capabilities, secrets)?;
        let builder = request.builder(method, &revealed)?;
        let timeout = request.timeout()?;
        let body = request.decoded_body()?;

        let response = self
            .run(builder, body, timeout)
            .map_err(|error| format!("{}: {error}", request.url))?;

        HttpResponse::received(response, &revealed)
    }

    fn run(
        &self,
        builder: Builder,
        body: Option<Vec<u8>>,
        timeout: Duration,
    ) -> Result<Response<Body>, String> {
        match body {
            Some(body) => self.call(builder.body(body), timeout),
            None => self.call(builder.body(()), timeout),
        }
    }

    fn call(
        &self,
        request: Result<Request<impl AsSendBody>, ureq::http::Error>,
        timeout: Duration,
    ) -> Result<Response<Body>, String> {
        let request = request.map_err(|error| error.to_string())?;

        request
            .with_agent(&self.agent)
            .configure()
            .timeout_global(Some(timeout))
            .run()
            .map_err(|error| error.to_string())
    }
}

fn reveal(
    request: &HttpRequest,
    destination: &Destination,
    capabilities: &Capabilities,
    secrets: Option<&SecretAccess<'_>>,
) -> Result<Vec<Revealed>, String> {
    if request.secrets.is_empty() {
        return Ok(Vec::new());
    }

    let secrets = secrets.ok_or("the core keeps no secrets to attach")?;

    secrets.reveal(&request.secrets, destination, capabilities)
}
