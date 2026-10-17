use russh::keys::PublicKey;
use russh::keys::agent::AgentIdentity;
use russh::keys::agent::client::{AgentClient, AgentStream};

#[cfg(windows)]
const OPENSSH_AGENT_PIPE: &str = r"\\.\pipe\openssh-ssh-agent";

type Client = AgentClient<Box<dyn AgentStream + Send + Unpin + 'static>>;

pub(super) struct Agent {
    client: Client,
}

impl Agent {
    pub(super) async fn connect() -> Option<Self> {
        let client = Self::client().await?;

        Some(Self { client })
    }

    pub(super) async fn held_keys(&mut self) -> Vec<PublicKey> {
        let Ok(identities) = self.client.request_identities().await else {
            return Vec::new();
        };

        identities
            .into_iter()
            .filter_map(|identity| match identity {
                AgentIdentity::PublicKey { key, .. } => Some(key),
                AgentIdentity::Certificate { .. } => None,
            })
            .collect()
    }

    pub(super) fn signer(&mut self) -> &mut Client {
        &mut self.client
    }

    #[cfg(unix)]
    async fn client() -> Option<Client> {
        let client = AgentClient::connect_env().await.ok()?;

        Some(client.dynamic())
    }

    #[cfg(windows)]
    async fn client() -> Option<Client> {
        let pipe =
            std::env::var("SSH_AUTH_SOCK").unwrap_or_else(|_| OPENSSH_AGENT_PIPE.to_owned());
        let client = AgentClient::connect_named_pipe(pipe).await.ok()?;

        Some(client.dynamic())
    }
}
