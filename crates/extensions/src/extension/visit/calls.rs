use serde_json::Value;
use synt_actions::Action;
use synt_journal::Name;
use synt_proto::ActionCall;

use super::Visit;
use crate::builtin::ExtensionHttp;
use crate::call::ExtensionCall;
use crate::call::FailureCode;
use crate::http::{HttpRequest, HttpResponse};
use crate::interface::GuestError;

impl Visit {
    pub(super) fn call_core(
        &self,
        action: String,
        target: Option<String>,
        params: &str,
    ) -> Result<String, GuestError> {
        if !self.manifest().capabilities.may_call(&action) {
            return Err(GuestError::not_allowed(format!(
                "{action} is not listed in capabilities.call of {}",
                self.id()
            )));
        }

        let params = parse_params(params)?;
        let answer = self.dispatch(action, target, params)?;

        Ok(answer.to_string())
    }

    pub(super) fn send(&self, request: &HttpRequest) -> Result<HttpResponse, GuestError> {
        self.allowed(!self.manifest().capabilities.http.is_empty(), "http hosts")?;

        let params = serde_json::to_value(request)
            .map_err(|error| GuestError::new(FailureCode::InvalidRequest, error.to_string()))?;
        let answer = self.dispatch(ExtensionHttp::ID.to_owned(), None, params)?;

        serde_json::from_value(answer)
            .map_err(|error| GuestError::new(FailureCode::Failed, format!("the response: {error}")))
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

        let extension = Name::new(self.id())
            .map_err(|reason| GuestError::new(FailureCode::Internal, reason.to_string()))?;
        let call = ExtensionCall::new(
            extension,
            ActionCall {
                action,
                target,
                params,
            },
        );

        outside
            .runtime
            .block_on(outside.caller.call(call, outside.origin.as_ref()))
            .map_err(GuestError::from)
    }
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
