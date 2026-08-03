use std::time::SystemTime;

use crate::core::types::{Config, Location};

pub(super) struct CacheItem {
    location: Location,
    expiry_time: SystemTime,
    retries: Option<u8>,
    config: Config,
}

impl CacheItem {
    pub(super) fn new(location: Location, config: Config) -> Self {
        let attack_time = location.attack_time;
        CacheItem {
            location,
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

    pub(super) fn update(&mut self, location: Location) -> Result<(), ()> {
        self.retries = None; // Reset retry count
        if self.location == location {
            return Err(());
        }
        self.location = location;
        // Enforce minimum update interval
        self.expiry_time =
            if let Some(min_time) = SystemTime::now().checked_add(self.config.min_interval) {
                min_time.max(self.location.attack_time)
            } else {
                self.location.attack_time
            };
        Ok(())
    }

    pub(super) fn location(&self) -> &Location {
        &self.location
    }

    pub(super) fn expiry_time(&self) -> SystemTime {
        self.expiry_time
    }

    pub(super) fn retries(&self) -> Option<u8> {
        self.retries
    }
}
