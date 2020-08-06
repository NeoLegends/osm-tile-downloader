use anyhow::{Context, Result};
use futures::prelude::*;
use reqwest::StatusCode;
use std::{f64::consts::PI, path::Path, time::Duration};
use tokio::fs;

use crate::fetch::BACKOFF_DELAY;
use crate::url::UrlFormat;

const LAT_MIN: f64 = -85_f64 / 180_f64 * PI;
const LAT_MAX: f64 = 85_f64 / 180_f64 * PI;
const LON_MIN: f64 = -1_f64 * PI;
const LON_MAX: f64 = PI;

/// An OSM slippy-map tile with x, y and z-coordinate.
/// ref: https://wiki.openstreetmap.org/wiki/Slippy_map_tilenames
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Tile {
    pub x: usize,
    pub y: usize,
    pub z: u8,
}

impl Tile {
    pub fn new(x: usize, y: usize, z: u8) -> Self {
        Self { x, y, z }
    }

    pub fn from_coords_and_zoom(lat_rad: f64, lon_rad: f64, zoom: u8) -> Self {
        assert!(zoom > 0);
        assert!(lat_rad >= LAT_MIN && lat_rad <= LAT_MAX);
        assert!(lon_rad >= LON_MIN && lon_rad <= LON_MAX);

        // scale factor
        let n = 2_f64.powi(zoom as i32);

        let lon_deg = lon_rad * 180_f64 / PI;

        let x = (lon_deg + 180_f64) / 360_f64 * n;
        let y = (1_f64 - lat_rad.tan().asinh() / PI) / 2_f64 * n;

        Self::new(x as usize, y as usize, zoom)
    }

    /// Fetches the given tile from the given URL using the given HTTP client.
    pub async fn fetch_from(
        &self,
        client: &reqwest::Client,
        url_fmt: &UrlFormat,
        output_folder: &Path,
        fetch_existing: bool,
    ) -> Result<()> {
        let formatted_url = url_fmt.tile_url(&self)?;

        let output_file = {
            let mut target = output_folder.join(self.z.to_string());
            target.push(self.x.to_string());
            fs::create_dir_all(&target).await.with_context(|| {
                format!(
                    "failed creating output directory for tile {}x{}x{}",
                    self.x, self.y, self.z
                )
            })?;
            target.push(format!("{}.png", self.y));

            target
        };

        // if the tile's already been downloaded, skip it
        if !fetch_existing && output_file.exists() {
            return Ok(());
        }

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

                tokio::time::delay_for(retry_after).await;
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
                .map_err(|e| tokio::io::Error::new(tokio::io::ErrorKind::Other, e));

            break tokio::io::stream_reader(response_stream);
        };

        let mut output_file = tokio::fs::File::create(output_file).await?;
        tokio::io::copy(&mut response_reader, &mut output_file)
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
