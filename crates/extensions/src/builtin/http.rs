use std::sync::{Arc, OnceLock};

use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk, blocking};
use synt_journal::ResourceRef;

use super::CallingActor;
use crate::ExtensionError;
use crate::Extensions;
use crate::http::{Exchange, HttpRequest, HttpResponse, RecordedParams, SecretAccess};

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

    fn sender(context: &Context) -> Result<String, ActionError> {
        let extension = context.calling_extension().ok_or_else(|| {
            ActionError::failed("only an extension sends requests through extension.http")
        })?;

        Ok(extension.as_str().to_owned())
    }
}

impl Action for ExtensionHttp {
    const ID: &'static str = "extension.http";
    const TITLE: &'static str = "Send an HTTP request for an extension to a host it lists";
    const RISK: Risk = Risk::Read;

    type Params = HttpRequest;
    type Output = HttpResponse;

    fn risk(&self, _target: Option<&ResourceRef>, params: &HttpRequest) -> Risk {
        if params.only_reads() {
            return Risk::Read;
        }

        Risk::Write
    }

    fn recorded_params(&self, params: &Value) -> Value {
        RecordedParams::new(params).to_value()
    }

    fn summary(&self, output: &HttpResponse) -> Value {
        json!({ "status": output.status(), "body_bytes": output.body_size() })
    }

    async fn run(
        &self,
        context: &Context,
        params: HttpRequest,
    ) -> Result<HttpResponse, ActionError> {
        let sender = Self::sender(context)?;
        let loaded = self.extensions.loaded(&sender)?;
        let manifest = loaded.guest().manifest();
        let capabilities = manifest.capabilities().clone();
        let extension = manifest.id().clone();
        let secrets = self.extensions.services().secrets().cloned();
        let exchange = Arc::clone(&self.exchange);

        blocking(move || -> Result<_, ExtensionError> {
            let access = secrets
                .as_deref()
                .map(|source| SecretAccess::new(source, &extension));

            let response = exchange.get_or_init(Exchange::new).send(
                &params,
                &capabilities,
                access.as_ref(),
            )?;

            Ok(response)
        })
        .await
    }
}
