use std::fs;
use std::path::PathBuf;

use nix::unistd::getuid;

use super::{LoginStatus, Program, Registration, Tool};
use crate::{Found, PlatformError, home_directory, replace_file};

const LABEL: &str = "so.synt.core";
const LAUNCHCTL: &str = "/bin/launchctl";
const PLUTIL: &str = "/usr/bin/plutil";
const PROGRAM_KEY: &str = "<key>ProgramArguments</key>";
const STRING_OPEN: &str = "<string>";
const STRING_CLOSE: &str = "</string>";
const REGISTRATION: &str = "launch agent";

pub(super) struct LaunchAgent {
    plist: PathBuf,
}

impl LaunchAgent {
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

        Some(Self::unescape(&after_key[start..start + length]))
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
}

impl Registration for LaunchAgent {
    fn for_this_user() -> Result<Self, PlatformError> {
        let plist = home_directory()?
            .join("Library/LaunchAgents")
            .join(format!("{LABEL}.plist"));

        Ok(Self { plist })
    }

    fn install(&self, program: &Program) -> Result<(), PlatformError> {
        let program = program.unicode(REGISTRATION)?;
        let contents = Self::plist(&Self::escape(program));

        if let Some(folder) = self.plist.parent() {
            fs::create_dir_all(folder)?;
        }
        replace_file(&self.plist, contents.as_bytes())?;

        Tool::new(PLUTIL).require(["-lint".as_ref(), self.plist.as_os_str()])?;

        Ok(())
    }

    fn uninstall(&self) -> Result<bool, PlatformError> {
        if !self.plist.try_exists()? {
            return Ok(false);
        }

        let service = format!("gui/{}/{LABEL}", getuid());
        Tool::new(LAUNCHCTL).succeeds(["bootout", service.as_str()])?;

        fs::remove_file(&self.plist)?;

        Ok(true)
    }

    fn status(&self) -> Result<LoginStatus, PlatformError> {
        let Some(contents) = fs::read_to_string(&self.plist).found()? else {
            return Ok(LoginStatus::NotRegistered);
        };

        Ok(LoginStatus::Registered {
            place: format!("launch agent {}", self.plist.display()),
            program: Self::program_in(&contents).map(PathBuf::from),
        })
    }
}
