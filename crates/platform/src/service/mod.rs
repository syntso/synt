mod login;
mod program_check;
mod tool;

#[cfg(target_os = "macos")]
mod launch_agent;

#[cfg(all(unix, not(target_os = "macos")))]
mod systemd_unit;

#[cfg(windows)]
mod logon_task;

pub use login::{LoginStart, LoginStatus};
