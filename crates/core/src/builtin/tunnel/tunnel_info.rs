use serde::Serialize;
use serde_json::{Value, json};
use synt_ssh::TunnelStatus;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TunnelInfo {
    pub id: u64,
    pub target: String,
    pub local_port: u16,
    pub remote_host: String,
    pub remote_port: u16,
    pub state: String,
    pub active: usize,
    pub connections: u64,
    pub sent_bytes: u64,
    pub received_bytes: u64,
    pub last_error: Option<String>,
}

impl TunnelInfo {
    pub(super) fn ports(&self) -> Value {
        json!({
            "id": self.id,
            "target": self.target,
            "local_port": self.local_port,
            "remote_host": self.remote_host,
            "remote_port": self.remote_port,
        })
    }
}

impl From<TunnelStatus> for TunnelInfo {
    fn from(status: TunnelStatus) -> Self {
        Self {
            id: status.id.value(),
            target: status.target,
            local_port: status.local_port,
            remote_host: status.destination.host().to_owned(),
            remote_port: status.destination.port(),
            state: status.state.name().to_owned(),
            active: status.active,
            connections: status.connections,
            sent_bytes: status.sent_bytes,
            received_bytes: status.received_bytes,
            last_error: status.last_error,
        }
    }
}
