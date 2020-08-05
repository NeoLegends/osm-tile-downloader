mod validators;

use anyhow::Result;
use clap::{
    app_from_crate, crate_authors, crate_description, crate_name, crate_version,
    AppSettings, Arg,
};
use std::{path::Path, time::Duration};

use osm_tile_downloader::*;
use validators::*;

const BBOX_NORTH_ARG: &str = "BBOX_NORTH";
const BBOX_SOUTH_ARG: &str = "BBOX_SOUTH";
const BBOX_WEST_ARG: &str = "BBOX_WEST";
const BBOX_EAST_ARG: &str = "BBOX_EAST";
const OUTPUT_ARG: &str = "OUTPUT";
const PARALLEL_FETCHES_ARG: &str = "PARALLEL_FETCHES";
const REQUEST_RETRIES_ARG: &str = "REQUEST_RETRIES";
const ZOOM_ARG: &str = "ZOOM";
const MIN_ZOOM_ARG: &str = "MIN_ZOOM";
const MAX_ZOOM_ARG: &str = "MAX_ZOOM";
const URL_ARG: &str = "URL";
const TIMEOUT_ARG: &str = "TIMEOUT";
const FETCH_EXISTING_ARG: &str = "FETCH_EXISTING";
const DRY_RUN_ARG: &str = "DRY_RUN";

#[tokio::main]
async fn main() -> Result<()> {
    let matches = app_from_crate!()
        .setting(AppSettings::GlobalVersion)
        .setting(AppSettings::VersionlessSubcommands)
        .arg(
            Arg::with_name(BBOX_NORTH_ARG)
                .help("Latitude of north bounding box boundary (in degrees)")
                .validator(is_geo_coord)
                .required(true)
                .takes_value(true)
                .allow_hyphen_values(true)
                .short("n")
                .long("north"),
        )
        .arg(
            Arg::with_name(BBOX_SOUTH_ARG)
                .help("Latitude of south bounding box boundary (in degrees)")
                .validator(is_geo_coord)
                .required(true)
                .takes_value(true)
                .allow_hyphen_values(true)
                .short("s")
                .long("south"),
        )
        .arg(
            Arg::with_name(BBOX_EAST_ARG)
                .help("Longitude of east bounding box boundary (in degrees)")
                .validator(is_geo_coord)
                .required(true)
                .takes_value(true)
                .allow_hyphen_values(true)
                .short("e")
                .long("east"),
        )
        .arg(
            Arg::with_name(BBOX_WEST_ARG)
                .help("Longitude of west bounding box boundary (in degrees)")
                .validator(is_geo_coord)
                .required(true)
                .takes_value(true)
                .allow_hyphen_values(true)
                .short("w")
                .long("west"),
        )
        .arg(
            Arg::with_name(PARALLEL_FETCHES_ARG)
                .help("The amount of tiles fetched in parallel.")
                .validator(is_positive_u8)
                .default_value("5")
                .takes_value(true)
                .short("r")
                .long("rate"),
        )
        .arg(
            Arg::with_name(REQUEST_RETRIES_ARG)
                .help("The amount of times to retry a failed HTTP request.")
                .validator(is_positive_u8)
                .default_value("3")
                .takes_value(true)
                .long("retries"),
        )
        .arg(
            Arg::with_name(TIMEOUT_ARG)
                .help("The timeout (in seconds) for fetching a single tile. Pass 0 for no timeout.")
                .validator(is_numeric::<u64>)
                .default_value("10")
                .takes_value(true)
                .short("t")
                .long("timeout"),
        )
        .arg(
            Arg::with_name(MIN_ZOOM_ARG)
                .help("The minimum zoom level to fetch")
                .validator(is_positive_u8)
                .default_value("1")
                .takes_value(true)
                .long("min-zoom"),
        )
        .arg(
            Arg::with_name(MAX_ZOOM_ARG)
                .help("The maximum zoom level to fetch")
                .validator(is_positive_u8)
                .default_value("18")
                .takes_value(true)
                .long("max-zoom"),
        )
        .arg(
            Arg::with_name(ZOOM_ARG)
                .help("Only fetch a single zoom level (implies min=x/max=x)")
                .validator(is_positive_u8)
                .takes_value(true)
                .long("zoom")
                .short("z")
        )
        .arg(
            Arg::with_name(OUTPUT_ARG)
                .help("The folder to output the tiles to. May contain format specifiers (and subfolders) to specify how the files will be laid out on disk.")
                .default_value("output")
                .takes_value(true)
                .short("o")
                .long("output"),
        )
        .arg(
            Arg::with_name(URL_ARG)
                .help("The URL with format specifiers `{x}`, `{y}`, `{z}` to fetch the tiles from. Also supports the format specifier `{s}` which is replaced with `a`, `b` or `c` randomly to spread the load between different servers.")
                .required(true)
                .takes_value(true)
                .short("u")
                .long("url")
        )
        .arg(
            Arg::with_name(FETCH_EXISTING_ARG)
                .help("Fetch tiles that we've already downloaded (this usually isn't required)")
                .required(false)
                .takes_value(false)
                .long("fetch-existing")
        )
        .arg(
            Arg::with_name(DRY_RUN_ARG)
                .help("Don't actually fetch anything, just determine how many tiles would be fetched.")
                .required(false)
                .takes_value(false)
                .long("dry-run")
        )
        .get_matches();

    let (min_zoom, max_zoom) = match matches.value_of(ZOOM_ARG) {
        Some(val) => {
            let zoom = val.parse().unwrap();
            (zoom, zoom)
        }
        None => (
            matches.value_of(MIN_ZOOM_ARG).unwrap().parse().unwrap(),
            matches.value_of(MAX_ZOOM_ARG).unwrap().parse().unwrap(),
        ),
    };

    let bounding_box = BoundingBox::new_deg(
        matches.value_of(BBOX_NORTH_ARG).unwrap().parse().unwrap(),
        matches.value_of(BBOX_EAST_ARG).unwrap().parse().unwrap(),
        matches.value_of(BBOX_SOUTH_ARG).unwrap().parse().unwrap(),
        matches.value_of(BBOX_WEST_ARG).unwrap().parse().unwrap(),
    );

    let dry_run = matches.is_present(DRY_RUN_ARG);

    if dry_run {
        let tile_count = bounding_box.tiles(min_zoom, max_zoom).count();
        eprintln!(
            "would download {} tiles (approx {}, assuming 10 kb per tile)",
            tile_count,
            pretty_bytes::converter::convert((tile_count as f64) * 10_000f64)
        );
    } else {
        let config = Config {
            min_zoom,
            max_zoom,
            bounding_box,
            fetch_existing: matches.is_present(FETCH_EXISTING_ARG),
            fetch_rate: matches
                .value_of(PARALLEL_FETCHES_ARG)
                .unwrap()
                .parse()
                .unwrap(),
            output_folder: Path::new(matches.value_of(OUTPUT_ARG).unwrap()),
            request_retries_amount: matches
                .value_of(REQUEST_RETRIES_ARG)
                .unwrap()
                .parse()
                .unwrap(),
            url: matches.value_of(URL_ARG).unwrap(),
            timeout: Duration::from_secs(
                matches.value_of(TIMEOUT_ARG).unwrap().parse().unwrap(),
            ),
        };

        fetch(config).await?;
    }

    Ok(())
}
