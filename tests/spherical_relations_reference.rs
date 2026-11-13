#![cfg(feature = "geojson")]
use kartis::*;
use serde_json::Value;
use std::cell::OnceCell;
fn n(v: &Value) -> f64 {
    if let Some(n) = v.as_f64() {
        return n;
    }
    match v.as_str().unwrap() {
        "n:-0" => -0.,
        "n:NaN" => f64::NAN,
        "n:+Infinity" => f64::INFINITY,
        "n:-Infinity" => f64::NEG_INFINITY,
        _ => panic!(),
    }
}
fn pos(v: &Value) -> [f64; 2] {
    [n(&v[0]), n(&v[1])]
}
fn positions(v: &Value) -> Vec<[f64; 2]> {
    v.as_array().unwrap().iter().map(pos).collect()
}
enum Owned<'a> {
    Empty,
    Sphere,
    Point([f64; 2]),
    MultiPoint(Vec<[f64; 2]>),
    Line(Vec<[f64; 2]>),
    Lines(Vec<Vec<[f64; 2]>>),
    Polygon(Vec<Vec<[f64; 2]>>),
    Polygons(Vec<Vec<Vec<[f64; 2]>>>),
    Collection(Vec<Owned<'a>>, OnceCell<Vec<GeoSphericalGeometry<'a>>>),
}
impl<'a> Owned<'a> {
    fn parse(v: &Value) -> Self {
        let c = &v["coordinates"];
        match v["type"].as_str() {
            Some("Sphere") => Self::Sphere,
            Some("Point") => Self::Point(pos(c)),
            Some("MultiPoint") => Self::MultiPoint(positions(c)),
            Some("LineString") => Self::Line(positions(c)),
            Some("MultiLineString") => {
                Self::Lines(c.as_array().unwrap().iter().map(positions).collect())
            }
            Some("Polygon") => Self::Polygon(c.as_array().unwrap().iter().map(positions).collect()),
            Some("MultiPolygon") => Self::Polygons(
                c.as_array()
                    .unwrap()
                    .iter()
                    .map(|p| p.as_array().unwrap().iter().map(positions).collect())
                    .collect(),
            ),
            Some("GeometryCollection") => Self::Collection(
                v["geometries"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(Self::parse)
                    .collect(),
                OnceCell::new(),
            ),
            Some("Feature") => Self::parse(&v["geometry"]),
            Some("FeatureCollection") => Self::Collection(
                v["features"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(Self::parse)
                    .collect(),
                OnceCell::new(),
            ),
            _ => Self::Empty,
        }
    }
    fn borrowed(&'a self) -> GeoSphericalGeometry<'a> {
        match self {
            Self::Empty => GeoSphericalGeometry::Empty,
            Self::Sphere => GeoSphericalGeometry::Sphere,
            Self::Point(p) => GeoSphericalGeometry::Point(*p),
            Self::MultiPoint(p) => GeoSphericalGeometry::MultiPoint(p),
            Self::Line(p) => GeoSphericalGeometry::LineString(p),
            Self::Lines(p) => GeoSphericalGeometry::MultiLineString(p),
            Self::Polygon(p) => GeoSphericalGeometry::Polygon(p),
            Self::Polygons(p) => GeoSphericalGeometry::MultiPolygon(p),
            Self::Collection(parts, cache) => GeoSphericalGeometry::GeometryCollection(
                cache.get_or_init(|| parts.iter().map(Self::borrowed).collect()),
            ),
        }
    }
}
#[test]
fn original_centroid_with_directionless_and_checked_profiles() {
    let data: Value = serde_json::from_str(include_str!(
        "fixtures/geo-spherical-relations/geometry-centroid.json"
    ))
    .unwrap();
    let (mut values, mut none, mut rejected, mut max_error) = (0, 0, 0, 0_f64);
    for c in data["geometries"].as_array().unwrap() {
        let id = c["id"].as_str().unwrap();
        let owned = Owned::parse(&c["geometry"]);
        let actual = geo_spherical_centroid(owned.borrowed());
        if c["profile"].as_str().unwrap().starts_with("checked-error") {
            assert!(actual.is_err(), "{id} {actual:?}");
            rejected += 1;
            continue;
        }
        let expected = pos(&c["nativeCentroid"]);
        let actual = actual.unwrap_or_else(|e| panic!("{id}: {e}"));
        if !expected.iter().all(|x| x.is_finite()) {
            assert!(actual.is_none(), "{id} {actual:?}");
            none += 1;
        } else {
            let a = actual.unwrap_or_else(|| panic!("{id} missing {expected:?}"));
            for (a, b) in a.into_iter().zip(expected) {
                let error = (a - b).abs();
                max_error = max_error.max(error);
                assert!(error <= 1e-12, "{id}: {a} != {b}, error {error}");
            }
            values += 1;
        }
    }
    println!(
        "original centroid values={values}, None={none}, checkedErr={rejected}, max coordinate error degrees={max_error:e}"
    );
    assert_eq!((values, none, rejected), (31, 9, 11));
}
#[test]
fn original_contains_including_exact_and_epsilon_boundaries() {
    let data: Value = serde_json::from_str(include_str!(
        "fixtures/geo-spherical-relations/geometry-centroid.json"
    ))
    .unwrap();
    let queries: Value = serde_json::from_str(include_str!(
        "fixtures/geo-spherical-relations/contains.json"
    ))
    .unwrap();
    let (mut matched, mut rejected) = (0, 0);
    for q in queries.as_array().unwrap() {
        let id = q["id"].as_str().unwrap();
        let c = data["geometries"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == id)
            .unwrap();
        let owned = Owned::parse(&c["geometry"]);
        let actual = geo_spherical_contains(owned.borrowed(), pos(&q["point"]));
        if q["profile"].as_str().unwrap().starts_with("checked-error") {
            assert!(actual.is_err(), "{id} {:?}: {actual:?}", q["point"]);
            rejected += 1;
        } else {
            assert_eq!(
                actual.unwrap_or_else(|e| panic!("{id} {:?}: {e}", q["point"])),
                q["nativeContains"].as_bool().unwrap(),
                "{id} {:?}",
                q["point"]
            );
            matched += 1;
        }
    }
    println!("original contains matched={matched}, checkedErr={rejected}");
    assert_eq!((matched, rejected), (362, 105));
}
