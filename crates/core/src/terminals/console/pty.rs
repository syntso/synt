use std::future;

use bytes::Bytes;
use futures_util::FutureExt;
use futures_util::future::BoxFuture;
use synt_platform::{Pty, PtyOutput};
use synt_ssh::{Algorithms, ShellEnd, TerminalSize};

use super::{Console, ConsoleOutput};
use crate::CoreError;
use crate::terminals::SessionKind;

const LABEL: &str = "local";

impl Console for Pty {
    fn kind(&self) -> SessionKind {
        SessionKind::Local
    }

    fn label(&self) -> String {
        LABEL.to_owned()
    }

    fn algorithms(&self) -> Option<&Algorithms> {
        None
    }

    fn write(&self, data: Bytes) -> BoxFuture<'_, Result<(), CoreError>> {
        async move { Pty::write(self, data.to_vec()).await.map_err(CoreError::Pty) }.boxed()
    }

    fn resize(&self, size: TerminalSize) -> BoxFuture<'_, Result<(), CoreError>> {
        let resized = Pty::resize(self, size.columns, size.rows).map_err(CoreError::Pty);

        future::ready(resized).boxed()
    }

    fn hang_up(&self) -> BoxFuture<'_, bool> {
        future::ready(Pty::hang_up(self).is_ok()).boxed()
    }

    fn close(&self) -> BoxFuture<'_, ()> {
        self.stop();

        future::ready(()).boxed()
    }
}

impl ConsoleOutput for PtyOutput {
    fn next(&mut self) -> BoxFuture<'_, Option<Bytes>> {
        async move { PtyOutput::next(self).await.map(Bytes::from) }.boxed()
    }

    fn end(&self) -> ShellEnd {
        let end = PtyOutput::end(self);

        ShellEnd {
            exit: end.exit,
            signal: end.signal,
            lost: false,
        }
    }
}
