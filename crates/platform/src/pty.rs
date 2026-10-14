mod cursor_question;
mod end;
mod native;
mod output;
mod pipes;
#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use std::sync::{Mutex, MutexGuard};

use portable_pty::{ChildKiller, CommandBuilder, MasterPty, PtySize, native_pty_system};
use tokio::sync::mpsc;

pub(crate) use cursor_question::CursorQuestion;
pub use end::PtyEnd;
use native::NativePty;
pub use output::PtyOutput;
use pipes::Pipes;
#[cfg(unix)]
use unix::Unix as Os;
#[cfg(windows)]
use windows::Windows as Os;

use crate::{Lock, PlatformError};

const INHERITED_TERMINAL: [&str; 4] = [
    "TERM_PROGRAM",
    "TERM_PROGRAM_VERSION",
    "TERM_SESSION_ID",
    "COLORTERM",
];

type Master = Box<dyn MasterPty + Send>;
type Killer = Box<dyn ChildKiller + Send + Sync>;

pub struct Pty {
    program: String,
    process: Option<u32>,
    master: Mutex<Option<Master>>,
    killer: Mutex<Killer>,
    input: mpsc::Sender<Vec<u8>>,
}

impl Pty {
    pub fn open(term: &str, columns: u16, rows: u16) -> Result<(Self, PtyOutput), PlatformError> {
        let pair = native_pty_system()
            .openpty(Self::size(columns, rows))
            .map_err(PlatformError::terminal)?;

        let command = Self::login_command(term);
        let program = command.get_shell();

        let child = pair
            .slave
            .spawn_command(command)
            .map_err(PlatformError::terminal)?;
        drop(pair.slave);

        let mut killer = child.clone_killer();
        let process = child.process_id();

        let (input, output) = match Pipes::start(pair.master.as_ref(), child) {
            Ok(started) => started,
            Err(error) => {
                Os::force_stop(process, killer.as_mut());
                return Err(error);
            }
        };

        let pty = Self {
            program,
            process,
            master: Mutex::new(Some(pair.master)),
            killer: Mutex::new(killer),
            input,
        };

        Ok((pty, output))
    }

    pub fn program(&self) -> &str {
        &self.program
    }

    pub async fn write(&self, data: Vec<u8>) -> Result<(), PlatformError> {
        self.input
            .send(data)
            .await
            .map_err(|_| PlatformError::ShellGone)
    }

    pub fn resize(&self, columns: u16, rows: u16) -> Result<(), PlatformError> {
        let master = self.master();

        let Some(master) = master.as_ref() else {
            return Err(PlatformError::TerminalClosed);
        };

        master
            .resize(Self::size(columns, rows))
            .map_err(PlatformError::terminal)
    }

    pub fn hang_up(&self) -> Result<(), PlatformError> {
        self.killer.locked().kill()?;

        Ok(())
    }

    pub fn stop(&self) {
        Os::force_stop(self.process, self.killer.locked().as_mut());

        self.master().take();
    }

    fn master(&self) -> MutexGuard<'_, Option<Master>> {
        self.master.locked()
    }

    fn login_command(term: &str) -> CommandBuilder {
        let mut command = CommandBuilder::new_default_prog();

        for name in INHERITED_TERMINAL {
            command.env_remove(name);
        }
        command.env("TERM", term);

        if let Some(shell) = Os::login_shell() {
            command.env("SHELL", shell);
        }

        command
    }

    fn size(columns: u16, rows: u16) -> PtySize {
        PtySize {
            rows,
            cols: columns,
            pixel_width: 0,
            pixel_height: 0,
        }
    }
}
