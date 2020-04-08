use futures::{prelude::*, stream};
use indicatif::ProgressBar;
use reqwest::Client;
use std::{
    collections::HashMap,
    f64,
    fmt::Debug,
    fs,
    io::{Error, ErrorKind},
    path::Path,
    time::Duration,
    u64,
};
use tokio::{self, prelude::*};

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

pub async fn fetch(cfg: Config<'_>) -> Result<(), Error> {
    assert!(
        !cfg.output_folder.exists() || cfg.output_folder.is_dir(),
        "output must be a directory",
    );

    if !cfg.output_folder.exists() {
        fs::create_dir_all(cfg.output_folder)?;
    }

    let pb = ProgressBar::new(cfg.tiles().count() as u64);

    let mut builder = Client::builder();
    if cfg.timeout_secs > 0 {
        builder = builder.timeout(Duration::from_secs(cfg.timeout_secs));
    }

    let client = builder
        .build()
        .map_err(|e| Error::new(ErrorKind::Other, e))?;

    stream::iter(pb.wrap_iter(cfg.tiles()))
        .for_each_concurrent(cfg.fetch_rate as usize, |tile| {
            let tile_2 = tile;

            tile.fetch_from(&client, cfg.url, cfg.output_folder)
                .map(move |res| {
                    if let Err(e) = res {
                        eprintln!(
                            "Failed fetching {}/{}/{}: {:?}",
                            tile_2.z, tile_2.x, tile_2.y, e
                        );
                    }
                })
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
    pub fn tiles(&self) -> impl Iterator<Item = Tile> {
        self.bounding_box.tiles(self.zoom_level)
    }
}

impl Tile {
    pub async fn fetch_from(
        self,
        client: &Client,
        url_fmt: &str,
        output_folder: &Path,
    ) -> Result<(), Error> {
        let mut map = HashMap::with_capacity(3);
        map.insert("x".to_owned(), self.x);
        map.insert("y".to_owned(), self.y);
        map.insert("z".to_owned(), self.z as usize);

        let formatted_url =
            strfmt::strfmt(url_fmt, &map).expect("failed formatting URL");
        let mut resp = client
            .get(&formatted_url)
            .send()
            .map_err(|e| Error::new(ErrorKind::Other, e))
            .await?;

        let mut target = output_folder.join(self.z.to_string());
        target.push(self.x.to_string());

        tokio::fs::create_dir_all(&target).await?;

        target.push(self.y.to_string());
        let mut file = tokio::fs::File::create(target).await?;

        while let Some(chunk) = resp
            .chunk()
            .await
            .map_err(|e| Error::new(ErrorKind::Other, e))?
        {
            file.write_all(&chunk).await?;
        }

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
