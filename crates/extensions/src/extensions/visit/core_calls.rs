use std::sync::Arc;

use serde_json::Value;
use synt_actions::Action;
use synt_journal::Name;
use synt_proto::ActionCall;

use crate::builtin::ExtensionHttp;
use crate::call::{ExtensionCall, FailureCode};
use crate::extension::{Guest, Outside};
use crate::http::{HttpRequest, HttpResponse};
use crate::interface::GuestError;

pub(super) struct CoreCalls {
    guest: Arc<Guest>,
    outside: Option<Outside>,
}

impl CoreCalls {
    pub(super) fn new(guest: Arc<Guest>, outside: Option<Outside>) -> Self {
        Self { guest, outside }
    }

    pub(super) fn call(
        &self,
        action: String,
        target: Option<String>,
        params: &str,
    ) -> Result<String, GuestError> {
        if !self.guest.manifest().capabilities().may_call(&action) {
            return Err(GuestError::not_allowed(format!(
                "{action} is not listed in capabilities.call of {}",
                self.guest.id()
            )));
        }

        let params = Self::parse_params(params)?;
        let answer = self.dispatch(action, target, params)?;

        Ok(answer.to_string())
    }

    pub(super) fn send(&self, request: &HttpRequest) -> Result<HttpResponse, GuestError> {
        let uses_http = self.guest.manifest().capabilities().uses_http();
        self.guest.require(uses_http, "http hosts")?;

        let params = serde_json::to_value(request)
            .map_err(|error| GuestError::new(FailureCode::InvalidRequest, error.to_string()))?;
        let answer = self.dispatch(ExtensionHttp::ID.to_owned(), None, params)?;

        serde_json::from_value(answer)
            .map_err(|error| GuestError::failed(format!("the response: {error}")))
    }

    fn dispatch(
        &self,
        action: String,
        target: Option<String>,
        params: Value,
    ) -> Result<Value, GuestError> {
        let Some(outside) = &self.outside else {
            return Err(GuestError::new(
                FailureCode::Unavailable,
                "the core cannot be reached from here; only actions, views, panels, timers \
                 and events may call it",
            ));
        };

        let extension = Name::new(self.guest.id())
            .map_err(|reason| GuestError::new(FailureCode::Internal, reason.to_string()))?;
        let request = ActionCall {
            action,
            target,
            params,
        };

        Ok(outside.call(ExtensionCall::new(extension, request))?)
    }

    fn parse_params(params: &str) -> Result<Value, GuestError> {
        if params.trim().is_empty() {
            return Ok(Value::Null);
        }

        serde_json::from_str(params).map_err(|reason| {
            GuestError::new(
                FailureCode::InvalidParams,
                format!("params are not JSON: {reason}"),
            )
        })
    }
}
