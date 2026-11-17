//! Original d3-geo 3.1.1 multiplexed streams, captured by the companion producer.
//! ISC source/producer hashes and license notice are recorded in reference.json.
//! Samplers differ: compare fit, material winding, area, bounds and actual Scene
//! coverage, never sampled vertex counts or vertex order. All tolerances and the
//! counted .05-pixel boundary exclusion were frozen before the first Rust run.
#![cfg(feature = "geojson")]

use glory::render::PathCommand;
use glory::style::Color;
use kartis::*;
use serde_json::Value;

fn n(v: &Value) -> f64 {
    v.as_f64().unwrap()
}
fn a(v: &Value) -> &[Value] {
    v.as_array().unwrap()
}
fn p(v: &Value) -> [f64; 2] {
    [n(&v[0]), n(&v[1])]
}
fn close(actual: f64, expected: f64, limit: f64, context: &str) {
    assert!(
        (actual - expected).abs() <= limit,
        "{context}: {actual:.15} != {expected:.15}; error {} > {limit}",
        (actual - expected).abs()
    );
}
fn source(case: &Value) -> Vec<GeoFeature> {
    a(&case["features"])
        .iter()
        .map(|f| GeoFeature {
            id: f["id"].as_str().unwrap().into(),
            label: f["label"].as_str().unwrap().into(),
            value: f["value"].as_f64(),
            polygons: a(&f["polygons"])
                .iter()
                .map(|polygon| {
                    a(polygon)
                        .iter()
                        .map(|ring| a(ring).iter().map(p).collect())
                        .collect()
                })
                .collect(),
        })
        .collect()
}
fn config(case: &Value) -> ChartConfig {
    let c = &case["config"];
    let m = &c["margin"];
    ChartConfig {
        width: n(&c["width"]),
        height: n(&c["height"]),
        margin: Margin {
            left: n(&m["left"]),
            right: n(&m["right"]),
            top: n(&m["top"]),
            bottom: n(&m["bottom"]),
        },
        ..Default::default()
    }
}
fn options(case: &Value) -> GeoUsaMapOptions {
    let o = &case["options"];
    GeoUsaMapOptions {
        fit: if o["fit"] == "Data" {
            GeoFit::Data
        } else {
            GeoFit::World
        },
        padding: n(&o["padding"]),
        scale: o["scale"].as_f64(),
        translate: (!o["translate"].is_null()).then(|| p(&o["translate"])),
        border: None,
        fill: Some(Color(80, 120, 180, 255)),
        path: GeoPathOptions {
            precision: n(&o["precision"]),
            ..Default::default()
        },
        ..Default::default()
    }
}
fn contours(
    mark: &Mark,
    actual_k: f64,
    actual_t: [f64; 2],
    native_k: f64,
    native_t: [f64; 2],
) -> Vec<Vec<[f64; 2]>> {
    let Geometry::Path(commands) = &mark.geometry else {
        panic!("map geometry is not a path")
    };
    let mut rings = vec![];
    let mut current = vec![];
    for command in commands {
        match command {
            PathCommand::Move(q) => {
                assert!(current.is_empty());
                current.push([
                    native_t[0] + (q.x - actual_t[0]) * native_k / actual_k,
                    native_t[1] + (q.y - actual_t[1]) * native_k / actual_k,
                ]);
            }
            PathCommand::Line(q) => current.push([
                native_t[0] + (q.x - actual_t[0]) * native_k / actual_k,
                native_t[1] + (q.y - actual_t[1]) * native_k / actual_k,
            ]),
            PathCommand::Close => rings.push(std::mem::take(&mut current)),
            _ => panic!("unexpected curved map command"),
        }
    }
    assert!(current.is_empty());
    rings
}
fn area(ring: &[[f64; 2]]) -> f64 {
    ring.iter()
        .zip(ring.iter().cycle().skip(1))
        .take(ring.len())
        .map(|(p, q)| p[0] * q[1] - q[0] * p[1])
        .sum::<f64>()
        / 2.
}
fn perimeter(rings: &[Vec<[f64; 2]>]) -> f64 {
    rings
        .iter()
        .map(|r| {
            r.iter()
                .zip(r.iter().cycle().skip(1))
                .take(r.len())
                .map(|(p, q)| (p[0] - q[0]).hypot(p[1] - q[1]))
                .sum::<f64>()
        })
        .sum()
}
fn bounds(rings: &[Vec<[f64; 2]>]) -> [f64; 4] {
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for p in rings.iter().flatten() {
        b[0] = b[0].min(p[0]);
        b[1] = b[1].min(p[1]);
        b[2] = b[2].max(p[0]);
        b[3] = b[3].max(p[1]);
    }
    b
}

