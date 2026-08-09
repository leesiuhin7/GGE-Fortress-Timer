mod config;
mod location;

use mongodb::{Client, Collection, IndexModel, bson::doc, error::Error};

use crate::storage::{config::ConfigDocument, location::LocationDocument};

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
}
