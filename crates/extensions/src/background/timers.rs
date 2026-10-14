use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use tokio::sync::Notify;
use tokio::time::{self, Instant};

use super::timer::Timer;

const MOST_TIMERS: usize = 16;
const SHORTEST: u32 = 1;
const LONGEST: u32 = 24 * 60 * 60;
const LONGEST_NAME: usize = 64;
const IDLE: Duration = Duration::from_secs(3600);

#[derive(Debug, Clone, Default)]
pub(crate) struct Timers {
    board: Arc<Mutex<BTreeMap<String, Timer>>>,
    changed: Arc<Notify>,
}

impl Timers {
    pub(crate) fn set(&self, name: &str, every_seconds: u32) -> Result<(), String> {
        check_name(name)?;

        if !(SHORTEST..=LONGEST).contains(&every_seconds) {
            return Err(format!(
                "a timer fires every {SHORTEST} to {LONGEST} seconds, not {every_seconds}"
            ));
        }

        let mut board = self.board();
        if !board.contains_key(name) && board.len() >= MOST_TIMERS {
            return Err(format!("an extension keeps at most {MOST_TIMERS} timers"));
        }

        let every = Duration::from_secs(u64::from(every_seconds));
        board.insert(name.to_owned(), Timer::starting_now(every));
        drop(board);

        self.changed.notify_one();
        Ok(())
    }

    pub(crate) fn clear(&self, name: &str) {
        self.board().remove(name);
        self.changed.notify_one();
    }

    pub(crate) async fn due(&self) -> Vec<String> {
        loop {
            let changed = self.changed.notified();
            let next = self.next_deadline();

            tokio::select! {
                () = time::sleep_until(next) => {}
                () = changed => continue,
            }

            let due = self.take_due(Instant::now());
            if !due.is_empty() {
                return due;
            }
        }
    }

    fn next_deadline(&self) -> Instant {
        self.board()
            .values()
            .map(|timer| timer.next)
            .min()
            .unwrap_or_else(|| Instant::now() + IDLE)
    }

    fn take_due(&self, now: Instant) -> Vec<String> {
        let mut board = self.board();

        board
            .iter_mut()
            .filter(|(_, timer)| timer.is_due(now))
            .map(|(name, timer)| {
                timer.advance(now);
                name.clone()
            })
            .collect()
    }

    fn board(&self) -> MutexGuard<'_, BTreeMap<String, Timer>> {
        self.board.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

fn check_name(name: &str) -> Result<(), String> {
    let plain = name
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || "-_.".contains(character));

    if name.is_empty() || name.len() > LONGEST_NAME || !plain {
        return Err(format!(
            "a timer name holds 1 to {LONGEST_NAME} letters, digits, '-', '_' and '.'"
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{MOST_TIMERS, Timers};

    #[test]
    fn refuses_odd_names_intervals_and_too_many_timers() {
        let timers = Timers::default();

        assert!(timers.set("", 5).is_err());
        assert!(timers.set("refresh", 0).is_err());
        assert!(timers.set("refresh", 90_000).is_err());

        for index in 0..MOST_TIMERS {
            timers.set(&format!("t{index}"), 60).unwrap();
        }
        assert!(timers.set("one-more", 60).is_err());
        assert!(timers.set("t0", 30).is_ok());
    }

    #[tokio::test(start_paused = true)]
    async fn a_timer_fires_on_its_interval_until_cleared() {
        let timers = Timers::default();
        timers.set("refresh", 2).unwrap();
        timers.set("slow", 60).unwrap();

        let started = tokio::time::Instant::now();
        assert_eq!(timers.due().await, ["refresh"]);
        assert_eq!(started.elapsed(), Duration::from_secs(2));

        assert_eq!(timers.due().await, ["refresh"]);
        assert_eq!(started.elapsed(), Duration::from_secs(4));

        timers.clear("refresh");
        assert_eq!(timers.due().await, ["slow"]);
        assert_eq!(started.elapsed(), Duration::from_secs(60));
    }
}
