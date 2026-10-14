use std::borrow::Borrow;

use synt_actions::ActionError;
use synt_ssh::SshError;

pub(crate) fn failure(error: impl Borrow<SshError>) -> ActionError {
    let error = error.borrow();

    let reason = match error {
        SshError::HostKeyUnknown { host, fingerprint } => format!(
            "the host key of {host} is not in known_hosts (fingerprint {fingerprint}); \
             check the fingerprint and add the key to ~/.ssh/known_hosts before trying again"
        ),
        SshError::HostKeyChanged { host, fingerprint } => format!(
            "the host key of {host} does not match known_hosts (server sent {fingerprint}); \
             someone may be intercepting the connection, so it was refused"
        ),
        SshError::HostKeyOtherType { host, fingerprint } => format!(
            "{host} presented a key of a type not pinned in known_hosts (fingerprint \
             {fingerprint}), while a key of another type is pinned; do not replace the pinned \
             key without checking the new one out of band"
        ),
        SshError::Timeout(limit) => {
            format!("the command was not started within {} s", limit.as_secs())
        }
        SshError::CommandTimeout { .. } => error.to_string(),
        other => format!("ssh: {other}"),
    };

    ActionError::Failed(reason)
}
