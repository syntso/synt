use std::fs;
use std::io;
use std::path::Path;

use crate::{PlatformError, native};

pub(super) fn check_program(program: &Path) -> Result<(), PlatformError> {
    if !program.is_absolute() {
        return Err(refused(program, "it is not an absolute path"));
    }

    let folder = program
        .parent()
        .ok_or_else(|| refused(program, "it has no folder"))?;

    if !fs::metadata(program)?.is_file() {
        return Err(refused(program, "it is not a file"));
    }

    for place in [program, folder] {
        if native::writable_by_others(&fs::metadata(place)?) {
            let reason = format!("{} can be changed by other accounts", place.display());
            return Err(refused(program, &reason));
        }
    }

    Ok(())
}

fn refused(program: &Path, reason: &str) -> PlatformError {
    io::Error::new(
        io::ErrorKind::PermissionDenied,
        format!(
            "{} is not registered to start at login: {reason}",
            program.display()
        ),
    )
    .into()
}
