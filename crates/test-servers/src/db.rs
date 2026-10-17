use crate::run_folder::RunFolder;

const SWITCH: &str = "SYNT_DB_TESTS";
const RUN_FOLDER: &str = "../../tests/db/run";
const PORTS_FILE: &str = "ports";
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
        let run = RunFolder::switched_on(SWITCH, START, RUN_FOLDER, "databases")?;
        let ports = run.read(PORTS_FILE);

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
