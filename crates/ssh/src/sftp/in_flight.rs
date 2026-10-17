use std::collections::VecDeque;
use std::future::Future;

use russh_sftp::client::error::Error;
use tokio::task::JoinHandle;

pub(super) const REQUEST_BYTES: usize = 32 * 1024;
const REQUESTS_IN_FLIGHT: usize = 64;

const STOPPED: &str = "an sftp request stopped unexpectedly";

pub(super) struct InFlight<T> {
    requests: VecDeque<JoinHandle<Result<T, Error>>>,
}

impl<T: Send + 'static> InFlight<T> {
    pub(super) fn new() -> Self {
        Self {
            requests: VecDeque::with_capacity(REQUESTS_IN_FLIGHT),
        }
    }

    pub(super) fn is_full(&self) -> bool {
        self.requests.len() >= REQUESTS_IN_FLIGHT
    }

    pub(super) fn push<Request>(&mut self, request: Request)
    where
        Request: Future<Output = Result<T, Error>> + Send + 'static,
    {
        self.requests.push_back(tokio::spawn(request));
    }

    pub(super) async fn next(&mut self) -> Option<Result<T, Error>> {
        let oldest = self.requests.pop_front()?;

        let answer = oldest
            .await
            .unwrap_or_else(|_| Err(Error::UnexpectedBehavior(STOPPED.to_owned())));

        Some(answer)
    }

    pub(super) fn cancel(&mut self) {
        for request in self.requests.drain(..) {
            request.abort();
        }
    }
}

impl<T> Drop for InFlight<T> {
    fn drop(&mut self) {
        for request in &self.requests {
            request.abort();
        }
    }
}
