use std::env;
use std::fs;
use std::io;
use std::path::{self, Path, PathBuf};

use directories::BaseDirs;

use super::NativeDirectories;

const SYSTEM_ROOT: &str = "SystemRoot";
const DEFAULT_SYSTEM_ROOT: &str = r"C:\Windows";

pub(super) struct Windows;

impl Windows {
    pub(super) fn system_root() -> PathBuf {
        env::var_os(SYSTEM_ROOT)
            .map(PathBuf::from)
            .filter(|root| root.is_absolute())
            .unwrap_or_else(|| PathBuf::from(DEFAULT_SYSTEM_ROOT))
    }

    fn local_app_data() -> io::Result<PathBuf> {
        let base = BaseDirs::new().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "no home directory for this user")
        })?;

        Ok(base.data_local_dir().to_owned())
    }

    fn resolved(path: &Path) -> io::Result<PathBuf> {
        let absolute = path::absolute(path)?;
        let mut existing = absolute.as_path();
        let mut missing = Vec::new();

        while !existing.try_exists()? {
            let (Some(name), Some(parent)) = (existing.file_name(), existing.parent()) else {
                break;
            };

            missing.push(name.to_owned());
            existing = parent;
        }

        let mut resolved = fs::canonicalize(existing)?;
        resolved.extend(missing.iter().rev());

        Ok(resolved)
    }

    fn inside(path: &Path, root: &Path) -> bool {
        Self::folded(path).starts_with(&Self::folded(root))
    }

    fn folded(path: &Path) -> Vec<String> {
        path.components()
            .map(|component| component.as_os_str().to_string_lossy().to_lowercase())
            .collect()
    }

    fn outside(path: &Path, local: &Path) -> io::Error {
        io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!(
                "{} is outside {}: on Windows the core cannot yet give a folder an owner-only \
                 access list, so its data must live inside the user's local application data, \
                 which only this user and administrators can open",
                path.display(),
                local.display()
            ),
        )
    }
}

impl NativeDirectories for Windows {
    fn create_private_directory(path: &Path) -> io::Result<()> {
        let local = fs::canonicalize(Self::local_app_data()?)?;
        let target = Self::resolved(path)?;

        if !Self::inside(&target, &local) {
            return Err(Self::outside(path, &local));
        }

        fs::create_dir_all(path)
    }
}
