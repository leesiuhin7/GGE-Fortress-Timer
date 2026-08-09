use mongodb::bson::{DateTime, Document, doc};
use serde::{Deserialize, Serialize};

use crate::core::{Location, Position};

#[derive(Serialize, Deserialize)]
pub(super) struct LocationDocument {
    x: u32,
    y: u32,
    kingdom_id: u8,
    attack_time: DateTime,
}

impl From<Location> for LocationDocument {
    fn from(value: Location) -> Self {
        let Location {
            position: Position { x, y, kingdom },
            attack_time,
        } = value;
        LocationDocument {
            x,
            y,
            kingdom_id: kingdom.into(),
            attack_time: attack_time.into(),
        }
    }
}

impl TryFrom<LocationDocument> for Location {
    type Error = ();

    fn try_from(value: LocationDocument) -> Result<Self, Self::Error> {
        let LocationDocument {
            x,
            y,
            kingdom_id,
            attack_time,
        } = value;
        Ok(Location {
            position: Position {
                x,
                y,
                kingdom: kingdom_id.try_into()?,
            },
            attack_time: attack_time.into(),
        })
    }
}

pub(super) fn location_filter(position: Position) -> Document {
    let Position { x, y, kingdom } = position;
    let kingdom_id = u32::from(u8::from(kingdom));
    doc! {
        "x": x,
        "y": y,
        "kingdom_id": kingdom_id,
    }
}
