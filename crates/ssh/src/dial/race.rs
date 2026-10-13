use std::io;
use std::net::SocketAddr;
use std::time::Duration;

use tokio::net::TcpStream;
use tokio::task::JoinSet;
use tokio::time;

const STAGGER: Duration = Duration::from_millis(250);
const ATTEMPT_LIMIT: Duration = Duration::from_secs(10);

pub(crate) async fn race(addresses: &[SocketAddr]) -> io::Result<TcpStream> {
    let mut waiting = interleaved(addresses).into_iter();
    let mut attempts = JoinSet::new();
    let mut last_error = None;

    loop {
        if attempts.is_empty() {
            let Some(next) = waiting.next() else {
                return Err(last_error.unwrap_or_else(no_addresses));
            };
            attempts.spawn(attempt(next));
        }

        let has_more = !waiting.as_slice().is_empty();

        tokio::select! {
            Some(finished) = attempts.join_next() => match finished {
                Ok(Ok(stream)) => return Ok(stream),
                Ok(Err(error)) => {
                    last_error = Some(error);

                    if let Some(next) = waiting.next() {
                        attempts.spawn(attempt(next));
                    }
                }
                Err(stopped) => last_error = Some(io::Error::other(stopped)),
            },
            () = time::sleep(STAGGER), if has_more => {
                if let Some(next) = waiting.next() {
                    attempts.spawn(attempt(next));
                }
            }
        }
    }
}

async fn attempt(address: SocketAddr) -> io::Result<TcpStream> {
    let connecting = TcpStream::connect(address);

    let stream = time::timeout(ATTEMPT_LIMIT, connecting)
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, format!("{address} timed out")))??;

    stream.set_nodelay(true)?;

    Ok(stream)
}

fn interleaved(addresses: &[SocketAddr]) -> Vec<SocketAddr> {
    let Some(first) = addresses.first() else {
        return Vec::new();
    };

    let (mut preferred, mut other): (Vec<SocketAddr>, Vec<SocketAddr>) = addresses
        .iter()
        .partition(|address| address.is_ipv6() == first.is_ipv6());

    preferred.reverse();
    other.reverse();

    let mut ordered = Vec::with_capacity(addresses.len());

    while !preferred.is_empty() || !other.is_empty() {
        ordered.extend(preferred.pop());
        ordered.extend(other.pop());
    }

    ordered
}

fn no_addresses() -> io::Error {
    io::Error::new(io::ErrorKind::NotFound, "no address to connect to")
}
