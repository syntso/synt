use std::path::PathBuf;

use portable_pty::{Child, ChildKiller};

use super::{NativePty, PtyEnd};

pub(super) struct Windows;

impl NativePty for Windows {
    const ASKS_FOR_CURSOR: bool = true;

    fn login_shell() -> Option<PathBuf> {
        None
    }

    fn wait_for(mut child: Box<dyn Child + Send + Sync>) -> PtyEnd {
        child.wait().map_or_else(
            |_| PtyEnd::default(),
            |status| PtyEnd {
                exit: Some(status.exit_code()),
                signal: None,
            },
        )
    }

    fn force_stop(_process: Option<u32>, killer: &mut dyn ChildKiller) {
        let _ = killer.kill();
    }
}
