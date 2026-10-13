use std::io;
use std::process;

use sysinfo::{Pid, Process, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
use tokio::task;

use super::Stream;
use crate::{Peer, UserId};

pub(crate) async fn peer_of(stream: &Stream) -> io::Result<Peer> {
    let Stream::Server(pipe) = stream else {
        return Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "only the core's end of a pipe can see who is on the other end",
        ));
    };

    let process = pipe.client_process_id()?;
    let user = task::spawn_blocking(move || user_of(process))
        .await
        .map_err(io::Error::other)?;

    Ok(Peer::new(user.ok(), Some(process)))
}

pub(crate) fn current_user() -> io::Result<UserId> {
    user_of(process::id())
}

fn user_of(process: u32) -> io::Result<UserId> {
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
