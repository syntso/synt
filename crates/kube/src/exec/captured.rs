use synt_proto::CappedText;

use super::ExitStatus;

pub(super) struct Captured {
    pub(super) status: ExitStatus,
    pub(super) stdout: CappedText,
    pub(super) stderr: CappedText,
}
