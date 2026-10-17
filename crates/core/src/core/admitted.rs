use synt_journal::Seq;

use crate::grants::GrantId;

pub(super) struct Admitted {
    pub(super) request: Seq,
    pub(super) grant: Option<GrantId>,
}
