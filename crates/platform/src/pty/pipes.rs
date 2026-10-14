use std::io::{self, ErrorKind, Read, Write};
use std::thread;

use portable_pty::{Child, MasterPty};
use tokio::sync::{mpsc, oneshot};

use super::{CursorQuestion, NativePty, Os, PtyOutput};
use crate::PlatformError;

const READ_CHUNK: usize = 16 * 1024;
const INPUT_BACKLOG: usize = 256;
const OUTPUT_BACKLOG: usize = 64;

pub(super) struct Pipes {
    reader: Box<dyn Read + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
}

impl Pipes {
    pub(super) fn start(
        master: &(dyn MasterPty + Send),
        child: Box<dyn Child + Send + Sync>,
    ) -> Result<(mpsc::Sender<Vec<u8>>, PtyOutput), PlatformError> {
        let pipes = Self {
            reader: master.try_clone_reader().map_err(PlatformError::terminal)?,
            writer: master.take_writer().map_err(PlatformError::terminal)?,
            child,
        };

        Ok(pipes.run()?)
    }

    fn run(self) -> io::Result<(mpsc::Sender<Vec<u8>>, PtyOutput)> {
        let (input, typed) = mpsc::channel(INPUT_BACKLOG);
        let (shown, data) = mpsc::channel(OUTPUT_BACKLOG);
        let (exited, exit) = oneshot::channel();
        let question = Self::cursor_question(&input);
        let Self {
            reader,
            writer,
            child,
        } = self;

        Self::spawn("synt-pty-wait", move || {
            let _ = exited.send(Os::wait_for(child));
        })?;
        Self::spawn("synt-pty-read", move || {
            Self::read_into(reader, &shown, question);
        })?;
        Self::spawn("synt-pty-write", move || Self::write_from(writer, typed))?;

        Ok((input, PtyOutput::new(data, exit)))
    }

    fn cursor_question(input: &mpsc::Sender<Vec<u8>>) -> CursorQuestion {
        if Os::ASKS_FOR_CURSOR {
            return CursorQuestion::expected(input.clone());
        }

        CursorQuestion::not_expected()
    }

    fn spawn(name: &str, work: impl FnOnce() + Send + 'static) -> io::Result<()> {
        thread::Builder::new().name(name.to_owned()).spawn(work)?;

        Ok(())
    }

    fn read_into(
        mut reader: Box<dyn Read + Send>,
        output: &mpsc::Sender<Vec<u8>>,
        mut question: CursorQuestion,
    ) {
        let mut buffer = vec![0_u8; READ_CHUNK];

        loop {
            let read = match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => read,
                Err(error) if error.kind() == ErrorKind::Interrupted => continue,
                Err(_) => break,
            };

            let shown = question.screen(&buffer[..read]);

            if !shown.is_empty() && output.blocking_send(shown).is_err() {
                return;
            }
        }

        let rest = question.rest();

        if !rest.is_empty() {
            let _ = output.blocking_send(rest);
        }
    }

    fn write_from(mut writer: Box<dyn Write + Send>, mut typed: mpsc::Receiver<Vec<u8>>) {
        while let Some(data) = typed.blocking_recv() {
            let written = writer.write_all(&data).and_then(|()| writer.flush());

            if written.is_err() {
                return;
            }
        }
    }
}
