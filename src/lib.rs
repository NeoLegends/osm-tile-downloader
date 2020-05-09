//! Download OpenStreetMap-tiles to your disk en-masse.
//!
//! **Use with absolute caution.** Downloading tiles en-masse can hog
//! down a tile server easily. I am not responsible for any damage this
//! tool may cause.
//!
//! # Usage
//!
//! This tool is available on [crates.io](https://crates.io) and can be
//! installed via `cargo install osm-tile-downloader`. It features a helpful
//! CLI you can access via `-h` / `--help`.
//!
//! It is also available as a library.
//!
//! # CLI Example
//!
//! ```bash
//! osm-tile-downloader \
//!   --north 50.811 \
//!   --east 6.1649 \
//!   --south 50.7492 \
//!   --west 6.031 \
//!   --url https://\{s\}.tile.openstreetmap.de/\{z\}/\{x\}/\{y\}.png \
//!   --output ./tiles \
//!   --rate 10
//! ```
//!
//! # Library Example
//! ```rust
//! use osm_tile_downloader::{fetch, BoundingBox, Config};
//! use std::path::Path;
//!
//! # #[tokio::main]
//! # async fn main() {
//! let config = Config {
//!     bounding_box: BoundingBox::new_deg(50.811, 6.1649, 50.7492, 6.031),
//!     fetch_rate: 10,
//!     output_folder: Path::new("./tiles"),
//!     request_retries_amount: 3,
//!     url: "https://{s}.tile.openstreetmap.de/{z}/{x}/{y}.png",
//!     timeout_secs: 30,
//!     zoom_level: 10,
//! };
//!
//! fetch(config).await.expect("failed fetching tiles");
//! # }
//! ```

use anyhow::{Context, Result};
use futures::{prelude::*, stream};
use indicatif::ProgressBar;
use rand::{self, seq::SliceRandom};
use reqwest::{Client, StatusCode};
use std::{
    collections::HashMap,
    f64,
    fmt::Debug,
    io::{Error as IoError, ErrorKind},
    path::Path,
    time::Duration,
    u64,
};
use tokio::{
    self,
    fs::{self, File},
    io, time,
};

const BACKOFF_DELAY: Duration = Duration::from_secs(10);

/// A bounding box consisting of north, east, south and west coordinate boundaries
/// given from 0 to 2π.
///
/// # Example
/// ```rust
/// # use osm_tile_downloader::BoundingBox;
/// let aachen_germany = BoundingBox::new_deg(50.811, 6.1649, 50.7492, 6.031);
/// ```
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct BoundingBox {
    north: f64,
    west: f64,
    east: f64,
    south: f64,
}

/// Tile fetching configuration.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct Config<'a> {
    /// Bounding box in top, right, bottom, left order.
    pub bounding_box: BoundingBox,

    /// Maximum number of parallel downloads.
    pub fetch_rate: u8,

    /// The folder to output the data to.
    pub output_folder: &'a Path,

    /// How many times to retry a failed HTTP request.
    pub request_retries_amount: u8,

    /// The URL to download individual tiles from including the replacement
    /// specifiers `{x}`, `{y}` and `{z}`.
    pub url: &'a str,

    /// Timeout for fetching a single tile.
    pub timeout_secs: u64,

    /// The zoom level to download to.
    pub zoom_level: u8,
}

/// An OSM slippy-map tile with x, y and z-coordinate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Tile {
    x: usize,
    y: usize,
    z: u8,
}

