use domain::{Instant, ports::Clock};
use std::{
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs())
    }

    fn sleep(&self, seconds: u64) {
        thread::sleep(Duration::from_secs(seconds));
    }
}
