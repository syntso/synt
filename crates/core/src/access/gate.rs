use std::future::Future;

use synt_actions::Handle;
use synt_journal::Actor;

use super::Decision;

pub trait Gate: Send + Sync + 'static {
    fn decide(&self, actor: &Actor, action: &Handle) -> impl Future<Output = Decision> + Send;
}
