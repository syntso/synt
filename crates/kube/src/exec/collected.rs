use tokio::io::{AsyncRead, AsyncReadExt};

const CHUNK: usize = 32 * 1024;

#[derive(Debug, Default)]
pub(crate) struct Collected {
    pub(crate) bytes: Vec<u8>,
    pub(crate) truncated: bool,
}

impl Collected {
    pub(crate) async fn read(reader: Option<impl AsyncRead + Unpin>, limit: usize) -> Self {
        let Some(mut reader) = reader else {
            return Self::default();
        };

        let mut collected = Self::default();
        let mut buffer = vec![0; CHUNK];

        while let Ok(read) = reader.read(&mut buffer).await {
            if read == 0 {
                break;
            }

            collected.keep(&buffer[..read], limit);
        }

        collected
    }

    pub(crate) fn text(&self) -> String {
        String::from_utf8_lossy(&self.bytes).into_owned()
    }

    fn keep(&mut self, data: &[u8], limit: usize) {
        let room = limit.saturating_sub(self.bytes.len());

        if data.len() > room {
            self.truncated = true;
        }

        self.bytes.extend_from_slice(&data[..data.len().min(room)]);
    }
}

#[cfg(test)]
mod tests {
    use super::Collected;

    #[tokio::test]
    async fn output_past_the_limit_is_cut_and_marked() {
        let collected = Collected::read(Some(&b"0123456789"[..]), 4).await;

        assert_eq!(collected.text(), "0123");
        assert!(collected.truncated);
    }

    #[tokio::test]
    async fn output_within_the_limit_is_whole() {
        let collected = Collected::read(Some(&b"ok\n"[..]), 4).await;

        assert_eq!(collected.text(), "ok\n");
        assert!(!collected.truncated);
    }
}
