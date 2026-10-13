use russh::client::Msg;
use russh::{Channel, ChannelMsg};

use super::{ExecOutput, StreamBuffer, signal_name};
use crate::{Algorithms, SshError};

const STDERR: u32 = 1;

pub(super) struct Collector {
    stdout: StreamBuffer,
    stderr: StreamBuffer,
    exit: Option<u32>,
    signal: Option<String>,
}

impl Collector {
    pub(super) fn new(output_bytes: usize) -> Self {
        Self {
            stdout: StreamBuffer::new(output_bytes),
            stderr: StreamBuffer::new(output_bytes),
            exit: None,
            signal: None,
        }
    }

    pub(super) async fn drain(mut self, channel: &mut Channel<Msg>) -> Result<Self, SshError> {
        while let Some(message) = channel.wait().await {
            match message {
                ChannelMsg::Data { data } => self.stdout.push(&data),
                ChannelMsg::ExtendedData { data, ext: STDERR } => self.stderr.push(&data),
                ChannelMsg::ExitStatus { exit_status } => self.exit = Some(exit_status),
                ChannelMsg::ExitSignal {
                    signal_name: signal,
                    ..
                } => {
                    self.signal = Some(signal_name(&signal));
                }
                ChannelMsg::Failure => {
                    return Err(SshError::Channel(
                        "the server refused to run the command".to_owned(),
                    ));
                }
                ChannelMsg::Close => return Ok(self),
                _ => {}
            }
        }

        if self.exit.is_none() && self.signal.is_none() {
            return Err(SshError::ConnectionLost);
        }

        Ok(self)
    }

    pub(super) fn finish(self, duration_ms: u64, algorithms: Option<Algorithms>) -> ExecOutput {
        let truncated = self.stdout.truncated() || self.stderr.truncated();

        ExecOutput {
            exit: self.exit,
            signal: self.signal,
            stdout: self.stdout.into_text(),
            stderr: self.stderr.into_text(),
            truncated,
            duration_ms,
            algorithms,
        }
    }
}
