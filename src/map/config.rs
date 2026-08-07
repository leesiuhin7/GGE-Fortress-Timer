use std::time::Duration;

use crate::core::Kingdom;

pub(crate) struct Config {
    pub(crate) message_cooldown: Duration,
    pub(crate) explore_cooldown: Duration,
    pub(crate) explore_kingdoms: Vec<Kingdom>,
}
