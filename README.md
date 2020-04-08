# osm-tile-downloader

A tool that can download OSM tiles to your local disk en-masse.

**Use with absolute caution.** Downloading tiles en-masse can hog down a tile server easily. I am not responsible for any damage this tool may cause.

## Usage

This tool is available on [crates.io](https://crates.io) and can be installed via `cargo install osm-tile-downloader`.

It features a helpful CLI you can access via `-h` / `--help`.

### Example

```
osm-tile-downloader \
  --north 50.811 \
  --east 6.1649 \
  --south 50.7492 \
  --west 6.031 \
  --url https://a.tile.openstreetmap.se/hydda/full/\{z\}/\{x\}/\{y\}.png \
  --output ./tiles \
  --rate 10
```
