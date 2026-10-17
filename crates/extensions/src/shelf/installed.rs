use rusqlite::Row;
use rusqlite::types::Type;
use semver::Version;
use synt_journal::Timestamp;

use crate::install::Package;
use crate::manifest::{ExtensionId, OneLine};
use crate::sha256::Sha256;
use crate::{ExtensionError, ManifestError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Installed {
    id: ExtensionId,
    version: OneLine,
    sha256: Sha256,
    manifest_sha256: Sha256,
    signed: bool,
    enabled: bool,
    added_at: Timestamp,
}

impl Installed {
    pub(crate) fn new(package: &Package, manifest_sha256: Sha256, added_at: Timestamp) -> Self {
        let manifest = package.manifest();

        Self {
            id: manifest.id().clone(),
            version: manifest.version().clone(),
            sha256: package.sha256().clone(),
            manifest_sha256,
            signed: package.signed(),
            enabled: true,
            added_at,
        }
    }

    pub(crate) fn id(&self) -> &ExtensionId {
        &self.id
    }

    pub(crate) fn version(&self) -> &OneLine {
        &self.version
    }

    pub(crate) fn sha256(&self) -> &Sha256 {
        &self.sha256
    }

    pub(crate) fn manifest_sha256(&self) -> &Sha256 {
        &self.manifest_sha256
    }

    pub(crate) fn signed(&self) -> bool {
        self.signed
    }

    pub(crate) fn enabled(&self) -> bool {
        self.enabled
    }

    pub(crate) fn added_at(&self) -> Timestamp {
        self.added_at
    }

    #[must_use]
    pub(crate) fn enabled_as(self, enabled: bool) -> Self {
        Self { enabled, ..self }
    }

    pub(crate) fn check_not_newer_than(&self, offered: &OneLine) -> Result<(), ExtensionError> {
        let offered_version =
            Version::parse(offered.as_str()).map_err(|source| ExtensionError::NotSemver {
                version: offered.to_string(),
                source,
            })?;
        let Ok(current) = Version::parse(self.version.as_str()) else {
            return Ok(());
        };

        if offered_version < current {
            return Err(ExtensionError::Downgrade {
                id: self.id.to_string(),
                offered: offered.to_string(),
                current: current.to_string(),
            });
        }

        Ok(())
    }

    pub(super) fn from_row(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: Self::checked(row, "id")?,
            version: Self::checked(row, "version")?,
            sha256: row.get("sha256")?,
            manifest_sha256: row.get("manifest_sha256")?,
            signed: row.get("signed")?,
            enabled: row.get("enabled")?,
            added_at: Timestamp::from_millis(row.get("installed_at")?),
        })
    }

    fn checked<T: TryFrom<String, Error = ManifestError>>(
        row: &Row<'_>,
        column: &str,
    ) -> rusqlite::Result<T> {
        let text: String = row.get(column)?;

        T::try_from(text).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
        })
    }
}
