use super::ConnectionState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectionStatus {
    pub target: String,
    pub via: Option<String>,
    pub state: ConnectionState,
    pub leases: usize,
    pub error: Option<String>,
}
