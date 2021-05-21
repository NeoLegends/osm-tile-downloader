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
//!   --url "https://{s}.tile.openstreetmap.de/{z}/{x}/{y}.png" \
//!   --north 50.811 \
//!   --east 6.1649 \
//!   --south 50.7492 \
//!   --west 6.031 \
//!   --output ./tiles \
//!   --rate 10
//! ```
//!
//! # Library Example
//! ```rust
//! use osm_tile_downloader::{fetch, BoundingBox, Config, UrlFormat};
//! use std::time::Duration;
//!
//! # #[tokio::main]
//! # async fn main() {
//! let config = Config {
//!     bounding_box: BoundingBox::new_deg(50.811, 6.1649, 50.7492, 6.031),
//!     fetch_rate: 10,
//!     output_folder: "./tiles".into(),
//!     request_retries_amount: 3,
//!     url: UrlFormat::from_string("https://{s}.tile.openstreetmap.de/{z}/{x}/{y}.png".into()),
//!     timeout: Duration::from_secs(30),
//!     min_zoom: 1,
//!     max_zoom: 2,
//!     fetch_existing: false,
//! };
//!
//! fetch(config).await.expect("failed fetching tiles");
//! # }
//! ```

mod bounding_box;
mod config;
mod fetch;
mod tile;
mod url;

pub use bounding_box::{BoundingBox, Fixture};
pub use config::Config;
pub use fetch::fetch;
pub use tile::Tile;
pub use url::UrlFormat;

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
        let tile = Tile::from_coords_and_zoom(
            50.7929f64.to_radians(),
            6.0402f64.to_radians(),
            18,
        );
        assert_eq!((tile.x, tile.y), (135470, 87999));
    }
}
