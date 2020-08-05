use std::{f64, str::FromStr};

pub(crate) fn is_numeric<T: FromStr>(v: String) -> Result<(), String> {
    v.parse::<T>()
        .map(|_| ())
        .map_err(|_| "must be numeric".to_owned())
}

pub(crate) fn is_positive_u8(v: String) -> Result<(), String> {
    let val = v.parse::<u8>().map_err(|_| "must be numeric".to_owned())?;
    if val > 0 {
        Ok(())
    } else {
        Err("must be > 0".to_owned())
    }
}

pub(crate) fn is_geo_coord(v: String) -> Result<(), String> {
    let val = v.parse::<f64>().map_err(|_| "must be numeric".to_owned())?;

    if val < -180f64 {
        return Err("must be >= -180°".to_owned());
    } else if val >= 180f64 {
        return Err("must be < 180°".to_owned());
    }

    Ok(())
}

pub(crate) fn is_bb_fixture(v: String) -> Result<(), String> {
    if v.to_lowercase().starts_with("us") {
        return Ok(());
    }

    Err("US is the only supported fixture".to_owned())
}
