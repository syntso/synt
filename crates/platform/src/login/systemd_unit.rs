use std::fs;
use std::io;
use std::path::PathBuf;

use directories::BaseDirs;

use super::{LoginStatus, Program, Registration, Tool};
use crate::{Found, PlatformError, replace_file};

const UNIT: &str = "synt-core.service";
const EXEC_START: &str = "ExecStart=";
const SYSTEMCTL_PLACES: [&str; 2] = ["/usr/bin/systemctl", "/bin/systemctl"];
const REGISTRATION: &str = "systemd unit";

pub(super) struct SystemdUnit {
    path: PathBuf,
    systemctl: Tool,
}

impl SystemdUnit {
    fn systemctl() -> Tool {
        let place = SYSTEMCTL_PLACES
            .into_iter()
            .map(PathBuf::from)
            .find(|place| place.is_file())
            .unwrap_or_else(|| PathBuf::from(SYSTEMCTL_PLACES[0]));

        Tool::new(place)
    }

    fn unit(program: &str) -> String {
        format!(
            "[Unit]\n\
             Description=Synt core\n\
             \n\
             [Service]\n\
             {EXEC_START}\"{program}\"\n\
             Restart=no\n\
             \n\
             [Install]\n\
             WantedBy=default.target\n"
        )
    }

    fn program_in(contents: &str) -> Option<String> {
        let line = contents
            .lines()
            .find_map(|line| line.strip_prefix(EXEC_START))?;
        let quoted = line.trim().strip_prefix('"')?.strip_suffix('"')?;

        Some(Self::unescape(quoted))
    }

    fn escape(text: &str) -> String {
        text.replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('%', "%%")
            .replace('$', "$$")
    }

    fn unescape(text: &str) -> String {
        text.replace("$$", "$")
            .replace("%%", "%")
            .replace("\\\"", "\"")
            .replace("\\\\", "\\")
    }
}

impl Registration for SystemdUnit {
    fn for_this_user() -> Result<Self, PlatformError> {
        let base = BaseDirs::new()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no home directory"))?;
        let path = base.config_dir().join("systemd/user").join(UNIT);

        Ok(Self {
            path,
            systemctl: Self::systemctl(),
        })
    }

    fn install(&self, program: &Program) -> Result<(), PlatformError> {
        let program = program.unicode(REGISTRATION)?;
        let contents = Self::unit(&Self::escape(program));

        if let Some(folder) = self.path.parent() {
            fs::create_dir_all(folder)?;
        }
        replace_file(&self.path, contents.as_bytes())?;

        self.systemctl.require(["--user", "daemon-reload"])?;
        self.systemctl.require(["--user", "enable", UNIT])?;

        Ok(())
    }

    fn uninstall(&self) -> Result<bool, PlatformError> {
        if !self.path.try_exists()? {
            return Ok(false);
        }

        self.systemctl.succeeds(["--user", "disable", UNIT])?;
        fs::remove_file(&self.path)?;
        self.systemctl.succeeds(["--user", "daemon-reload"])?;

        Ok(true)
    }

    fn status(&self) -> Result<LoginStatus, PlatformError> {
        let Some(contents) = fs::read_to_string(&self.path).found()? else {
            return Ok(LoginStatus::NotRegistered);
        };

        let enabled = self
            .systemctl
            .succeeds(["--user", "is-enabled", "--quiet", UNIT])?;
        let state = if enabled { "enabled" } else { "disabled" };

        Ok(LoginStatus::Registered {
            place: format!("systemd user unit {} ({state})", self.path.display()),
            program: Self::program_in(&contents).map(PathBuf::from),
        })
    }
}
