use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Destination {
    host: String,
    port: u16,
}

impl Destination {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
        }
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

impl Display for Destination {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        if self.host.contains(':') {
            return write!(formatter, "[{}]:{}", self.host, self.port);
        }

        write!(formatter, "{}:{}", self.host, self.port)
    }
}
