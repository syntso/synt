use serde::Serialize;

use crate::projects::Problem;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProjectInfo {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub path: String,
    pub read_only: bool,
    pub resources: usize,
    pub problems: Vec<Problem>,
}
