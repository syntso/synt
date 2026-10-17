use std::any::Any;
use std::sync::Arc;
use std::time::Duration;

use synt_actions::{ActionError, Completed, Context, Prepared};
use synt_journal::Seq;
use tokio::time;
use tokio_util::sync::CancellationToken;

use crate::grants::Hold;
use crate::link::Link;
use crate::recorder::Call;

const STOP_GRACE: Duration = Duration::from_secs(10);
const REVOKED: &str = "stopped: the standing grant this call used was revoked";
const HALTED: &str = "stopped: the work this call belongs to was stopped";

#[derive(Clone, Copy)]
pub(super) struct Invocation<'a> {
    pub(super) link: &'a Arc<Link>,
    pub(super) halt: Option<&'a CancellationToken>,
}

impl Invocation<'_> {
    pub(super) fn stop(&self) -> CancellationToken {
        self.halt
            .map_or_else(CancellationToken::new, CancellationToken::child_token)
    }

    pub(super) fn stopped(&self) -> ActionError {
        let reason = match self.halt {
            Some(halt) if halt.is_cancelled() => HALTED,
            _ => REVOKED,
        };

        ActionError::failed(reason)
    }

    pub(super) fn context(
        &self,
        call: &Call<'_>,
        request: Seq,
        prepared: &Prepared,
        stop: &CancellationToken,
        hold: Option<Hold>,
    ) -> Context {
        let connection: Arc<dyn Any + Send + Sync> = Arc::clone(self.link) as _;
        let destination = prepared.destination().map(str::to_owned);

        let context = Context::new(call.actor.clone(), request, call.target.cloned())
            .with_risk(prepared.risk())
            .with_destination(destination)
            .with_connection(connection)
            .with_stop(stop.clone());

        match hold {
            Some(hold) => context.with_hold(Arc::new(hold)),
            None => context,
        }
    }

    pub(super) async fn run(
        &self,
        prepared: Prepared,
        context: Context,
        stop: &CancellationToken,
    ) -> Result<Completed, ActionError> {
        let mut running = tokio::spawn(async move { prepared.run(&context).await });

        tokio::select! {
            finished = &mut running => {
                return finished
                    .unwrap_or_else(|_| Err(ActionError::failed("the action stopped unexpectedly")));
            }
            () = stop.cancelled() => {}
        }

        if time::timeout(STOP_GRACE, &mut running).await.is_err() {
            running.abort();
        }

        Err(self.stopped())
    }
}
