use std::collections::VecDeque;

use bytes::Bytes;
use russh::client::Msg;
use russh::{ChannelWriteHalf, Sig};

use super::{ShellOutput, TerminalSize, await_reply};
use crate::{Algorithms, Lease, SshError, Target};

pub struct Shell {
    lease: Lease,
    writer: ChannelWriteHalf<Msg>,
    algorithms: Option<Algorithms>,
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
            .await?;
        await_reply(&mut reader, &mut early, "a terminal").await?;

        writer.request_shell(true).await?;
        await_reply(&mut reader, &mut early, "a shell").await?;

        let output = ShellOutput::new(reader, early);
        let algorithms = lease.algorithms();

        let shell = Self {
            lease,
            writer,
            algorithms,
        };

        Ok((shell, output))
    }

    pub fn target(&self) -> &Target {
        self.lease.target()
    }

    pub fn algorithms(&self) -> Option<&Algorithms> {
        self.algorithms.as_ref()
    }

    pub async fn write(&self, data: Bytes) -> Result<(), SshError> {
        self.writer.data_bytes(data).await?;

        Ok(())
    }

    pub async fn resize(&self, size: TerminalSize) -> Result<(), SshError> {
        self.writer
            .window_change(u32::from(size.columns), u32::from(size.rows), 0, 0)
            .await?;

        Ok(())
    }

    pub async fn hang_up(&self) -> Result<(), SshError> {
        self.writer.signal(Sig::HUP).await?;

        Ok(())
    }

    pub async fn close(&self) {
        let _ = self.writer.close().await;
    }
}
