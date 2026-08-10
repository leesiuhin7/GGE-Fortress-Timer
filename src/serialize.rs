use std::time::{Duration, SystemTime};

use crate::core::{Location, Position};

fn flatten_location(location: Location) -> (u32, u32, u8, u64) {
    let Location {
        position: Position { x, y, kingdom },
        attack_time,
    } = location;
    (
        x,
        y,
        u8::from(kingdom),
        attack_time
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_secs(),
    )
}

pub(crate) fn serialize_locations(locations: Vec<Location>) -> String {
    serde_json::to_string(
        &locations
            .into_iter()
            .map(flatten_location)
            .collect::<Vec<_>>(),
    )
    .unwrap() // Using unwrap as this shouldn't fail
}
