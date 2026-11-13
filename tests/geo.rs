use std::f64::consts::{FRAC_PI_2, PI};

use kartis::*;
fn feature(id: &str, rings: Vec<Vec<[f64; 2]>>) -> GeoFeature {
    GeoFeature {
        id: id.into(),
        label: id.into(),
        value: Some(5.),
        polygons: vec![rings],
    }
}
fn rectangle(left: f64, right: f64, bottom: f64, top: f64) -> Vec<[f64; 2]> {
    vec![
        [left, bottom],
        [right, bottom],
        [right, top],
        [left, top],
        [left, bottom],
    ]
}
fn options(projection: GeoProjection) -> GeoMapOptions {
    GeoMapOptions {
        projection,
        border: None,
        ..Default::default()
    }
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-10, "{a} != {b}");
}
#[test]
fn known_projection_points_rotation_and_visibility() {
    let p = project_geo_point([90., 45.], &options(GeoProjection::Equirectangular))
        .unwrap()
        .unwrap();
    close(p[0], FRAC_PI_2);
    close(p[1], PI / 4.);
    let p = project_geo_point([0., 45.], &options(GeoProjection::Mercator))
        .unwrap()
        .unwrap();
    close(p[0], 0.);
    close(p[1], (1. + 2_f64.sqrt()).ln());
    let p = project_geo_point([90., 0.], &options(GeoProjection::EqualEarth))
        .unwrap()
        .unwrap();
    close(p[0], 1.3533149918480374);
    close(p[1], 0.);
    let p = project_geo_point([90., 0.], &options(GeoProjection::NaturalEarth))
        .unwrap()
        .unwrap();
    close(p[0], 1.3676923617403155);
    close(p[1], 0.);
    let p = project_geo_point([30., 0.], &options(GeoProjection::Orthographic))
        .unwrap()
        .unwrap();
    close(p[0], 0.5);
    close(p[1], 0.);
    assert!(
        project_geo_point([150., 0.], &options(GeoProjection::Orthographic))
            .unwrap()
            .is_none()
    );
    let p = project_geo_point(
        [75., 30.],
        &GeoMapOptions {
            center: [75., 30.],
            ..Default::default()
        },
    )
    .unwrap()
    .unwrap();
    close(p[0], 0.);
    close(p[1], 0.);
    let p = project_geo_point(
        [30., 0.],
        &GeoMapOptions {
            projection: GeoProjection::Orthographic,
            rotation: 90.,
            ..Default::default()
        },
    )
    .unwrap()
    .unwrap();
    close(p[0], 0.);
    close(p[1], -0.5);
    assert!(
        project_geo_point([0., 89.], &options(GeoProjection::Mercator))
            .unwrap()
            .is_none()
    );
}
#[test]
fn dateline_splits_boundaries_and_keeps_holes() {
    let c = ChartConfig::default();
    let land = feature(
        "dateline",
        vec![
            rectangle(160., -160., -20., 20.),
            rectangle(170., -170., -8., 8.),
        ],
    );
    let o = options(GeoProjection::Equirectangular);
    let scene = geographic_map(&c, &[land], &o).unwrap();
    assert_eq!(scene.marks.len(), 1);
    let scale = (c.inner_width() / (2. * PI)).min(c.inner_height() / PI);
    let center = [
        c.margin.left + c.inner_width() / 2.,
        c.margin.top + c.inner_height() / 2.,
    ];
    let pixel = |lon: f64, lat: f64| {
        (
            center[0] + lon.to_radians() * scale,
            center[1] - lat.to_radians() * scale,
        )
    };
    let (x, y) = pixel(165., 0.);
    assert!(scene.hit_test(x, y).is_some());
    let (x, y) = pixel(175., 0.);
    assert!(
        scene.hit_test(x, y).is_none(),
        "clipped hole must stay empty"
    );
    let (x, y) = pixel(-165., 0.);
    assert!(scene.hit_test(x, y).is_some());
    let (x, y) = pixel(0., 0.);
    assert!(
        scene.hit_test(x, y).is_none(),
        "dateline polygons must not fill across the entire world"
    );
    if let Geometry::Path(path) = &scene.marks[0].geometry {
        assert_eq!(
            path.iter()
                .filter(|p| matches!(p, glory::render::PathCommand::Move(_)))
                .count(),
            4
        );
    }
}
#[test]
fn orthographic_exact_horizon_arcs_visible_holes_and_hidden_polygons() {
    let c = ChartConfig::default();
    let o = options(GeoProjection::Orthographic);
    let visible = feature(
        "visible",
        vec![
            rectangle(-30., 30., -30., 30.),
            rectangle(-10., 10., -10., 10.),
        ],
    );
    let hidden = feature("hidden", vec![rectangle(150., 170., -10., 10.)]);
    let clipped = feature("clipped", vec![rectangle(60., 120., -20., 20.)]);
    let scene = geographic_map(&c, &[visible, hidden, clipped], &o).unwrap();
    assert_eq!(scene.marks.len(), 2);
    assert!(scene.marks.iter().all(|m| m.series != "hidden"));
    let cx = c.margin.left + c.inner_width() / 2.;
    let cy = c.margin.top + c.inner_height() / 2.;
    let r = c.inner_width().min(c.inner_height()) / 2.;
    assert!(
        scene.hit_test(cx, cy).is_none(),
        "visible hole must remain empty"
    );
    assert!(scene.hit_test(cx + r * 0.3, cy).is_some());
    assert!(
        scene.hit_test(cx + r * 0.98, cy).is_some(),
        "horizon closure must follow the disk arc, not a straight chord"
    );
    assert!(scene.hit_test(cx + r * 1.01, cy).is_none());
    if let Geometry::Path(commands) = &scene
        .marks
        .iter()
        .find(|m| m.series == "clipped")
        .unwrap()
        .geometry
    {
        let mut horizon = 0;
        for cmd in commands {
            if let glory::render::PathCommand::Line(p) = cmd
                && ((p.x - cx).hypot(p.y - cy) - r).abs() < 1e-7
            {
                horizon += 1;
            }
        }
        assert!(horizon >= 5, "curved horizon needs intermediate arc points");
    }
}
#[test]
fn fit_and_projection_profiles_reject_ambiguous_geometry() {
    let c = ChartConfig::default();
    let land = feature("area", vec![rectangle(-10., 10., -10., 10.)]);
    for projection in [
        GeoProjection::Mercator,
        GeoProjection::EqualEarth,
        GeoProjection::NaturalEarth,
    ] {
        let scene = choropleth_with_options(
            &c,
            std::slice::from_ref(&land),
            &GeoMapOptions {
                projection,
                fit: GeoFit::Data,
                padding: 12.,
                ..Default::default()
            },
        )
        .unwrap();
        scene.validate().unwrap();
        assert_eq!(scene.marks.len(), 1);
    }
    let ambiguous = feature(
        "antipodal",
        vec![vec![[0., 0.], [180., 0.], [90., 10.], [0., 0.]]],
    );
    assert!(geographic_map(&c, &[ambiguous], &GeoMapOptions::default()).is_err());
    let bad_hole = feature(
        "bad-hole",
        vec![rectangle(-10., 10., -10., 10.), rectangle(40., 50., 0., 5.)],
    );
    assert!(geographic_map(&c, &[bad_hole], &GeoMapOptions::default()).is_err());
    let polar = feature(
        "pole",
        vec![vec![[0., 80.], [120., 80.], [-120., 80.], [0., 80.]]],
    );
    assert!(
        geographic_map(&c, &[polar], &GeoMapOptions::default()).is_err(),
        "pole-winding rings must be explicitly rejected"
    );
    let concave = feature(
        "concave",
        vec![vec![
            [60., -30.],
            [120., -30.],
            [120., 30.],
            [80., 10.],
            [60., 30.],
            [60., -30.],
        ]],
    );
    geographic_map(&c, &[concave], &options(GeoProjection::Orthographic))
        .unwrap()
        .validate()
        .unwrap();
}

#[test]
fn orthographic_holes_crossing_the_horizon_remain_subtracted() {
    let c = ChartConfig::default();
    let land = feature(
        "limb-hole",
        vec![
            rectangle(60., 120., -30., 30.),
            rectangle(80., 110., -10., 10.),
        ],
    );
    let scene = geographic_map(&c, &[land], &options(GeoProjection::Orthographic)).unwrap();
    let cx = c.margin.left + c.inner_width() / 2.;
    let cy = c.margin.top + c.inner_height() / 2.;
    let r = c.inner_height().min(c.inner_width()) / 2.;
    assert!(scene.hit_test(cx + r * 0.95, cy).is_some());
    assert!(
        scene.hit_test(cx + r * 0.995, cy).is_none(),
        "a horizon-crossing hole must open a notch at the globe limb"
    );
}
