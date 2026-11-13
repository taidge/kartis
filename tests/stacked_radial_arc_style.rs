use glory::render::{GraphicPoint, GraphicStroke, PathCommand as C, StrokeCap};
use kartis::*;
use std::f64::consts::{PI, TAU};

fn config() -> ChartConfig {
    ChartConfig {
        width: 220.,
        height: 220.,
        margin: Margin {
            left: 10.,
            right: 10.,
            top: 10.,
            bottom: 10.,
        },
        ..Default::default()
    }
}

fn data() -> (Vec<String>, Vec<BarSeries>) {
    (
        vec!["narrow".into(), "wide".into()],
        vec![BarSeries::new("ring", vec![Some(2.), Some(8.)])],
    )
}

fn options() -> StackedRadialBarOptions {
    StackedRadialBarOptions {
        inner_radius: 0.4,
        end_angle: PI,
        padding: 0.,
        axes: PolarAxesOptions {
            radial_start: None,
            circular_outer: None,
            grid: PolarGridStyle {
                radial: false,
                circular: false,
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    }
}

fn matches_reference(point: GraphicPoint, [x, y]: [f64; 2]) -> bool {
    (point.x - (110. + x)).hypot(point.y - (110. + y)) < 1e-7
}

#[test]
fn padded_corner_endpoints_match_d3_for_bars_and_tracks_in_both_directions() {
    let c = config();
    let (categories, mut rows) = data();
    let mut o = options();
    o.pad_angle = 0.08;
    let mut style = StackedRadialArcStyle {
        corner_radius: 12.,
        ..Default::default()
    };
    let legacy = layout_stacked_radial_bar(&c, &categories, &rows, &o).unwrap();
    let layout =
        layout_stacked_radial_bar_with_arc_style(&c, &categories, &rows, &o, &style).unwrap();
    assert_eq!(legacy.rings, layout.rings);
    assert_eq!(legacy.segments, layout.segments);
    assert_ne!(legacy.scene.marks, layout.scene.marks);
    assert_eq!(layout.scene.marks.len(), 3);

    // d3-shape 3.2.0 arc().digits(9), d3-scale with Nivo's band and linear
    // value scale wiring. Tangencies become cubic endpoints in Kartis.
    let track = layout.scene.marks[0].geometry.commands();
    let narrow = layout.scene.marks[1].geometry.commands();
    for path in [&track, &narrow] {
        assert!(
            matches!(path[0], C::Move(p) if matches_reference(p, [4.306983102, -86.475905905]))
        );
        assert!(path.iter().any(
            |c| matches!(c, C::Cubic { end, .. } if matches_reference(*end, [18.530662616, -98.268074892]))
        ));
    }
    assert!(
        narrow.iter().any(
            |c| matches!(c, C::Line(p) if matches_reference(*p, [25.391834726, -42.276339560]))
        )
    );
    assert!(narrow.iter().any(
        |c| matches!(c, C::Cubic { end, .. } if matches_reference(*end, [4.306983102, -49.127223145]))
    ));

    o.start_angle = 2.7;
    o.end_angle = -0.3;
    o.pad_angle = 0.11;
    rows[0].values = vec![Some(1.), Some(5.)];
    style.corner_radius = 30.;
    style.pad_radius = PiePadRadius::Fixed(35.);
    let layout =
        layout_stacked_radial_bar_with_arc_style(&c, &categories, &rows, &o, &style).unwrap();
    let track = layout.scene.marks[0].geometry.commands();
    let narrow = layout.scene.marks[1].geometry.commands();
    assert!(matches!(track[0], C::Move(p) if matches_reference(p, [28.363740596, 55.498256205])));
    assert!(matches!(narrow[0], C::Move(p) if matches_reference(p, [35.574608299, 70.752003309])));
    assert!(
        narrow.iter().any(
            |c| matches!(c, C::Line(p) if matches_reference(*p, [38.500224567, 30.403913588]))
        )
    );
    assert!(narrow.iter().any(
        |c| matches!(c, C::Cubic { end, .. } if matches_reference(*end, [22.689636232, 43.495352176]))
    ));
    let legacy = layout_stacked_radial_bar(&c, &categories, &rows, &o).unwrap();
    assert_eq!(layout.segments, legacy.segments);
}

#[test]
fn corners_change_painted_and_pickable_ink_without_losing_owned_labels() {
    let c = config();
    let (categories, rows) = data();
    let mut o = options();
    o.tracks = false;
    o.labels = Some(Default::default());
    o.colors
        .insert(categories[0].clone(), Color(220, 30, 40, 255));
    let style = StackedRadialArcStyle {
        corner_radius: 12.,
        ..Default::default()
    };
    let legacy = layout_stacked_radial_bar(&c, &categories, &rows, &o).unwrap();
    let layout =
        layout_stacked_radial_bar_with_arc_style(&c, &categories, &rows, &o, &style).unwrap();
    assert_eq!(layout.segments, legacy.segments);
    let corner = layout.frame.point(98., 0.025);
    let interior = layout.frame.point(80., 0.25);
    let picked =
        |scene: &Scene, [x, y]: [f64; 2]| scene.hit_test(x, y).map(|mark| mark.series.clone());
    assert_eq!(picked(&legacy.scene, corner), Some(categories[0].clone()));
    assert_eq!(picked(&layout.scene, corner), None);
    assert_eq!(picked(&layout.scene, interior), Some(categories[0].clone()));
    assert_ne!(
        render::to_svg(&layout.scene).unwrap(),
        render::to_svg(&legacy.scene).unwrap()
    );

    #[cfg(feature = "raster")]
    {
        use render::raster::*;
        let render = |scene: &Scene| {
            let mut ink = c.scene().unwrap();
            ink.marks.clone_from(&scene.marks);
            render_raster(&ink, &FontRegistry::new(), &Default::default()).unwrap()
        };
        let pixel = |image: &RenderedImage, [x, y]: [f64; 2]| {
            let index = (y as usize * 220 + x as usize) * 4;
            <[u8; 4]>::try_from(&image.rgba[index..index + 4]).unwrap()
        };
        let legacy = render(&legacy.scene);
        let rounded = render(&layout.scene);
        let red = [220, 30, 40, 255];
        let Color(r, g, b, a) = layout.scene.background;
        assert_eq!(pixel(&legacy, corner), red);
        assert_eq!(pixel(&rounded, corner), [r, g, b, a]);
        assert_eq!(pixel(&rounded, interior), red);
    }

    o.hidden.insert(categories[0].clone());
    let hidden = stacked_radial_bar_with_arc_style(&c, &categories, &rows, &o, &style).unwrap();
    assert_eq!(hidden, layout.scene.without_series(&o.hidden));
}

#[test]
fn padded_collapse_keeps_raw_slots_and_only_the_visible_border() {
    let c = config();
    let (mut categories, _) = data();
    categories.push("zero".into());
    let rows = [BarSeries::new("ring", vec![Some(1.), Some(2.), Some(0.)])];
    let mut o = options();
    o.end_angle = 0.14;
    o.pad_angle = 0.135;
    o.labels = Some(Default::default());
    let style = StackedRadialArcStyle {
        corner_radius: 20.,
        ..Default::default()
    };
    let empty =
        layout_stacked_radial_bar_with_arc_style(&c, &categories, &rows, &o, &style).unwrap();
    assert!(empty.scene.marks.is_empty());
    assert!(empty.scene.labels.is_empty());
    assert!(empty.segments.iter().all(|segment| !segment.visible));
    assert_eq!(empty.segments[0].cumulative, [0., 1.]);

    o.border = Some(GraphicStroke {
        color: Color(30, 70, 90, 255),
        width: 2.,
        cap: StrokeCap::Butt,
    });
    o.paints.insert(
        categories[0].clone(),
        FillPaint::Linear {
            start: [0., 0.],
            end: [100., 100.],
            stops: vec![
                GradientStop::new(0., Color(150, 30, 20, 255)),
                GradientStop::new(1., Color(20, 30, 150, 255)),
            ],
        },
    );
    let stroked =
        layout_stacked_radial_bar_with_arc_style(&c, &categories, &rows, &o, &style).unwrap();
    assert_eq!(stroked.scene.marks.len(), 2);
    assert!(stroked.scene.labels.is_empty());
    assert!(stroked.segments[0].visible);
    assert!(!stroked.segments[2].visible);
    let first = &stroked.scene.marks[0];
    assert!(!first.has_fill());
    assert_eq!(first.stroke, o.border);
    assert!(matches!(
        first.geometry.commands().as_slice(),
        [C::Move(start), C::Line(end), C::Close]
            if matches_reference(*start, [2.333121611, -99.972779013])
                && matches_reference(*end, [0.933248644, -39.989111605])
    ));
    let [x, y] = stroked.frame.point(70., 0.14 / 6.);
    assert_eq!(stroked.scene.hit_test(x, y), Some(first));
    o.opacity = 0.;
    let transparent =
        stacked_radial_bar_with_arc_style(&c, &categories, &rows, &o, &style).unwrap();
    assert!(transparent.marks.is_empty());
}

#[test]
fn full_counterclockwise_ring_keeps_its_hole_and_ignores_corner_and_padding() {
    let c = config();
    let categories = ["total".into()];
    let rows = [BarSeries::new("ring", vec![Some(1.)])];
    let mut o = options();
    o.start_angle = 0.2;
    o.end_angle = 0.2 - TAU;
    let plain =
        stacked_radial_bar_with_arc_style(&c, &categories, &rows, &o, &Default::default()).unwrap();
    o.pad_angle = 1.;
    let style = StackedRadialArcStyle {
        corner_radius: 20.,
        ..Default::default()
    };
    let styled = stacked_radial_bar_with_arc_style(&c, &categories, &rows, &o, &style).unwrap();
    assert_eq!(styled, plain);
    assert!(styled.hit_test(110., 110.).is_none());
    assert_eq!(styled.hit_test(180., 110.).unwrap().series, "total");
    assert!(matches!(
        styled.marks[0].geometry.commands()[0],
        C::Move(p) if matches_reference(p, [19.866933080, -98.006657784])
    ));
    for style in [
        StackedRadialArcStyle {
            corner_radius: f64::NAN,
            ..style
        },
        StackedRadialArcStyle {
            pad_radius: PiePadRadius::Fixed(-1.),
            ..style
        },
    ] {
        assert!(stacked_radial_bar_with_arc_style(&c, &categories, &rows, &o, &style).is_err());
    }
}
