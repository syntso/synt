use std::fs;
use std::io;
use std::path::Path;

const DEEPEST: usize = 8;
const MOST_LISTED: usize = 1_000;

pub(super) struct Walk {
    found: Vec<String>,
}

impl Walk {
    pub(super) fn files(root: &Path) -> io::Result<Vec<String>> {
        let mut walk = Self { found: Vec::new() };

        walk.folder(root, "", 0)?;

        walk.found.sort();
        Ok(walk.found)
    }

    fn folder(&mut self, folder: &Path, prefix: &str, depth: usize) -> io::Result<()> {
        if depth >= DEEPEST || self.found.len() >= MOST_LISTED {
            return Ok(());
        }

        let entries = match fs::read_dir(folder) {
            Ok(entries) => entries,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error),
        };

        for entry in entries {
            self.entry(&entry?, prefix, depth)?;
        }

        Ok(())
    }

    fn entry(&mut self, entry: &fs::DirEntry, prefix: &str, depth: usize) -> io::Result<()> {
        let name = entry.file_name().to_string_lossy().into_owned();
        let kind = entry.file_type()?;

        if name.starts_with('.') || kind.is_symlink() {
            return Ok(());
        }

        let relative = match prefix {
            "" => name,
            _ => format!("{prefix}/{name}"),
        };

        if kind.is_dir() {
            return self.folder(&entry.path(), &relative, depth + 1);
        }

        if kind.is_file() {
            self.found.push(relative);
        }

        Ok(())
    }
}
