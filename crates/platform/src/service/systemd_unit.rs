use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use directories::BaseDirs;

use super::tool;
use crate::{LoginStatus, PlatformError, replace_file};

const UNIT: &str = "synt-core.service";
const EXEC_START: &str = "ExecStart=";
const SYSTEMCTL_PLACES: [&str; 2] = ["/usr/bin/systemctl", "/bin/systemctl"];

pub(super) struct SystemdUnit {
    path: PathBuf,
    systemctl: PathBuf,
}

impl SystemdUnit {
    pub(super) fn for_this_user() -> Result<Self, PlatformError> {
        let base = BaseDirs::new()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no home directory"))?;
        let path = base.config_dir().join("systemd/user").join(UNIT);

        Ok(Self {
            path,
            systemctl: systemctl(),
        })
    }

    pub(super) fn install(&self, program: &Path) -> Result<(), PlatformError> {
        let program = program.to_str().ok_or_else(not_unicode)?;
        let contents = unit(&escape(program));

        if let Some(folder) = self.path.parent() {
            fs::create_dir_all(folder)?;
        }
        replace_file(&self.path, contents.as_bytes())?;

        tool::require(&self.systemctl, ["--user", "daemon-reload"])?;
        tool::require(&self.systemctl, ["--user", "enable", UNIT])?;

        Ok(())
    }

    pub(super) fn uninstall(&self) -> Result<bool, PlatformError> {
        if !self.path.try_exists()? {
            return Ok(false);
        }

        tool::succeeds(&self.systemctl, ["--user", "disable", UNIT])?;
        fs::remove_file(&self.path)?;
        tool::succeeds(&self.systemctl, ["--user", "daemon-reload"])?;

        Ok(true)
    }

    pub(super) fn status(&self) -> Result<LoginStatus, PlatformError> {
        let contents = match fs::read_to_string(&self.path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Ok(LoginStatus::NotRegistered);
            }
            Err(error) => return Err(error.into()),
        };

        let enabled = tool::succeeds(&self.systemctl, ["--user", "is-enabled", "--quiet", UNIT])?;
        let state = if enabled { "enabled" } else { "disabled" };

        Ok(LoginStatus::Registered {
            place: format!("systemd user unit {} ({state})", self.path.display()),
            program: program_in(&contents).map(PathBuf::from),
        })
    }
}

fn systemctl() -> PathBuf {
    SYSTEMCTL_PLACES
        .into_iter()
        .map(PathBuf::from)
        .find(|place| place.is_file())
        .unwrap_or_else(|| PathBuf::from(SYSTEMCTL_PLACES[0]))
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

    Some(unescape(quoted))
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

fn not_unicode() -> PlatformError {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "the core's path is not valid Unicode, so a systemd unit cannot name it",
    )
    .into()
}
