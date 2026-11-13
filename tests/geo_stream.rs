use glory::render::{GraphicPoint, PathCommand as C};
use kartis::*;
fn c() -> ChartConfig {
    ChartConfig {
        width: 600.,
        height: 400.,
        margin: Margin {
            left: 50.,
            right: 50.,
            top: 50.,
            bottom: 50.,
        },
        ..Default::default()
    }
}
fn segments(g: &Geometry) -> Vec<([f64; 2], [f64; 2])> {
    let mut p = None;
    let mut out = vec![];
    for command in g.commands() {
        match command {
            C::Move(v) => p = Some([v.x, v.y]),
            C::Line(v) => {
                let q = [v.x, v.y];
                if let Some(a) = p {
                    out.push((a, q));
                }
                p = Some(q);
            }
            _ => panic!(),
        }
    }
    out
}
fn distance(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let norm = dx.hypot(dy);
    if norm == 0. {
        return (p[0] - a[0]).hypot(p[1] - a[1]);
    }
    let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (norm * norm)).clamp(0., 1.);
    (p[0] - a[0] - t * dx).hypot(p[1] - a[1] - t * dy)
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
#[test]
fn transverse_matches_actual_d3_points_with_center_and_roll() {
    let mut count = 0;
    for row in include_str!("fixtures/geo-transverse.csv").lines().skip(2) {
        let v = row
            .split('|')
            .map(|s| s.parse::<f64>().unwrap())
            .collect::<Vec<_>>();
        let o = GeoMapOptions {
            projection: GeoProjection::TransverseMercator,
            center: [v[0], v[1]],
            rotation: v[2],
            ..Default::default()
        };
        let p = project_geo_point([v[3], v[4]], &o).unwrap().unwrap();
        close(p[0], v[5]);
        close(p[1], v[6]);
        count += 1;
    }
    assert!(count > 250);
    assert!(
        project_geo_point(
            [90., 0.],
            &GeoMapOptions {
                projection: GeoProjection::TransverseMercator,
                ..Default::default()
            }
        )
        .unwrap()
        .is_none()
    );
    let view = GeoMapOptions {
        projection: GeoProjection::TransverseMercator,
        center: [20., 0.],
        ..Default::default()
    };
    assert!(project_geo_point([0., 90.], &view).unwrap().is_some());
    let singular = GeoFeature {
        id: "singular".into(),
        label: String::new(),
        value: None,
        polygons: vec![vec![vec![
            [-80., -50.],
            [-40., -50.],
            [-40., 5.],
            [-80., 5.],
        ]]],
    };
    assert!(
        geographic_map_layout_with_path_options(&c(), &[singular], &view, &Default::default())
            .is_err()
    );
    let mut map = geographic_map_layout(&c(), &[], &view).unwrap();
    let grid = map
        .add_graticule(&Default::default(), &Default::default())
        .unwrap();
    assert_eq!(grid.lines.len(), 53);
    assert!(grid.sampling.points > 2573);
}
#[test]
fn actual_d3_major_minor_graticule_source_and_budget() {
    let g = geo_graticule(&Default::default()).unwrap();
    assert_eq!(g.len(), 53);
    let mut count = 0;
    for row in include_str!("fixtures/geo-graticule.csv").lines().skip(2) {
        let v = row.split('|').collect::<Vec<_>>();
        let i: usize = v[0].parse().unwrap();
        let j: usize = v[1].parse().unwrap();
        let p = g[i].feature.lines[0][j];
        close(p[0], v[2].parse().unwrap());
        close(p[1], v[3].parse().unwrap());
        count += 1;
    }
    assert!(count > 2500);
    let mut bad = GeoGraticuleOptions {
        precision: 0.01,
        ..Default::default()
    };
    assert!(geo_graticule(&bad).is_err());
    bad = Default::default();
    bad.step_minor[0] = f64::NAN;
    assert!(geo_graticule(&bad).is_err());
}
#[test]
fn stream_clips_seam_poles_and_caps_without_false_bridges() {
    let cases = [
        (
            GeoProjection::Equirectangular,
            vec![[170., 30.], [-170., 45.]],
        ),
        (GeoProjection::Equirectangular, vec![[0., 80.], [180., 80.]]),
        (GeoProjection::Orthographic, vec![[-80., 0.], [80., 0.]]),
    ];
    for (projection, positions) in cases {
        let view = GeoMapOptions {
            projection,
            clip_angle: projection_azimuthal(projection).then_some(60.),
            scale: Some(100.),
            ..Default::default()
        };
        let l = geographic_lines(
            &c(),
            &[GeoLineFeature::new("route", positions)],
            &view,
            &Default::default(),
        )
        .unwrap();
        let g = l.metadata.lines[0].geometry.as_ref().unwrap();
        if projection == GeoProjection::Equirectangular {
            assert!(l.metadata.lines[0].components >= 2);
            for (a, b) in segments(g) {
                assert!((a[0] - b[0]).abs() < 320.);
            }
        } else {
            let d = l.projection.projection_disk().unwrap();
            for (a, b) in segments(g) {
                for p in [a, b] {
                    assert!((p[0] - d.center[0]).hypot(p[1] - d.center[1]) <= d.radius + 1e-7);
                }
            }
            assert!(l.scene.marks[0].clip_path.is_some());
        }
    }
}
fn projection_azimuthal(p: GeoProjection) -> bool {
    matches!(p, GeoProjection::Orthographic | GeoProjection::Gnomonic)
}
#[test]
fn independent_d3_stream_oracle_points_follow_the_same_projected_arcs() {
    let cases = [
        (
            GeoProjection::Equirectangular,
            vec![[170., 30.], [-170., 45.]],
        ),
        (
            GeoProjection::Equirectangular,
            vec![[-85., 10.], [85., 10.]],
        ),
        (
            GeoProjection::TransverseMercator,
            vec![[10., -60.], [60., 30.]],
        ),
        (GeoProjection::Orthographic, vec![[-75., 25.], [75., 25.]]),
        (GeoProjection::Orthographic, vec![[-80., 0.], [80., 0.]]),
        (GeoProjection::Gnomonic, vec![[-80., 20.], [80., 20.]]),
    ];
    let layouts = cases
        .into_iter()
        .map(|(projection, p)| {
            geographic_lines(
                &c(),
                &[GeoLineFeature::new("r", p)],
                &GeoMapOptions {
                    projection,
                    clip_angle: projection_azimuthal(projection).then_some(60.),
                    scale: Some(120.),
                    translate: Some([300., 200.]),
                    ..Default::default()
                },
                &GeoStreamOptions {
                    clip_plot: false,
                    ..Default::default()
                },
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let mut n = 0;
    for row in include_str!("fixtures/geo-stream-paths.csv")
        .lines()
        .skip(2)
    {
        let v = row.split('|').collect::<Vec<_>>();
        let index: usize = v[0].parse().unwrap();
        let p = [v[4].parse().unwrap(), v[5].parse().unwrap()];
        let g = layouts[index].metadata.lines[0].geometry.as_ref().unwrap();
        let error = segments(g)
            .iter()
            .map(|(a, b)| distance(p, *a, *b))
            .fold(f64::INFINITY, f64::min);
        assert!(error < 0.3, "case{index} {p:?} error{error}");
        n += 1;
    }
    assert!(n > 50);
}
#[test]
fn adaptive_high_scale_dense_great_arc_error_and_data_fit_are_real() {
    let data = [GeoLineFeature::new("high", vec![[-85., 10.], [82., 13.]])];
    let options = GeoStreamOptions {
        path: Some(GeoPathOptions {
            precision: 0.02,
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut view = GeoMapOptions {
        fit: GeoFit::Data,
        padding: 7.,
        ..Default::default()
    };
    let l = geographic_lines(&c(), &data, &view, &options).unwrap();
    let g = l.metadata.lines[0].geometry.as_ref().unwrap();
    let plot = l.projection.plot();
    for (a, b) in segments(g) {
        for p in [a, b] {
            assert!(
                p[0] >= plot.x + 7. - 1e-8
                    && p[0] <= plot.x + plot.width - 7. + 1e-8
                    && p[1] >= plot.y + 7. - 1e-8
                    && p[1] <= plot.y + plot.height - 7. + 1e-8
            );
        }
    }
    let a = data[0].lines[0][0];
    let b = data[0].lines[0][1];
    let unit = |p: [f64; 2]| {
        let (l, f) = (p[0].to_radians(), p[1].to_radians());
        [f.cos() * l.cos(), f.cos() * l.sin(), f.sin()]
    };
    let av = unit(a);
    let bv = unit(b);
    let theta = av.iter().zip(bv).map(|(a, b)| a * b).sum::<f64>().acos();
    for i in 0..2001 {
        let t = i as f64 / 2000.;
        let mut v = [0.; 3];
        for j in 0..3 {
            v[j] = (av[j] * ((1. - t) * theta).sin() + bv[j] * (t * theta).sin()) / theta.sin();
        }
        let p = l
            .projection
            .project([
                v[1].atan2(v[0]).to_degrees(),
                v[2].atan2(v[0].hypot(v[1])).to_degrees(),
            ])
            .unwrap()
            .unwrap();
        let e = segments(g)
            .iter()
            .map(|(a, b)| distance(p, *a, *b))
            .fold(f64::INFINITY, f64::min);
        assert!(e <= 0.021, "dense error{e}");
    }
    view.fit = GeoFit::World;
    let world = geographic_lines(&c(), &data, &view, &options).unwrap();
    assert!(l.projection.scale() > world.projection.scale());
    view.scale = Some(40000.);
    let high = geographic_lines(&c(), &data, &view, &options).unwrap();
    assert!(high.metadata.sampling.points > l.metadata.sampling.points * 2);
    close(high.projection.scale(), 40000.);
}
#[test]
fn polygon_refinement_shared_fit_overlay_order_and_failures_are_transactional() {
    let c = c();
    let feature = GeoFeature {
        id: "land".into(),
        label: "Land".into(),
        value: Some(1.),
        polygons: vec![vec![vec![[-45., 15.], [45., 15.], [35., 50.], [-35., 50.]]]],
    };
    let view = GeoMapOptions {
        fit: GeoFit::Data,
        ..Default::default()
    };
    let legacy = geographic_map_layout(&c, std::slice::from_ref(&feature), &view).unwrap();
    let mut map = geographic_map_layout_with_path_options(
        &c,
        &[feature],
        &view,
        &GeoPathOptions {
            precision: 0.001,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(map.projection.scale() <= legacy.projection.scale() + 1e-8);
    assert!(
        map.scene.marks[0].geometry.commands().len()
            > legacy.scene.marks[0].geometry.commands().len()
    );
    let projection = map.projection.clone();
    let route = GeoLineFeature::new("r", vec![[-20., 25.], [20., 35.]]);
    let metadata = map
        .add_lines(std::slice::from_ref(&route), &Default::default())
        .unwrap();
    assert_eq!(map.projection, projection);
    let C::Move(GraphicPoint { x, y }) = metadata.lines[0].geometry.as_ref().unwrap().commands()[0]
    else {
        panic!()
    };
    let expected = projection.project([-20., 25.]).unwrap().unwrap();
    close(x, expected[0]);
    close(y, expected[1]);
    let before = map.clone();
    assert!(
        map.add_lines(std::slice::from_ref(&route), &Default::default())
            .is_err()
    );
    assert_eq!(map, before);
    let bad = GeoStreamOptions {
        path: Some(GeoPathOptions {
            max_work: 1,
            ..Default::default()
        }),
        ..Default::default()
    };
    assert!(
        map.add_lines(
            &[GeoLineFeature::new("bad", vec![[-80., 25.], [80., 25.]])],
            &bad
        )
        .is_err()
    );
    assert_eq!(map, before);
    map.add_graticule(
        &Default::default(),
        &GeoStreamOptions {
            behind: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(matches!(map.scene.draw_order[0],DrawItem::Mark(i)if i>=2));
    assert!(
        map.scene
            .marks
            .iter()
            .filter(|m| m.id.starts_with("geo-line:graticule:"))
            .all(|m| m.label.is_empty())
    );
    map.scene.validate().unwrap();
}
#[cfg(feature = "raster")]
#[test]
fn cpu_route_ink_and_full_stroke_circle_clipping_are_real() {
    use render::raster::*;
    let view = GeoMapOptions {
        projection: GeoProjection::Orthographic,
        clip_angle: Some(60.),
        scale: Some(100.),
        translate: Some([300., 200.]),
        ..Default::default()
    };
    let mut o = GeoStreamOptions::default();
    o.style.stroke.color = Color(200, 10, 20, 255);
    o.style.stroke.width = 12.;
    let l = geographic_lines(
        &c(),
        &[GeoLineFeature::new("r", vec![[-80., 0.], [80., 0.]])],
        &view,
        &o,
    )
    .unwrap();
    let image = render_raster(&l.scene, &FontRegistry::new(), &Default::default()).unwrap();
    let pixel = |x: usize, y: usize| &image.rgba[(y * 600 + x) * 4..][..4];
    assert_eq!(pixel(300, 200), [200, 10, 20, 255]);
    assert_eq!(pixel(390, 200), [255; 4]);
    assert_ne!(pixel(380, 200), [255; 4]);
    for projection in [GeoProjection::Orthographic, GeoProjection::Gnomonic] {
        let mut options = GeoStreamOptions::default();
        options.style.stroke.width = 20.;
        options.style.stroke.cap = glory::render::StrokeCap::Round;
        options.style.stroke.color = Color(200, 10, 20, 255);
        let tiny = geographic_lines(
            &c(),
            &[GeoLineFeature::new("tiny", vec![[0., 0.], [5e-11, 0.]])],
            &GeoMapOptions {
                projection,
                fit: GeoFit::Data,
                ..Default::default()
            },
            &options,
        )
        .unwrap();
        let frame = render_raster(&tiny.scene, &FontRegistry::new(), &Default::default()).unwrap();
        let at = |x: usize, y: usize| &frame.rgba[(y * 600 + x) * 4..][..4];
        assert_eq!(at(60, 200), [200, 10, 20, 255]);
        assert_eq!(at(45, 200), [255; 4]);
        assert_eq!(at(555, 200), [255; 4]);
    }
}
#[test]
fn short_subject_edge_on_cap_is_a_great_circle_not_a_synthetic_small_circle() {
    let angle = 1_f64.to_radians();
    let vertex = |theta: f64| {
        let v = [
            0.5,
            3_f64.sqrt() / 2. * theta.cos(),
            3_f64.sqrt() / 2. * theta.sin(),
        ];
        [v[1].atan2(v[0]).to_degrees(), v[2].asin().to_degrees()]
    };
    let a = vertex(0.);
    let b = vertex(angle);
    let feature = GeoFeature {
        id: "triangle".into(),
        label: String::new(),
        value: None,
        polygons: vec![vec![vec![[0., 0.], a, b]]],
    };
    let view = GeoMapOptions {
        projection: GeoProjection::Gnomonic,
        clip_angle: Some(60.),
        scale: Some(100000.),
        translate: Some([300., 200.]),
        border: None,
        ..Default::default()
    };
    let map = geographic_map_layout_with_path_options(
        &c(),
        &[feature],
        &view,
        &GeoPathOptions {
            precision: 0.0001,
            ..Default::default()
        },
    )
    .unwrap();
    let p = map.projection.project(a).unwrap().unwrap();
    let q = map.projection.project(b).unwrap().unwrap();
    let origin = map.projection.project([0., 0.]).unwrap().unwrap();
    let cross = |a: [f64; 2], b: [f64; 2], p: [f64; 2]| {
        (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
    };
    let sign = cross(p, q, origin).signum();
    for command in map.scene.marks[0].geometry.commands() {
        if let C::Move(v) | C::Line(v) = command {
            assert!(
                cross(p, q, [v.x, v.y]) * sign >= -0.001,
                "subject edge bowed outside its true gnomonic chord"
            );
        }
    }
}
#[test]
fn tiny_azimuthal_data_fit_omits_only_a_provably_redundant_cap_mask() {
    let data = [GeoLineFeature::new("tiny", vec![[0., 0.], [5e-11, 0.]])];
    for projection in [GeoProjection::Orthographic, GeoProjection::Gnomonic] {
        let view = GeoMapOptions {
            projection,
            fit: GeoFit::Data,
            ..Default::default()
        };
        let layout = geographic_lines(&c(), &data, &view, &Default::default()).unwrap();
        assert!(layout.scene.marks[0].clip_path.is_none());
        assert_eq!(layout.scene.marks[0].clip, Some(ClipRect::plot(&c())));
        let seg = segments(&layout.scene.marks[0].geometry);
        close(seg[0].0[0], 50.);
        close(seg.last().unwrap().1[0], 550.);
        assert!(
            geographic_lines(
                &c(),
                &data,
                &view,
                &GeoStreamOptions {
                    clip_plot: false,
                    ..Default::default()
                }
            )
            .is_err()
        );
        let horizon = [GeoLineFeature::new(
            "edge",
            vec![[59.9999999999, 0.], [60., 0.]],
        )];
        let edge = geographic_lines(
            &c(),
            &horizon,
            &GeoMapOptions {
                clip_angle: Some(60.),
                ..view
            },
            &Default::default(),
        );
        if let Ok(edge) = edge {
            assert!(edge.scene.marks[0].clip_path.is_some());
        }
    }
}
