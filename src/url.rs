use anyhow::{Context, Result};
use maplit::hashmap;
use std::{cell::RefCell, fmt, sync::Mutex};
use strfmt::strfmt;

use crate::tile::Tile;

const OSM_SERVERS: &[&str] = &["a", "b", "c"];

/// A tile URL formatter. Tile URLs are allowed to contain any of
/// the following tokens:
///
/// - `x`: the X coordinate of the tile
/// - `y`: the Y coordinate of the tile
/// - `z`: the Z coordinate (zoom level) of the tile
/// - `s`: the subdomain, sequentially chosen from `["a", "b", "c"]`
///
/// Subdomains (`{s}`) aren't required, but they help with parallel
/// downloads. Format tokens should be surrounded by curly brackets.
///
/// # Example
/// ```rust
/// # use anyhow::Result;
/// use osm_tile_downloader::{Tile, UrlFormat};
///
/// # fn main() -> Result<()> {
/// let format_str = "https://{s}.foo.com/{x}/{y}/{z}.png".to_owned();
/// let url_fmt = UrlFormat::from_string(format_str);
/// let tile = Tile::new(1, 2, 3);
///
/// assert_eq!(url_fmt.tile_url(&tile)?, "https://a.foo.com/1/2/3.png");
/// assert!(url_fmt.tile_url(&tile)?.starts_with("https://b.foo.com"));
/// assert!(url_fmt.tile_url(&tile)?.starts_with("https://c.foo.com"));
/// assert!(url_fmt.tile_url(&tile)?.starts_with("https://a.foo.com"));
/// assert!(url_fmt.tile_url(&tile)?.starts_with("https://b.foo.com"));
/// assert!(url_fmt.tile_url(&tile)?.starts_with("https://c.foo.com"));
/// # Ok(())
/// # }
/// ```
pub struct UrlFormat {
    inc: Mutex<RefCell<u8>>,
    format_str: String,
}

impl UrlFormat {
    /// Create a new URL formatter from a given format string.
    pub fn from_string(format_str: String) -> Self {
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

    /// Get a formatted URL for the given tile.
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
