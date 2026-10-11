use std::{
    cmp::Reverse,
    collections::{
        BinaryHeap, HashMap,
        binary_heap::PeekMut,
        hash_map::Entry::{Occupied, Vacant},
    },
    ops::{AddAssign, SubAssign},
    time::{Duration, SystemTime},
};

use itertools::Itertools;
use tokio::sync::mpsc;

use crate::core::{Grid, Kingdom};

fn sq_dist_from_center(grid: &Grid) -> f64 {
    let Grid { x, y, kingdom: _ } = grid;
    let center_x = f64::from(*x) + 0.5;
    let center_y = f64::from(*y) + 0.5;
    let dx = center_x - 49.0;
    let dy = center_y - 49.0;
    dx.powi(2) + dy.powi(2)
}

#[derive(PartialEq, Eq, Clone)]
pub(crate) struct ExplorerItem {
    pub(crate) explore_time: SystemTime,
    pub(crate) grid: Grid,
}

impl ExplorerItem {
    fn new(grid: Grid, cooldown: Duration) -> Self {
        ExplorerItem {
            explore_time: SystemTime::now() + cooldown,
            grid,
        }
    }

    fn with_time(grid: Grid, explore_time: SystemTime) -> Self {
        ExplorerItem { explore_time, grid }
    }
}

impl PartialOrd for ExplorerItem {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ExplorerItem {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.explore_time
            .cmp(&other.explore_time)
            // Prioritize ones that are closer to the center
            .then(sq_dist_from_center(&self.grid).total_cmp(&sq_dist_from_center(&other.grid)))
    }
}

pub(crate) struct Explorer {
    ignore_counts: HashMap<Grid, u32>,
    grids: BinaryHeap<Reverse<ExplorerItem>>,
    cooldown: Duration,
    update_sender: mpsc::Sender<ExplorerItem>,
}

impl Explorer {
    pub(crate) fn new(
        update_sender: mpsc::Sender<ExplorerItem>,
        kingdoms: Vec<Kingdom>,
        cooldown: Duration,
    ) -> Self {
        let mut ignore_counts = HashMap::new();
        let mut grids = BinaryHeap::new();

        let time = SystemTime::now(); // Use the same time for all items to prevent affecting order
        for ((x, y), kingdom) in (0..=98)
            .cartesian_product(0..=98)
            .cartesian_product(kingdoms)
        {
            let grid = Grid { x, y, kingdom };
            ignore_counts.insert(grid.clone(), 0);
            grids.push(Reverse(ExplorerItem::with_time(grid, time)));
        }

        Explorer {
            ignore_counts,
            grids,
            cooldown,
            update_sender,
        }
    }

    pub(crate) fn load_items(&mut self, items: impl Iterator<Item = ExplorerItem>) {
        for item in items {
            self.load_item(item);
        }
    }

    /// Waits until a [`Grid`] needs to be explored
    ///
    /// # Cancellation
    ///
    /// This operation is not cancellation safe. Cancellation may cause a [`Grid`] to be
    /// lost forever.
    pub(super) async fn get_next(&mut self) -> Option<Result<Grid, ()>> {
        loop {
            let peek = self.grids.peek_mut()?;
            let Reverse(ExplorerItem {
                explore_time: time,
                grid,
            }) = &*peek;

            // Using unwrap as ignore_counts should be a superset of grids
            let ignore_count = self.ignore_counts.get_mut(grid).unwrap();
            if *ignore_count > 0 {
                ignore_count.sub_assign(1);
                PeekMut::pop(peek);
                continue; // Move to the next item
            }

            let duration = time
                .duration_since(SystemTime::now())
                .unwrap_or(Duration::ZERO);
            tokio::time::sleep(duration).await;

            // Deferring removal for cancellation safety
            let Reverse(ExplorerItem {
                explore_time: _,
                grid,
            }) = PeekMut::pop(peek);

            // Removing the entry to maintain the state of ignore_counts
            // Using unwrap as the entry is guaranteed to exist
            self.ignore_counts.remove(&grid).unwrap();
            // Reset cooldown
            return Some(self.refresh_cooldown(grid.clone()).await.map(|()| grid));
        }
    }

    /// Refreshes the explore cooldown of the grid position.
    ///
    /// # Cancellation
    ///
    /// This operation is cancellation safe. Cancellation aborts the operation.
    pub(super) async fn refresh_cooldown(&mut self, grid: Grid) -> Result<(), ()> {
        let item = ExplorerItem::new(grid, self.random_cooldown());
        self.update_sender
            .send(item.clone())
            .await
            .map_err(|_| ())?;
        self.load_item(item);
        Ok(())
    }

    fn load_item(&mut self, item: ExplorerItem) {
        match self.ignore_counts.entry(item.grid.clone()) {
            Vacant(entry) => {
                entry.insert(0);
            }
            Occupied(mut entry) => {
                entry.get_mut().add_assign(1);
            }
        }
        self.grids.push(Reverse(item));
    }

    fn random_cooldown(&self) -> Duration {
        // Use random factor of [0.5, 1.5)
        self.cooldown.mul_f64(rand::random::<f64>() + 0.5)
    }
}
