use futures_util::{StreamExt, TryStreamExt};
use mongodb::{
    bson::{DateTime, Document, doc},
    error::Error,
    results::DeleteResult,
};
use serde::{Deserialize, Serialize};

use crate::{
    core::{Location, Position},
    storage::Storage,
};

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

fn location_filter(position: Position) -> Document {
    let Position { x, y, kingdom } = position;
    let kingdom_id = u32::from(u8::from(kingdom));
    doc! {
        "x": x,
        "y": y,
        "kingdom_id": kingdom_id,
    }
}

impl Storage {
    pub(crate) async fn get_locations(&self) -> Result<Vec<Location>, Error> {
        let cursor = self.collections.locations.find(doc! {}).await?;
        cursor
            .map(|result| match result {
                Ok(document) => document
                    .try_into()
                    .map_err(|()| Error::custom("Deserialize")),
                Err(error) => Err(error),
            })
            .try_collect::<Vec<_>>()
            .await
    }

    pub(crate) async fn update_location(&self, location: Location) -> Result<(), Error> {
        let filter = location_filter(location.position.clone());
        let document: LocationDocument = location.into();

        self.collections
            .locations
            .find_one_and_replace(filter, document)
            .upsert(true)
            .await
            .map(|_| ())
    }

    pub(crate) async fn remove_location(&self, position: Position) -> Result<DeleteResult, Error> {
        let filter = location_filter(position);
        self.collections.locations.delete_many(filter).await
    }
}
