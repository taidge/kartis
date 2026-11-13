#![cfg(feature = "geojson")]
use kartis::*;
use serde_json::Value;
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
enum Owned {
    Empty,
    Sphere,
    Point([f64; 2]),
    MultiPoint(Vec<[f64; 2]>),
    Line(Vec<[f64; 2]>),
    Lines(Vec<Vec<[f64; 2]>>),
    Polygon(Vec<Vec<[f64; 2]>>),
    Polygons(Vec<Vec<Vec<[f64; 2]>>>),
}
impl Owned {
    fn borrowed(&self) -> GeoSphericalGeometry<'_> {
        match self {
            Self::Empty => GeoSphericalGeometry::Empty,
            Self::Sphere => GeoSphericalGeometry::Sphere,
            Self::Point(p) => GeoSphericalGeometry::Point(*p),
            Self::MultiPoint(p) => GeoSphericalGeometry::MultiPoint(p),
            Self::Line(p) => GeoSphericalGeometry::LineString(p),
            Self::Lines(p) => GeoSphericalGeometry::MultiLineString(p),
            Self::Polygon(p) => GeoSphericalGeometry::Polygon(p),
            Self::Polygons(p) => GeoSphericalGeometry::MultiPolygon(p),
        }
    }
}
fn flatten(v: &Value, out: &mut Vec<Owned>) {
    let coordinates = &v["coordinates"];
    out.push(match v["type"].as_str() {
        Some("Sphere") => Owned::Sphere,
        Some("Point") => Owned::Point(pos(coordinates)),
        Some("MultiPoint") => Owned::MultiPoint(positions(coordinates)),
        Some("LineString") => Owned::Line(positions(coordinates)),
        Some("MultiLineString") => Owned::Lines(
            coordinates
                .as_array()
                .unwrap()
                .iter()
                .map(positions)
                .collect(),
        ),
        Some("Polygon") => Owned::Polygon(
            coordinates
                .as_array()
                .unwrap()
                .iter()
                .map(positions)
                .collect(),
        ),
        Some("MultiPolygon") => Owned::Polygons(
            coordinates
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_array().unwrap().iter().map(positions).collect())
                .collect(),
        ),
        Some("Feature") => {
            flatten(&v["geometry"], out);
            return;
        }
        Some("FeatureCollection") => {
            for child in v["features"].as_array().unwrap() {
                flatten(child, out);
            }
            return;
        }
        Some("GeometryCollection") => {
            for child in v["geometries"].as_array().unwrap() {
                flatten(child, out);
            }
            return;
        }
        _ => Owned::Empty,
    });
}
#[test]
fn original_geometries_length_area_and_explicit_rejections() {
    let data: Value =
        serde_json::from_str(include_str!("fixtures/geo-spherical/geometry.json")).unwrap();
    let mut accepted = 0;
    let mut rejected = 0;
    for case in data["geometries"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        let mut owned = Vec::new();
        flatten(&case["geometry"], &mut owned);
        let borrowed: Vec<_> = owned.iter().map(Owned::borrowed).collect();
        let g = GeoSphericalGeometry::GeometryCollection(&borrowed);
        let (a, l) = (geo_spherical_area(g), geo_spherical_length(g));
        if [
            "polygon-unclosed-native",
            "polygon-empty-ring",
            "line-nan",
            "line-infinity",
        ]
        .contains(&id)
        {
            assert!(a.is_err() && l.is_err(), "{id}");
            rejected += 1;
            continue;
        }
        let a = a.unwrap_or_else(|e| panic!("{id}: {e}"));
        let l = l.unwrap_or_else(|e| panic!("{id}: {e}"));
        assert!(
            (a - n(&case["area"])).abs() <= 1e-12,
            "area {id} {a} !={}",
            case["area"]
        );
        assert!(
            (l - n(&case["length"])).abs() <= 1e-12,
            "length {id} {l} !={}",
            case["length"]
        );
        accepted += 1;
    }
    assert_eq!((accepted, rejected), (33, 4));
}
#[test]
fn original_distance_queries() {
    let data: Value =
        serde_json::from_str(include_str!("fixtures/geo-spherical/distance.json")).unwrap();
    let mut count = 0;
    for case in data.as_array().unwrap() {
        let (a, b) = (pos(&case["a"]), pos(&case["b"]));
        let result = geo_spherical_distance(a, b);
        if a.iter().chain(b.iter()).any(|v| !v.is_finite())
            || a[0].abs() > 180.
            || b[0].abs() > 180.
            || a[1].abs() > 90.
            || b[1].abs() > 90.
        {
            assert!(result.is_err());
            continue;
        }
        let value = result.unwrap();
        assert!(
            (value - n(&case["native"])).abs() <= 1e-12,
            "{} {value}",
            case["id"]
        );
        count += 1;
    }
    eprintln!("accepted original distance controls={count}");
}
#[test]
fn original_circle_samples_closure_and_checked_policy() {
    let data: Value =
        serde_json::from_str(include_str!("fixtures/geo-spherical/circle.json")).unwrap();
    let (mut accepted, mut errors, mut skipped, mut max_error) = (0, 0, 0, 0f64);
    for case in data.as_array().unwrap() {
        if !case["status"]
            .as_str()
            .unwrap()
            .starts_with("executed-native")
        {
            skipped += 1;
            continue;
        }
        let center = pos(&case["center"]);
        let radius = n(&case["radius"]);
        let precision = n(&case["precision"]);
        let result = geo_circle(&GeoCircleOptions {
            center,
            radius_degrees: radius,
            precision_degrees: precision,
            ..Default::default()
        });
        if center.iter().any(|v| !v.is_finite())
            || center[0].abs() > 180.
            || center[1].abs() > 90.
            || !radius.is_finite()
            || !(0. ..=180.).contains(&radius)
            || !precision.is_finite()
            || precision <= 0.
            || precision > 90.
        {
            assert!(result.is_err(), "{}", case["id"]);
            errors += 1;
            continue;
        }
        let actual = result.unwrap();
        let expected = positions(&case["geometry"]["coordinates"][0]);
        assert!(
            actual.len() == expected.len() || actual.len() == expected.len() + 1,
            "{} native{} rust{}",
            case["id"],
            expected.len(),
            actual.len()
        );
        assert_eq!(
            actual.first().unwrap().map(f64::to_bits),
            actual.last().unwrap().map(f64::to_bits)
        );
        for (a, b) in actual.iter().zip(&expected) {
            let vector = |p: [f64; 2]| {
                let [l, f] = p.map(f64::to_radians);
                [f.cos() * l.cos(), f.cos() * l.sin(), f.sin()]
            };
            let (a, b) = (vector(*a), vector(*b));
            let error = (a[0] - b[0]).hypot(a[1] - b[1]).hypot(a[2] - b[2]);
            max_error = max_error.max(error);
            assert!(error <= 1e-12, "circle {} vectorerror{error:e}", case["id"]);
        }
        accepted += 1;
    }
    eprintln!(
        "circle accepted={accepted} errors={errors} skipped={skipped} max_vector_error={max_error:e}"
    );
}
