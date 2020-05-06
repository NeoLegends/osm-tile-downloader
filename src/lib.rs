use anyhow::{Context, Result};
use futures::{prelude::*, stream};
use indicatif::ProgressBar;
use rand::{self, seq::SliceRandom};
use reqwest::Client;
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

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct BoundingBox {
    north: f64,
    west: f64,
    east: f64,
    south: f64,
}

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Tile {
    x: usize,
    y: usize,
    z: u8,
}

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
                for _ in 0..cfg.request_retries_amount {
                    let res = tile
                        .fetch_from(&http_client, cfg.url, cfg.output_folder)
                        .await;

                    if res.is_ok() {
                        return;
                    }

                    time::delay_for(Duration::from_secs(3)).await;
                }

                eprintln!("Failed fetching tile {}x{}x{}.", tile.z, tile.x, tile.y);
            }
        })
        .await;

    pb.finish();

    Ok(())
}

impl BoundingBox {
    pub fn new_deg(north: f64, east: f64, south: f64, west: f64) -> Self {
        Self::new(
            north.to_radians(),
            east.to_radians(),
            south.to_radians(),
            west.to_radians(),
        )
    }

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

    pub fn north(&self) -> f64 {
        self.north
    }

    pub fn east(&self) -> f64 {
        self.east
    }

    pub fn south(&self) -> f64 {
        self.south
    }

    pub fn west(&self) -> f64 {
        self.west
    }

    pub fn tuple(&self) -> (f64, f64, f64, f64) {
        (self.north, self.east, self.south, self.west)
    }

    pub fn tiles(&self, upto_zoom: u8) -> impl Iterator<Item = Tile> + Debug {
        assert!(upto_zoom >= 1);

        let (north, east, south, west) = self.tuple();

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
    pub fn tiles(&self) -> impl Iterator<Item = Tile> + Debug {
        self.bounding_box.tiles(self.zoom_level)
    }
}

impl Tile {
    pub async fn fetch_from(
        self,
        client: &Client,
        url_fmt: &str,
        output_folder: &Path,
    ) -> Result<()> {
        const OSM_SERVERS: &[&'static str] = &["a", "b", "c"];

        let formatted_url = {
            let mut map = HashMap::with_capacity(3);
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

        let mut response_reader = {
            let raw_response =
                client.get(&formatted_url).send().await.with_context(|| {
                    format!("failed fetching tile {}x{}x{}", self.x, self.y, self.z)
                })?;
            let status_checked_response =
                raw_response.error_for_status().with_context(|| {
                    format!(
                        "received invalid status code fecthing tile {}x{}x{}",
                        self.x, self.y, self.z
                    )
                })?;
            let response_stream = status_checked_response
                .bytes_stream()
                .map_err(|e| IoError::new(ErrorKind::Other, e));

            io::stream_reader(response_stream)
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
    fn tile_index() {
        assert_eq!(
            tile_indices(18, 6.0402f64.to_radians(), 50.7929f64.to_radians()),
            (135470, 87999)
        );
    }
}
