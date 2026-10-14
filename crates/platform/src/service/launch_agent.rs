use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use nix::unistd::getuid;

use super::tool;
use crate::files::found;
use crate::{LoginStatus, PlatformError, home_directory, replace_file};

const LABEL: &str = "so.synt.core";
const LAUNCHCTL: &str = "/bin/launchctl";
const PLUTIL: &str = "/usr/bin/plutil";
const PROGRAM_KEY: &str = "<key>ProgramArguments</key>";
const STRING_OPEN: &str = "<string>";
const STRING_CLOSE: &str = "</string>";

pub(super) struct LaunchAgent {
    plist: PathBuf,
}

impl LaunchAgent {
    pub(super) fn for_this_user() -> Result<Self, PlatformError> {
        let plist = home_directory()?
            .join("Library/LaunchAgents")
            .join(format!("{LABEL}.plist"));

        Ok(Self { plist })
    }

    pub(super) fn install(&self, program: &Path) -> Result<(), PlatformError> {
        let program = program.to_str().ok_or_else(not_unicode)?;
        let contents = plist(&escape(program));

        if let Some(folder) = self.plist.parent() {
            fs::create_dir_all(folder)?;
        }
        replace_file(&self.plist, contents.as_bytes())?;

        tool::require(
            Path::new(PLUTIL),
            ["-lint".as_ref(), self.plist.as_os_str()],
        )?;

        Ok(())
    }

    pub(super) fn uninstall(&self) -> Result<bool, PlatformError> {
        if !self.plist.try_exists()? {
            return Ok(false);
        }

        let service = format!("gui/{}/{LABEL}", getuid());
        tool::succeeds(Path::new(LAUNCHCTL), ["bootout", service.as_str()])?;

        fs::remove_file(&self.plist)?;

        Ok(true)
    }

    pub(super) fn status(&self) -> Result<LoginStatus, PlatformError> {
        let Some(contents) = found(fs::read_to_string(&self.plist))? else {
            return Ok(LoginStatus::NotRegistered);
        };

        Ok(LoginStatus::Registered {
            place: format!("launch agent {}", self.plist.display()),
            program: program_in(&contents).map(PathBuf::from),
        })
    }
}

fn plist(program: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>Label</key>
	<string>{LABEL}</string>
	{PROGRAM_KEY}
	<array>
		<string>{program}</string>
	</array>
	<key>RunAtLoad</key>
	<true/>
	<key>KeepAlive</key>
	<false/>
	<key>ProcessType</key>
	<string>Background</string>
</dict>
</plist>
"#
    )
}

fn program_in(contents: &str) -> Option<String> {
    let after_key = &contents[contents.find(PROGRAM_KEY)?..];
    let start = after_key.find(STRING_OPEN)? + STRING_OPEN.len();
    let length = after_key[start..].find(STRING_CLOSE)?;

    Some(unescape(&after_key[start..start + length]))
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn unescape(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

fn not_unicode() -> PlatformError {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "the core's path is not valid Unicode, so a launch agent cannot name it",
    )
    .into()
}
