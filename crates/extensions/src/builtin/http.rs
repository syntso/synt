use std::sync::{Arc, OnceLock};

use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_journal::ResourceRef;

use super::CallingActor;
use crate::blocking::Blocking;
use crate::http::{Exchange, HttpRequest, HttpResponse, RecordedParams, SecretAccess};
use crate::{ExtensionError, Extensions};

const READING_METHODS: [&str; 3] = ["GET", "HEAD", "OPTIONS"];

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
        RecordedParams::new(params).to_value()
    }

    fn summary(&self, output: &HttpResponse) -> Value {
        json!({ "status": output.status, "body_bytes": output.body_size() })
    }

    async fn run(
        &self,
        context: &Context,
        params: HttpRequest,
    ) -> Result<HttpResponse, ActionError> {
        let extension = context
            .calling_extension()
            .ok_or_else(|| {
                ActionError::failed("only an extension sends requests through extension.http")
            })?
            .as_str()
            .to_owned();
        let capabilities = self
            .extensions
            .loaded(&extension)?
            .guest
            .manifest()
            .capabilities
            .clone();
        let secrets = self.extensions.services().secrets.get().cloned();
        let exchange = Arc::clone(&self.exchange);

        Blocking::run(move || {
            let access = secrets
                .as_deref()
                .map(|source| SecretAccess::new(source, &extension));

            exchange
                .get_or_init(Exchange::new)
                .send(&params, &capabilities, access.as_ref())
                .map_err(ExtensionError::Refused)
        })
        .await
    }
}
