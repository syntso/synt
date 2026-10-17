use std::sync::Arc;

use synt_actions::Context;
use synt_proto::EndReason;

use crate::terminals::Session;

pub(super) fn close_when_revoked(context: &Context, session: Arc<Session>) {
    let Some(hold) = context.hold() else {
        return;
    };
    let stop = context.stop().clone();

    tokio::spawn(async move {
        tokio::select! {
            () = stop.cancelled() => {
                session.close(EndReason::Closed).await;
            }
            () = session.finished() => {}
        }

        drop(hold);
    });
}
