use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ForwardStatus {
    pub target: String,
    pub namespace: String,
    pub pod: String,
    pub remote_port: u16,
    pub local_port: u16,
    pub connections: u64,
    pub active: usize,
    pub sent_bytes: u64,
    pub received_bytes: u64,
    pub last_error: Option<String>,
}
