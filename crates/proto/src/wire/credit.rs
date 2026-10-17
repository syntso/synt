use bytes::Bytes;

const LENGTH: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Credit(u64);

impl Credit {
    pub const OPENING: Self = Self(1024 * 1024);

    pub const fn new(bytes: u64) -> Self {
        Self(bytes)
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    pub fn encode(self) -> Bytes {
        Bytes::copy_from_slice(&self.0.to_be_bytes())
    }

    pub fn decode(data: &[u8]) -> Option<Self> {
        let bytes: [u8; LENGTH] = data.try_into().ok()?;

        Some(Self(u64::from_be_bytes(bytes)))
    }
}

#[cfg(test)]
mod tests {
    use super::Credit;

    #[test]
    fn round_trips_through_bytes() {
        let credit = Credit::new(5_000);

        assert_eq!(Credit::decode(&credit.encode()), Some(credit));
        assert_eq!(Credit::decode(&[1, 2, 3]), None);
    }
}
