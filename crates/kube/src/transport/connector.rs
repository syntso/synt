use std::future::Future;
use std::io;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use http::Uri;
use hyper_util::rt::TokioExecutor;
use kube::client::ConfigExt;
use kube::{Client, Config};
use tower::{BoxError, Service, ServiceBuilder};

use super::{Address, Dialer, Socket};
use crate::KubeError;

type Connecting = Pin<Box<dyn Future<Output = io::Result<Socket>> + Send>>;

#[derive(Clone)]
pub(crate) struct Connector {
    dialer: Arc<dyn Dialer>,
}

impl Connector {
    pub(crate) fn new(dialer: Arc<dyn Dialer>) -> Self {
        Self { dialer }
    }

    pub(crate) fn client(self, config: &Config) -> Result<Client, KubeError> {
        let connector = config.rustls_https_connector_with_connector(self)?;
        let http =
            hyper_util::client::legacy::Client::builder(TokioExecutor::new()).build(connector);

        let service = ServiceBuilder::new()
            .layer(config.base_uri_layer())
            .option_layer(config.auth_layer()?)
            .layer(config.extra_headers_layer()?)
            .map_err(BoxError::from)
            .service(http);

        Ok(Client::new(service, config.default_namespace.clone()))
    }
}

impl Service<Uri> for Connector {
    type Response = Socket;
    type Error = io::Error;
    type Future = Connecting;

    fn poll_ready(&mut self, _context: &mut Context<'_>) -> Poll<Result<(), io::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, uri: Uri) -> Connecting {
        let dialer = Arc::clone(&self.dialer);

        Box::pin(async move {
            let address = Address::try_from(&uri)?;
            let stream = dialer.dial(&address.host, address.port).await?;

            Ok(Socket::new(stream))
        })
    }
}
