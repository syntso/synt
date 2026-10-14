use std::fs;
use std::path::{Path, PathBuf};

use crate::PlatformError;
use crate::files::Ownership;

#[derive(Debug, Clone)]
pub(super) struct Program {
    path: PathBuf,
}

impl Program {
    pub(super) fn checked(path: &Path) -> Result<Self, PlatformError> {
        if !path.is_absolute() {
            return Err(Self::refused(path, "it is not an absolute path"));
        }

        let folder = path
            .parent()
            .ok_or_else(|| Self::refused(path, "it has no folder"))?;

        if !fs::metadata(path)?.is_file() {
            return Err(Self::refused(path, "it is not a file"));
        }

        for place in [path, folder] {
            if fs::metadata(place)?.writable_by_others() {
                return Err(PlatformError::ProgramExposed {
                    program: path.to_owned(),
                    place: place.to_owned(),
                });
            }
        }

        Ok(Self {
            path: path.to_owned(),
        })
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    #[cfg(unix)]
    pub(super) fn unicode(&self, registration: &'static str) -> Result<&str, PlatformError> {
        self.path()
            .to_str()
            .ok_or(PlatformError::NotUnicode { registration })
    }

    fn refused(path: &Path, reason: &'static str) -> PlatformError {
        PlatformError::ProgramRefused {
            program: path.to_owned(),
            reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::Program;

    #[test]
    fn a_relative_path_is_refused() {
        let refused = Program::checked(Path::new("synt-core")).unwrap_err();

        assert_eq!(
            refused.to_string(),
            "synt-core is not registered to start at login: it is not an absolute path"
        );
    }

    #[test]
    fn a_folder_is_refused() {
        let folder = tempfile::tempdir().unwrap();

        let refused = Program::checked(folder.path()).unwrap_err();

        assert!(refused.to_string().ends_with("it is not a file"));
    }
}
