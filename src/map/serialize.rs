use crate::core::{Grid, Kingdom};

pub(super) struct MapArguments {
    kingdom: Kingdom,
    x1: u16,
    y1: u16,
    x2: u16,
    y2: u16,
}

impl MapArguments {
    fn new(kingdom: Kingdom, x: u16, y: u16) -> Result<Self, ()> {
        if x <= 1274 && y <= 1274 {
            Ok(MapArguments {
                kingdom,
                x1: x,
                y1: y,
                x2: x + 12,
                y2: y + 12,
            })
        } else {
            Err(())
        }
    }

    pub(super) fn as_json(&self) -> String {
        let kingdom_id: u8 = self.kingdom.clone().into();
        format!(
            r#"{{"KID":{},"AX1":{},"AY1":{},"AX2":{},"AY2":{}}}"#,
            kingdom_id, self.x1, self.y1, self.x2, self.y2,
        )
    }
}

impl TryFrom<Grid> for MapArguments {
    type Error = ();

    fn try_from(value: Grid) -> Result<Self, Self::Error> {
        let Grid { x, y, kingdom } = value;
        Self::new(
            kingdom,
            u16::try_from(x * 13).map_err(|_| ())?,
            u16::try_from(y * 13).map_err(|_| ())?,
        )
    }
}
