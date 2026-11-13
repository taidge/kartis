use glory::render::PathCommand;
use kartis::*;
fn projections() -> Vec<GeoProjection> {
    let mut v = vec![GeoProjection::Albers];
    for p in [
        [0., 60.],
        [30., 30.],
        [60., 30.],
        [-60., -30.],
        [-30., 30.],
        [0.0000572957796; 2],
        [89.99; 2],
    ] {
        v.extend([
            GeoProjection::conic_equal_area(p).unwrap(),
            GeoProjection::conic_equidistant(p).unwrap(),
            GeoProjection::conic_conformal(p).unwrap(),
        ]);
    }
    v
}
fn config() -> ChartConfig {
    ChartConfig {
        width: 800.,
        height: 600.,
        ..Default::default()
    }
}
fn near(a: [f64; 2], b: [f64; 2], e: f64) {
    assert!((a[0] - b[0]).hypot(a[1] - b[1]) <= e, "{a:?} != {b:?}");
}
fn vector(p: [f64; 2]) -> [f64; 3] {
    let [l, f] = p.map(f64::to_radians);
    [f.cos() * l.cos(), f.cos() * l.sin(), f.sin()]
}
fn sphere(a: [f64; 2], b: [f64; 2]) {
    let (a, b) = (vector(a), vector(b));
    assert!((a[0] - b[0]).hypot(a[1] - b[1]).hypot(a[2] - b[2]) <= 1e-12);
}
#[test]
fn checked_order_copy_equality_and_illegal_parameters() {
    fn copy_eq<T: Copy + Eq>() {}
    copy_eq::<GeoProjection>();
    copy_eq::<GeoConicParallels>();
    for p in [[90., 0.], [-90., 0.], [f64::NAN, 0.], [0., f64::INFINITY]] {
        assert!(GeoConicParallels::new(p).is_err());
    }
    assert_eq!(
        GeoConicParallels::new([-0., 0.]).unwrap(),
        GeoConicParallels::new([0., -0.]).unwrap()
    );
    assert_eq!(
        GeoConicParallels::new([60., 30.]).unwrap().degrees(),
        [60., 30.]
    );
}
#[test]
fn finite_sector_world_encloses_dense_visible_domain() {
    for projection in projections() {
        let view = GeoMapOptions {
            projection,
            padding: 4.,
            ..Default::default()
        };
        let c = geographic_map_layout(&config(), &[], &view)
            .unwrap()
            .projection;
        let plot = c.plot();
        for lon in (-180..=180).step_by(3) {
            for lat in (-90..=90).step_by(3) {
                if let Some(q) = c.project([lon as f64, lat as f64]).unwrap() {
                    assert!(
                        q[0] >= plot.x - 1e-7
                            && q[0] <= plot.x + plot.width + 1e-7
                            && q[1] >= plot.y - 1e-7
                            && q[1] <= plot.y + plot.height + 1e-7,
                        "{projection:?} {lon},{lat} ->{q:?}"
                    );
                }
            }
        }
    }
}
#[test]
fn authored_camera_and_intrinsic_inverse_share_one_context() {
    for projection in [
        GeoProjection::Albers,
        GeoProjection::conic_equal_area_default(),
        GeoProjection::conic_equidistant_default(),
        GeoProjection::conic_conformal_default(),
    ] {
        for (center, rotation) in [([0., 0.], 0.), ([35., 20.], 25.), ([-80., 30.], -30.)] {
            let view = GeoMapOptions {
                projection,
                center,
                rotation,
                scale: Some(75.),
                translate: Some([321., 237.]),
                ..Default::default()
            };
            let c = geographic_map_layout(&config(), &[], &view)
                .unwrap()
                .projection;
            for p in [
                [-122., 38.],
                [-74., 41.],
                [0., 0.],
                [40., -30.],
                [170., 65.],
            ] {
                if let Some(q) = c.project(p).unwrap() {
                    sphere(c.invert(q).unwrap().unwrap(), p);
                }
            }
        }
    }
}
fn land() -> Vec<GeoFeature> {
    vec![GeoFeature {
        id: "land".into(),
        label: "Land".into(),
        value: Some(7.),
        polygons: vec![vec![
            vec![
                [-120., 20.],
                [-70., 20.],
                [-70., 50.],
                [-120., 50.],
                [-120., 20.],
            ],
            vec![
                [-105., 30.],
                [-105., 40.],
                [-90., 40.],
                [-90., 30.],
                [-105., 30.],
            ],
        ]],
    }]
}
#[test]
fn choropleth_hole_symbol_routes_graticule_use_the_accepted_context() {
    for projection in [
        GeoProjection::Albers,
        GeoProjection::conic_conformal_default(),
        GeoProjection::conic_equidistant_default(),
    ] {
        let view = GeoMapOptions {
            projection,
            fit: GeoFit::Data,
            padding: 8.,
            center: [12., 8.],
            rotation: 23.,
            ..Default::default()
        };
        let mut map = choropleth_layout_with_path_options(
            &config(),
            &land(),
            &view,
            &GeoPathOptions::default(),
        )
        .unwrap();
        let original = map.projection.clone();
        let marks = map.scene.marks.clone();
        let p = [-120., 20.];
        let q = original.project(p).unwrap().unwrap();
        let Geometry::Path(commands) = &marks[0].geometry else {
            panic!()
        };
        assert_eq!(
            commands
                .iter()
                .filter(|p| matches!(p, PathCommand::Close))
                .count(),
            2
        );
        map.add_symbols(
            &[GeoSymbol::new("corner", p, Some(5.))],
            &GeoSymbolOptions {
                clip: false,
                ..Default::default()
            },
        )
        .unwrap();
        let Geometry::Circle { x, y, .. } = map.scene.marks.last().unwrap().geometry else {
            panic!()
        };
        near([x, y], q, 1e-8);
        map.add_lines(
            &[GeoLineFeature::new("route", vec![p, [-74., 41.]])],
            &GeoStreamOptions::default(),
        )
        .unwrap();
        map.add_graticule(
            &GeoGraticuleOptions {
                extent_major: [[-130., 10.], [-60., 60.]],
                extent_minor: [[-130., 10.], [-60., 60.]],
                ..Default::default()
            },
            &GeoStreamOptions::default(),
        )
        .unwrap();
        assert_eq!(map.projection, original);
        assert_eq!(map.scene.marks[0], marks[0]);
        map.scene.validate().unwrap();
        let before = map.scene.clone();
        assert!(
            map.add_lines(
                &[GeoLineFeature::new("bad", vec![[0., 0.], [180., 0.]])],
                &GeoStreamOptions::default()
            )
            .is_err()
        );
        assert_eq!(map.scene, before);
    }
}
#[test]
fn conformal_caps_seam_and_precision_errors_are_distinct() {
    let view = GeoMapOptions {
        projection: GeoProjection::conic_conformal_default(),
        ..Default::default()
    };
    assert!(project_geo_point([0., 90.], &view).unwrap().is_none());
    let lines = geographic_lines(
        &config(),
        &[GeoLineFeature::new("pole", vec![[0., 80.], [0., 90.]])],
        &view,
        &GeoStreamOptions::default(),
    )
    .unwrap();
    assert_eq!(lines.metadata.lines[0].components, 1);
    let seam = geographic_lines(
        &config(),
        &[GeoLineFeature::new("seam", vec![[170., 30.], [-170., 30.]])],
        &view,
        &GeoStreamOptions::default(),
    )
    .unwrap();
    assert_eq!(seam.metadata.lines[0].components, 2);
    let view = GeoMapOptions {
        projection: GeoProjection::conic_equal_area([0.0000572957796; 2]).unwrap(),
        scale: Some(0.1),
        ..Default::default()
    };
    let c = geographic_map_layout(&config(), &[], &view)
        .unwrap()
        .projection;
    let q = c.project([-45., -90.]).unwrap().unwrap();
    assert!(c.invert(q).unwrap_err().0.contains("precision"));
    assert!(c.invert([f64::NAN, 0.]).is_err());
}

