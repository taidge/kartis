use std::f64::consts::{FRAC_PI_2, PI};

use glory::render::PathCommand as Cmd;
use kartis::*;
fn projections() -> [GeoProjection; 3] {
    [
        GeoProjection::AzimuthalEqualArea,
        GeoProjection::AzimuthalEquidistant,
        GeoProjection::Stereographic,
    ]
}
fn options(projection: GeoProjection) -> GeoMapOptions {
    GeoMapOptions {
        projection,
        ..Default::default()
    }
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-7, "{a} != {b}");
}
fn rectangle(a: f64, b: f64, c: f64, d: f64) -> Vec<[f64; 2]> {
    vec![[a, c], [b, c], [b, d], [a, d], [a, c]]
}
fn land(id: &str, rings: Vec<Vec<[f64; 2]>>) -> GeoFeature {
    GeoFeature {
        id: id.into(),
        label: id.into(),
        value: Some(1.),
        polygons: vec![rings],
    }
}

#[test]
fn azimuthal_known_points_match_primary_d3_geo_3_1_1_raw_goldens() {
    // Captured from official d3-geo 3.1.1 raw functions, without fitting.
    let input = [
        [0., 0.],
        [30., 0.],
        [0., 30.],
        [30., 30.],
        [90., 0.],
        [45., 45.],
    ];
    let goldens = [
        [
            [0., 0.],
            [0.5176380902050414, 0.],
            [0., 0.5176380902050414],
            [0.4629100498862757, 0.5345224838248487],
            [std::f64::consts::SQRT_2, 0.],
            [0.5773502691896257, 0.8164965809277259],
        ],
        [
            [0., 0.],
            [0.5235987755982988, 0.],
            [0., 0.5235987755982988],
            [0.47314062827742126, 0.5463357382010355],
            [FRAC_PI_2, 0.],
            [0.6045997880780726, 0.8550332201079093],
        ],
        [
            [0., 0.],
            [0.26794919243112264, 0.],
            [0., 0.26794919243112264],
            [0.24743582965269675, 0.2857142857142857],
            [1., 0.],
            [0.3333333333333333, 0.4714045207910316],
        ],
    ];
    for (projection, golden) in projections().into_iter().zip(goldens) {
        for (p, expected) in input.into_iter().zip(golden) {
            let actual = project_geo_point(p, &options(projection)).unwrap().unwrap();
            close(actual[0], expected[0]);
            close(actual[1], expected[1]);
        }
    }
}
#[test]
fn spherical_area_distance_and_stereographic_radius_properties_hold() {
    for projection in projections() {
        let p = project_geo_point([25., 20.], &options(projection))
            .unwrap()
            .unwrap();
        let angle = (25_f64.to_radians().cos() * 20_f64.to_radians().cos()).acos();
        let radius = match projection {
            GeoProjection::AzimuthalEqualArea => 2. * (angle / 2.).sin(),
            GeoProjection::AzimuthalEquidistant => angle,
            _ => (angle / 2.).tan(),
        };
        close(p[0].hypot(p[1]), radius);
    }
    // Lambert equal-area's planar Jacobian equals spherical area density cosφ.
    let h = 1e-5;
    let project = |l: f64, f: f64| {
        project_geo_point(
            [l.to_degrees(), f.to_degrees()],
            &options(GeoProjection::AzimuthalEqualArea),
        )
        .unwrap()
        .unwrap()
    };
    for (l, f) in [(0.2, 0.4), (0.7, -0.3), (-0.4, 0.6)] {
        let xp = project(l + h, f);
        let xm = project(l - h, f);
        let yp = project(l, f + h);
        let ym = project(l, f - h);
        let determinant =
            ((xp[0] - xm[0]) * (yp[1] - ym[1]) - (xp[1] - xm[1]) * (yp[0] - ym[0])) / (4. * h * h);
        close(determinant, f.cos());
    }
}
#[test]
fn hemisphere_horizon_holes_rotation_and_symbol_visibility_share_one_projection() {
    let c = ChartConfig::default();
    for projection in projections() {
        let o = options(projection);
        assert_eq!(project_geo_point([120., 0.], &o).unwrap(), None);
        let mut rotated = o.clone();
        rotated.center = [120., 20.];
        rotated.rotation = 37.;
        let center = project_geo_point([120., 20.], &rotated).unwrap().unwrap();
        close(center[0], 0.);
        close(center[1], 0.);
        let features = [
            land(
                "hole",
                vec![
                    rectangle(-30., 30., -30., 30.),
                    rectangle(-10., 10., -10., 10.),
                ],
            ),
            land("horizon", vec![rectangle(60., 120., -20., 20.)]),
            land("hidden", vec![rectangle(150., 170., -10., 10.)]),
        ];
        let mut layout = geographic_map_layout(&c, &features, &o).unwrap();
        assert_eq!(layout.scene.marks.len(), 2);
        let center = layout.projection.project([0., 0.]).unwrap().unwrap();
        assert!(layout.scene.hit_test(center[0], center[1]).is_none());
        let inside = layout.projection.project([20., 0.]).unwrap().unwrap();
        assert!(layout.scene.hit_test(inside[0], inside[1]).is_some());
        let edge = layout.projection.project([89., 0.]).unwrap().unwrap();
        assert!(
            layout.scene.hit_test(edge[0], edge[1]).is_some(),
            "horizon is a curved boundary, not a chord"
        );
        layout
            .add_symbols(
                &[
                    GeoSymbol::new("front", [0., 0.], Some(1.)),
                    GeoSymbol::new("back", [150., 0.], Some(10.)),
                ],
                &GeoSymbolOptions {
                    clip: false,
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(
            layout
                .scene
                .marks
                .iter()
                .filter(|m| m.id.starts_with("geo-symbol:"))
                .count(),
            1
        );
        let horizon = layout
            .scene
            .marks
            .iter()
            .find(|m| m.series == "horizon")
            .unwrap();
        let cx = c.margin.left + c.inner_width() / 2.;
        let cy = c.margin.top + c.inner_height() / 2.;
        let radius = c.inner_height() / 2.;
        assert!(
            horizon
                .geometry
                .commands()
                .iter()
                .filter(|cmd| match cmd {
                    Cmd::Line(p) => (p.x - cx).hypot(p.y - cy).abs_diff(radius) < 1e-6,
                    _ => false,
                })
                .count()
                >= 5
        );
    }
}
trait Difference {
    fn abs_diff(self, other: Self) -> Self;
}
impl Difference for f64 {
    fn abs_diff(self, other: Self) -> Self {
        (self - other).abs()
    }
}
#[test]
fn data_fit_overlay_and_rejected_horizon_profiles_are_consistent() {
    let c = ChartConfig::default();
    let feature = land("land", vec![rectangle(-30., 30., -15., 15.)]);
    for projection in projections() {
        let o = GeoMapOptions {
            projection,
            fit: GeoFit::Data,
            padding: 10.,
            ..Default::default()
        };
        let mut layout = geographic_map_layout(&c, std::slice::from_ref(&feature), &o).unwrap();
        let before = layout.projection.clone();
        let point = before.project([-30., -15.]).unwrap().unwrap();
        layout
            .add_symbols(
                &[GeoSymbol::new("corner", [-30., -15.], Some(1.))],
                &GeoSymbolOptions::default(),
            )
            .unwrap();
        assert_eq!(layout.projection, before);
        let mark = layout
            .scene
            .marks
            .iter()
            .find(|m| m.id.ends_with(":corner"))
            .unwrap();
        let Geometry::Circle { x, y, .. } = mark.geometry else {
            panic!()
        };
        close(x, point[0]);
        close(y, point[1]);
        let concave = land(
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
        geographic_map(&c, &[concave], &o)
            .unwrap()
            .validate()
            .unwrap();
        layout.scene.validate().unwrap();
    }
}
fn area(geometry: &Geometry) -> f64 {
    match geometry {
        Geometry::Circle { radius, .. } => PI * radius * radius,
        Geometry::Rect { width, height, .. } => width * height,
        Geometry::Path(path) => {
            let points = path
                .iter()
                .filter_map(|c| match c {
                    Cmd::Move(p) | Cmd::Line(p) => Some((p.x, p.y)),
                    _ => None,
                })
                .collect::<Vec<_>>();
            (0..points.len())
                .map(|i| {
                    let a = points[i];
                    let b = points[(i + 1) % points.len()];
                    a.0 * b.1 - b.0 * a.1
                })
                .sum::<f64>()
                .abs()
                / 2.
        }
    }
}
#[test]
fn noncircular_symbols_and_size_legends_preserve_equal_area_and_identity() {
    for shape in [
        GeoSymbolShape::Circle,
        GeoSymbolShape::Square,
        GeoSymbolShape::Diamond,
        GeoSymbolShape::Triangle,
        GeoSymbolShape::Cross,
    ] {
        let mut layout =
            geographic_map_layout(&ChartConfig::default(), &[], &Default::default()).unwrap();
        let options = GeoSymbolOptions {
            shape,
            domain: Some([0., 4.]),
            radius_range: [0., 20.],
            stroke: None,
            size_legend: Some(GeoSizeLegend {
                layout: GeoLegendOptions::default(),
                values: vec![1., 4.],
            }),
            ..Default::default()
        };
        let mut data = vec![
            GeoSymbol::new("one", [-20., 0.], Some(1.)),
            GeoSymbol::new("four", [20., 0.], Some(4.)),
        ];
        let before = layout.clone();
        layout.add_symbols(&data, &options).unwrap();
        let one = layout
            .scene
            .marks
            .iter()
            .find(|m| m.id.ends_with(":one"))
            .unwrap();
        let four = layout
            .scene
            .marks
            .iter()
            .find(|m| m.id.ends_with(":four"))
            .unwrap();
        close(area(&one.geometry), PI * 100.);
        close(area(&four.geometry), PI * 400.);
        for (i, value) in [1., 4.].into_iter().enumerate() {
            let m = layout
                .scene
                .marks
                .iter()
                .find(|m| m.id == format!("geo-size-legend:{i}"))
                .unwrap();
            close(area(&m.geometry), value * PI * 100.);
            assert!(m.label.is_empty() && m.series.is_empty());
        }
        data.reverse();
        let mut reordered = before;
        reordered.add_symbols(&data, &options).unwrap();
        assert_eq!(layout.scene, reordered.scene);
        let center = layout.projection.project([-20., 0.]).unwrap().unwrap();
        let mut interaction = interaction::Interaction::default();
        assert_eq!(
            interaction.pointer(&layout.scene, center[0], center[1]),
            Some("geo-symbol:3:one")
        );
        render::to_svg(&layout.scene).unwrap();
        render::to_canvas_script(&layout.scene, "symbols").unwrap();
    }
}
#[test]
fn cross_and_triangle_hits_follow_shape_interior_and_not_bounding_boxes() {
    for (shape, offset) in [
        (GeoSymbolShape::Cross, [18., 18.]),
        (GeoSymbolShape::Triangle, [18., -18.]),
        (GeoSymbolShape::Diamond, [10., 20.]),
    ] {
        let mut layout =
            geographic_map_layout(&ChartConfig::default(), &[], &Default::default()).unwrap();
        let center = layout.projection.project([0., 0.]).unwrap().unwrap();
        layout
            .add_symbols(
                &[GeoSymbol::new("s", [0., 0.], Some(1.))],
                &GeoSymbolOptions {
                    shape,
                    domain: Some([0., 1.]),
                    radius_range: [0., 20.],
                    stroke: None,
                    ..Default::default()
                },
            )
            .unwrap();
        let mut interaction = interaction::Interaction::default();
        assert!(
            interaction
                .pointer(&layout.scene, center[0], center[1])
                .is_some()
        );
        assert!(
            interaction
                .pointer(&layout.scene, center[0] + offset[0], center[1] + offset[1])
                .is_none(),
            "{shape:?} must not pick its empty corner"
        );
    }
}

#[cfg(feature = "raster")]
#[test]
fn every_azimuthal_symbol_shape_replays_real_cpu_pixels_and_backfaces_stay_empty() {
    use kartis::render::raster::{FontRegistry, RasterOptions, render_raster};
    for projection in projections() {
        for shape in [
            GeoSymbolShape::Circle,
            GeoSymbolShape::Square,
            GeoSymbolShape::Diamond,
            GeoSymbolShape::Triangle,
            GeoSymbolShape::Cross,
        ] {
            let mut layout =
                geographic_map_layout(&ChartConfig::default(), &[], &options(projection)).unwrap();
            let center = layout.projection.project([0., 0.]).unwrap().unwrap();
            layout
                .add_symbols(
                    &[
                        GeoSymbol::new("front", [0., 0.], Some(1.)),
                        GeoSymbol::new("back", [150., 0.], Some(1.)),
                    ],
                    &GeoSymbolOptions {
                        shape,
                        domain: Some([0., 1.]),
                        radius_range: [0., 20.],
                        color: GeoSymbolColor::Fixed(Color(31, 112, 190, 255)),
                        stroke: None,
                        ..Default::default()
                    },
                )
                .unwrap();
            assert_eq!(layout.scene.marks.len(), 1);
            let image = render_raster(
                &layout.scene,
                &FontRegistry::new(),
                &RasterOptions::default(),
            )
            .unwrap();
            let i = (center[1] as usize * image.width as usize + center[0] as usize) * 4;
            assert_eq!(&image.rgba[i..i + 4], &[31, 112, 190, 255]);
            let area = image
                .rgba
                .as_chunks::<4>()
                .0
                .iter()
                .map(|p| (255. - f64::from(p[0])) / (255. - 31.))
                .sum::<f64>();
            assert!(
                (area - PI * 400.).abs() < 12.,
                "{projection:?} {shape:?}: raster area {area}"
            );
        }
    }
}
