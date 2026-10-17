use serde::Serialize;
use serde_json::{Value, json};
use synt_actions::{Action, ActionError, Context, Risk};
use synt_ssh::{ConnectionState, ConnectionStatus, Pool};

#[derive(Debug, Clone)]
pub struct SshConnections {
    pool: Pool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ConnectionInfo {
    pub target: String,
    pub via: Option<String>,
    pub state: ConnectionState,
    pub leases: usize,
    pub error: Option<String>,
}

impl SshConnections {
    pub fn new(pool: Pool) -> Self {
        Self { pool }
    }
}

impl Action for SshConnections {
    const ID: &'static str = "ssh.connections";
    const TITLE: &'static str = "List the shared SSH connections";
    const RISK: Risk = Risk::Read;

    type Params = ();
    type Output = Vec<ConnectionInfo>;

    fn summary(&self, output: &Vec<ConnectionInfo>) -> Value {
        let connected = output
            .iter()
            .filter(|connection| connection.state == ConnectionState::Connected)
            .count();

        json!({ "connections": output.len(), "connected": connected })
    }

    async fn run(&self, _context: &Context, (): ()) -> Result<Vec<ConnectionInfo>, ActionError> {
        let connections = self.pool.connections();

        Ok(connections.into_iter().map(ConnectionInfo::from).collect())
    }
}

impl From<ConnectionStatus> for ConnectionInfo {
    fn from(status: ConnectionStatus) -> Self {
        Self {
            target: status.target,
            via: status.via,
            state: status.state,
            leases: status.leases,
            error: status.error,
        }
    }
}
