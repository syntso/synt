use std::sync::Arc;

use synt_actions::{ActionError, Context};
use synt_ssh::TerminalSize;

use crate::link::Link;
use crate::terminals::{Session, SessionId, Terminals};

const LARGEST_SIDE: u16 = 1000;
const DEFAULT_TERM: &str = "xterm-256color";
const LONGEST_TERM: usize = 64;

pub(super) fn connection(context: &Context) -> Result<Arc<Link>, ActionError> {
    context
        .connection::<Link>()
        .ok_or_else(|| ActionError::failed("this call has no connection to carry a terminal"))
}

pub(super) fn find(terminals: &Terminals, id: SessionId) -> Result<Arc<Session>, ActionError> {
    terminals
        .get(id)
        .ok_or_else(|| ActionError::failed(format!("there is no open session {id}")))
}

pub(super) fn terminal_size(columns: u16, rows: u16) -> Result<TerminalSize, ActionError> {
    let fits = |side: u16| (1..=LARGEST_SIDE).contains(&side);

    if !fits(columns) || !fits(rows) {
        return Err(ActionError::InvalidParams(format!(
            "cols and rows must be between 1 and {LARGEST_SIDE}, got {columns}x{rows}"
        )));
    }

    Ok(TerminalSize { columns, rows })
}

pub(super) fn term(requested: Option<&str>) -> Result<&str, ActionError> {
    let Some(term) = requested else {
        return Ok(DEFAULT_TERM);
    };

    let allowed = |character: char| character.is_ascii_alphanumeric() || "-_.+".contains(character);

    if term.is_empty() || term.len() > LONGEST_TERM || !term.chars().all(allowed) {
        return Err(ActionError::InvalidParams(format!(
            "term must be 1 to {LONGEST_TERM} letters, digits or -_.+"
        )));
    }

    Ok(term)
}
