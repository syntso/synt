use bytes::Bytes;
use futures_util::future::BoxFuture;
use synt_ssh::ShellEnd;

pub(crate) trait ConsoleOutput: Send {
    fn next(&mut self) -> BoxFuture<'_, Option<Bytes>>;

    fn end(&self) -> ShellEnd;
}
