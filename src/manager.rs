use mongodb::error::Error as DbError;
use tokio::sync::mpsc;
use tracing::warn;

use crate::{
    client::{IdleClient, Response},
    config::Config,
    core::{LocationCache, Operation},
    map::{Adapter, deserialize},
    storage::Storage,
};

pub(crate) enum Error {
    LoadConfig(DbError),
    LoadLocations(DbError),
}

pub(crate) struct Manager<'a> {
    storage: &'a Storage,
}

impl<'a> Manager<'a> {
    pub(crate) fn new(storage: &'a Storage) -> Self {
        Manager { storage }
    }

    pub(crate) async fn run(self) -> Result<(), Error> {
        let Config {
            cache: cache_config,
            map: map_config,
            client: client_config,
        } = self.storage.get_config().await.map_err(Error::LoadConfig)?;
        let locations = self
            .storage
            .get_locations()
            .await
            .map_err(Error::LoadLocations)?;

        let (request_sender, request_receiver) = mpsc::channel(1024);
        let (update_sender, update_receiver) = mpsc::channel(1024);
        let (action_sender, mut action_receiver) = mpsc::channel(1024);

        let mut cache =
            LocationCache::new(request_sender, update_receiver, action_sender, cache_config);
        cache.load_locations(locations.into_iter());

        let idle_client = IdleClient::new(client_config);
        let mut client = idle_client.connect();

        let adapter = Adapter::new(request_receiver, client.sender(), map_config);

        let mut perform_operations = async move || -> Option<()> {
            loop {
                let operation = action_receiver.recv().await?;
                let result = match operation {
                    Operation::Update(location) => self.storage.update_location(location).await,
                    Operation::Remove(position) => {
                        self.storage.remove_location(position).await.map(|_| ())
                    }
                };
                if result.is_err() {
                    warn!("Database operation failed.");
                }
            }
        };
        let mut forward_responses = async move || -> Option<()> {
            loop {
                let responses = client.recv().await.ok()?;
                for Response { command, payload } in responses {
                    if command != "gaa" {
                        continue;
                    }
                    let locations = deserialize(&payload)?;
                    for location in locations {
                        update_sender.send(location).await.ok()?;
                    }
                }
            }
        };

        tokio::select! {
            () = adapter.run() => (),
            () = cache.run() => (),
            _ = perform_operations() => (),
            _ = forward_responses() => (),
        };
        Ok(())
    }
}
