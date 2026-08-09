use crate::{
    client::Config as ClientConfig, core::Config as CacheConfig, map::Config as MapConfig,
};

pub(crate) struct Config {
    pub(crate) cache: CacheConfig,
    pub(crate) map: MapConfig,
    pub(crate) client: ClientConfig,
}
