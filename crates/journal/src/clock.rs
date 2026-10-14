mod system;

use crate::timestamp::Timestamp;

pub use system::SystemClock;

pub trait Clock: Send + Sync {
    fn now(&self) -> Timestamp;
}
