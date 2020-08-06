use std::{fmt::Debug, path::PathBuf, time::Duration};

use crate::bounding_box::BoundingBox;
use crate::tile::Tile;
use crate::url::UrlFormat;

/// Tile fetching configuration.
#[derive(Debug, PartialEq)]
pub struct Config {
    /// Bounding box in top, right, bottom, left order.
    pub bounding_box: BoundingBox,

    /// Whether to skip tiles that are already downloaded.
    pub fetch_existing: bool,

    /// Maximum number of parallel downloads.
    pub fetch_rate: u8,

    /// The folder to output the data to.
    pub output_folder: PathBuf,

    /// How many times to retry a failed HTTP request.
    pub request_retries_amount: u8,

    /// The URL to download individual tiles from including the replacement
    /// specifiers `{x}`, `{y}` and `{z}`.
    pub url: UrlFormat,

    /// Timeout for fetching a single tile.
    ///
    /// Pass the zero duration to disable the timeout.
    pub timeout: Duration,

    /// The minimum zoom level to download to.
    pub min_zoom: u8,

    /// The maximum zoom level to download to.
    pub max_zoom: u8,
}

impl Config {
    /// Creates an iterator iterating over all tiles in the contained bounding box.
    pub fn tiles(&self) -> impl Iterator<Item = Tile> + Debug {
        self.bounding_box.tiles(self.min_zoom, self.max_zoom)
    }
}
