use std::io;
use std::process;

use sysinfo::{Pid, Process, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

use super::{NativeUser, UserId};

pub(super) struct Windows;

impl Windows {
    pub(super) fn of_process(process: u32) -> io::Result<UserId> {
        let pid = Pid::from_u32(process);
        let refresh = ProcessRefreshKind::nothing().with_user(UpdateKind::Always);

        let mut system = System::new();
        system.refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), true, refresh);

        let user = system
            .process(pid)
            .and_then(Process::user_id)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("the system knows no user for process {process}"),
                )
            })?;

        Ok(UserId::new(user.to_string()))
    }
}

impl NativeUser for Windows {
    fn current() -> io::Result<UserId> {
        Self::of_process(process::id())
    }
}
