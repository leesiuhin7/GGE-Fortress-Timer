use std::time::{Duration, SystemTime};

#[derive(PartialEq, Eq, Hash, Clone)]
pub(crate) enum Kingdom {
    GreatEmpire = 0,
    BurningSands = 1,
    EverwinterGlacier = 2,
    FirePeaks = 3,
    StormIslands = 4,
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub(crate) struct Position {
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) kingdom: Kingdom,
}

#[derive(PartialEq, Eq, Clone)]
pub(crate) struct Location {
    pub(crate) position: Position,
    pub(crate) attack_time: SystemTime,
}

pub(crate) struct Grid {
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) kingdom: Kingdom,
}

impl From<Position> for Grid {
    fn from(value: Position) -> Self {
        let Position { x, y, kingdom } = value;
        let grid_x = x / 13;
        let grid_y = y / 13;
        Grid {
            x: grid_x,
            y: grid_y,
            kingdom,
        }
    }
}

#[derive(Clone)]
pub(crate) struct Config {
    pub(crate) max_retries: u8,
    pub(crate) retry_interval: Duration,
    pub(crate) min_interval: Duration,
}
