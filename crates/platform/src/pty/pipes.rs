use std::io::{self, ErrorKind, Read, Write};
use std::thread;

use tokio::sync::mpsc;

const READ_CHUNK: usize = 16 * 1024;

pub(super) fn spawn(name: &str, work: impl FnOnce() + Send + 'static) -> io::Result<()> {
    thread::Builder::new().name(name.to_owned()).spawn(work)?;

    Ok(())
}

pub(super) fn read_into(mut reader: Box<dyn Read + Send>, output: &mpsc::Sender<Vec<u8>>) {
    let mut buffer = vec![0_u8; READ_CHUNK];

    loop {
        let read = match reader.read(&mut buffer) {
            Ok(0) => return,
            Ok(read) => read,
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(_) => return,
        };

        if output.blocking_send(buffer[..read].to_vec()).is_err() {
            return;
        }
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
