use synt_actions::Context;
use tokio_util::sync::CancellationToken;

pub(crate) fn keep_hold_while_open(context: &Context, stop: CancellationToken) {
    let Some(hold) = context.hold() else {
        return;
    };

    tokio::spawn(async move {
        stop.cancelled().await;
        drop(hold);
    });
}
