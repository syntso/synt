mod link;

pub(crate) use link::LinkProgress;

use std::future::Future;

use synt_proto::RunbookStep;

pub(crate) trait Progress: Send + Sync {
    fn report(&self, step: RunbookStep) -> impl Future<Output = ()> + Send;
}
