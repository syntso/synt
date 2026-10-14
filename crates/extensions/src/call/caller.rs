use std::future::Future;
use std::pin::Pin;

use serde_json::Value;
use synt_actions::Context;

use super::{CallFailure, ExtensionCall};

pub type CallFuture<'a> = Pin<Box<dyn Future<Output = Result<Value, CallFailure>> + Send + 'a>>;

pub trait Caller: Send + Sync + 'static {
    fn call<'a>(&'a self, call: ExtensionCall, origin: Option<&'a Context>) -> CallFuture<'a>;
}
