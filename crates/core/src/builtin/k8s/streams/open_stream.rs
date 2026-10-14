use synt_journal::Actor;
use tokio_util::sync::CancellationToken;

pub(crate) struct OpenStream {
    pub(crate) actor: Actor,
    pub(crate) stop: CancellationToken,
}
