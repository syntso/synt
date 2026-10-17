mod end;
mod output;
mod terminal_size;

use bytes::Bytes;
use russh::client::Msg;
use russh::{ChannelWriteHalf, Sig};

pub use end::ShellEnd;
pub use output::ShellOutput;
pub use terminal_size::TerminalSize;

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
        let (reader, writer) = channel.split();
        let mut output = ShellOutput::new(reader);

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
        output.await_reply("a terminal").await?;

        writer.request_shell(true).await?;
        output.await_reply("a shell").await?;

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
