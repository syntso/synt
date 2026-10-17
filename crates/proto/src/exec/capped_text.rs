use tokio::io::{AsyncRead, AsyncReadExt};

const CHUNK: usize = 32 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CappedText {
    bytes: Vec<u8>,
    limit: usize,
    truncated: bool,
}

impl CappedText {
    pub fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
            truncated: false,
        }
    }

    pub async fn read_from(mut self, mut reader: impl AsyncRead + Unpin) -> Self {
        let mut buffer = vec![0; CHUNK];

        while let Ok(read) = reader.read(&mut buffer).await {
            if read == 0 {
                break;
            }

            self.push(&buffer[..read]);
        }

        self
    }

    pub fn push(&mut self, data: &[u8]) {
        let room = self.limit.saturating_sub(self.bytes.len());
        let kept = data.len().min(room);

        self.bytes.extend_from_slice(&data[..kept]);
        self.truncated |= kept < data.len();
    }

    pub fn truncated(&self) -> bool {
        self.truncated
    }

    pub fn into_text(self) -> String {
        String::from_utf8_lossy(&self.bytes).into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::CappedText;

    #[tokio::test]
    async fn output_past_the_limit_is_cut_and_marked() {
        let text = CappedText::new(4).read_from(&b"0123456789"[..]).await;

        assert!(text.truncated());
        assert_eq!(text.into_text(), "0123");
    }

    #[tokio::test]
    async fn output_within_the_limit_is_whole() {
        let text = CappedText::new(4).read_from(&b"ok\n"[..]).await;

        assert!(!text.truncated());
        assert_eq!(text.into_text(), "ok\n");
    }

    #[test]
    fn pushed_pieces_fill_up_to_the_limit() {
        let mut text = CappedText::new(5);

        text.push(b"abc");
        text.push(b"def");

        assert!(text.truncated());
        assert_eq!(text.into_text(), "abcde");
    }
}
