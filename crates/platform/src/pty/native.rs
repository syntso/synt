use std::path::PathBuf;

use portable_pty::{Child, ChildKiller};

use super::PtyEnd;

pub(super) trait NativePty {
    const ASKS_FOR_CURSOR: bool;

    fn login_shell() -> Option<PathBuf>;

    fn wait_for(child: Box<dyn Child + Send + Sync>) -> PtyEnd;

    fn force_stop(process: Option<u32>, killer: &mut dyn ChildKiller);
}
