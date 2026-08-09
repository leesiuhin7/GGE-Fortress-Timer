use std::time::Duration;

use tokio::sync::mpsc;

use crate::{
    client::{Command, Sender},
    core::Grid,
    map::{
        explorer::Explorer,
        serialize::MapArguments,
        utils::{DuplicateChecker, Throttle, add_surroundings, remove_duplicates},
    },
};

enum SendError {
    Serialize,
    Send,
    Explorer,
}

pub(crate) struct Adapter {
    request_receiver: mpsc::Receiver<Grid>,
    sender: Sender,
    explorer: Explorer,
    cooldown: Duration,
}

impl Adapter {
    pub(crate) fn new(
        explorer: Explorer,
        request_receiver: mpsc::Receiver<Grid>,
        sender: Sender,
        cooldown: Duration,
    ) -> Self {
        Adapter {
            request_receiver,
            sender,
            explorer,
            cooldown,
        }
    }

    pub(crate) async fn run(mut self) {
        let mut throttle = Throttle::new(self.cooldown);
        let mut checker = DuplicateChecker::new(Duration::from_mins(2));
        loop {
            throttle.wait().await;
            let Some(grid) = self.get_grid().await else {
                return; // Request channel closed
            };

            let grids = remove_duplicates(add_surroundings(grid).into_iter(), &mut checker)
                .collect::<Vec<_>>();

            if !grids.is_empty() {
                throttle.send().await;
                for grid in grids {
                    let result = self.send(grid).await;
                    if result.is_err() {
                        return;
                    }
                }
            }
        }
    }

    async fn get_grid(&mut self) -> Option<Grid> {
        tokio::select! {
            biased;
            grid = self.request_receiver.recv() => grid,
            Some(grid) = self.explorer.get_next() => grid.ok(),
        }
    }

    async fn send(&mut self, grid: Grid) -> Result<(), SendError> {
        let map_args: MapArguments = grid.clone().try_into().map_err(|()| SendError::Serialize)?;
        let payload = map_args.as_json();
        self.sender
            .send(Command {
                command: "gaa".into(),
                payload,
            })
            .await
            .map_err(|()| SendError::Send)?;

        self.explorer
            .refresh_cooldown(grid)
            .await
            .map_err(|()| SendError::Explorer)
    }
}
