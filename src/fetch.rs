use anyhow::{Context, Result};
use clap::crate_version;
use futures::{prelude::*, stream};
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;
use tokio::fs;

use crate::config::Config;

pub(crate) const BACKOFF_DELAY: Duration = Duration::from_secs(10);
const ZERO_DURATION: Duration = Duration::from_secs(0);

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
///     max_zoom: 10,
/// };
///
/// fetch(config).await.expect("failed fetching tiles");
/// # }
/// ```
///
/// # Panics
/// Panics if the specified output folder exists and is not a folder but a file.
pub async fn fetch(cfg: Config) -> Result<()> {
    let output_folder = cfg.output_folder.as_path();

    assert!(
        !output_folder.exists() || output_folder.is_dir(),
        "output must be a directory",
    );

    if !output_folder.exists() {
        fs::create_dir_all(output_folder)
            .await
            .context("failed to create root output directory")?;
    }

    let pb = ProgressBar::new(cfg.tiles().count() as u64);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:60.cyan/blue} {pos:>7}/{len:7} ETA: {eta} {msg}")
            .progress_chars("##-")
    );

    let mut builder = reqwest::Client::builder();
    if cfg.timeout > ZERO_DURATION {
        builder = builder.timeout(cfg.timeout);
    }

    let mut headers = reqwest::header::HeaderMap::new();
    headers.append(
        reqwest::header::USER_AGENT,
        format!("osm-tile-downloader_rs_{}", crate_version!())
            .parse()
            .unwrap(),
    );

    let client = builder
        .default_headers(headers)
        .build()
        .with_context(|| "failed creating HTTP client")?;

    let num_retries = cfg.request_retries_amount;
    let fetch_existing = cfg.fetch_existing;
    let url_fmt = &cfg.url;

    let progress_bar = pb.wrap_iter(cfg.tiles());
    let s = stream::iter(progress_bar);
    s.for_each_concurrent(cfg.fetch_rate as usize, |tile| {
        let http_client = client.clone();

        async move {
            let mut res = Ok(());

            for _ in 0..num_retries {
                res = tile
                    .fetch_from(&http_client, url_fmt, output_folder, fetch_existing)
                    .await;

                if res.is_ok() {
                    return;
                }

                tokio::time::delay_for(BACKOFF_DELAY).await;
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

    pb.finish_and_clear();

    Ok(())
}
