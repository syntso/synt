mod open;
mod output;
mod pty;
mod shell;

pub(crate) use open::OpenConsole;
pub(crate) use output::ConsoleOutput;

use bytes::Bytes;
use futures_util::future::BoxFuture;
use synt_ssh::{Algorithms, TerminalSize};

use super::SessionKind;
use crate::CoreError;

pub(crate) trait Console: Send + Sync {
    fn kind(&self) -> SessionKind;

    fn label(&self) -> String;

    fn algorithms(&self) -> Option<&Algorithms>;

    fn write(&self, data: Bytes) -> BoxFuture<'_, Result<(), CoreError>>;

    fn resize(&self, size: TerminalSize) -> BoxFuture<'_, Result<(), CoreError>>;

    fn hang_up(&self) -> BoxFuture<'_, bool>;

    fn close(&self) -> BoxFuture<'_, ()>;
}
