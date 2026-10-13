mod directories;
mod endpoint;
mod listener;
mod shutdown;
mod stream;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix as native;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows as native;

pub use directories::data_directory;
pub use endpoint::Endpoint;
pub use listener::LocalListener;
pub use shutdown::shutdown_signal;
pub use stream::LocalStream;
