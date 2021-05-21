# osm-tile-downloader

Download OpenStreetMap-tiles to your disk en-masse.

**Use with absolute caution.** Downloading tiles en-masse can hog
down a tile server easily. I am not responsible for any damage this
tool may cause.

## Usage

This tool is available on [crates.io](https://crates.io) and can be
installed via `cargo install osm-tile-downloader`. It features a helpful
CLI you can access via `-h` / `--help`.

It is also available as a library.

## CLI Example

```bash
osm-tile-downloader \
  --north 50.811 \
  --east 6.1649 \
  --south 50.7492 \
  --west 6.031 \
  --url "https://{s}.tile.openstreetmap.de/{z}/{x}/{y}.png" \
  --output ./tiles \
  --rate 10
```

## Library Example

```rust
use osm_tile_downloader::{fetch, BoundingBox, Config};
use std::path::Path;
use std::time::Duration;

async fn fetch_tiles() {
    let config = Config {
        bounding_box: BoundingBox::new_deg(50.811, 6.1649, 50.7492, 6.031),
        fetch_rate: 10,
        output_folder: Path::new("./tiles"),
        request_retries_amount: 3,
        url: "https://{s}.tile.openstreetmap.de/{z}/{x}/{y}.png",
        timeout: Duration::new(30, 0),
        zoom_level: 10,
    };
    fetch(config).await.expect("failed fetching tiles");
}

fn main() {
    let mut rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(fetch_tiles());
}
```

License: MIT
