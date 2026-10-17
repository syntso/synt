use super::StandingGrant;

pub(crate) struct Revocation {
    pub(crate) grant: StandingGrant,
    pub(crate) stopped: usize,
}
