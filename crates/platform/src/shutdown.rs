mod native;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use native::NativeShutdown;
#[cfg(unix)]
use unix::Unix as Os;
#[cfg(windows)]
use windows::Windows as Os;

pub async fn shutdown_signal() {
    Os::signal().await;
}
