use std::time::SystemTime;

use crate::core::types::Config;

pub(super) struct CacheItem {
    attack_time: SystemTime,
    expiry_time: SystemTime,
    retries: Option<u8>,
    config: Config,
}

impl CacheItem {
    pub(super) fn new(attack_time: SystemTime, config: Config) -> Self {
        CacheItem {
            attack_time,
            expiry_time: attack_time,
            retries: None,
            config,
        }
    }

    pub(super) fn delay(&mut self) -> Result<(), ()> {
        match self.expiry_time.checked_add(self.config.retry_interval) {
            Some(expriy_time) => {
                self.expiry_time = expriy_time;
                // Increment retry count
                self.retries = Some(match self.retries {
                    Some(count) => count + 1,
                    None => 0,
                });
                Ok(())
            }
            None => Err(()),
        }
    }

    pub(super) fn update(&mut self, attack_time: SystemTime) -> Result<(), ()> {
        self.retries = None; // Reset retry count
        if self.attack_time == attack_time {
            return Err(());
        }
        self.attack_time = attack_time;
        // Enforce minimum update interval
        self.expiry_time =
            if let Some(min_time) = SystemTime::now().checked_add(self.config.min_interval) {
                min_time.max(self.attack_time)
            } else {
                self.attack_time
            };
        Ok(())
    }

    pub(super) fn attack_time(&self) -> SystemTime {
        self.attack_time
    }

    pub(super) fn expiry_time(&self) -> SystemTime {
        self.expiry_time
    }

    pub(super) fn retries(&self) -> Option<u8> {
        self.retries
    }
}
