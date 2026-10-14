use super::Timestamp;

pub trait Clock: Send + Sync {
    fn now(&self) -> Timestamp;
}
