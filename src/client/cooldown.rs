use std::time::Duration;

use backoff::{ExponentialBackoff, ExponentialBackoffBuilder};

pub(crate) struct Config {
    base: Duration,
    max: Duration,
    multiplier: f64,
}

impl Config {
    pub(crate) fn new(base: Duration, max: Duration, multiplier: f64) -> Result<Self, ()> {
        if multiplier > 0.0 {
            Ok(Config {
                base,
                max,
                multiplier,
            })
        } else {
            Err(())
        }
    }
}

impl From<Config> for ExponentialBackoff {
    fn from(value: Config) -> Self {
        let Config {
            base,
            max,
            multiplier,
        } = value;
        ExponentialBackoffBuilder::new()
            .with_initial_interval(base)
            .with_max_elapsed_time(None)
            .with_max_interval(max)
            .with_multiplier(multiplier)
            .build()
    }
}
