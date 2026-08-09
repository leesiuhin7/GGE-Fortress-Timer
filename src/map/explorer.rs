use std::{
    cmp::Reverse,
    collections::{
        BinaryHeap, HashMap,
        binary_heap::PeekMut,
        hash_map::Entry::{Occupied, Vacant},
    },
    ops::{AddAssign, SubAssign},
    time::Duration,
};

use itertools::Itertools;
use tokio::time::Instant;

use crate::core::{Grid, Kingdom};

fn sq_dist_from_center(grid: &Grid) -> f64 {
    let Grid { x, y, kingdom: _ } = grid;
    let center_x = f64::from(*x) + 0.5;
    let center_y = f64::from(*y) + 0.5;
    let dx = center_x - 49.0;
    let dy = center_y - 49.0;
    dx.powi(2) + dy.powi(2)
}

#[derive(PartialEq, Eq)]
struct Item {
    time: Instant,
    grid: Grid,
}

impl Item {
    fn new(grid: Grid, cooldown: Duration) -> Self {
        Item {
            time: Instant::now() + cooldown,
            grid,
        }
    }

    fn with_time(grid: Grid, time: Instant) -> Self {
        Item { time, grid }
    }
}

impl PartialOrd for Item {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Item {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time
            .cmp(&other.time)
            // Prioritize ones that are closer to the center
            .then(sq_dist_from_center(&self.grid).total_cmp(&sq_dist_from_center(&other.grid)))
    }
}

pub(super) struct Explorer {
    ignore_counts: HashMap<Grid, u32>,
    grids: BinaryHeap<Reverse<Item>>,
    cooldown: Duration,
}

impl Explorer {
    pub(super) fn new(kingdoms: Vec<Kingdom>, cooldown: Duration) -> Self {
        let mut ignore_counts = HashMap::new();
        let mut grids = BinaryHeap::new();

        let time = Instant::now(); // Use the same time for all items to prevent affecting order
        for ((x, y), kingdom) in (0..=98)
            .cartesian_product(0..=98)
            .cartesian_product(kingdoms)
        {
            let grid = Grid { x, y, kingdom };
            ignore_counts.insert(grid.clone(), 0);
            grids.push(Reverse(Item::with_time(grid, time)));
        }

        Explorer {
            ignore_counts,
            grids,
            cooldown,
        }
    }

    /// Waits until a [`Grid`] needs to be explored
    ///
    /// # Cancellation
    ///
    /// This operation is cancellation safe. Cancellation simply aborts waiting.
    pub(super) async fn get_next(&mut self) -> Option<Grid> {
        loop {
            let peek = self.grids.peek_mut()?;
            let Reverse(Item { time, grid }) = &*peek;

            // Using unwrap as ignore_counts should be a superset of grids
            let ignore_count = self.ignore_counts.get_mut(grid).unwrap();
            if *ignore_count > 0 {
                ignore_count.sub_assign(1);
                PeekMut::pop(peek);
                continue; // Move to the next item
            }

            tokio::time::sleep_until(*time).await;
            // Deferring removal for cancellation safety
            let Reverse(Item { time: _, grid }) = PeekMut::pop(peek);
            self.refresh_cooldown(grid.clone()); // Reset cooldown
            return Some(grid);
        }
    }

    pub(super) fn refresh_cooldown(&mut self, grid: Grid) {
        match self.ignore_counts.entry(grid.clone()) {
            Vacant(entry) => {
                entry.insert(0);
            }
            Occupied(mut entry) => {
                entry.get_mut().add_assign(1);
            }
        }
        self.grids
            .push(Reverse(Item::new(grid, self.random_cooldown())));
    }

    fn random_cooldown(&self) -> Duration {
        // Use random factor of [0.5, 1.5)
        self.cooldown.mul_f64(rand::random::<f64>() + 0.5)
    }
}
