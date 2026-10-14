use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Rows {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
    pub offset: u64,
    pub more: bool,
    pub next_offset: Option<u64>,
    pub affected: Option<u64>,
    pub bytes: usize,
    pub values_cut: bool,
}

impl Rows {
    pub fn count(&self) -> usize {
        self.rows.len()
    }
}
