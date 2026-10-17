use synt_journal::{Actor, Seq};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, Copy)]
pub(crate) struct OnBehalf<'a> {
    pub(crate) actor: &'a Actor,
    pub(crate) request: Seq,
    pub(crate) approved: bool,
    pub(crate) planned: Option<&'a str>,
    pub(crate) stop: Option<&'a CancellationToken>,
}

impl OnBehalf<'_> {
    pub(crate) fn approves(&self, destination: Option<&str>) -> bool {
        self.approved && self.planned == destination
    }
}
