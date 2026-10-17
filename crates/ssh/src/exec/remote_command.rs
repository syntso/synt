use std::future::Future;
use std::pin::pin;
use std::time::{Duration, Instant};

use russh::client::Msg;
use russh::{Channel, ChannelMsg, Sig};
use tokio::time;

use synt_proto::ExecLimits;

use super::{Collector, ExecOutcome, Interruption};
use crate::{Lease, Pool, SshError, Target};

const STOP_GRACE: Duration = Duration::from_secs(2);

pub(crate) struct RemoteCommand {
    channel: Channel<Msg>,
}

impl RemoteCommand {
    pub(crate) async fn run(
        pool: &Pool,
        target: &Target,
        command: &str,
        limits: ExecLimits,
        stop: impl Future<Output = ()>,
    ) -> Result<ExecOutcome, SshError> {
        let started = Instant::now();
        let deadline = time::Instant::from_std(started) + limits.timeout;
        let mut stop = pin!(stop);

        let leasing = time::timeout_at(deadline, pool.acquire(target));
        let lease = tokio::select! {
            () = &mut stop => return Err(SshError::Cancelled),
            leased = leasing => leased.map_err(|_| SshError::Timeout(limits.timeout))??,
        };

        let starting = time::timeout_at(deadline, Self::start(&lease, command));
        let remote = tokio::select! {
            () = &mut stop => return Err(SshError::Cancelled),
            started = starting => started.map_err(|_| SshError::Timeout(limits.timeout))??,
        };

        let collected = remote.collect(limits, deadline, stop).await?;

        Ok(collected.finish(started.elapsed(), lease.algorithms()))
    }

    async fn start(lease: &Lease, command: &str) -> Result<Self, SshError> {
        let channel = lease.open_session().await?;

        channel.exec(true, command).await?;

        Ok(Self { channel })
    }

    async fn collect(
        mut self,
        limits: ExecLimits,
        deadline: time::Instant,
        mut stop: impl Future<Output = ()> + Unpin,
    ) -> Result<Collector, SshError> {
        let collector = Collector::new(limits.output_bytes);

        let interruption = tokio::select! {
            drained = time::timeout_at(deadline, collector.drain(&mut self.channel)) => match drained {
                Ok(collected) => return collected,
                Err(_) => Interruption::TimedOut,
            },
            () = &mut stop => Interruption::Stopped,
        };

        let ended = self.halt().await;

        match interruption {
            Interruption::TimedOut => Err(SshError::CommandTimeout {
                limit: limits.timeout,
                ended,
            }),
            Interruption::Stopped => Err(SshError::Stopped { ended }),
        }
    }

    async fn halt(&mut self) -> bool {
        let ended = self.signal_and_wait(Sig::TERM).await || self.signal_and_wait(Sig::KILL).await;

        let _ = self.channel.close().await;

        ended
    }

    async fn signal_and_wait(&mut self, signal: Sig) -> bool {
        if self.channel.signal(signal).await.is_err() {
            return false;
        }

        time::timeout(STOP_GRACE, self.ended()).await.is_ok()
    }

    async fn ended(&mut self) {
        while let Some(message) = self.channel.wait().await {
            if matches!(
                message,
                ChannelMsg::ExitStatus { .. } | ChannelMsg::ExitSignal { .. } | ChannelMsg::Close
            ) {
                return;
            }
        }
    }
}
