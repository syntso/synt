#![allow(clippy::unwrap_used)]

#[path = ".."]
mod shared {
    pub mod common;
}

mod servers;
mod sqlite;

use shared::common;
