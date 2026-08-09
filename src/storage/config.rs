use std::time::Duration;

use serde::Deserialize;

use crate::{
    client::{Config as ClientConfig, CooldownConfig, SessionConfig},
    config::Config,
    core::{Config as CacheConfig, Kingdom},
    map::Config as MapConfig,
};

#[derive(Deserialize)]
struct CacheConfigDocument {
    max_retries: u8,
    retry_interval: f64,
    min_interval: f64,
}

impl From<CacheConfigDocument> for CacheConfig {
    fn from(value: CacheConfigDocument) -> Self {
        let CacheConfigDocument {
            max_retries,
            retry_interval,
            min_interval,
        } = value;
        CacheConfig {
            max_retries,
            retry_interval: Duration::from_secs_f64(retry_interval),
            min_interval: Duration::from_secs_f64(min_interval),
        }
    }
}

#[derive(Deserialize)]
struct MapConfigDocument {
    message_cooldown: f64,
    explore_cooldown: f64,
    explore_kingdoms: Vec<u8>,
}

impl TryFrom<MapConfigDocument> for MapConfig {
    type Error = ();

    fn try_from(value: MapConfigDocument) -> Result<Self, Self::Error> {
        let MapConfigDocument {
            message_cooldown,
            explore_cooldown,
            explore_kingdoms,
        } = value;
        Ok(MapConfig {
            message_cooldown: Duration::from_secs_f64(message_cooldown),
            explore_cooldown: Duration::from_secs_f64(explore_cooldown),
            explore_kingdoms: explore_kingdoms
                .into_iter()
                .filter_map(|kingdom_id| Kingdom::try_from(kingdom_id).ok())
                .collect(),
        })
    }
}

#[derive(Deserialize)]
struct SessionConfigDocument {
    url: String,
    server: String,
    username: String,
    password: String,
    silence_timeout: f64,
}

impl From<SessionConfigDocument> for SessionConfig {
    fn from(value: SessionConfigDocument) -> Self {
        let SessionConfigDocument {
            url,
            server,
            username,
            password,
            silence_timeout,
        } = value;
        SessionConfig {
            url,
            server,
            username,
            password,
            silence_timeout: Duration::from_secs_f64(silence_timeout),
        }
    }
}

#[derive(Deserialize)]
struct CooldownConfigDocument {
    base: f64,
    max: f64,
    multiplier: f64,
}

impl TryFrom<CooldownConfigDocument> for CooldownConfig {
    type Error = ();

    fn try_from(value: CooldownConfigDocument) -> Result<Self, Self::Error> {
        let CooldownConfigDocument {
            base,
            max,
            multiplier,
        } = value;
        CooldownConfig::new(
            Duration::from_secs_f64(base),
            Duration::from_secs_f64(max),
            multiplier,
        )
    }
}

#[derive(Deserialize)]
pub(super) struct ConfigDocument {
    cache: CacheConfigDocument,
    map: MapConfigDocument,
    session: SessionConfigDocument,
    cooldown: CooldownConfigDocument,
}

impl TryFrom<ConfigDocument> for Config {
    type Error = ();

    fn try_from(value: ConfigDocument) -> Result<Self, Self::Error> {
        let ConfigDocument {
            cache,
            map,
            session,
            cooldown,
        } = value;
        Ok(Config {
            cache: cache.into(),
            map: map.try_into()?,
            client: ClientConfig {
                session: session.into(),
                cooldown: cooldown.try_into()?,
            },
        })
    }
}
