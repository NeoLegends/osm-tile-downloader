use std::f64::consts::PI;
use std::fmt::Debug;

use crate::tile::Tile;

/// A bounding box consisting of north, east, south and west coordinate boundaries
/// given from 0 to 2π.
///
/// # Example
/// ```rust
/// # use osm_tile_downloader::BoundingBox;
/// let aachen_germany = BoundingBox::new_deg(50.811, 6.1649, 50.7492, 6.031);
/// ```
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct BoundingBox {
    pub north: f64,
    pub west: f64,
    pub east: f64,
    pub south: f64,
}

impl BoundingBox {
    /// Create a new bounding box from the specified coordinates specified in degrees
    /// (-180(E) to 180(W)° latitude, -85(S) to 85(N)° longitude).
    ///
    /// # Example
    /// ```rust
    /// # use osm_tile_downloader::BoundingBox;
    /// let aachen_germany = BoundingBox::new_deg(50.811, 6.1649, 50.7492, 6.031);
    /// ```
    ///
    /// # Panics
    /// Panics if the coordinates aren't in the closed range [-180, 180].
    pub fn new_deg(north: f64, east: f64, south: f64, west: f64) -> Self {
        Self::new(
            north.to_radians(),
            east.to_radians(),
            south.to_radians(),
            west.to_radians(),
        )
    }

    /// Create a new bounding box from the specified coordinates specified in radians (0-2π).
    ///
    /// # Panics
    /// Panics if the coordinates aren't in the closed range [-π, π].
    pub fn new(north: f64, east: f64, south: f64, west: f64) -> Self {
        assert!(north >= -1f64 * PI && north <= PI);
        assert!(east >= -1f64 * PI && east <= PI);
        assert!(south >= -1f64 * PI && south <= PI);
        assert!(west >= -1f64 * PI && west <= PI);

        BoundingBox {
            north,
            east,
            south,
            west,
        }
    }

    /// Creates an iterator iterating over all tiles in the bounding box.
    ///
    /// # Panics
    /// Panics if `min_zoom` or `max_zoom` are invalid.
    pub fn tiles(
        &self,
        min_zoom: u8,
        max_zoom: u8,
    ) -> impl Iterator<Item = Tile> + Debug {
        assert!(min_zoom >= 1);
        assert!(max_zoom >= 1);
        assert!(min_zoom <= max_zoom);

        let (n, e, s, w) = (self.north, self.east, self.south, self.west);

        (min_zoom..=max_zoom).flat_map(move |zoom| {
            let nw = Tile::from_coords_and_zoom(n, w, zoom);
            let se = Tile::from_coords_and_zoom(s, e, zoom);

            ((nw.x)..=(se.x)).flat_map(move |x| {
                ((nw.y)..=(se.y)).map(move |y| Tile::new(x, y, zoom))
            })
        })
    }
}

/// A bounding box fixture containing preset coordinates for a known geographic
/// region (a continent, country, city, etc).
pub enum Fixture {
    USA,
    AachenGermany,
}

impl std::str::FromStr for Fixture {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use Fixture::*;

        if s.to_lowercase().starts_with("us") {
            return Ok(USA);
        }

        if s.to_lowercase().starts_with("aachen") {
            return Ok(AachenGermany);
        }

        Err("unrecognized fixture")
    }
}

impl std::convert::From<Fixture> for BoundingBox {
    fn from(fixture: Fixture) -> Self {
        use Fixture::*;

        match fixture {
            USA => Self::new_deg(49.4325, -65.7421, 23.8991, -125.3321),
            AachenGermany => Self::new_deg(50.811, 6.1649, 50.7492, 6.031),
        }
    }
}
