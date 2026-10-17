use std::time::Duration;

use ureq::http::request::Builder;
use ureq::http::{Request, Response};
use ureq::unversioned::transport::DefaultConnector;
use ureq::{Agent, AsSendBody, Body, RequestExt};

use super::{
    CheckedResolver, Destination, HttpRequest, HttpResponse, Revealed, SecretAccess, TrustedTls,
};
use crate::error::HttpError;
use crate::manifest::Capabilities;

pub(crate) struct Exchange {
    agent: Agent,
}

impl Exchange {
    pub(crate) fn new() -> Self {
        let config = Agent::config_builder()
            .tls_config(TrustedTls::config())
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
    ) -> Result<HttpResponse, HttpError> {
        let method = request.checked_method()?;
        let destination = Destination::listed_in(request.url(), capabilities)?;
        request.check_headers()?;

        let revealed = Self::reveal(request, &destination, capabilities, secrets)?;
        let builder = request.builder(method, &revealed)?;
        let timeout = request.timeout()?;
        let body = request.decoded_body()?;

        let response = self.run(request.url(), builder, body, timeout)?;

        HttpResponse::received(response, &revealed)
    }

    fn reveal(
        request: &HttpRequest,
        destination: &Destination,
        capabilities: &Capabilities,
        secrets: Option<&SecretAccess<'_>>,
    ) -> Result<Vec<Revealed>, HttpError> {
        if request.secrets().is_empty() {
            return Ok(Vec::new());
        }

        let secrets = secrets.ok_or(HttpError::NoSecrets)?;

        secrets.reveal(request.secrets(), destination, capabilities)
    }

    fn run(
        &self,
        url: &str,
        builder: Builder,
        body: Option<Vec<u8>>,
        timeout: Duration,
    ) -> Result<Response<Body>, HttpError> {
        match body {
            Some(body) => self.call(url, builder.body(body), timeout),
            None => self.call(url, builder.body(()), timeout),
        }
    }

    fn call(
        &self,
        url: &str,
        request: Result<Request<impl AsSendBody>, ureq::http::Error>,
        timeout: Duration,
    ) -> Result<Response<Body>, HttpError> {
        let request = request.map_err(|source| HttpError::Request {
            url: url.to_owned(),
            source,
        })?;

        request
            .with_agent(&self.agent)
            .configure()
            .timeout_global(Some(timeout))
            .run()
            .map_err(|source| HttpError::Transport {
                url: url.to_owned(),
                source,
            })
    }
}
