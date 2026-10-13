use bytes::Bytes;

const LENGTH: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Credit(pub u64);

impl Credit {
    pub const OPENING: Self = Self(1024 * 1024);

    pub fn encode(self) -> Bytes {
        Bytes::copy_from_slice(&self.0.to_be_bytes())
    }

    pub fn decode(data: &[u8]) -> Option<Self> {
        let bytes: [u8; LENGTH] = data.try_into().ok()?;

        Some(Self(u64::from_be_bytes(bytes)))
    }
}
