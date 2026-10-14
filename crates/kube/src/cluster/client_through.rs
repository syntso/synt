use std::sync::Arc;

use hyper_util::rt::TokioExecutor;
use kube::client::ConfigExt;
use kube::{Client, Config};
use tower::{BoxError, ServiceBuilder};

use crate::KubeError;
use crate::transport::{DialedConnector, Dialer};

pub(super) fn client_through(
    config: &Config,
    dialer: Arc<dyn Dialer>,
) -> Result<Client, KubeError> {
    let connector = config
        .rustls_https_connector_with_connector(DialedConnector::new(dialer))
        .map_err(|error| connect_failure(&error))?;
    let http = hyper_util::client::legacy::Client::builder(TokioExecutor::new()).build(connector);

    let service = ServiceBuilder::new()
        .layer(config.base_uri_layer())
        .option_layer(
            config
                .auth_layer()
                .map_err(|error| connect_failure(&error))?,
        )
        .layer(
            config
                .extra_headers_layer()
                .map_err(|error| connect_failure(&error))?,
        )
        .map_err(BoxError::from)
        .service(http);

    Ok(Client::new(service, config.default_namespace.clone()))
}

fn connect_failure(error: &kube::Error) -> KubeError {
    KubeError::Connect(error.to_string())
}
