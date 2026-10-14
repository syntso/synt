use rusqlite::Row;
use semver::Version;

use super::database::unsigned_number;
use crate::ExtensionError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Installed {
    pub id: String,
    pub version: String,
    pub sha256: String,
    pub manifest_sha256: String,
    pub signed: bool,
    pub enabled: bool,
    pub added_at: u64,
}

impl Installed {
    pub(crate) fn check_not_newer_than(&self, offered: &str) -> Result<(), ExtensionError> {
        let offered = Version::parse(offered).map_err(|error| {
            ExtensionError::Refused(format!(
                "the listed version {offered} is not semver: {error}"
            ))
        })?;
        let Ok(current) = Version::parse(&self.version) else {
            return Ok(());
        };

        if offered < current {
            return Err(ExtensionError::Refused(format!(
                "the registry offers {} {offered}, older than the installed {current}",
                self.id
            )));
        }

        Ok(())
    }

    pub(super) fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get("id")?,
            version: row.get("version")?,
            sha256: row.get("sha256")?,
            manifest_sha256: row.get("manifest_sha256")?,
            signed: row.get("signed")?,
            enabled: row.get("enabled")?,
            added_at: unsigned_number(row.get("installed_at")?),
        })
    }
}
