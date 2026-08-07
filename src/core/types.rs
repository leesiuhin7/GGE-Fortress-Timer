use std::time::{Duration, SystemTime};

#[derive(PartialEq, Eq, Hash, Clone)]
pub(crate) enum Kingdom {
    GreatEmpire = 0,
    BurningSands = 1,
    EverwinterGlacier = 2,
    FirePeaks = 3,
    StormIslands = 4,
}

impl TryFrom<u8> for Kingdom {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::GreatEmpire),
            1 => Ok(Self::BurningSands),
            2 => Ok(Self::EverwinterGlacier),
            3 => Ok(Self::FirePeaks),
            4 => Ok(Self::StormIslands),
            _ => Err(()),
        }
    }
}

impl From<Kingdom> for u8 {
    fn from(value: Kingdom) -> Self {
        value as u8
    }
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

#[derive(PartialEq, Eq, Hash, Clone)]
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
