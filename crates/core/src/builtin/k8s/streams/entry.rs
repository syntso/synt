use synt_journal::Actor;
use tokio_util::sync::CancellationToken;

pub(crate) struct OpenStream {
    actor: Actor,
    stop: CancellationToken,
}

impl OpenStream {
    pub(crate) fn new(actor: Actor, stop: CancellationToken) -> Self {
        Self { actor, stop }
    }

    pub(crate) fn may_be_closed_by(&self, actor: &Actor) -> bool {
        self.actor == *actor || *actor == Actor::Person
    }

    pub(crate) fn close(&self) {
        self.stop.cancel();
    }
}
