use std::time::{Duration, SystemTime};

use serde_json::Value;

use crate::core::{Kingdom, Location, Position};

pub(crate) fn deserialize(payload: &str) -> Option<Vec<Location>> {
    let value: Value = serde_json::from_str(payload).ok()?;
    let kingdom_id: u8 = value.get("KID")?.as_u64()?.try_into().ok()?;
    let locations = value.get("AI")?.as_array()?;

    let kingdom: Kingdom = kingdom_id.try_into().ok()?;
    Some(
        locations
            .iter()
            .filter_map(|value| decode_location(value, kingdom.clone()))
            .collect(),
    )
}

fn decode_location(value: &Value, kingdom: Kingdom) -> Option<Location> {
    let location_type = value.get(0)?.as_u64()?;
    if location_type != 11 {
        return None;
    }
    let x: u32 = value.get(1)?.as_u64()?.try_into().ok()?;
    let y: u32 = value.get(2)?.as_u64()?.try_into().ok()?;
    let remaining_time: u64 = value.get(5)?.as_i64()?.max(0).try_into().ok()?;
    let attack_time = SystemTime::now().checked_add(Duration::from_secs(remaining_time))?;

    Some(Location {
        position: Position { x, y, kingdom },
        attack_time,
    })
}
