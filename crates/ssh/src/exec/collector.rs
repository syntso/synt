use std::time::Duration;

use russh::client::Msg;
use russh::{Channel, ChannelMsg};
use synt_proto::{CappedText, ExecOutput};

use super::ExecOutcome;
use crate::signal_name::SignalName;
use crate::{Algorithms, SshError};

const STDERR: u32 = 1;

pub(super) struct Collector {
    stdout: CappedText,
    stderr: CappedText,
    exit: Option<u32>,
    signal: Option<String>,
}

impl Collector {
    pub(super) fn new(output_bytes: usize) -> Self {
        Self {
            stdout: CappedText::new(output_bytes),
            stderr: CappedText::new(output_bytes),
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
                ChannelMsg::ExitSignal { signal_name, .. } => {
                    self.signal = Some(signal_name.to_name());
                }
                ChannelMsg::Failure => return Err(SshError::Refused("to run the command")),
                ChannelMsg::Close => return Ok(self),
                _ => {}
            }
        }

        if self.exit.is_none() && self.signal.is_none() {
            return Err(SshError::ConnectionLost);
        }

        Ok(self)
    }

    pub(super) fn finish(self, elapsed: Duration, algorithms: Option<Algorithms>) -> ExecOutcome {
        ExecOutcome {
            exit: self.exit,
            signal: self.signal,
            output: ExecOutput::new(self.stdout, self.stderr, elapsed),
            algorithms,
        }
    }
}
