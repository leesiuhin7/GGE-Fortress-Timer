use std::time::{Duration, SystemTime};

pub(super) async fn sleep_until(time: SystemTime) {
    let duration = time
        .duration_since(SystemTime::now())
        .unwrap_or(Duration::ZERO);
    tokio::time::sleep(duration).await;
}
