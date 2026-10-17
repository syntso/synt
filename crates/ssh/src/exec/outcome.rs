use synt_proto::ExecOutput;

use crate::Algorithms;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecOutcome {
    pub exit: Option<u32>,
    pub signal: Option<String>,
    pub output: ExecOutput,
    pub algorithms: Option<Algorithms>,
}
