use super::{Console, ConsoleOutput};

pub(crate) struct OpenConsole {
    pub(crate) console: Box<dyn Console>,
    pub(crate) output: Box<dyn ConsoleOutput>,
}

impl OpenConsole {
    pub(crate) fn new(
        console: impl Console + 'static,
        output: impl ConsoleOutput + 'static,
    ) -> Self {
        Self {
            console: Box::new(console),
            output: Box::new(output),
        }
    }
}
