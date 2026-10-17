use bytes::Bytes;
use futures_util::FutureExt;
use futures_util::future::BoxFuture;
use synt_ssh::{Algorithms, Shell, ShellEnd, ShellOutput, TerminalSize};

use super::{Console, ConsoleOutput};
use crate::CoreError;
use crate::terminals::SessionKind;

impl Console for Shell {
    fn kind(&self) -> SessionKind {
        SessionKind::Ssh
    }

    fn label(&self) -> String {
        self.target().label()
    }

    fn algorithms(&self) -> Option<&Algorithms> {
        Shell::algorithms(self)
    }

    fn write(&self, data: Bytes) -> BoxFuture<'_, Result<(), CoreError>> {
        async move { Shell::write(self, data).await.map_err(CoreError::Shell) }.boxed()
    }

    fn resize(&self, size: TerminalSize) -> BoxFuture<'_, Result<(), CoreError>> {
        async move { Shell::resize(self, size).await.map_err(CoreError::Shell) }.boxed()
    }

    fn hang_up(&self) -> BoxFuture<'_, bool> {
        async move { Shell::hang_up(self).await.is_ok() }.boxed()
    }

    fn close(&self) -> BoxFuture<'_, ()> {
        Shell::close(self).boxed()
    }
}

impl ConsoleOutput for ShellOutput {
    fn next(&mut self) -> BoxFuture<'_, Option<Bytes>> {
        ShellOutput::next(self).boxed()
    }

    fn end(&self) -> ShellEnd {
        ShellOutput::end(self)
    }
}
