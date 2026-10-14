use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use http::Uri;
use tower::Service;

use super::{DialedStream, Dialer};

const HTTPS_PORT: u16 = 443;
const HTTP_PORT: u16 = 80;

type Connecting = Pin<Box<dyn Future<Output = io::Result<DialedStream>> + Send>>;

#[derive(Clone)]
pub(crate) struct DialedConnector {
    dialer: Arc<dyn Dialer>,
}

impl DialedConnector {
    pub(crate) fn new(dialer: Arc<dyn Dialer>) -> Self {
        Self { dialer }
    }
}

impl Service<Uri> for DialedConnector {
    type Response = DialedStream;
    type Error = io::Error;
    type Future = Connecting;

    fn poll_ready(&mut self, _context: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, uri: Uri) -> Connecting {
        let dialer = Arc::clone(&self.dialer);

        Box::pin(async move {
            let (host, port) = address(&uri)?;
            let stream = dialer.dial(&host, port).await?;

            Ok(DialedStream::new(stream))
        })
    }
}

fn address(uri: &Uri) -> io::Result<(String, u16)> {
    let host = uri
        .host()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, format!("{uri} has no host")))?;
    let host = host
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_owned();

    let default_port = if uri.scheme_str() == Some("http") {
        HTTP_PORT
    } else {
        HTTPS_PORT
    };

    Ok((host, uri.port_u16().unwrap_or(default_port)))
}

#[cfg(test)]
mod tests {
    use http::Uri;

    use super::address;

    fn parts(text: &str) -> (String, u16) {
        address(&text.parse::<Uri>().expect("a uri")).expect("an address")
    }

    #[test]
    fn the_port_comes_from_the_url_or_its_scheme() {
        assert_eq!(parts("https://k3s:6443"), ("k3s".to_owned(), 6443));
        assert_eq!(
            parts("https://api.example.com"),
            ("api.example.com".to_owned(), 443)
        );
        assert_eq!(parts("http://localhost"), ("localhost".to_owned(), 80));
    }

    #[test]
    fn an_ipv6_host_loses_its_brackets() {
        assert_eq!(
            parts("https://[fd00::1]:6443"),
            ("fd00::1".to_owned(), 6443)
        );
    }
}
