use std::os::unix::process::ExitStatusExt;
use std::path::PathBuf;
use std::process::ExitStatus;

use nix::sys::signal::{self, Signal};
use nix::unistd::{self, Pid, User};
use portable_pty::{Child, ChildKiller};

use super::{NativePty, PtyEnd};

pub(super) struct Unix;

impl Unix {
    fn end_of(status: ExitStatus) -> PtyEnd {
        let exit = status.code().and_then(|code| u32::try_from(code).ok());
        let signal = status.signal().map(Self::signal_name);

        PtyEnd { exit, signal }
    }

    fn signal_name(number: i32) -> String {
        match Signal::try_from(number) {
            Ok(signal) => signal.as_str().trim_start_matches("SIG").to_owned(),
            Err(_) => number.to_string(),
        }
    }
}

impl NativePty for Unix {
    const ASKS_FOR_CURSOR: bool = false;

    fn login_shell() -> Option<PathBuf> {
        let user = User::from_uid(unistd::getuid()).ok()??;

        if user.shell.as_os_str().is_empty() {
            return None;
        }

        Some(user.shell)
    }

    fn wait_for(child: Box<dyn Child + Send + Sync>) -> PtyEnd {
        let child: Box<dyn Child> = child;

        match child.downcast::<std::process::Child>() {
            Ok(mut process) => process.wait().map(Self::end_of).unwrap_or_default(),
            Err(mut other) => other.wait().map_or_else(
                |_| PtyEnd::default(),
                |status| PtyEnd {
                    exit: Some(status.exit_code()),
                    signal: status.signal().map(str::to_owned),
                },
            ),
        }
    }

    fn force_stop(process: Option<u32>, _killer: &mut dyn ChildKiller) {
        let Some(process) = process.and_then(|process| i32::try_from(process).ok()) else {
            return;
        };

        let _ = signal::kill(Pid::from_raw(process), Signal::SIGKILL);
    }
}
