use bytes::Bytes;
use synt_platform::PtyOutput;
use synt_ssh::{ShellEnd, ShellOutput};

pub(crate) enum ConsoleOutput {
    Remote(ShellOutput),
    Local(PtyOutput),
}

impl ConsoleOutput {
    pub(crate) async fn next(&mut self) -> Option<Bytes> {
        match self {
            Self::Remote(output) => output.next().await,
            Self::Local(output) => output.next().await.map(Bytes::from),
        }
    }

    pub(crate) fn end(&self) -> ShellEnd {
        match self {
            Self::Remote(output) => output.end(),
            Self::Local(output) => {
                let end = output.end();

                ShellEnd {
                    exit: end.exit,
                    signal: end.signal,
                    lost: false,
                }
            }
        }
    }
}
