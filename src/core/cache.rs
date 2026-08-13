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

enum UpdateResult {
    Updated(Location),
    NotUpdated,
}

enum RequestResult {
    Success,
    Failed,
    NotFound,
    NotNeeded,
    ItemRemoved(Position),
}

pub(crate) enum Operation {
    Update(Location),
    Remove(Position),
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
    action_sender: mpsc::Sender<Operation>,
    config: Config,
}

impl LocationCache {
    pub(crate) fn new(
        request_sender: mpsc::Sender<Grid>,
        update_receiver: mpsc::Receiver<Location>,
        action_sender: mpsc::Sender<Operation>,
        config: Config,
    ) -> Self {
        LocationCache {
            cache: HashMap::new(),
            pending: BinaryHeap::new(),
            request_sender,
            update_receiver,
            action_sender,
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

            let sleep = async move || {
                utils::sleep_until(update?.time).await;
                Some(())
            };
            let action = tokio::select! {
                location = self.update_receiver.recv() => Action::HandleUpdate(location),
                Some(()) = sleep() => Action::RequestUpdate,
                else => Action::None,
            };

            let error = match action {
                Action::HandleUpdate(None) => true, // Update channel closed
                Action::HandleUpdate(Some(location)) => match self.handle_update(location) {
                    UpdateResult::Updated(location) => self
                        .action_sender
                        .send(Operation::Update(location))
                        .await
                        .is_err(),
                    UpdateResult::NotUpdated => false,
                },
                Action::RequestUpdate => match self.request_update().await {
                    RequestResult::ItemRemoved(position) => self
                        .action_sender
                        .send(Operation::Remove(position))
                        .await
                        .is_err(),
                    RequestResult::Failed => true,
                    _ => false,
                },
                Action::None => false,
            };
            if error {
                return;
            }
        }
    }

    /// Loads the specified [`Location`]s into the cache.
    ///
    /// Treats each [`Location`] as an update without sending [`Operation`] as output.
    /// This should be used to set the initial state of the cache.
    pub(crate) fn load_locations(&mut self, locations: impl Iterator<Item = Location>) {
        for location in locations {
            self.handle_update(location);
        }
    }

    fn handle_update(&mut self, location: Location) -> UpdateResult {
        let Location {
            position,
            attack_time,
        } = location.clone();

        let expiry_time = match self.cache.entry(position.clone()) {
            Occupied(mut entry) => {
                let item = entry.get_mut();
                match item.update(attack_time).map(|()| item.expiry_time()) {
                    Ok(expiry_time) => expiry_time,
                    Err(()) => {
                        return UpdateResult::NotUpdated;
                    }
                }
            }
            Vacant(entry) => {
                let item = entry.insert(CacheItem::new(attack_time, self.config.clone()));
                item.expiry_time()
            }
        };

        self.pending
            .push(Reverse(PendingUpdate::new(expiry_time, position)));

        UpdateResult::Updated(location)
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

        if time < item.expiry_time() {
            return RequestResult::NotNeeded;
        }

        let _ = item.delay();
        if item.retries().unwrap_or(0) > self.config.max_retries {
            // Remove item after retries exceed limit
            self.cache.remove(&position).unwrap(); // Item is guaranteed to be in cache
            return RequestResult::ItemRemoved(position);
        }

        // Push update back with new expiry time to allow retires in the future
        self.pending.push(Reverse(PendingUpdate {
            time: item.expiry_time(),
            position: position.clone(),
        }));

        let result = self.request_sender.send(position.into()).await;
        if result.is_err() {
            return RequestResult::Failed; // Request channel closed
        }
        RequestResult::Success
    }
}
