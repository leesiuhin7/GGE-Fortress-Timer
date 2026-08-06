use crate::client::{cooldown, session};

pub(crate) struct Config {
    pub(crate) session: session::Config,
    pub(crate) cooldown: cooldown::Config,
}
