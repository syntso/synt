use std::collections::VecDeque;

use bytes::Bytes;
use russh::client::Msg;
use russh::{ChannelWriteHalf, Sig};

use super::{ShellOutput, TerminalSize, await_reply};
use crate::{Lease, SshError, Target};

pub struct Shell {
    lease: Lease,
    writer: ChannelWriteHalf<Msg>,
}

impl Shell {
    pub async fn open(
        lease: Lease,
        term: &str,
        size: TerminalSize,
    ) -> Result<(Self, ShellOutput), SshError> {
        let channel = lease.open_session().await?;
        let (mut reader, writer) = channel.split();
        let mut early = VecDeque::new();

        writer
            .request_pty(
                true,
                term,
                u32::from(size.columns),
                u32::from(size.rows),
                0,
                0,
                &[],
            )
            .await
            .map_err(|error| channel_error(&error))?;
        await_reply(&mut reader, &mut early, "a terminal").await?;

        writer
            .request_shell(true)
            .await
            .map_err(|error| channel_error(&error))?;
        await_reply(&mut reader, &mut early, "a shell").await?;

        let output = ShellOutput::new(reader, early);

        Ok((Self { lease, writer }, output))
    }

    pub fn target(&self) -> &Target {
        self.lease.target()
    }

    pub async fn write(&self, data: Bytes) -> Result<(), SshError> {
        self.writer
            .data_bytes(data)
            .await
            .map_err(|error| channel_error(&error))
    }

    pub async fn resize(&self, size: TerminalSize) -> Result<(), SshError> {
        self.writer
            .window_change(u32::from(size.columns), u32::from(size.rows), 0, 0)
            .await
            .map_err(|error| channel_error(&error))
    }

    pub async fn hang_up(&self) -> Result<(), SshError> {
        self.writer
            .signal(Sig::HUP)
            .await
            .map_err(|error| channel_error(&error))
    }

    pub async fn close(&self) {
        let _ = self.writer.close().await;
    }
}

fn channel_error(error: &russh::Error) -> SshError {
    SshError::Channel(error.to_string())
}
