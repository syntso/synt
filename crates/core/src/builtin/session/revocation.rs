use std::sync::Arc;

use synt_actions::Context;
use synt_proto::EndReason;

use crate::terminal::Terminal;

pub(super) fn close_when_revoked(context: &Context, terminal: Arc<Terminal>) {
    let Some(hold) = context.hold() else {
        return;
    };
    let stop = context.stop().clone();

    tokio::spawn(async move {
        tokio::select! {
            () = stop.cancelled() => {
                terminal.close(EndReason::Closed).await;
            }
            () = terminal.finished() => {}
        }

        drop(hold);
    });
}
