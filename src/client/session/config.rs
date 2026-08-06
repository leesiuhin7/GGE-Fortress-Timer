use std::time::Duration;

#[derive(Clone)]
pub(crate) struct Config {
    pub(crate) url: String,
    pub(crate) server: String,
    pub(crate) username: String,
    pub(crate) password: String,
    pub(crate) silence_timeout: Duration,
}

pub(super) const ORIGIN: &str = "https://empire-html5.goodgamestudios.com";
