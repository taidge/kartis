use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::*;
use std::{
    collections::BTreeMap,
    f64::consts::{FRAC_PI_2, PI, TAU},
};
fn config() -> ChartConfig {
    ChartConfig {
        width: 400.,
        height: 300.,
        margin: Margin {
            left: 50.,
            right: 50.,
            top: 30.,
            bottom: 30.,
        },
        ..Default::default()
    }
}
fn frame() -> PolarFrame {
    PolarFrame {
        center: [200., 150.],
        inner_radius: 40.,
        outer_radius: 100.,
        start_angle: 0.,
        end_angle: PI,
    }
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}
#[test]
fn numeric_and_category_ticks_keep_real_scale_format_and_identity() {
    let scale = ChartScale::new(ScaleKind::Linear, [0., 100.], [40., 100.]).unwrap();
    let t = polar_numeric_ticks(
        &scale,
        &PolarTickOptions {
            values: Some(vec![0., 25., 50., 100., 100., 200.]),
            format: ValueFormat::Fixed(1),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(t.len(), 4);
    assert_eq!(t[1].label, "25.0");
    close(t[1].position, 55.);
    let ids = vec!["x:tick".into(), "y-grid-".into()];
    let labels = BTreeMap::from([("x:tick".into(), "Cost".into())]);
    let ticks = polar_category_ticks(&ids, [6., 2.], &labels).unwrap();
    assert_eq!(ticks[0].id, "x:tick");
    assert_eq!(ticks[0].label, "Cost");
    close(ticks[0].position, 5.);
    close(ticks[1].position, 3.);
    let log = ChartScale::new(ScaleKind::Log, [1., 100.], [40., 100.]).unwrap();
    assert!(
        polar_numeric_ticks(
            &log,
            &PolarTickOptions {
                values: Some(vec![0.]),
                ..Default::default()
            }
        )
        .is_err()
    );
}
#[test]
fn axes_ticks_grid_and_rotation_are_actual_scene_geometry() {
    let c = config();
    let f = frame();
    let radial = vec![PolarTick::new("mid", "Radius", 70.)];
    let circular = vec![PolarTick::new("east", "Quarter", FRAC_PI_2)];
    let mut o = PolarAxesOptions::default();
    o.circular_outer.as_mut().unwrap().rotation = PolarTextRotation::Tangential;
    let s = polar_axes(&c, &f, &radial, &circular, &o).unwrap();
    let m = |id: &str| s.marks.iter().find(|m| m.id == id).unwrap();
    assert_eq!(
        m("polar:axis-start").geometry,
        Geometry::Path(vec![
            C::Move(P::new(200., 110.)),
            C::Line(P::new(200., 50.))
        ])
    );
    assert_eq!(
        m("polar:tick-start:3:mid").geometry,
        Geometry::Path(vec![C::Move(P::new(200., 80.)), C::Line(P::new(195., 80.))])
    );
    let Geometry::Path(path) = &m("polar:tick-outer:4:east").geometry else {
        panic!()
    };
    let C::Line(end) = path[1] else { panic!() };
    close(end.x, 305.);
    close(end.y, 150.);
    let label = s.labels.iter().find(|l| l.text == "Quarter").unwrap();
    close(label.rotation, 90.);
    assert_eq!(label.anchor, TextAnchor::Middle);
    assert!(s.label_series.is_empty());
    assert!(
        s.marks
            .iter()
            .all(|m| m.label.is_empty() && m.series.is_empty())
    );
    assert!(render::to_svg(&s).unwrap().contains("rotate(90"));
}
#[test]
fn directed_partial_arcs_wrap_and_complete_circle_seam_ticks_are_checked() {
    let c = config();
    let mut f = frame();
    f.start_angle = 5.5;
    f.end_angle = 3.;
    let t = vec![
        PolarTick::new("one", "One", 5.),
        PolarTick::new("two", "Two", 3.5),
    ];
    let s = polar_axes(&c, &f, &[], &t, &Default::default()).unwrap();
    assert_eq!(
        s.marks
            .iter()
            .filter(|m| m.id.starts_with("polar:grid-angle:"))
            .count(),
        2
    );
    f.start_angle = 0.;
    f.end_angle = TAU;
    let t = vec![
        PolarTick::new("zero", "0", 0.),
        PolarTick::new("turn", "100", TAU),
    ];
    let s = polar_axes(&c, &f, &[], &t, &Default::default()).unwrap();
    assert_eq!(
        s.marks
            .iter()
            .filter(|m| m.id.starts_with("polar:tick-outer:"))
            .count(),
        1
    );
    assert_eq!(s.labels.len(), 1);
    // Adding a turn to a large legal endpoint loses a few endpoint ulps.
    // Those are still a single complete circle; a genuine extra arc is not.
    for start in [100000., 999990., -999990.] {
        for direction in [-1., 1.] {
            f.start_angle = start;
            f.end_angle = start + direction * TAU;
            assert!(f.is_full_turn());
            let ticks = [
                PolarTick::new("a", "0", f.start_angle),
                PolarTick::new("b", "1", f.end_angle),
            ];
            let scene = polar_axes(&c, &f, &[], &ticks, &Default::default()).unwrap();
            assert_eq!(scene.labels.len(), 1);
            f.end_angle += direction * 1e-7;
            assert!(f.validate().is_err(), "multi-turn extent must still reject");
        }
    }
    f.start_angle = 0.;
    f.end_angle = 1.;
    assert!(
        polar_axes(
            &c,
            &f,
            &[],
            &[PolarTick::new("outside", "outside", 2.)],
            &Default::default()
        )
        .is_err()
    );
}
#[test]
fn append_is_transactional_and_preserves_existing_mixed_order_and_label_slots() {
    let c = config();
    let mut s = c.scene().unwrap();
    let mut m = Mark::new(
        "datum",
        Geometry::Circle {
            x: 200.,
            y: 100.,
            radius: 5.,
        },
        Color(0, 0, 0, 255),
    );
    m.series = "owner".into();
    s.marks.push(m);
    s.labels.push(Label {
        x: 200.,
        y: 100.,
        text: "Owned".into(),
        size: 12.,
        color: Color(0, 0, 0, 255),
        anchor: TextAnchor::Middle,
        rotation: 0.,
    });
    s.label_series.insert(0, "owner".into());
    s.draw_order = vec![DrawItem::Label(0), DrawItem::Mark(0)];
    append_polar_axes(
        &mut s,
        &c,
        &frame(),
        &[PolarTick::new("mid", "Mid", 70.)],
        &[],
        &Default::default(),
    )
    .unwrap();
    assert_eq!(s.label_series[&0], "owner");
    let order = s.draw_items();
    let a = order.iter().position(|v| *v == DrawItem::Label(0)).unwrap();
    assert_eq!(order[a + 1], DrawItem::Mark(0));
    assert!(a > 0);
    let before = s.clone();
    assert!(append_polar_axes(&mut s, &c, &frame(), &[], &[], &Default::default()).is_err());
    assert_eq!(s, before);
    let mut bad = PolarAxesOptions::default();
    bad.grid.width = f64::NAN;
    assert!(append_polar_axes(&mut s, &c, &frame(), &[], &[], &bad).is_err());
    assert_eq!(s, before);
    let ticks = (0..1001)
        .map(|i| PolarTick::new(i.to_string(), "", 70.))
        .collect::<Vec<_>>();
    assert!(polar_axes(&c, &frame(), &ticks, &[], &Default::default()).is_err());
}
#[test]
fn polar_bar_wrapper_uses_shared_axes_at_actual_area_scaled_radii_without_changing_bars() {
    let c = config();
    let categories = vec!["a".into(), "b".into()];
    let rows = vec![BarSeries::new("s", vec![Some(50.), Some(100.)])];
    let o = PolarBarOptions {
        area_scale: true,
        labels: false,
        ..Default::default()
    };
    let old = polar_bar(&c, &categories, &rows, &o).unwrap();
    let new = polar_bar_with_axes(
        &c,
        &categories,
        &rows,
        &o,
        &Default::default(),
        &PolarTickOptions {
            values: Some(vec![0., 50., 100.]),
            ..Default::default()
        },
    )
    .unwrap();
    for m in old.marks.iter().filter(|m| m.id.starts_with("polar-bar-")) {
        assert_eq!(
            m.geometry,
            new.marks.iter().find(|n| n.id == m.id).unwrap().geometry
        );
    }
    assert!(!new.marks.iter().any(|m| m.id == "polar-grid-1"));
    let grid = new
        .marks
        .iter()
        .find(|m| m.id == "polar:grid-radius:16:4049000000000000")
        .unwrap();
    let C::Move(p) = grid.geometry.commands()[0] else {
        panic!()
    };
    let inner = 120. * o.inner_radius;
    close(
        p.y,
        150. - (inner * inner + (120_f64.powi(2) - inner * inner) * 0.5).sqrt(),
    );
    let zero = vec![BarSeries::new("s", vec![Some(0.), Some(0.)])];
    let zero = polar_bar_with_axes(
        &c,
        &categories,
        &zero,
        &o,
        &Default::default(),
        &PolarTickOptions {
            values: Some(vec![0., 1.]),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(zero.labels.iter().any(|l| l.text == "1"));
}
#[cfg(feature = "raster")]
#[test]
fn cpu_draws_only_the_requested_partial_grid_arc() {
    use render::raster::*;
    let c = config();
    let mut f = frame();
    f.end_angle = FRAC_PI_2;
    let mut o = PolarAxesOptions {
        radial_start: None,
        circular_outer: None,
        ..Default::default()
    };
    o.grid.color = Some(Color(240, 20, 30, 255));
    o.grid.width = 4.;
    let s = polar_axes(&c, &f, &[PolarTick::new("r", "", 80.)], &[], &o).unwrap();
    let image = render_raster(&s, &FontRegistry::new(), &Default::default()).unwrap();
    let pixel = |x: usize, y: usize| &image.rgba[(y * 400 + x) * 4..(y * 400 + x) * 4 + 4];
    assert_eq!(pixel(201, 70), [240, 20, 30, 255]);
    assert_eq!(pixel(120, 150), [255, 255, 255, 255]);
}
