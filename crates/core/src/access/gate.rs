use std::future::Future;

use super::{AccessRequest, Decision};

pub trait Gate: Send + Sync + 'static {
    fn settled(&self, _request: &AccessRequest<'_>) -> Option<Decision> {
        None
    }

    fn decide(&self, request: &AccessRequest<'_>) -> impl Future<Output = Decision> + Send;
}
