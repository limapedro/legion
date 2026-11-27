//! Sleeping process for idle CPU / RSS sampling.
//! Usage: idle <seconds>
use std::thread::sleep;
use std::time::Duration;

fn main() {
    let secs: u64 = std::env::args()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(10);
    sleep(Duration::from_secs(secs));
}
