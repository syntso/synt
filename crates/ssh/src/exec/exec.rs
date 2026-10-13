use std::time::{Duration, Instant};

use russh::client::Msg;
use russh::{Channel, ChannelMsg, Sig};
use tokio::time;

use super::{Collector, ExecLimits, ExecOutput};
use crate::session::Connection;
use crate::{SshError, Target};

const STOP_GRACE: Duration = Duration::from_secs(2);

pub async fn exec(
    target: &Target,
    command: &str,
    limits: ExecLimits,
) -> Result<ExecOutput, SshError> {
    let started = Instant::now();
    let deadline = time::Instant::from_std(started) + limits.timeout;

    let connection = time::timeout_at(deadline, Connection::open(target))
        .await
        .map_err(|_| SshError::Timeout(limits.timeout))??;

    let outcome = run(&connection, command, limits, deadline).await;
    connection.close().await;

    let collected = outcome?;

    Ok(collected.finish(milliseconds(started.elapsed())))
}

async fn run(
    connection: &Connection,
    command: &str,
    limits: ExecLimits,
    deadline: time::Instant,
) -> Result<Collector, SshError> {
    let mut channel = time::timeout_at(deadline, start(connection, command))
        .await
        .map_err(|_| SshError::Timeout(limits.timeout))??;

    let collector = Collector::new(limits.output_bytes);
    let drained = time::timeout_at(deadline, collector.drain(&mut channel)).await;

    match drained {
        Ok(collected) => collected,
        Err(_) => Err(stop(&mut channel, limits.timeout).await),
    }
}

async fn start(connection: &Connection, command: &str) -> Result<Channel<Msg>, SshError> {
    let channel = connection
        .target()?
        .channel_open_session()
        .await
        .map_err(|error| SshError::Channel(error.to_string()))?;

    channel
        .exec(true, command)
        .await
        .map_err(|error| SshError::Channel(error.to_string()))?;

    Ok(channel)
}

async fn stop(channel: &mut Channel<Msg>, limit: Duration) -> SshError {
    let ended =
        signal_and_wait(channel, Sig::TERM).await || signal_and_wait(channel, Sig::KILL).await;

    let _ = channel.close().await;

    SshError::CommandTimeout { limit, ended }
}

async fn signal_and_wait(channel: &mut Channel<Msg>, signal: Sig) -> bool {
    if channel.signal(signal).await.is_err() {
        return false;
    }

    time::timeout(STOP_GRACE, ended(channel)).await.is_ok()
}

async fn ended(channel: &mut Channel<Msg>) {
    while let Some(message) = channel.wait().await {
        if matches!(
            message,
            ChannelMsg::ExitStatus { .. } | ChannelMsg::ExitSignal { .. } | ChannelMsg::Close
        ) {
            return;
        }
    }
}

fn milliseconds(elapsed: Duration) -> u64 {
    u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX)
}
