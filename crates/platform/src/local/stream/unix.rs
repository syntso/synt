use std::ffi::OsStr;
use std::io;

#[cfg(target_os = "macos")]
use nix::sys::socket::{getsockopt, sockopt::LocalPeerToken};
use tokio::net::UnixStream;

use super::NativeStream;
#[cfg(target_os = "macos")]
use crate::peer::AuditToken;
use crate::{Peer, UserId};

impl NativeStream for UnixStream {
    async fn open(address: &OsStr) -> io::Result<Self> {
        Self::connect(address).await
    }

    async fn peer(&self) -> io::Result<Peer> {
        let credentials = self.peer_cred()?;

        let user = UserId::new(credentials.uid());
        let process = credentials
            .pid()
            .and_then(|process| u32::try_from(process).ok());
        let peer = Peer::new(Some(user), process);

        #[cfg(target_os = "macos")]
        let peer = peer.with_audit_token(audit_token(self));

        Ok(peer)
    }
}

#[cfg(target_os = "macos")]
fn audit_token(stream: &UnixStream) -> Option<AuditToken> {
    let token = getsockopt(stream, LocalPeerToken).ok()?;

    let mut bytes = [0; 32];
    let (chunks, _) = bytes.as_chunks_mut::<4>();
    for (chunk, value) in chunks.iter_mut().zip(token.val) {
        *chunk = value.to_ne_bytes();
    }

    Some(bytes)
}