#[test]
fn original_composite_stream_fit_winding_bounds_area_and_scene_coverage() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/geo-usa-map/values.json")).unwrap();
    let cases = a(&fixture["cases"]);
    assert_eq!(cases.len(), 24);
    let (mut states, mut included, mut excluded, mut point_queries) = (0, 0, 0, 0);
    let (mut max_scale_relative, mut max_center, mut max_bounds) = (0_f64, 0_f64, 0_f64);
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let features = source(case);
        let snapshot = features.clone();
        let c = config(case);
        let o = options(case);
        let layout =
            geographic_usa_map_layout(&c, &features, &o).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(features, snapshot, "{name}: mutated source");
        let ctx = &layout.projection;
        let core = ctx.usa_projection().unwrap();
        let (ak, at) = (core.scale(), core.translate());
        let native = &case["native"];
        let (nk, nt) = (n(&native["scale"]), p(&native["translate"]));
        let allow = &case["allowances"];
        let raw_limit = n(&allow["rawCenter"]);
        let relative = (ak / nk - 1.).abs();
        max_scale_relative = max_scale_relative.max(relative);
        if o.fit == GeoFit::World {
            close(ak, nk, 1e-8, name);
        } else {
            assert!(
                relative <= n(&allow["fitRelativeLimit"]),
                "{name}: scale relative {relative} exceeds {}",
                allow["fitRelativeLimit"]
            );
        }
        let center = ctx.fitted_raw_center();
        let nc = p(&native["rawCenter"]);
        let fit_center = p(&native["fitCenter"]);
        for i in 0..2 {
            max_center = max_center.max((center[i] - nc[i]).abs());
            close(center[i], nc[i], raw_limit, name);
            close(ctx.translate()[i], fit_center[i], 1e-8, name);
            let expected = if i == 0 {
                fit_center[i] - center[i] * ak
            } else {
                fit_center[i] + center[i] * ak
            };
            close(at[i], expected, 1e-7, name);
            close(
                at[i],
                nt[i],
                ak * raw_limit + (ak - nk).abs() * nc[i].abs() + 1e-7,
                name,
            );
        }
        let mut visible = 0;
        for (src, expected) in features.iter().zip(a(&native["features"])) {
            states += 1;
            assert_eq!(src.id, expected["id"].as_str().unwrap());
            let mark = layout.scene.marks.iter().find(|m| m.series == src.id);
            if expected["bounds"].is_null() {
                assert!(mark.is_none(), "{name}: {} should be invisible", src.id);
                let empty = c.scene().unwrap();
                let mut local_included = 0;
                let mut local_excluded = 0;
                for query in a(&expected["coverage"]) {
                    if query["excluded"].as_bool().unwrap() {
                        local_excluded += 1;
                        continue;
                    }
                    local_included += 1;
                    let q = p(&query["position"]);
                    let actual = [
                        at[0] + (q[0] - nt[0]) * ak / nk,
                        at[1] + (q[1] - nt[1]) * ak / nk,
                    ];
                    assert_eq!(
                        empty.hit_test(actual[0], actual[1]).is_some(),
                        query["inside"].as_bool().unwrap(),
                        "{name}/{} invisible coverage {q:?}",
                        src.id
                    );
                }
                assert_eq!(
                    local_included,
                    expected["coverageCounts"]["included"].as_u64().unwrap() as usize
                );
                assert_eq!(
                    local_excluded,
                    expected["coverageCounts"]["excluded"].as_u64().unwrap() as usize
                );
                included += local_included;
                excluded += local_excluded;
                continue;
            }
            visible += 1;
            let mark = mark.unwrap_or_else(|| panic!("{name}: missing {}", src.id));
            assert_eq!(mark.id, format!("map:{}", src.id));
            assert_eq!(mark.label, src.label);
            assert_eq!(mark.value, src.value.unwrap_or(0.));
            assert_eq!(mark.stroke, None);
            let rings = contours(mark, ak, at, nk, nt);
            let area_limit = n(&allow["materialContourArea"]);
            let positive = rings.iter().filter(|r| area(r) > area_limit).count();
            let negative = rings.iter().filter(|r| area(r) < -area_limit).count();
            assert_eq!(
                positive,
                expected["windingCounts"]["positive"].as_u64().unwrap() as usize,
                "{name}: {} positive winding",
                src.id
            );
            assert_eq!(
                negative,
                expected["windingCounts"]["negative"].as_u64().unwrap() as usize,
                "{name}: {} negative winding",
                src.id
            );
            let geometry_limit =
                n(&allow["geometryPrecision"]) * (1. + nk / ak) + n(&allow["coordinateRoundoff"]);
            for (actual, want) in bounds(&rings).into_iter().zip(a(&expected["bounds"])) {
                max_bounds = max_bounds.max((actual - n(want)).abs());
                close(
                    actual,
                    n(want),
                    geometry_limit,
                    &format!("{name}/{} bounds", src.id),
                );
            }
            let area_actual = rings.iter().map(|r| area(r)).sum::<f64>();
            let area_tolerance = (n(&expected["perimeter"]) + perimeter(&rings)) * geometry_limit
                + 4. * rings.len() as f64 * geometry_limit.powi(2);
            close(
                area_actual,
                n(&expected["signedArea"]),
                area_tolerance,
                &format!("{name}/{} signed area", src.id),
            );
            let mut isolated = c.scene().unwrap();
            isolated.marks.push(mark.clone());
            let (mut local_included, mut local_excluded) = (0, 0);
            for query in a(&expected["coverage"]) {
                if query["excluded"].as_bool().unwrap() {
                    local_excluded += 1;
                    continue;
                }
                local_included += 1;
                let q = p(&query["position"]);
                let actual = [
                    at[0] + (q[0] - nt[0]) * ak / nk,
                    at[1] + (q[1] - nt[1]) * ak / nk,
                ];
                assert_eq!(
                    isolated.hit_test(actual[0], actual[1]).is_some(),
                    query["inside"].as_bool().unwrap(),
                    "{name}/{} coverage native={q:?}, actual={actual:?}, distance={}",
                    src.id,
                    query["distance"]
                );
            }
            assert_eq!(
                local_included,
                expected["coverageCounts"]["included"].as_u64().unwrap() as usize
            );
            assert_eq!(
                local_excluded,
                expected["coverageCounts"]["excluded"].as_u64().unwrap() as usize
            );
            included += local_included;
            excluded += local_excluded;
        }
        assert_eq!(
            layout.scene.marks.len(),
            visible,
            "{name}: one canonical mark per visible source"
        );
        for query in a(&native["points"]) {
            point_queries += 1;
            let actual = ctx.project_usa(p(&query["input"])).unwrap();
            if query["output"].is_null() {
                assert!(actual.is_none(), "{name}: unexpectedly visible point");
            } else {
                let actual = actual.unwrap().position;
                let want = p(&query["output"]);
                for i in 0..2 {
                    close(nt[i] + (actual[i] - at[i]) * nk / ak, want[i], 1e-7, name);
                }
            }
        }
    }
    assert_eq!(
        (states, included, excluded, point_queries),
        (46, 32890, 22, 192)
    );
    eprintln!(
        "D3 original composite: 24 cases, {states} states, {included} coverage included, {excluded} fixed-band excluded, {point_queries} point queries; max relative scale {max_scale_relative}, raw center {max_center}, normalized bounds {max_bounds}"
    );
}

#[test]
fn invisible_source_extreme_remains_in_the_choropleth_color_domain() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/geo-usa-map/values.json")).unwrap();
    let mut cases = 0;
    for case in a(&fixture["cases"])
        .iter()
        .filter(|c| c["group"] == "color-domain")
    {
        cases += 1;
        let features = source(case);
        let mut o = options(case);
        let missing = Color(13, 23, 43, 255);
        o.fill = Some(missing);
        let map = choropleth_usa_layout(&config(case), &features, &o).unwrap();
        assert!(
            !map.scene
                .marks
                .iter()
                .any(|m| m.series == "invisible-extreme")
        );
        for (id, color) in [
            ("zero-lower", Color(230, 240, 250, 255)),
            ("ten-alaska", Color(228, 238, 249, 255)),
            ("missing-hawaii", missing),
        ] {
            let mark = map.scene.marks.iter().find(|m| m.series == id).unwrap();
            assert_eq!(mark.fill, Some(color), "{} / {id}", case["name"]);
        }
    }
    assert_eq!(cases, 6);
}
