use std::env;
use std::fmt::{self, Display, Formatter};
use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use kube::config::Kubeconfig;

use super::ContextInfo;
use crate::KubeError;
use crate::error::first_line;

const VARIABLE: &str = "KUBECONFIG";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KubeconfigSource {
    Default,
    File(PathBuf),
}

impl KubeconfigSource {
    pub fn paths(&self) -> Vec<PathBuf> {
        match self {
            Self::File(path) => vec![path.clone()],
            Self::Default => Self::default_paths(),
        }
    }

    pub fn exists(&self) -> bool {
        self.paths().iter().any(|path| path.is_file())
    }

    pub fn load(&self) -> Result<Kubeconfig, KubeError> {
        let paths = self.paths();

        if paths.is_empty() {
            return Err(self.broken("there is no home folder to find ~/.kube/config in"));
        }

        let mut merged = Kubeconfig::default();

        for path in paths {
            let file = Kubeconfig::read_from(&path).map_err(|error| self.broken(error))?;
            merged = merged.merge(file).map_err(|error| self.broken(error))?;
        }

        Ok(merged)
    }

    pub fn modified(&self) -> Option<SystemTime> {
        self.paths()
            .iter()
            .filter_map(|path| {
                fs::metadata(path)
                    .and_then(|metadata| metadata.modified())
                    .ok()
            })
            .max()
    }

    pub fn contexts(&self) -> Result<Vec<ContextInfo>, KubeError> {
        let kubeconfig = self.load()?;
        let current = kubeconfig.current_context.as_deref();

        let contexts = kubeconfig
            .contexts
            .iter()
            .map(|named| ContextInfo::new(&kubeconfig, named, current))
            .collect();

        Ok(contexts)
    }

    pub(crate) fn broken(&self, reason: impl Display) -> KubeError {
        KubeError::Kubeconfig {
            place: self.to_string(),
            reason: first_line(&reason),
        }
    }

    fn default_paths() -> Vec<PathBuf> {
        if let Some(value) = env::var_os(VARIABLE) {
            let listed: Vec<PathBuf> = env::split_paths(&value)
                .filter(|path| !path.as_os_str().is_empty())
                .collect();

            if !listed.is_empty() {
                return listed;
            }
        }

        env::home_dir()
            .map(|home| vec![home.join(".kube").join("config")])
            .unwrap_or_default()
    }
}

impl Display for KubeconfigSource {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let paths: Vec<String> = self
            .paths()
            .iter()
            .map(|path| path.display().to_string())
            .collect();

        if paths.is_empty() {
            return formatter.write_str("~/.kube/config");
        }

        formatter.write_str(&paths.join(", "))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::KubeconfigSource;

    const BROKEN: &str = "apiVersion: v1\nusers:\n- name: admin\n  user:\n    token: SECRET-TOKEN-123\n    bad: [unclosed\n";

    #[test]
    fn a_broken_kubeconfig_is_reported_without_quoting_its_lines() {
        let folder = tempfile::tempdir().expect("a temporary folder");
        let path = folder.path().join("config");
        fs::write(&path, BROKEN).expect("written");

        let error = KubeconfigSource::File(path)
            .load()
            .expect_err("broken")
            .to_string();

        assert!(error.contains("line 6"), "{error}");
        assert!(!error.contains("SECRET-TOKEN-123"), "{error}");
    }

    #[test]
    fn a_missing_kubeconfig_says_where_it_was_looked_for() {
        let folder = tempfile::tempdir().expect("a temporary folder");
        let path = folder.path().join("nothing");

        let source = KubeconfigSource::File(path.clone());
        let error = source.load().expect_err("missing").to_string();

        assert!(!source.exists());
        assert!(error.contains(&path.display().to_string()), "{error}");
    }
}
