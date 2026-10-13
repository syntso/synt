use std::collections::VecDeque;

use bytes::Bytes;
use russh::{ChannelMsg, ChannelReadHalf};

use crate::SshError;

pub(super) async fn await_reply(
    reader: &mut ChannelReadHalf,
    early: &mut VecDeque<Bytes>,
    request: &str,
) -> Result<(), SshError> {
    while let Some(message) = reader.wait().await {
        match message {
            ChannelMsg::Success => return Ok(()),
            ChannelMsg::Failure => {
                return Err(SshError::Channel(format!("the server refused {request}")));
            }
            ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, .. } => {
                early.push_back(data);
            }
            ChannelMsg::Close => break,
            _ => {}
        }
    }

    Err(SshError::Channel(format!(
        "the channel closed before the server answered {request}"
    )))
}
