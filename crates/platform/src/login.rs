#[cfg(target_os = "macos")]
mod launch_agent;
#[cfg(windows)]
mod logon_task;
mod program;
mod registration;
mod start;
mod status;
#[cfg(all(unix, not(target_os = "macos")))]
mod systemd_unit;
mod tool;

#[cfg(target_os = "macos")]
use launch_agent::LaunchAgent as Os;
#[cfg(windows)]
use logon_task::LogonTask as Os;
use program::Program;
use registration::Registration;
pub use start::LoginStart;
pub use status::LoginStatus;
#[cfg(all(unix, not(target_os = "macos")))]
use systemd_unit::SystemdUnit as Os;
use tool::Tool;
