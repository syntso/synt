use mysql_async::{Opts, OptsBuilder, SslOpts};

use crate::tls::SystemRoots;
use crate::{DbError, Server, TlsMode};

impl TryFrom<&Server> for Opts {
    type Error = DbError;

    fn try_from(server: &Server) -> Result<Self, DbError> {
        let (host, port) = server.dial()?;

        let builder = OptsBuilder::default()
            .ip_or_hostname(host)
            .tcp_port(port)
            .prefer_socket(false)
            .user(server.user())
            .pass(server.password())
            .db_name(server.database())
            .ssl_opts(ssl_options(server));

        Ok(builder.into())
    }
}

fn ssl_options(server: &Server) -> Option<SslOpts> {
    match server.tls() {
        TlsMode::Disable => None,
        TlsMode::Prefer | TlsMode::Require => Some(
            SslOpts::default()
                .with_danger_accept_invalid_certs(true)
                .with_danger_skip_domain_validation(true),
        ),
        TlsMode::VerifyFull => Some(verified(server)),
    }
}

fn verified(server: &Server) -> SslOpts {
    let roots = SystemRoots::load().into_der().map(Into::into).collect();
    let options = SslOpts::default()
        .with_disable_built_in_roots(true)
        .with_root_certs(roots);

    if !server.is_tunneled() {
        return options;
    }

    options.with_danger_tls_hostname_override(server.host().map(str::to_owned))
}
