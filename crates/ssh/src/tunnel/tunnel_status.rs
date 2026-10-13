use super::{Destination, TunnelId};
use crate::ConnectionState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TunnelStatus {
    pub id: TunnelId,
    pub target: String,
    pub local_port: u16,
    pub destination: Destination,
    pub state: ConnectionState,
    pub active: usize,
    pub connections: u64,
    pub sent_bytes: u64,
    pub received_bytes: u64,
    pub last_error: Option<String>,
}