/// Asynchronously fetch the open street map tiles specified in `cfg` and save them
/// to the file system.
///
/// Creates the required directories recursively and overwrites any existing files
/// at the destination.
///
/// # Example
/// ```rust
/// use osm_tile_downloader::{fetch, BoundingBox, Config};
/// # use std::path::Path;
///
/// # #[tokio::main]
/// # async fn main() {
/// let config = Config {
///     bounding_box: BoundingBox::new_deg(50.811, 6.1649, 50.7492, 6.031),
///     fetch_rate: 10,
///     output_folder: Path::new("./tiles"),
///     request_retries_amount: 3,
///     url: "https://{s}.tile.openstreetmap.de/{z}/{x}/{y}.png",
///     timeout_secs: 30,
///     zoom_level: 10,
/// };
///
/// fetch(config).await.expect("failed fetching tiles");
/// # }
/// ```
///
/// # Panics
/// Panics if the specified output folder exists and is not a folder but a file.
pub async fn fetch(cfg: Config<'_>) -> Result<()> {
    assert!(
        !cfg.output_folder.exists() || cfg.output_folder.is_dir(),
        "output must be a directory",
    );

    if !cfg.output_folder.exists() {
        fs::create_dir_all(cfg.output_folder)
            .await
            .context("failed to create root output directory")?;
    }

    let pb = ProgressBar::new(cfg.tiles().count() as u64);

    let mut builder = Client::builder();
    if cfg.timeout_secs > 0 {
        builder = builder.timeout(Duration::from_secs(cfg.timeout_secs));
    }

    let client = builder
        .build()
        .with_context(|| "failed creating HTTP client")?;

    stream::iter(pb.wrap_iter(cfg.tiles()))
        .for_each_concurrent(cfg.fetch_rate as usize, |tile| {
            let http_client = client.clone();

            async move {
                let mut res = Ok(());

                for _ in 0..cfg.request_retries_amount {
                    res = tile
                        .fetch_from(&http_client, cfg.url, cfg.output_folder)
                        .await;

                    if res.is_ok() {
                        return;
                    }

                    time::delay_for(BACKOFF_DELAY).await;
                }

                eprintln!(
                    "Failed fetching tile {}x{}x{}: {:?}",
                    tile.z,
                    tile.x,
                    tile.y,
                    res.unwrap_err(),
                );
            }
        })
        .await;

    pb.finish();

    Ok(())
}

impl BoundingBox {
    /// Create a new bounding box from the specified coordinates specified in degrees (0-360°).
    ///
    /// # Example
    /// ```rust
    /// # use osm_tile_downloader::BoundingBox;
    /// let aachen_germany = BoundingBox::new_deg(50.811, 6.1649, 50.7492, 6.031);
    /// ```
    ///
    /// # Panics
    /// Panics if the coordinates are < 0, or >= 360.
    pub fn new_deg(north: f64, east: f64, south: f64, west: f64) -> Self {
        Self::new(
            north.to_radians(),
            east.to_radians(),
            south.to_radians(),
            west.to_radians(),
        )
    }

    /// Create a new bounding box from the specified coordinates specified in radians (0-2π).
    ///
    /// # Panics
    /// Panics if the coordinates are < 0, or >= 2π.
    pub fn new(north: f64, east: f64, south: f64, west: f64) -> Self {
        assert!(north >= 0.0 && north < 2f64 * f64::consts::PI);
        assert!(east >= 0.0 && east < 2f64 * f64::consts::PI);
        assert!(south >= 0.0 && south < 2f64 * f64::consts::PI);
        assert!(west >= 0.0 && west < 2f64 * f64::consts::PI);

        BoundingBox {
            north,
            east,
            south,
            west,
        }
    }

    /// Gets the north coordinate.
    pub fn north(&self) -> f64 {
        self.north
    }

    /// Gets the east coordinate.
    pub fn east(&self) -> f64 {
        self.east
    }

    /// Gets the south coordinate.
    pub fn south(&self) -> f64 {
        self.south
    }

    /// Gets the west coordinate.
    pub fn west(&self) -> f64 {
        self.west
    }

    /// Creates an iterator iterating over all tiles in the bounding box.
    pub fn tiles(&self, upto_zoom: u8) -> impl Iterator<Item = Tile> + Debug {
        assert!(upto_zoom >= 1);

        let (north, east, south, west) = (self.north, self.east, self.south, self.west);

        (1..=upto_zoom).flat_map(move |level| {
            let (top_x, top_y) = tile_indices(level, west, north);
            let (bot_x, bot_y) = tile_indices(level, east, south);

            (top_x..=bot_x).flat_map(move |x| {
                (top_y..=bot_y).map(move |y| Tile { x, y, z: level })
            })
        })
    }
}

