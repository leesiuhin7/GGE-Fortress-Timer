mod config;
mod location;

use futures_util::{StreamExt, TryStreamExt};
use mongodb::{Client, Collection, IndexModel, bson::doc, error::Error, results::DeleteResult};

use crate::{
    config::Config,
    core::{Location, Position},
    storage::{
        config::ConfigDocument,
        location::{LocationDocument, location_filter},
    },
};

struct Collections {
    locations: Collection<LocationDocument>,
    config: Collection<ConfigDocument>,
}

pub(crate) struct Storage {
    #[allow(dead_code)] // Keep ownership of client to prevent dropping
    client: Client,
    collections: Collections,
}

impl Storage {
    pub(crate) async fn connect(uri: &str) -> Result<Storage, Error> {
        let client = Client::with_uri_str(uri).await?;
        let db = client.database("gge-fortress-timer");

        let locations: Collection<LocationDocument> = db.collection("locations");
        locations
            .create_index(
                IndexModel::builder()
                    .keys(doc! {"kingdom_id": 1, "x": 1, "y": 1})
                    .build(),
            )
            .await?;

        let config = db.collection("config");

        Ok(Storage {
            client,
            collections: Collections { locations, config },
        })
    }

    pub(crate) async fn get_locations(&self) -> Result<Vec<Location>, Error> {
        let cursor = self.collections.locations.find(doc! {}).await?;
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

    pub(crate) async fn get_config(&self) -> Result<Config, Error> {
        match self.collections.config.find_one(doc! {}).await {
            Ok(Some(document)) => {
                Config::try_from(document).map_err(|()| Error::custom("Serialize"))
            }
            Ok(None) => Err(Error::custom("Config not found")),
            Err(error) => Err(error),
        }
    }
}
