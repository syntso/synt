use synt_actions::ActionError;
use synt_ssh::TerminalSize;

const LARGEST_SIDE: u16 = 1000;

pub(super) trait Dimensions {
    fn columns(&self) -> u16;

    fn rows(&self) -> u16;

    fn size(&self) -> Result<TerminalSize, ActionError> {
        let (columns, rows) = (self.columns(), self.rows());
        let fits = |side: u16| (1..=LARGEST_SIDE).contains(&side);

        if !fits(columns) || !fits(rows) {
            return Err(ActionError::InvalidParams(format!(
                "cols and rows must be between 1 and {LARGEST_SIDE}, got {columns}x{rows}"
            )));
        }

        Ok(TerminalSize { columns, rows })
    }
}
