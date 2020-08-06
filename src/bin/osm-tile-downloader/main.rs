mod args;
mod validators;

use anyhow::Result;
use args::Args;
use osm_tile_downloader::{fetch, Config};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let dry_run = args.dry_run;
    let config: Config = args.into();

    if dry_run {
        let tile_count = config
            .bounding_box
            .tiles(config.min_zoom, config.max_zoom)
            .count();

        eprintln!(
            "would download {} tiles (approx {}, assuming 10 kb per tile)",
            tile_count,
            pretty_bytes::converter::convert((tile_count as f64) * 10_000f64)
        );

        Ok(())
    } else {
        fetch(config).await
    }
}
