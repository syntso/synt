use std::sync::{Mutex, MutexGuard, PoisonError};

use portable_pty::{Child, ChildKiller, CommandBuilder, MasterPty, PtySize, native_pty_system};
use tokio::sync::{mpsc, oneshot};

use super::{PtyOutput, pipes};
use crate::{PlatformError, native};

const INPUT_BACKLOG: usize = 256;
const OUTPUT_BACKLOG: usize = 64;
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
            .openpty(size(columns, rows))
            .map_err(|error| pty_error(&error))?;

        let command = login_command(term);
        let program = command.get_shell();

        let child = pair
            .slave
            .spawn_command(command)
            .map_err(|error| pty_error(&error))?;
        drop(pair.slave);

        let mut killer = child.clone_killer();
        let process = child.process_id();

        match start_pipes(pair.master.as_ref(), child) {
            Ok((input, output)) => {
                let pty = Self {
                    program,
                    process,
                    master: Mutex::new(Some(pair.master)),
                    killer: Mutex::new(killer),
                    input,
                };

                Ok((pty, output))
            }
            Err(error) => {
                native::force_stop(process, killer.as_mut());
                Err(error)
            }
        }
    }

    pub fn program(&self) -> &str {
        &self.program
    }

    pub async fn write(&self, data: Vec<u8>) -> Result<(), PlatformError> {
        self.input
            .send(data)
            .await
            .map_err(|_| PlatformError::Pty("the shell no longer takes input".to_owned()))
    }

    pub fn resize(&self, columns: u16, rows: u16) -> Result<(), PlatformError> {
        let master = self.master();

        let Some(master) = master.as_ref() else {
            return Err(PlatformError::Pty("the terminal is closed".to_owned()));
        };

        master
            .resize(size(columns, rows))
            .map_err(|error| pty_error(&error))
    }

    pub fn hang_up(&self) -> Result<(), PlatformError> {
        self.killer
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .kill()
            .map_err(PlatformError::from)
    }

    pub fn stop(&self) {
        let mut killer = self.killer.lock().unwrap_or_else(PoisonError::into_inner);
        native::force_stop(self.process, killer.as_mut());

        self.master().take();
    }

    fn master(&self) -> MutexGuard<'_, Option<Master>> {
        self.master.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

fn start_pipes(
    master: &(dyn MasterPty + Send),
    child: Box<dyn Child + Send + Sync>,
) -> Result<(mpsc::Sender<Vec<u8>>, PtyOutput), PlatformError> {
    let reader = master
        .try_clone_reader()
        .map_err(|error| pty_error(&error))?;
    let writer = master.take_writer().map_err(|error| pty_error(&error))?;

    let (input, typed) = mpsc::channel(INPUT_BACKLOG);
    let (shown, data) = mpsc::channel(OUTPUT_BACKLOG);
    let (exited, exit) = oneshot::channel();

    pipes::spawn("synt-pty-wait", move || {
        let _ = exited.send(native::wait_for(child));
    })?;
    pipes::spawn("synt-pty-read", move || pipes::read_into(reader, &shown))?;
    pipes::spawn("synt-pty-write", move || pipes::write_from(writer, typed))?;

    Ok((input, PtyOutput::new(data, exit)))
}

fn login_command(term: &str) -> CommandBuilder {
    let mut command = CommandBuilder::new_default_prog();

    for name in INHERITED_TERMINAL {
        command.env_remove(name);
    }
    command.env("TERM", term);

    if let Some(shell) = native::login_shell() {
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

fn pty_error(error: &impl ToString) -> PlatformError {
    PlatformError::Pty(error.to_string())
}
