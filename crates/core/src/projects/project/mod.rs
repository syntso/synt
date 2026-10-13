mod file;
mod info;
mod project;

pub use info::ProjectInfo;
pub(crate) use project::{Project, RESOURCES, is_project_id};
