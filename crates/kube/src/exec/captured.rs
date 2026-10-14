use super::{Collected, ExitStatus};

pub(super) struct Captured {
    pub(super) status: ExitStatus,
    pub(super) stdout: Collected,
    pub(super) stderr: Collected,
}
