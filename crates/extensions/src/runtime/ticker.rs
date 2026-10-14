use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use wasmtime::Engine;

const TICK: Duration = Duration::from_millis(20);

pub(super) struct Ticker {
    running: Arc<AtomicBool>,
}

impl Ticker {
    pub(super) fn start(engine: Engine) -> std::io::Result<Self> {
        let running = Arc::new(AtomicBool::new(true));
        let still_running = Arc::clone(&running);

        thread::Builder::new()
            .name("synt-extension-ticker".to_owned())
            .spawn(move || tick(&engine, &still_running))?;

        Ok(Self { running })
    }
}

impl Drop for Ticker {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

fn tick(engine: &Engine, running: &AtomicBool) {
    while running.load(Ordering::Relaxed) {
        thread::sleep(TICK);
        engine.increment_epoch();
    }
}
