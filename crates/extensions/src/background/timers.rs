mod cadence;
mod name;

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Duration;

use tokio::sync::Notify;
use tokio::time::{self, Instant};

pub(crate) use name::TimerName;

use cadence::Cadence;

use crate::ExtensionError;

const MOST_TIMERS: usize = 16;
const SHORTEST: u32 = 1;
const LONGEST: u32 = 24 * 60 * 60;
const IDLE: Duration = Duration::from_secs(3600);

#[derive(Debug, Clone, Default)]
pub(crate) struct Timers {
    board: Arc<Mutex<BTreeMap<TimerName, Cadence>>>,
    changed: Arc<Notify>,
}

impl Timers {
    pub(crate) fn set(&self, name: &str, every_seconds: u32) -> Result<(), ExtensionError> {
        let name = TimerName::try_from(name)?;

        if !(SHORTEST..=LONGEST).contains(&every_seconds) {
            return Err(ExtensionError::TimerInterval {
                shortest: SHORTEST,
                longest: LONGEST,
                asked: every_seconds,
            });
        }

        let mut board = self.board();
        if !board.contains_key(&name) && board.len() >= MOST_TIMERS {
            return Err(ExtensionError::TooManyTimers(MOST_TIMERS));
        }

        let every = Duration::from_secs(u64::from(every_seconds));
        board.insert(name, Cadence::starting_now(every));
        drop(board);

        self.changed.notify_one();
        Ok(())
    }

    pub(crate) fn clear(&self, name: &str) {
        self.board().remove(name);
        self.changed.notify_one();
    }

    pub(crate) async fn due(&self) -> Vec<TimerName> {
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
            .map(Cadence::next)
            .min()
            .unwrap_or_else(|| Instant::now() + IDLE)
    }

    fn take_due(&self, now: Instant) -> Vec<TimerName> {
        let mut board = self.board();

        board
            .iter_mut()
            .filter(|(_, cadence)| cadence.is_due(now))
            .map(|(name, cadence)| {
                cadence.advance(now);
                name.clone()
            })
            .collect()
    }

    fn board(&self) -> MutexGuard<'_, BTreeMap<TimerName, Cadence>> {
        self.board.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{MOST_TIMERS, TimerName, Timers};

    fn names(due: &[TimerName]) -> Vec<String> {
        due.iter().map(ToString::to_string).collect()
    }

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
        assert_eq!(names(&timers.due().await), ["refresh"]);
        assert_eq!(started.elapsed(), Duration::from_secs(2));

        assert_eq!(names(&timers.due().await), ["refresh"]);
        assert_eq!(started.elapsed(), Duration::from_secs(4));

        timers.clear("refresh");
        assert_eq!(names(&timers.due().await), ["slow"]);
        assert_eq!(started.elapsed(), Duration::from_secs(60));
    }
}
