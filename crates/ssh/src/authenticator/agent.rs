use russh::keys::agent::client::{AgentClient, AgentStream};

#[cfg(windows)]
const OPENSSH_AGENT_PIPE: &str = r"\\.\pipe\openssh-ssh-agent";

pub(super) type Agent = AgentClient<Box<dyn AgentStream + Send + Unpin + 'static>>;

#[cfg(unix)]
pub(super) async fn connect() -> Option<Agent> {
    let client = AgentClient::connect_env().await.ok()?;

    Some(client.dynamic())
}

#[cfg(windows)]
pub(super) async fn connect() -> Option<Agent> {
    let pipe = std::env::var("SSH_AUTH_SOCK").unwrap_or_else(|_| OPENSSH_AGENT_PIPE.to_owned());
    let client = AgentClient::connect_named_pipe(pipe).await.ok()?;

    Some(client.dynamic())
}
