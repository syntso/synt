use std::future::Future;
use std::pin::pin;
use std::time::{Duration, Instant};

use russh::client::Msg;
use russh::{Channel, ChannelMsg, Sig};
use tokio::time;

use super::{Collector, ExecLimits, ExecOutput};
use crate::{Lease, Pool, SshError, Target};

const STOP_GRACE: Duration = Duration::from_secs(2);

enum Interruption {
    TimedOut,
    Stopped,
}

pub async fn exec(
    pool: &Pool,
    target: &Target,
    command: &str,
    limits: ExecLimits,
    stop: impl Future<Output = ()>,
) -> Result<ExecOutput, SshError> {
    let started = Instant::now();
    let deadline = time::Instant::from_std(started) + limits.timeout;
    let mut stop = pin!(stop);

    let leasing = time::timeout_at(deadline, pool.acquire(target));
    let lease = tokio::select! {
        () = &mut stop => return Err(SshError::Cancelled),
        leased = leasing => leased.map_err(|_| SshError::Timeout(limits.timeout))??,
    };

    let collected = run(&lease, command, limits, deadline, stop).await?;
    let elapsed = milliseconds(started.elapsed());

    Ok(collected.finish(elapsed, lease.algorithms()))
}

async fn run(
    lease: &Lease,
    command: &str,
    limits: ExecLimits,
    deadline: time::Instant,
    mut stop: impl Future<Output = ()> + Unpin,
) -> Result<Collector, SshError> {
    let starting = time::timeout_at(deadline, start(lease, command));
    let mut channel = tokio::select! {
        () = &mut stop => return Err(SshError::Cancelled),
        started = starting => started.map_err(|_| SshError::Timeout(limits.timeout))??,
    };

    let collector = Collector::new(limits.output_bytes);

    let interruption = tokio::select! {
        drained = time::timeout_at(deadline, collector.drain(&mut channel)) => match drained {
            Ok(collected) => return collected,
            Err(_) => Interruption::TimedOut,
        },
        () = &mut stop => Interruption::Stopped,
    };

    let ended = halt(&mut channel).await;

    match interruption {
        Interruption::TimedOut => Err(SshError::CommandTimeout {
            limit: limits.timeout,
            ended,
        }),
        Interruption::Stopped => Err(SshError::Stopped { ended }),
    }
}

async fn start(lease: &Lease, command: &str) -> Result<Channel<Msg>, SshError> {
    let channel = lease.open_session().await?;

    channel
        .exec(true, command)
        .await
        .map_err(|error| SshError::Channel(error.to_string()))?;

    Ok(channel)
}

async fn halt(channel: &mut Channel<Msg>) -> bool {
    let ended =
        signal_and_wait(channel, Sig::TERM).await || signal_and_wait(channel, Sig::KILL).await;

    let _ = channel.close().await;

    ended
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
