use synt_journal::{Actor, Seq};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Copy)]
pub(crate) struct OnBehalf<'a> {
    pub(crate) actor: &'a Actor,
    pub(crate) request: Seq,
    pub(crate) approved: bool,
    pub(crate) stop: Option<&'a CancellationToken>,
}
