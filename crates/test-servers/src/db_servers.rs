use std::env;
use std::fs;
use std::path::Path;

const SWITCH: &str = "SYNT_DB_TESTS";
const PORTS_FILE: &str = "../../tests/db/run/ports";
const START: &str = "tests/db/up.sh";

#[derive(Debug, Clone)]
pub struct DbServers {
    ports: String,
}

impl DbServers {
    pub const ADDRESS: &'static str = "127.0.0.1";
    pub const USER: &'static str = "synt";
    pub const PASSWORD: &'static str = "synt-test-password";
    pub const DATABASE: &'static str = "synt";

    pub fn running() -> Option<Self> {
        if env::var(SWITCH).as_deref() != Ok("1") {
            eprintln!("skipped: set {SWITCH}=1 after {START} to run tests against databases");
            return None;
        }

        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(PORTS_FILE);
        let ports = fs::read_to_string(&path).unwrap_or_else(|error| {
            panic!(
                "{SWITCH}=1 but {} cannot be read ({error}); run {START} first",
                path.display()
            )
        });

        Some(Self { ports })
    }

    pub fn postgres_port(&self) -> u16 {
        self.port("postgres")
    }

    pub fn mysql_port(&self) -> u16 {
        self.port("mysql")
    }

    pub fn mariadb_port(&self) -> u16 {
        self.port("mariadb")
    }

    pub fn redis_port(&self) -> u16 {
        self.port("redis")
    }

    pub fn redis_tls_port(&self) -> u16 {
        self.port("redis_tls")
    }

    fn port(&self, service: &str) -> u16 {
        self.ports
            .lines()
            .find_map(|line| line.strip_prefix(service)?.strip_prefix('='))
            .and_then(|port| port.trim().parse().ok())
            .unwrap_or_else(|| panic!("{START} wrote no port for {service}"))
    }
}
