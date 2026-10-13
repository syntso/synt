use serde::Deserialize;

use super::RemotePath;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PathParams {
    pub path: RemotePath,
}
