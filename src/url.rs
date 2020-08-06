use anyhow::{Context, Result};
use maplit::hashmap;
use std::{cell::RefCell, fmt, sync::Mutex};
use strfmt::strfmt;

use crate::tile::Tile;

const OSM_SERVERS: &[&str] = &["a", "b", "c"];

pub struct UrlFormat {
    inc: Mutex<RefCell<u8>>,
    format_str: String,
}

impl UrlFormat {
    pub fn from_str(format_str: String) -> Self {
        Self {
            inc: Mutex::new(RefCell::new(0)),
            format_str,
        }
    }

    fn get_inc(&self) -> u8 {
        let inc = self.inc.lock().unwrap();
        let mut inc = inc.borrow_mut();

        let val = *inc;
        *inc += 1;

        val
    }

    pub fn tile_url(&self, tile: &Tile) -> Result<String> {
        let inc = self.get_inc() as usize;
        let vars = hashmap! {
            "s".to_owned() => OSM_SERVERS[inc % OSM_SERVERS.len()].to_owned(),
            "x".to_owned() => tile.x.to_string(),
            "y".to_owned() => tile.y.to_string(),
            "z".to_owned() => tile.z.to_string(),
        };

        strfmt(&self.format_str, &vars).context("failed formatting URL")
    }
}

impl PartialEq for UrlFormat {
    fn eq(&self, other: &Self) -> bool {
        self.format_str == other.format_str
    }
}

impl fmt::Debug for UrlFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UrlFormat")
            .field("format_str", &self.format_str)
            .finish()
    }
}
