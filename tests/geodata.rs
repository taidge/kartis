#![cfg(feature = "geojson")]
use kartis::geodata::*;
use kartis::*;
const GEO: &str = r#"{"type":"FeatureCollection","features":[{"type":"Feature","id":"native","properties":{"name":"Area","code":42,"population":null},"geometry":{"type":"Polygon","coordinates":[[[0,0],[10,0],[10,10],[0,10],[0,0]],[[2,2],[2,4],[4,4],[4,2],[2,2]]]}}]}"#;
const TOPO: &str = r#"{"type":"Topology","transform":{"scale":[0.1,0.1],"translate":[-10,20]},"arcs":[[[0,0],[10,0],[0,10]],[[0,0],[0,10],[10,0]]],"objects":{"land":{"type":"GeometryCollection","geometries":[{"type":"Polygon","id":"square","properties":{"name":"Topology area","value":7},"arcs":[[0,-2]]}]}}}"#;
#[test]
fn geojson_metadata_polygons_holes_and_choropleth() {
    let o = GeoParseOptions {
        id_property: Some("code".into()),
        value_property: Some("population".into()),
        missing_value: Some(8.),
        ..Default::default()
    };
    let f = parse_geojson(GEO, &o).unwrap();
    assert_eq!(f.len(), 1);
    assert_eq!(f[0].id, "42");
    assert_eq!(f[0].label, "Area");
    assert_eq!(f[0].value, Some(8.));
    assert_eq!(f[0].polygons[0].len(), 2);
    choropleth(&ChartConfig::default(), &f, Projection::Equirectangular).unwrap();
    let multi = r#"{"type":"Feature","id":1,"properties":{},"geometry":{"type":"MultiPolygon","coordinates":[[[[0,0],[1,0],[1,1],[0,0]]],[[[10,0],[11,0],[11,1],[10,0]]]]}}"#;
    assert_eq!(
        parse_geojson(multi, &GeoParseOptions::default()).unwrap()[0]
            .polygons
            .len(),
        2
    );
}
#[test]
fn topology_delta_reset_negative_reverse_and_stitching() {
    let f = parse_topojson(
        TOPO,
        "land",
        &GeoParseOptions {
            value_property: Some("value".into()),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        f[0].polygons[0][0],
        vec![
            [-10., 20.],
            [-9., 20.],
            [-9., 21.],
            [-10., 21.],
            [-10., 20.]
        ]
    );
    assert_eq!(f[0].value, Some(7.));
    let absolute = r#"{"type":"Topology","arcs":[[[0,0],[1,0],[1,1],[0,0]]],"objects":{"shape":{"type":"Polygon","id":"p","arcs":[[-1]]}}}"#;
    let f = parse_topojson(absolute, "shape", &GeoParseOptions::default()).unwrap();
    assert_eq!(
        f[0].polygons[0][0],
        vec![[0., 0.], [1., 1.], [1., 0.], [0., 0.]]
    );
    assert!(
        parse_topojson(
            &TOPO.replace("[0,-2]", "[0,1]"),
            "land",
            &GeoParseOptions::default()
        )
        .is_err()
    );
    assert!(
        parse_topojson(
            &TOPO.replace("[0,-2]", "[0,99]"),
            "land",
            &GeoParseOptions::default()
        )
        .is_err()
    );
    assert!(parse_topojson(TOPO, "missing", &GeoParseOptions::default()).is_err());
}
#[test]
fn budgets_malformed_rings_coordinates_and_values_reject() {
    assert!(
        parse_geojson(
            GEO,
            &GeoParseOptions {
                max_points: 4,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        parse_geojson(
            GEO,
            &GeoParseOptions {
                max_features: 0,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        parse_geojson(
            &GEO.replace("[10,10]", "[190,10]"),
            &GeoParseOptions::default()
        )
        .is_err()
    );
    assert!(
        parse_geojson(
            &GEO.replace("[0,10],[0,0]", "[0,10],[1,1]"),
            &GeoParseOptions::default()
        )
        .is_err()
    );
    assert!(
        parse_geojson(
            &GEO.replace("\"population\":null", "\"population\":\"8\""),
            &GeoParseOptions {
                value_property: Some("population".into()),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        parse_topojson(
            &TOPO.replace("[10,0]", "[10.5,0]"),
            "land",
            &GeoParseOptions::default()
        )
        .is_err()
    );
    assert!(
        parse_topojson(
            TOPO,
            "land",
            &GeoParseOptions {
                max_points: 4,
                ..Default::default()
            }
        )
        .is_err()
    );
}

#[test]
fn topology_multipolygon_and_duplicate_ids_are_checked() {
    let topology = r#"{"type":"Topology","arcs":[[[0,0],[1,0],[1,1],[0,0]],[[10,0],[11,0],[11,1],[10,0]]],"objects":{"islands":{"type":"MultiPolygon","id":"islands","arcs":[[[0]],[[1]]]}}}"#;
    let parsed = parse_topojson(topology, "islands", &GeoParseOptions::default()).unwrap();
    assert_eq!(parsed[0].polygons.len(), 2);
    let feature = r#"{"type":"Feature","id":"one","properties":{},"geometry":{"type":"Polygon","coordinates":[[[0,0],[1,0],[1,1],[0,0]]]}}"#;
    let duplicate =
        format!("{{\"type\":\"FeatureCollection\",\"features\":[{feature},{feature}]}}");
    assert!(parse_geojson(&duplicate, &GeoParseOptions::default()).is_err());
    let signed_zero = r#"{"type":"Feature","id":"bad","geometry":{"type":"Polygon","coordinates":[[[0,0],[-0.0,0],[1,1],[0,0]]]}}"#;
    assert!(
        parse_geojson(signed_zero, &GeoParseOptions::default()).is_err(),
        "signed zero must not create a distinct polygon vertex"
    );
}
