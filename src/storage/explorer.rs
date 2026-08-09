use futures_util::{StreamExt, TryStreamExt};
use mongodb::{
    bson::{DateTime, Document, doc},
    error::Error,
};
use serde::{Deserialize, Serialize};

use crate::{core::Grid, map::ExplorerItem, storage::Storage};

#[derive(Serialize, Deserialize)]
pub(super) struct ExplorerItemDocument {
    x: u32,
    y: u32,
    kingdom_id: u8,
    explore_time: DateTime,
}

impl From<ExplorerItem> for ExplorerItemDocument {
    fn from(value: ExplorerItem) -> Self {
        let ExplorerItem {
            explore_time,
            grid: Grid { x, y, kingdom },
        } = value;
        ExplorerItemDocument {
            x,
            y,
            kingdom_id: kingdom.into(),
            explore_time: explore_time.into(),
        }
    }
}

impl TryFrom<ExplorerItemDocument> for ExplorerItem {
    type Error = ();

    fn try_from(value: ExplorerItemDocument) -> Result<Self, Self::Error> {
        let ExplorerItemDocument {
            x,
            y,
            kingdom_id,
            explore_time,
        } = value;
        Ok(ExplorerItem {
            explore_time: explore_time.into(),
            grid: Grid {
                x,
                y,
                kingdom: kingdom_id.try_into()?,
            },
        })
    }
}

fn grid_filter(grid: Grid) -> Document {
    let Grid { x, y, kingdom } = grid;
    let kingdom_id = u32::from(u8::from(kingdom));
    doc! {
        "x": x,
        "y": y,
        "kingdom_id": kingdom_id,
    }
}

impl Storage {
    pub(crate) async fn get_explorer_items(&self) -> Result<Vec<ExplorerItem>, Error> {
        let cursor = self.collections.explorer.find(doc! {}).await?;
        cursor
            .map(|result| match result {
                Ok(document) => document
                    .try_into()
                    .map_err(|()| Error::custom("Deserialize")),
                Err(error) => Err(error),
            })
            .try_collect::<Vec<_>>()
            .await
    }

    pub(crate) async fn update_explorer_items(
        &self,
        explorer_item: ExplorerItem,
    ) -> Result<(), Error> {
        let filter = grid_filter(explorer_item.grid.clone());
        let document = ExplorerItemDocument::from(explorer_item);

        self.collections
            .explorer
            .find_one_and_replace(filter, document)
            .upsert(true)
            .await
            .map(|_| ())
    }
}