impl Config<'_> {
    /// Creates an iterator iterating over all tiles in the contained bounding box.
    pub fn tiles(&self) -> impl Iterator<Item = Tile> + Debug {
        self.bounding_box.tiles(self.zoom_level)
    }
}

impl Tile {
    /// Fetches the given tile from the given URL using the given HTTP client.
    pub async fn fetch_from(
        self,
        client: &Client,
        url_fmt: &str,
        output_folder: &Path,
    ) -> Result<()> {
        const OSM_SERVERS: &[&'static str] = &["a", "b", "c"];

        let formatted_url = {
            let mut map = HashMap::with_capacity(4);
            map.insert(
                "s".to_owned(),
                OSM_SERVERS
                    .choose(&mut rand::thread_rng())
                    .unwrap()
                    .to_string(),
            );
            map.insert("x".to_owned(), self.x.to_string());
            map.insert("y".to_owned(), self.y.to_string());
            map.insert("z".to_owned(), self.z.to_string());

            strfmt::strfmt(url_fmt, &map).context("failed formatting URL")?
        };

        let mut response_reader = loop {
            let raw_response =
                client.get(&formatted_url).send().await.with_context(|| {
                    format!("failed fetching tile {}x{}x{}", self.x, self.y, self.z)
                })?;

            if raw_response.status() == StatusCode::TOO_MANY_REQUESTS {
                let retry_after = raw_response
                    .headers()
                    .get("Retry-After")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|val| val.parse::<u64>().ok())
                    .map(Duration::from_secs)
                    .unwrap_or(BACKOFF_DELAY);

                time::delay_for(retry_after).await;
                continue;
            }

            let response_stream = raw_response
                .error_for_status()
                .with_context(|| {
                    format!(
                        "received invalid status code fetching tile {}x{}x{}",
                        self.x, self.y, self.z
                    )
                })?
                .bytes_stream()
                .map_err(|e| IoError::new(ErrorKind::Other, e));

            break io::stream_reader(response_stream);
        };

        let mut output_file = {
            let mut target = output_folder.join(self.z.to_string());
            target.push(self.x.to_string());
            fs::create_dir_all(&target).await.with_context(|| {
                format!(
                    "failed creating output directory for tile {}x{}x{}",
                    self.x, self.y, self.z
                )
            })?;
            target.push(self.y.to_string());

            File::create(target).await?
        };

        io::copy(&mut response_reader, &mut output_file)
            .await
            .with_context(|| {
                format!(
                    "failed streaming tile {}x{}x{} to disk",
                    self.x, self.y, self.z
                )
            })?;

        Ok(())
    }
}

fn tile_indices(zoom: u8, lon_rad: f64, lat_rad: f64) -> (usize, usize) {
    assert!(zoom > 0);
    assert!(lon_rad >= 0.0);
    assert!(lat_rad >= 0.0);

    let tile_x = {
        let deg = (lon_rad + f64::consts::PI) / (2f64 * f64::consts::PI);

        deg * 2f64.powi(zoom as i32)
    };
    let tile_y = {
        let trig = (lat_rad.tan() + (1f64 / lat_rad.cos())).ln();
        let inner = 1f64 - (trig / f64::consts::PI);

        inner * 2f64.powi(zoom as i32 - 1)
    };

    (tile_x as usize, tile_y as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[should_panic]
    fn bbox_panics_deg() {
        BoundingBox::new_deg(360.0, 0.0, 0.0, 0.0);
    }

    #[test]
    #[should_panic]
    fn bbox_panics_rad() {
        BoundingBox::new(7.0, 3.0, 3.0, 3.0);
    }

    #[test]
    fn tile_index() {
        assert_eq!(
            tile_indices(18, 6.0402f64.to_radians(), 50.7929f64.to_radians()),
            (135470, 87999)
        );
    }
}
