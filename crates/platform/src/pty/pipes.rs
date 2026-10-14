use std::io::{self, ErrorKind, Read, Write};
use std::thread;

use tokio::sync::mpsc;

use super::CursorQuestion;

const READ_CHUNK: usize = 16 * 1024;

pub(super) fn spawn(name: &str, work: impl FnOnce() + Send + 'static) -> io::Result<()> {
    thread::Builder::new().name(name.to_owned()).spawn(work)?;

    Ok(())
}

pub(super) fn read_into(
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

pub(super) fn write_from(mut writer: Box<dyn Write + Send>, mut typed: mpsc::Receiver<Vec<u8>>) {
    while let Some(data) = typed.blocking_recv() {
        let written = writer.write_all(&data).and_then(|()| writer.flush());

        if written.is_err() {
            return;
        }
    }
}
