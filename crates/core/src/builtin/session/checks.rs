use std::sync::Arc;

use synt_actions::{ActionError, Context};
use synt_ssh::TerminalSize;

use crate::link::Link;
use crate::terminal::{SessionId, Terminal, Terminals};

const LARGEST_SIDE: u16 = 1000;

pub(super) fn connection(context: &Context) -> Result<Arc<Link>, ActionError> {
    context
        .connection::<Link>()
        .ok_or_else(|| ActionError::failed("this call has no connection to carry a terminal"))
}

pub(super) fn find(terminals: &Terminals, id: SessionId) -> Result<Arc<Terminal>, ActionError> {
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
