use nix::spawn::{PosixSpawnAttr, PosixSpawnFlags};
use nix::sys::signal::{SigSet, Signal};
use nix::unistd::Pid;

pub(crate) fn detached_attributes() -> nix::Result<PosixSpawnAttr> {
    let mut attributes = PosixSpawnAttr::init()?;

    attributes.set_flags(flags())?;
    attributes.set_pgroup(Pid::from_raw(0))?;
    attributes.set_sigmask(&SigSet::empty())?;
    attributes.set_sigdefault(&catchable_signals())?;

    Ok(attributes)
}

fn flags() -> PosixSpawnFlags {
    PosixSpawnFlags::POSIX_SPAWN_SETPGROUP
        | PosixSpawnFlags::POSIX_SPAWN_SETSIGMASK
        | PosixSpawnFlags::POSIX_SPAWN_SETSIGDEF
        | own_descriptors_only()
}

#[cfg(target_os = "macos")]
fn own_descriptors_only() -> PosixSpawnFlags {
    PosixSpawnFlags::from_bits_retain(nix::libc::POSIX_SPAWN_CLOEXEC_DEFAULT)
}

#[cfg(not(target_os = "macos"))]
fn own_descriptors_only() -> PosixSpawnFlags {
    PosixSpawnFlags::empty()
}

fn catchable_signals() -> SigSet {
    let mut signals = SigSet::all();
    signals.remove(Signal::SIGKILL);
    signals.remove(Signal::SIGSTOP);
    signals
}
