use std::future::{self, Future};
use std::io;

#[cfg(target_os = "macos")]
use nix::sys::socket::{getsockopt, sockopt::LocalPeerToken};
use nix::unistd;

use super::Stream;
#[cfg(target_os = "macos")]
use crate::peer::AuditToken;
use crate::{Peer, UserId};

pub(crate) fn peer_of(stream: &Stream) -> impl Future<Output = io::Result<Peer>> + use<> {
    future::ready(identify(stream))
}

pub(crate) fn current_user() -> UserId {
    UserId::new(unistd::geteuid().as_raw())
}

fn identify(stream: &Stream) -> io::Result<Peer> {
    let credentials = stream.peer_cred()?;

    let user = UserId::new(credentials.uid());
    let process = credentials
        .pid()
        .and_then(|process| u32::try_from(process).ok());
    let peer = Peer::new(Some(user), process);

    #[cfg(target_os = "macos")]
    let peer = peer.with_audit_token(audit_token(stream));

    Ok(peer)
}

#[cfg(target_os = "macos")]
fn audit_token(stream: &Stream) -> Option<AuditToken> {
    let token = getsockopt(stream, LocalPeerToken).ok()?;

    let mut bytes = [0; 32];
    let (chunks, _) = bytes.as_chunks_mut::<4>();
    for (chunk, value) in chunks.iter_mut().zip(token.val) {
        *chunk = value.to_ne_bytes();
    }

    Some(bytes)
}
