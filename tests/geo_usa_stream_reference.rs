//! Independent original d3-geo 3.1.1 open-stream oracle. See the producer and
//! reference.json for ISC attribution, source hashes, fitted-frame provenance,
//! native multiplex interleaving and the predeclared geometric error policy.
#![cfg(feature = "geojson")]

use glory::render::PathCommand;
use glory::style::Color;
use kartis::*;
use serde_json::Value;

fn array(v: &Value) -> &[Value] {
    v.as_array().unwrap()
}
fn number(v: &Value) -> f64 {
    v.as_f64().unwrap()
}
fn point(v: &Value) -> [f64; 2] {
    [number(&v[0]), number(&v[1])]
}
fn pair(v: &Value) -> [[f64; 2]; 2] {
    [point(&v[0]), point(&v[1])]
}
fn count(v: &Value) -> usize {
    v.as_u64().unwrap() as usize
}
fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/geo-usa-stream/values.json")).unwrap()
}
fn config(map: &Value) -> ChartConfig {
    let c = &map["config"];
    let m = &c["margin"];
    ChartConfig {
        width: number(&c["width"]),
        height: number(&c["height"]),
        margin: Margin {
            left: number(&m["left"]),
            right: number(&m["right"]),
            top: number(&m["top"]),
            bottom: number(&m["bottom"]),
        },
        ..Default::default()
    }
}
fn map_source(map: &Value) -> Vec<GeoFeature> {
    array(&map["features"])
        .iter()
        .map(|f| GeoFeature {
            id: f["id"].as_str().unwrap().into(),
            label: f["label"].as_str().unwrap().into(),
            value: f["value"].as_f64(),
            polygons: array(&f["polygons"])
                .iter()
                .map(|p| {
                    array(p)
                        .iter()
                        .map(|r| array(r).iter().map(point).collect())
                        .collect()
                })
                .collect(),
        })
        .collect()
}
fn map_options(map: &Value) -> GeoUsaMapOptions {
    let o = &map["options"];
    GeoUsaMapOptions {
        fit: if o["fit"] == "Data" {
            GeoFit::Data
        } else {
            GeoFit::World
        },
        padding: number(&o["padding"]),
        scale: o["scale"].as_f64(),
        translate: (!o["translate"].is_null()).then(|| point(&o["translate"])),
        border: None,
        fill: Some(Color(80, 120, 180, 255)),
        path: GeoPathOptions {
            precision: number(&o["precision"]),
            ..Default::default()
        },
        ..Default::default()
    }
}
fn lines(case: &Value) -> Vec<GeoLineFeature> {
    array(&case["features"])
        .iter()
        .map(|f| GeoLineFeature {
            id: f["id"].as_str().unwrap().into(),
            label: f["label"].as_str().unwrap().into(),
            series: f["series"].as_str().unwrap().into(),
            value: f["value"].as_f64(),
            lines: array(&f["lines"])
                .iter()
                .map(|l| array(l).iter().map(point).collect())
                .collect(),
            style: None,
        })
        .collect()
}
fn paths(g: &Geometry) -> Vec<Vec<[f64; 2]>> {
    let Geometry::Path(commands) = g else {
        panic!("USA line part must be Path")
    };
    let mut lines = vec![];
    let mut current = vec![];
    for command in commands {
        match command {
            PathCommand::Move(p) => {
                if !current.is_empty() {
                    lines.push(std::mem::take(&mut current));
                }
                current.push([p.x, p.y]);
            }
            PathCommand::Line(p) => {
                assert!(!current.is_empty());
                current.push([p.x, p.y]);
            }
            _ => panic!("open line must not close or curve"),
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}
fn length(line: &[[f64; 2]]) -> f64 {
    line.windows(2)
        .map(|p| (p[0][0] - p[1][0]).hypot(p[0][1] - p[1][1]))
        .sum()
}
fn distance(lines: &[Vec<[f64; 2]>], p: [f64; 2]) -> f64 {
    lines
        .iter()
        .flat_map(|l| l.windows(2))
        .map(|s| {
            let a = s[0];
            let dx = s[1][0] - a[0];
            let dy = s[1][1] - a[1];
            let len = dx * dx + dy * dy;
            let t = if len == 0. {
                0.
            } else {
                (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len).clamp(0., 1.)
            };
            (p[0] - a[0] - t * dx).hypot(p[1] - a[1] - t * dy)
        })
        .fold(f64::INFINITY, f64::min)
}
fn bounds(lines: &[Vec<[f64; 2]>]) -> [f64; 4] {
    lines.iter().flatten().fold(
        [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ],
        |b, p| {
            [
                b[0].min(p[0]),
                b[1].min(p[1]),
                b[2].max(p[0]),
                b[3].max(p[1]),
            ]
        },
    )
}
fn samples(lines: &[Vec<[f64; 2]>]) -> Vec<[f64; 2]> {
    let mut p: Vec<_> = lines.iter().flatten().copied().collect();
    p.extend(
        lines
            .iter()
            .flat_map(|l| l.windows(2))
            .map(|s| [(s[0][0] + s[1][0]) / 2., (s[0][1] + s[1][1]) / 2.]),
    );
    p
}
fn region(name: &str) -> GeoUsaRegion {
    match name {
        "Lower48" => GeoUsaRegion::Lower48,
        "Alaska" => GeoUsaRegion::Alaska,
        "Hawaii" => GeoUsaRegion::Hawaii,
        _ => panic!("unknown native region"),
    }
}
fn grid_options(f: &Value) -> GeoGraticuleOptions {
    let g = &f["graticule"];
    GeoGraticuleOptions {
        extent_major: pair(&g["extentMajor"]),
        extent_minor: pair(&g["extentMinor"]),
        step_major: point(&g["stepMajor"]),
        step_minor: point(&g["stepMinor"]),
        precision: number(&g["precision"]),
    }
}

#[test]
fn original_graticule_major_minor_source_profile_is_preserved() {
    let f = fixture();
    let actual = geo_graticule(&grid_options(&f)).unwrap();
    let native = array(&f["nativeGraticule"]);
    assert_eq!(actual.len(), native.len());
    assert_eq!(actual.len(), 15);
    assert!(actual.iter().any(|g| g.major));
    assert!(actual.iter().any(|g| !g.major));
    for (g, native) in actual.iter().zip(native) {
        assert_eq!(g.feature.lines.len(), 1);
        assert_eq!(g.feature.lines[0].len(), array(native).len());
        for (a, b) in g.feature.lines[0].iter().zip(array(native)) {
            let b = point(b);
            for i in 0..2 {
                assert!((a[i] - b[i]).abs() < 1e-10);
            }
        }
    }
}

#[test]
fn original_child_streams_keep_regions_gaps_fit_and_geometric_coverage() {
    let f = fixture();
    let material_length = number(&f["materialLength"]);
    let tube_radius = number(&f["tubeRadius"]);
    let (mut states, mut regions, mut material, mut raw_short, mut included, mut excluded) =
        (0, 0, 0, 0, 0, 0);
    let (mut directed_queries, mut max_distance, mut max_bounds) = (0, 0_f64, 0_f64);
    let mut actual_short = 0;
    let mut native_nested = 0;
    for case in array(&f["cases"]) {
        let name = case["name"].as_str().unwrap();
        let map = &case["map"];
        let c = config(map);
        let source = map_source(map);
        let options = map_options(map);
        let map = geographic_usa_map_layout(&c, &source, &options).unwrap();
        let ctx = map.projection.clone();
        let mut scene = map.scene;
        let map_prefix = scene.marks.clone();
        let before = ctx.clone();
        let data = lines(case);
        let original = data.clone();
        let metadata = ctx
            .add_usa_lines(&mut scene, &data, &GeoStreamOptions::default())
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(ctx, before, "{name}: overlay refitted context");
        assert_eq!(data, original);
        assert_eq!(&scene.marks[..map_prefix.len()], map_prefix.as_slice());
        assert_eq!(metadata.lines.len(), data.len());
        assert_eq!(
            metadata.source_points,
            data.iter()
                .flat_map(|s| &s.lines)
                .map(Vec::len)
                .sum::<usize>()
        );
        assert!(
            metadata.source_points
                + metadata.coarse_points
                + metadata.sampling.points
                + metadata.clip_stats.output_points
                <= options.path.max_points
        );
        assert!(metadata.sampling.work + metadata.clip_stats.work <= options.path.max_work);
        let core = ctx.usa_projection().unwrap();
        let (ak, at) = (core.scale(), core.translate());
        let native = &case["map"]["native"];
        let (nk, nt) = (number(&native["scale"]), point(&native["translate"]));
        for (index, ((computed, src), want)) in metadata
            .lines
            .iter()
            .zip(&data)
            .zip(array(&case["native"]))
            .enumerate()
        {
            states += 1;
            native_nested += count(&want["multiplex"]["nestedLineStarts"]);
            assert_eq!(computed.source_index, index);
            assert_eq!(computed.id, src.id);
            assert_eq!(computed.label, src.label);
            assert_eq!(computed.series, src.series);
            assert_eq!(computed.value, src.value);
            assert_eq!(computed.mark_id, format!("geo-line:{}", src.id));
            assert!(computed.parts.len() <= 3);
            assert!(computed.parts.windows(2).all(|p| p[0].region < p[1].region));
            for expected in array(&want["parts"]) {
                regions += 1;
                let region = region(expected["region"].as_str().unwrap());
                let part = computed.parts.iter().find(|p| p.region == region);
                let raw = if let Some(part) = part {
                    let mark = scene.marks.iter().find(|m| m.id == part.mark_id).unwrap();
                    assert_eq!(mark.geometry, part.geometry);
                    assert_eq!(mark.series, src.series);
                    assert_eq!(mark.value, src.value.unwrap());
                    assert_eq!(mark.fill, None);
                    let raw = paths(&part.geometry);
                    assert_eq!(raw.len(), part.components);
                    let rect = core.region_clip(region);
                    for p in raw.iter().flatten() {
                        assert!(p.iter().all(|v| v.is_finite()));
                        assert!(
                            p[0] >= rect.x - 1e-7
                                && p[0] <= rect.x + rect.width + 1e-7
                                && p[1] >= rect.y - 1e-7
                                && p[1] <= rect.y + rect.height + 1e-7,
                            "{name}/{} outside own region: {p:?}",
                            src.id
                        );
                    }
                    raw
                } else {
                    vec![]
                };
                let normalized: Vec<Vec<[f64; 2]>> = raw
                    .iter()
                    .map(|line| {
                        line.iter()
                            .map(|p| {
                                [
                                    nt[0] + (p[0] - at[0]) * nk / ak,
                                    nt[1] + (p[1] - at[1]) * nk / ak,
                                ]
                            })
                            .collect()
                    })
                    .collect();
                let actual: Vec<_> = normalized
                    .iter()
                    .filter(|l| length(l) > material_length)
                    .cloned()
                    .collect();
                actual_short += normalized.len() - actual.len();
                raw_short += count(&expected["shortComponents"]);
                if normalized.len() - actual.len() != count(&expected["shortComponents"]) {
                    let native_lengths = array(&expected["lines"])
                        .iter()
                        .map(|l| array(l).iter().map(point).collect::<Vec<_>>())
                        .map(|l| length(&l))
                        .filter(|l| *l <= material_length)
                        .collect::<Vec<_>>();
                    let actual_lengths = normalized
                        .iter()
                        .map(|l| length(l))
                        .filter(|l| *l <= material_length)
                        .collect::<Vec<_>>();
                    eprintln!(
                        "Retained short-contact diagnostic {name}/{} {region:?}: native lengths {native_lengths:?}, actual lengths {actual_lengths:?}; material threshold {material_length}px",
                        src.id
                    );
                }
                let native: Vec<Vec<[f64; 2]>> = array(&expected["lines"])
                    .iter()
                    .map(|l| array(l).iter().map(point).collect::<Vec<_>>())
                    .filter(|l| length(l) > material_length)
                    .collect();
                let context = format!("{name}/{} {:?}", src.id, region);
                let limit = number(&expected["errorLimit"]);
                assert_eq!(
                    actual.len(),
                    count(&expected["materialComponents"]),
                    "{context}: material components"
                );
                assert_eq!(native.len(), actual.len());
                material += native.len();
                if !native.is_empty() {
                    for (a, b) in bounds(&actual).into_iter().zip(bounds(&native)) {
                        max_bounds = max_bounds.max((a - b).abs());
                        assert!(
                            (a - b).abs() <= limit,
                            "{context}: bounds {a} != {b}, limit {limit}"
                        );
                    }
                    for (from, to) in [(&actual, &native), (&native, &actual)] {
                        for p in samples(from) {
                            directed_queries += 1;
                            let d = distance(to, p);
                            max_distance = max_distance.max(d);
                            assert!(
                                d <= limit,
                                "{context}: directed segment distance {d} > {limit} at {p:?}"
                            );
                        }
                    }
                }
                let (mut local_included, mut local_excluded) = (0, 0);
                for query in array(&expected["coverage"]) {
                    if query["excluded"].as_bool().unwrap() {
                        local_excluded += 1;
                        continue;
                    }
                    local_included += 1;
                    let q = point(&query["position"]);
                    let d = distance(&actual, q);
                    assert_eq!(
                        d < tube_radius,
                        query["inside"].as_bool().unwrap(),
                        "{context}: 2px tube coverage at {q:?}, actual distance {d}, native {}",
                        query["distance"]
                    );
                }
                assert_eq!(
                    local_included,
                    count(&expected["coverageCounts"]["included"])
                );
                assert_eq!(
                    local_excluded,
                    count(&expected["coverageCounts"]["excluded"])
                );
                included += local_included;
                excluded += local_excluded;
            }
        }
        let marks = metadata.lines.iter().map(|l| l.parts.len()).sum::<usize>();
        assert_eq!(scene.marks.len(), map_prefix.len() + marks);
        scene.validate().unwrap();
        metadata.interaction_binding(&scene).unwrap();
    }
    assert_eq!(
        (
            states,
            regions,
            material,
            raw_short,
            included,
            excluded,
            native_nested
        ),
        (132, 396, 156, 5, 18002, 3, 6)
    );
    eprintln!(
        "Original USA open streams: 6 map frames, {states} sources, {regions} region states, {material} material components, native short={raw_short}/actual short={actual_short}, {directed_queries} bidirectional segment probes, {included} tube coverage included/{excluded} fixed-band excluded; native nested starts={native_nested}; max distance={max_distance}px, max bounds={max_bounds}px"
    );
}

#[test]
fn graticule_overlay_uses_original_source_without_refitting() {
    let f = fixture();
    let grid = grid_options(&f);
    for case in array(&f["cases"]) {
        let map = &case["map"];
        let c = config(map);
        let source = map_source(map);
        let options = map_options(map);
        let layout = geographic_usa_map_layout(&c, &source, &options).unwrap();
        let ctx = layout.projection;
        let mut direct = layout.scene.clone();
        let mut graticule = layout.scene;
        let data = geo_graticule(&grid)
            .unwrap()
            .into_iter()
            .map(|g| g.feature)
            .collect::<Vec<_>>();
        let o = GeoStreamOptions {
            interactive: false,
            ..Default::default()
        };
        let expected = ctx.add_usa_lines(&mut direct, &data, &o).unwrap();
        let actual = ctx
            .add_usa_graticule(&mut graticule, &grid, &GeoStreamOptions::default())
            .unwrap();
        assert_eq!(actual, expected);
        assert_eq!(direct, graticule);
    }
}
