use std::ffi::OsStr;
use std::io;
use std::time::Duration;

use interprocess::os::windows::named_pipe::pipe_mode::Bytes;
use interprocess::os::windows::named_pipe::tokio::PipeStream;
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeClient};
use tokio::task;
use tokio::time;
use tokio_util::either::Either;

use super::NativeStream;
use crate::{Peer, UserId};

const ERROR_PIPE_BUSY: i32 = 231;
const BUSY_RETRIES: u32 = 50;
const BUSY_PAUSE: Duration = Duration::from_millis(20);

pub(in crate::local) type Pipe = Either<PipeStream<Bytes, Bytes>, NamedPipeClient>;

impl NativeStream for Pipe {
    async fn open(address: &OsStr) -> io::Result<Self> {
        let mut attempts = 0;

        loop {
            match ClientOptions::new().open(address) {
                Ok(client) => return Ok(Either::Right(client)),
                Err(error)
                    if error.raw_os_error() == Some(ERROR_PIPE_BUSY) && attempts < BUSY_RETRIES =>
                {
                    attempts += 1;
                    time::sleep(BUSY_PAUSE).await;
                }
                Err(error) => return Err(error),
            }
        }
    }

    async fn peer(&self) -> io::Result<Peer> {
        let Either::Left(pipe) = self else {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "only the core's end of a pipe can see who is on the other end",
            ));
        };

        let process = pipe.client_process_id()?;
        let user = task::spawn_blocking(move || UserId::of_process(process))
            .await
            .map_err(io::Error::other)?;

        Ok(Peer::new(user.ok(), Some(process)))
    }
}
