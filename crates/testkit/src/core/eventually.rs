use std::time::Duration;

use tokio::time::{self, Instant};

const PATIENCE: Duration = Duration::from_secs(30);
const PAUSE: Duration = Duration::from_millis(50);

pub async fn eventually<T>(what: &str, mut probe: impl AsyncFnMut() -> Option<T>) -> T {
    let deadline = Instant::now() + PATIENCE;

    loop {
        if let Some(found) = probe().await {
            return found;
        }

        assert!(Instant::now() < deadline, "{what} did not happen in time");
        time::sleep(PAUSE).await;
    }
}
