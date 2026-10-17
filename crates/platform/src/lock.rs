use std::sync::{Mutex, MutexGuard, PoisonError};

pub trait Lock<T> {
    fn locked(&self) -> MutexGuard<'_, T>;
}

impl<T> Lock<T> for Mutex<T> {
    fn locked(&self) -> MutexGuard<'_, T> {
        self.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};
    use std::thread;

    use super::Lock;

    #[test]
    fn a_poisoned_mutex_still_hands_out_its_value() {
        let shared = Arc::new(Mutex::new(7));
        let poisoner = Arc::clone(&shared);

        let _ = thread::spawn(move || {
            let _held = poisoner.locked();
            panic!("poison the mutex");
        })
        .join();

        assert!(shared.is_poisoned());
        assert_eq!(*shared.locked(), 7);
    }
}
