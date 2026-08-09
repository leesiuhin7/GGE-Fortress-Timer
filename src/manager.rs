use mongodb::error::Error as DbError;
use tokio::sync::mpsc;
use tracing::warn;

use crate::{
    client::{IdleClient, Response},
    config::Config,
    core::{LocationCache, Operation},
    map::{Adapter, Explorer, deserialize},
    storage::Storage,
};

pub(crate) enum Error {
    Config(DbError),
    Locations(DbError),
    ExplorerItems(DbError),
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
        } = self.storage.get_config().await.map_err(Error::Config)?;
        let locations = self
            .storage
            .get_locations()
            .await
            .map_err(Error::Locations)?;
        let explorer_items = self
            .storage
            .get_explorer_items()
            .await
            .map_err(Error::ExplorerItems)?;

        let (request_sender, request_receiver) = mpsc::channel(1024);
        let (location_update_sender, location_update_receiver) = mpsc::channel(1024);
        let (action_sender, mut action_receiver) = mpsc::channel(1024);
        let (explorer_update_sender, mut explorer_update_receiver) = mpsc::channel(1024);

        let mut cache = LocationCache::new(
            request_sender,
            location_update_receiver,
            action_sender,
            cache_config,
        );
        cache.load_locations(locations.into_iter());

        let idle_client = IdleClient::new(client_config);
        let mut client = idle_client.connect();

        let mut explorer = Explorer::new(
            explorer_update_sender,
            map_config.explore_kingdoms,
            map_config.explore_cooldown,
        );
        explorer.load_items(explorer_items.into_iter());

        let adapter = Adapter::new(
            explorer,
            request_receiver,
            client.sender(),
            map_config.message_cooldown,
        );

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
        let mut update_explorer_items = async move || -> Option<()> {
            loop {
                let item = explorer_update_receiver.recv().await?;
                let result = self.storage.update_explorer_items(item).await;
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
                        location_update_sender.send(location).await.ok()?;
                    }
                }
            }
        };

        tokio::select! {
            () = adapter.run() => (),
            () = cache.run() => (),
            _ = perform_operations() => (),
            _ = update_explorer_items() => (),
            _ = forward_responses() => (),
        };
        Ok(())
    }
}
