use std::future::Future;
use std::time::Duration;

use synt_actions::ActionError;
use synt_ssh::SshError;
use tokio::time;

use super::failure;

const CONNECT_LIMIT: Duration = Duration::from_secs(30);

pub(crate) async fn within_connect_limit<T>(
    connecting: impl Future<Output = Result<T, SshError>>,
) -> Result<T, ActionError> {
    let connected = time::timeout(CONNECT_LIMIT, connecting)
        .await
        .map_err(|_| {
            ActionError::failed(format!(
                "could not connect within {} s",
                CONNECT_LIMIT.as_secs()
            ))
        })?;

    connected.map_err(|error| failure(&error))
}