fn interpolate(a: [f64; 2], b: [f64; 2], t: f64) -> [f64; 2] {
    let (a, b) = (vector(a), vector(b));
    let angle = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2])
        .clamp(-1., 1.)
        .acos();
    let u = ((1. - t) * angle).sin() / angle.sin();
    let v = (t * angle).sin() / angle.sin();
    let [x, y, z] = [
        a[0] * u + b[0] * v,
        a[1] * u + b[1] * v,
        a[2] * u + b[2] * v,
    ];
    [y.atan2(x).to_degrees(), z.atan2(x.hypot(y)).to_degrees()]
}
#[test]
fn datafit_encloses_continuous_curved_edges_without_changing_path_vertices() {
    for projection in [
        GeoProjection::Albers,
        GeoProjection::conic_equal_area_default(),
        GeoProjection::conic_equidistant_default(),
        GeoProjection::conic_conformal_default(),
    ] {
        let view = GeoMapOptions {
            projection,
            fit: GeoFit::Data,
            padding: 2.,
            ..Default::default()
        };
        let map = choropleth_layout_with_path_options(
            &config(),
            &land(),
            &view,
            &GeoPathOptions {
                precision: 0.0001,
                ..Default::default()
            },
        )
        .unwrap();
        let c = &map.projection;
        let r = c.plot();
        for ring in &land()[0].polygons[0] {
            for edge in ring.windows(2) {
                for i in 0..=1000 {
                    if let Some(q) = c
                        .project(interpolate(edge[0], edge[1], i as f64 / 1000.))
                        .unwrap()
                    {
                        assert!(
                            q[0] >= r.x
                                && q[0] <= r.x + r.width
                                && q[1] >= r.y
                                && q[1] <= r.y + r.height,
                            "{q:?}"
                        );
                    }
                }
            }
        }
        let svg = render::to_svg(&map.scene).unwrap();
        assert!(svg.contains("<path"));
        assert!(!svg.contains("NaN"));
        let route = [[-140., 15.], [-55., 58.]];
        let stream = geographic_lines(
            &config(),
            &[GeoLineFeature::new("arc", route.to_vec())],
            &view,
            &GeoStreamOptions::default(),
        )
        .unwrap();
        let c = &stream.projection;
        let r = c.plot();
        for i in 0..=10000 {
            if let Some(q) = c
                .project(interpolate(route[0], route[1], i as f64 / 10000.))
                .unwrap()
            {
                assert!(
                    q[0] >= r.x && q[0] <= r.x + r.width && q[1] >= r.y && q[1] <= r.y + r.height
                );
            }
        }
    }
}
