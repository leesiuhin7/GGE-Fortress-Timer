use std::{
    collections::{
        HashMap,
        hash_map::Entry::{Occupied, Vacant},
    },
    hash::Hash,
    time::Duration,
};

use tokio::time::Instant;

use crate::core::Grid;

pub(super) struct Throttle {
    next: Option<Instant>,
    cooldown: Duration,
}

impl Throttle {
    pub(super) fn new(cooldown: Duration) -> Self {
        Throttle {
            next: None,
            cooldown,
        }
    }

    pub(super) async fn wait(&self) {
        let Some(time) = self.next else {
            return;
        };
        tokio::time::sleep_until(time).await;
    }

    pub(super) async fn send(&mut self) {
        self.wait().await;
        self.next = Some(Instant::now() + self.cooldown);
    }
}

pub(super) struct DuplicateChecker<T: Hash + Eq + Clone> {
    table: HashMap<T, Instant>,
    window: Duration,
}

impl<T: Hash + Eq + Clone> DuplicateChecker<T> {
    pub(super) fn new(window: Duration) -> Self {
        DuplicateChecker {
            table: HashMap::new(),
            window,
        }
    }

    pub(super) fn send(&mut self, value: &T) -> Result<(), ()> {
        match self.table.entry(value.clone()) {
            Vacant(entry) => {
                entry.insert(Instant::now() + self.window);
            }
            Occupied(mut entry) => {
                let time = entry.get_mut();
                if *time > Instant::now() {
                    return Err(());
                }
                entry.insert(Instant::now() + self.window);
            }
        }
        Ok(())
    }
}

pub(super) fn add_surroundings(grid: Grid) -> [Grid; 4] {
    let Grid { x, y, kingdom } = grid;
    [(0, 0), (-1, -1), (0, -1), (-1, 0)].map(|(dx, dy)| Grid {
        x: x.clamp(1, 98).wrapping_add_signed(dx),
        y: y.clamp(1, 98).wrapping_add_signed(dy),
        kingdom: kingdom.clone(),
    })
}

pub(super) fn remove_duplicates(
    iter: impl Iterator<Item = Grid>,
    checker: &mut DuplicateChecker<Grid>,
) -> impl Iterator<Item = Grid> {
    iter.filter_map(|grid| checker.send(&grid).ok().map(|()| grid))
}
