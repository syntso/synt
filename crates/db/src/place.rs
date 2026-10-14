use std::net::SocketAddr;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Place {
    Network {
        host: String,
        port: u16,
        reached_at: Option<SocketAddr>,
    },
    File(PathBuf),
}
