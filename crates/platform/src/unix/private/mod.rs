mod directory;
#[cfg(not(target_os = "macos"))]
mod file;

pub(crate) use directory::create_private_directory;
#[cfg(not(target_os = "macos"))]
pub(crate) use file::replace_private_file;
