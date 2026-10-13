use serde::Serialize;

use super::ResourceEntry;
use crate::projects::Problem;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ResourceListing {
    pub project: String,
    pub resources: Vec<ResourceEntry>,
    pub problems: Vec<Problem>,
}
