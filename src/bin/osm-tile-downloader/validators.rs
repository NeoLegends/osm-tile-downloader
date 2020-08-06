use osm_tile_downloader::Fixture;

pub fn is_numeric_min(min: usize) -> impl Fn(String) -> Result<(), String> {
    move |v: String| {
        let val = v
            .parse::<usize>()
            .map_err(|_| "must be numeric".to_owned())?;

        if val < min {
            return Err("must be > 0".to_owned());
        }

        Ok(())
    }
}

pub fn is_geo_coord(v: String) -> Result<(), String> {
    let val = v.parse::<f64>().map_err(|_| "must be numeric".to_owned())?;

    if val < -180f64 {
        return Err("must be >= -180°".to_owned());
    } else if val > 180f64 {
        return Err("must be <= 180°".to_owned());
    }

    Ok(())
}

pub fn is_bb_fixture(v: String) -> Result<(), String> {
    v.parse::<Fixture>()
        .map(|_| ())
        .map_err(|_| "invalid fixture".to_owned())
}
