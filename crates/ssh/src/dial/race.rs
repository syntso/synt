use std::io;
use std::net::SocketAddr;
use std::time::Duration;
use std::vec;

use tokio::net::TcpStream;
use tokio::task::JoinSet;
use tokio::time;

const STAGGER: Duration = Duration::from_millis(250);
const ATTEMPT_LIMIT: Duration = Duration::from_secs(10);

pub(crate) struct Race {
    waiting: vec::IntoIter<SocketAddr>,
    attempts: JoinSet<io::Result<TcpStream>>,
    last_error: Option<io::Error>,
}

impl Race {
    pub(crate) fn between(addresses: &[SocketAddr]) -> Self {
        Self {
            waiting: Self::interleaved(addresses).into_iter(),
            attempts: JoinSet::new(),
            last_error: None,
        }
    }

    pub(crate) async fn winner(mut self) -> io::Result<TcpStream> {
        loop {
            if self.attempts.is_empty() && !self.start_next() {
                return Err(self.last_error.unwrap_or_else(Self::no_addresses));
            }

            let has_more = !self.waiting.as_slice().is_empty();

            tokio::select! {
                Some(finished) = self.attempts.join_next() => match finished {
                    Ok(Ok(stream)) => return Ok(stream),
                    Ok(Err(error)) => {
                        self.last_error = Some(error);
                        self.start_next();
                    }
                    Err(stopped) => self.last_error = Some(io::Error::other(stopped)),
                },
                () = time::sleep(STAGGER), if has_more => {
                    self.start_next();
                }
            }
        }
    }

    fn start_next(&mut self) -> bool {
        let Some(next) = self.waiting.next() else {
            return false;
        };

        self.attempts.spawn(Self::attempt(next));

        true
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
}
