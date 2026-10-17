use std::collections::VecDeque;

use bytes::Bytes;
use russh::{ChannelMsg, ChannelReadHalf};

use super::ShellEnd;
use crate::SshError;
use crate::signal_name::SignalName;

pub struct ShellOutput {
    reader: ChannelReadHalf,
    early: VecDeque<Bytes>,
    end: ShellEnd,
    finished: bool,
}

impl ShellOutput {
    pub(super) fn new(reader: ChannelReadHalf) -> Self {
        Self {
            reader,
            early: VecDeque::new(),
            end: ShellEnd::default(),
            finished: false,
        }
    }

    pub async fn next(&mut self) -> Option<Bytes> {
        if let Some(data) = self.early.pop_front() {
            return Some(data);
        }

        while !self.finished {
            if let Some(data) = self.receive().await {
                return Some(data);
            }
        }

        None
    }

    pub fn end(&self) -> ShellEnd {
        self.end.clone()
    }

    pub(super) async fn await_reply(&mut self, request: &'static str) -> Result<(), SshError> {
        while let Some(message) = self.reader.wait().await {
            match message {
                ChannelMsg::Success => return Ok(()),
                ChannelMsg::Failure => return Err(SshError::Refused(request)),
                ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, .. } => {
                    self.early.push_back(data);
                }
                ChannelMsg::Close => break,
                _ => {}
            }
        }

        Err(SshError::Unanswered(request))
    }

    async fn receive(&mut self) -> Option<Bytes> {
        let Some(message) = self.reader.wait().await else {
            self.finished = true;
            self.end.lost = self.end.exit.is_none() && self.end.signal.is_none();
            return None;
        };

        match message {
            ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, .. } => Some(data),
            ChannelMsg::ExitStatus { exit_status } => {
                self.end.exit = Some(exit_status);
                None
            }
            ChannelMsg::ExitSignal { signal_name, .. } => {
                self.end.signal = Some(signal_name.to_name());
                None
            }
            ChannelMsg::Close => {
                self.finished = true;
                None
            }
            _ => None,
        }
    }
}
