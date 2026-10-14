use std::path::PathBuf;

const HOME_PREFIX: &str = "~/";

pub(crate) fn local_path(text: &str) -> Result<PathBuf, String> {
    if let Some(rest) = text.strip_prefix(HOME_PREFIX) {
        let home = synt_platform::home_directory()
            .map_err(|error| format!("there is no home folder to expand ~ in: {error}"))?;

        return Ok(home.join(rest));
    }

    let path = PathBuf::from(text);

    if !path.is_absolute() {
        return Err(format!(
            "{text:?} must be an absolute path or a path starting with ~/"
        ));
    }

    Ok(path)
}
