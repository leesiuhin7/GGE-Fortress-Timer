use futures_util::{StreamExt, TryStreamExt};
use mongodb::{
    Client, Collection, IndexModel,
    bson::{DateTime, Document, doc},
    error::Error,
    results::DeleteResult,
};
use serde::{Deserialize, Serialize};

use crate::core::{Location, Position};

#[derive(Serialize, Deserialize)]
struct LocationDocument {
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

pub(crate) struct Storage {
    #[allow(dead_code)] // Keep ownership of client to prevent dropping
    client: Client,
    collection: Collection<LocationDocument>,
}

impl Storage {
    pub(crate) async fn connect(uri: &str) -> Result<Storage, Error> {
        let client = Client::with_uri_str(uri).await?;
        let db = client.database("gge-fortress-timer");
        let collection: Collection<LocationDocument> = db.collection("locations");

        collection
            .create_index(
                IndexModel::builder()
                    .keys(doc! {"kingdom_id": 1, "x": 1, "y": 1})
                    .build(),
            )
            .await?;

        Ok(Storage { client, collection })
    }

    pub(crate) async fn get_locations(&self) -> Result<Vec<Location>, Error> {
        let cursor = self.collection.find(doc! {}).await?;
        cursor
            .map(|result| match result {
                Ok(document) => document.try_into().map_err(|()| Error::custom("Serialize")),
                Err(error) => Err(error),
            })
            .try_collect::<Vec<_>>()
            .await
    }

    pub(crate) async fn update_location(&self, location: Location) -> Result<(), Error> {
        let filter = location_filter(location.position.clone());
        let document: LocationDocument = location.into();

        self.collection
            .find_one_and_replace(filter, document)
            .upsert(true)
            .await
            .map(|_| ())
    }

    pub(crate) async fn remove_location(&self, position: Position) -> Result<DeleteResult, Error> {
        let filter = location_filter(position);
        self.collection.delete_many(filter).await
    }
}
