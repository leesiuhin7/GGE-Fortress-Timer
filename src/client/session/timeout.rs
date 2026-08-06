use std::time::Duration;

use tokio::time::Instant;

pub(super) struct Timeout {
    timeout_at: Instant,
    timeout: Duration,
}

impl Timeout {
    pub(super) fn new(timeout: Duration) -> Self {
        Timeout {
            timeout_at: Instant::now() + timeout,
            timeout,
        }
    }

    pub(super) fn reset(&mut self) {
        self.timeout_at = Instant::now() + self.timeout;
    }

    pub(super) async fn wait(&self) {
        tokio::time::sleep_until(self.timeout_at).await;
    }
}
