use clap::{
    app_from_crate, crate_authors, crate_description, crate_name, crate_version,
    AppSettings, Arg, ArgMatches,
};
use std::{path::PathBuf, time::Duration};

use crate::validators::*;
use osm_tile_downloader::{BoundingBox, Config, Fixture, UrlFormat};

const URL_ARG: &str = "url";
const ZOOM_ARG: &str = "zoom";
const OUTPUT_DIR_ARG: &str = "output_dir";
const BBOX_FIXTURE_ARG: &str = "fixture";
const BBOX_NORTH_ARG: &str = "north";
const BBOX_SOUTH_ARG: &str = "south";
const BBOX_WEST_ARG: &str = "west";
const BBOX_EAST_ARG: &str = "east";
const MIN_ZOOM_ARG: &str = "min_zoom";
const MAX_ZOOM_ARG: &str = "max_zoom";
const TIMEOUT_ARG: &str = "timeout";
const DRY_RUN_ARG: &str = "dry_run";
const REQUEST_RETRIES_ARG: &str = "num_retries";
const PARALLEL_FETCHES_ARG: &str = "num_parallel";
const FETCH_EXISTING_ARG: &str = "should_fetch_existing";

pub struct Args {
    pub bounding_box: BoundingBox,
    pub parallel_fetches: u8,
    pub retries: u8,
    pub timeout: Duration,
    pub min_zoom: u8,
    pub max_zoom: u8,
    pub output_dir: PathBuf,
    pub url: String,
    pub fetch_existing: bool,
    pub dry_run: bool,
}

impl std::convert::From<Args> for Config {
    fn from(args: Args) -> Self {
        Self {
            bounding_box: args.bounding_box,
            fetch_existing: args.fetch_existing,
            fetch_rate: args.parallel_fetches,
            output_folder: args.output_dir,
            request_retries_amount: args.retries,
            url: UrlFormat::from_str(args.url),
            timeout: args.timeout,
            min_zoom: args.min_zoom,
            max_zoom: args.max_zoom,
        }
    }
}

impl Args {
    pub fn parse() -> Self {
        let matches = get_matches();

        let (min_zoom, max_zoom) = match matches.value_of(ZOOM_ARG) {
            // if `zoom` is set, use it for both min/max
            Some(val) => {
                let zoom = val.parse().unwrap();
                (zoom, zoom)
            }
            // otherwise, parse min/max separately
            None => (
                matches.value_of(MIN_ZOOM_ARG).unwrap().parse().unwrap(),
                matches.value_of(MAX_ZOOM_ARG).unwrap().parse().unwrap(),
            ),
        };

        let bounding_box = match matches.value_of(BBOX_FIXTURE_ARG) {
            // if a fixture is specified, construct the bounding box from that
            Some(f) => BoundingBox::from_fixture(f.parse::<Fixture>().unwrap()),
            // otherwise, parse the 4 coords separately
            None => BoundingBox::new_deg(
                matches.value_of(BBOX_NORTH_ARG).unwrap().parse().unwrap(),
                matches.value_of(BBOX_EAST_ARG).unwrap().parse().unwrap(),
                matches.value_of(BBOX_SOUTH_ARG).unwrap().parse().unwrap(),
                matches.value_of(BBOX_WEST_ARG).unwrap().parse().unwrap(),
            ),
        };

        let output_dir = {
            let mut buf = PathBuf::new();
            buf.push(matches.value_of(OUTPUT_DIR_ARG).unwrap());
            buf
        };

        Self {
            min_zoom,
            max_zoom,
            bounding_box,
            output_dir,
            parallel_fetches: matches
                .value_of(PARALLEL_FETCHES_ARG)
                .unwrap()
                .parse()
                .unwrap(),
            retries: matches
                .value_of(REQUEST_RETRIES_ARG)
                .unwrap()
                .parse()
                .unwrap(),
            timeout: Duration::from_secs(
                matches.value_of(TIMEOUT_ARG).unwrap().parse().unwrap(),
            ),
            url: matches.value_of(URL_ARG).unwrap().to_owned(),
            fetch_existing: matches.is_present(FETCH_EXISTING_ARG),
            dry_run: matches.is_present(DRY_RUN_ARG),
        }
    }
}

fn get_matches() -> ArgMatches<'static> {
    app_from_crate!()
        .setting(AppSettings::GlobalVersion)
        .setting(AppSettings::VersionlessSubcommands)
        .arg(
            Arg::with_name(BBOX_NORTH_ARG)
                .help("Latitude of north bounding box boundary (in degrees)")
                .required_unless(BBOX_FIXTURE_ARG)
                .validator(is_geo_coord)
                .takes_value(true)
                .allow_hyphen_values(true)
                .short("n")
                .long("north"),
        )
        .arg(
            Arg::with_name(BBOX_SOUTH_ARG)
                .help("Latitude of south bounding box boundary (in degrees)")
                .required_unless(BBOX_FIXTURE_ARG)
                .validator(is_geo_coord)
                .takes_value(true)
                .allow_hyphen_values(true)
                .short("s")
                .long("south"),
        )
        .arg(
            Arg::with_name(BBOX_EAST_ARG)
                .help("Longitude of east bounding box boundary (in degrees)")
                .required_unless(BBOX_FIXTURE_ARG)
                .validator(is_geo_coord)
                .takes_value(true)
                .allow_hyphen_values(true)
                .short("e")
                .long("east"),
        )
        .arg(
            Arg::with_name(BBOX_WEST_ARG)
                .help("Longitude of west bounding box boundary (in degrees)")
                .required_unless(BBOX_FIXTURE_ARG)
                .validator(is_geo_coord)
                .takes_value(true)
                .allow_hyphen_values(true)
                .short("w")
                .long("west"),
        )
        .arg(
            Arg::with_name(BBOX_FIXTURE_ARG)
                .help("Use a known, named bounding box (eg. USA)")
                .validator(is_bb_fixture)
                .takes_value(true)
                .short("f")
                .long("fixture"),
        )
        .arg(
            Arg::with_name(PARALLEL_FETCHES_ARG)
                .help("The amount of tiles fetched in parallel.")
                .validator(is_numeric_min(1))
                .default_value("5")
                .takes_value(true)
                .short("r")
                .long("rate"),
        )
        .arg(
            Arg::with_name(REQUEST_RETRIES_ARG)
                .help("The amount of times to retry a failed HTTP request.")
                .validator(is_numeric_min(0))
                .default_value("3")
                .takes_value(true)
                .long("retries"),
        )
        .arg(
            Arg::with_name(TIMEOUT_ARG)
                .help("The timeout (in seconds) for fetching a single tile. Pass 0 for no timeout.")
                .validator(is_numeric_min(0))
                .default_value("10")
                .takes_value(true)
                .short("t")
                .long("timeout"),
        )
        .arg(
            Arg::with_name(MIN_ZOOM_ARG)
                .help("The minimum zoom level to fetch")
                .validator(is_numeric_min(1))
                .default_value("1")
                .takes_value(true)
                .long("min-zoom"),
        )
        .arg(
            Arg::with_name(MAX_ZOOM_ARG)
                .help("The maximum zoom level to fetch")
                .validator(is_numeric_min(1))
                .default_value("18")
                .takes_value(true)
                .long("max-zoom"),
        )
        .arg(
            Arg::with_name(ZOOM_ARG)
                .help("Only fetch a single zoom level (implies min=x/max=x)")
                .validator(is_numeric_min(1))
                .takes_value(true)
                .long("zoom")
                .short("z")
        )
        .arg(
            Arg::with_name(OUTPUT_DIR_ARG)
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
        .get_matches()
}
