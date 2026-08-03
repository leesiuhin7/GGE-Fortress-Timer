use std::{
    cmp::Reverse,
    collections::{
        BinaryHeap, HashMap,
        hash_map::Entry::{Occupied, Vacant},
    },
    time::SystemTime,
};
use tokio::sync::mpsc;

use crate::core::{
    item::CacheItem,
    types::{Config, Grid, Location, Position},
    utils,
};

struct PendingUpdate {
    time: SystemTime,
    position: Position,
}

impl PendingUpdate {
    fn new(time: SystemTime, position: Position) -> Self {
        PendingUpdate { time, position }
    }
}

impl PartialEq for PendingUpdate {
    fn eq(&self, other: &Self) -> bool {
        self.time.eq(&other.time)
    }
}

impl Eq for PendingUpdate {}

impl PartialOrd for PendingUpdate {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for PendingUpdate {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time.cmp(&other.time)
    }
}

enum Action {
    HandleUpdate(Option<Location>),
    RequestUpdate,
    None,
}

enum RequestResult {
    Success,
    Failed,
    NotFound,
    NotNeeded,
    ItemRemoved,
}

/// A cache for [`Location`] to reduce the amount of requests needed to keep them up to date.
///
/// The cache avoids unnecessary requests by waiting until [`Location::attack_time`] has passed
/// and polls at an interval of [`Config::min_interval`] until it updates. It also evicts
/// [`Location`] after not receiving updates on it for a specified amount of requests
/// ([`Config::max_retries`]) at an interval of [`Config::retry_interval`].
pub(crate) struct LocationCache {
    cache: HashMap<Position, CacheItem>,
    pending: BinaryHeap<Reverse<PendingUpdate>>,
    request_sender: mpsc::Sender<Grid>,
    update_receiver: mpsc::Receiver<Location>,
    output_sender: mpsc::Sender<Vec<Location>>,
    config: Config,
}

impl LocationCache {
    pub(crate) fn new(
        request_sender: mpsc::Sender<Grid>,
        update_receiver: mpsc::Receiver<Location>,
        output_sender: mpsc::Sender<Vec<Location>>,
        config: Config,
    ) -> Self {
        LocationCache {
            cache: HashMap::new(),
            pending: BinaryHeap::new(),
            request_sender,
            update_receiver,
            output_sender,
            config,
        }
    }

    /// Starts the [`LocationCache`]
    ///
    /// # Cancellation
    ///
    /// This future is cancellation safe. Cancellation drops all [`mpsc::Sender`] and
    /// [`mpsc::Receiver`] which terminates the channels.
    pub(crate) async fn run(mut self) {
        loop {
            // Using peek for cancellation safety
            let update = self.pending.peek().map(|Reverse(update)| update);

            let action = tokio::select! {
                location = self.update_receiver.recv() => Action::HandleUpdate(location),
                () = utils::sleep_until(update.unwrap().time), if update.is_some() => Action::RequestUpdate,
                else => Action::None,
            };

            let error = match action {
                Action::HandleUpdate(None) => true, // Update channel closed
                Action::HandleUpdate(Some(location)) => {
                    if self.handle_update(location).is_ok() {
                        self.broadcast_locations().await.is_err() // Output channel closed
                    } else {
                        false
                    }
                }
                Action::RequestUpdate => {
                    matches!(self.request_update().await, RequestResult::Failed)
                }
                Action::None => false,
            };
            if error {
                return;
            }
        }
    }

    fn handle_update(&mut self, location: Location) -> Result<(), ()> {
        let position = location.position.clone();

        let expiry_time = match self.cache.entry(position.clone()) {
            Occupied(mut entry) => {
                let item = entry.get_mut();
                item.update(location).map(|()| item.expiry_time())
            }
            Vacant(entry) => {
                let item = entry.insert(CacheItem::new(location, self.config.clone()));
                Ok(item.expiry_time())
            }
        }?;

        self.pending
            .push(Reverse(PendingUpdate::new(expiry_time, position)));
        Ok(())
    }

    /// Sends all stored [`Location`] via [`mpsc::Sender`]
    ///
    /// # Cancellation
    ///
    /// This operation is cancellation safe. Cancellation leads to the message to not be sent.
    async fn broadcast_locations(&self) -> Result<(), ()> {
        let locations = self
            .cache
            .values()
            .map(|item| item.location().clone())
            .collect();
        self.output_sender.send(locations).await.map_err(|_| ())
    }

    /// Requests an update on the next cache item if it has expired.
    ///
    /// # Cancellation
    ///
    /// This operation is not cancellation safe. Cancellation prevents update and skips the item.
    async fn request_update(&mut self) -> RequestResult {
        let Reverse(PendingUpdate { time, position }) = self.pending.pop().unwrap();

        let Some(item) = self.cache.get_mut(&position) else {
            return RequestResult::NotFound;
        };

        let _ = item.delay();
        if item.retries().unwrap_or(0) > self.config.max_retries {
            // Remove item after retries exceed limit
            self.cache.remove(&position).unwrap(); // Item is guaranteed to be in cache
            return RequestResult::ItemRemoved;
        }

        if time < item.expiry_time() {
            return RequestResult::NotNeeded;
        }
        let result = self.request_sender.send(position.into()).await;
        if result.is_err() {
            return RequestResult::Failed; // Request channel closed
        }
        RequestResult::Success
    }
}
