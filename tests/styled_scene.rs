use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use kartis::*;
fn mark() -> Mark {
    let mut m = Mark::new(
        "dash",
        Geometry::Path(vec![C::Move(P::new(20., 50.)), C::Line(P::new(180., 50.))]),
        Color(40, 90, 180, 255),
    );
    m.fill = None;
    m.stroke = Some(GraphicStroke {
        color: Color(40, 90, 180, 255),
        width: 4.,
        cap: StrokeCap::Butt,
    });
    m.stroke_dash = Some(StrokeDash {
        segments: vec![10., 30.],
        offset: 0.,
    });
    m
}
#[test]
fn dashed_hit_testing_keeps_gaps_and_mask_intersections() {
    let mut s = ChartConfig::default().scene().unwrap();
    let mut m = mark();
    m.clip_path = Some(Box::new(ClipPath::circle(70., 50., 45.)));
    s.marks.push(m);
    s.validate().unwrap();
    assert!(s.hit_test(27., 50.).is_some());
    assert!(s.hit_test(45., 50.).is_none());
    assert!(s.hit_test(63., 50.).is_some());
    assert!(s.hit_test(142., 50.).is_none());
    let svg = render::to_svg(&s).unwrap();
    assert!(svg.contains("stroke-dasharray=\"10 30\""));
    assert!(svg.contains("clipPathUnits=\"userSpaceOnUse\""));
    let script = render::to_canvas_script(&s, "canvas").unwrap();
    assert!(script.contains("ctx.setLineDash([10,30])"));
    assert!(script.contains("ctx.setLineDash([]);ctx.lineDashOffset=0"));
}
#[test]
fn dash_fill_and_zero_cap_footprints_are_independent() {
    let mut s = ChartConfig::default().scene().unwrap();
    let mut m = mark();
    m.stroke_dash = Some(StrokeDash {
        segments: vec![0., 30.],
        offset: 0.,
    });
    m.stroke.as_mut().unwrap().cap = StrokeCap::Round;
    s.marks.push(m);
    s.validate().unwrap();
    assert!(s.hit_test(20., 50.).is_some());
    assert!(s.hit_test(35., 50.).is_none());
    let d = dash_geometry_detailed(
        &s.marks[0].geometry,
        s.marks[0].stroke_dash.as_ref().unwrap(),
        &Default::default(),
    )
    .unwrap();
    assert!(!d.dots.is_empty());
    assert!(d.stroke_geometry().commands().is_empty());
    s.marks[0].stroke.as_mut().unwrap().cap = StrokeCap::Butt;
    assert!(s.hit_test(20., 50.).is_none());
    s.marks[0].geometry = Geometry::Rect {
        x: 20.,
        y: 20.,
        width: 160.,
        height: 60.,
    };
    s.marks[0].fill = Some(Color(200, 100, 50, 255));
    assert!(s.hit_test(100., 50.).is_some());
}
#[test]
fn scene_admission_rejects_unbounded_dash_expansion_and_invalid_label_masks() {
    let mut s = ChartConfig::default().scene().unwrap();
    let mut m = mark();
    m.stroke_dash = Some(StrokeDash {
        segments: vec![0.000001, 0.000001],
        offset: 0.,
    });
    s.marks.push(m);
    assert!(s.validate().is_err());
    s.marks.clear();
    s.label_clips.insert(0, ClipPath::circle(20., 20., 10.));
    assert!(s.validate().is_err());
}
#[test]
fn pattern_exports_share_one_tile_and_admit_before_canvas_resize() {
    let mut s = ChartConfig::default().scene().unwrap();
    let pattern = PatternFill::default();
    for i in 0..3 {
        let mut m = Mark::new(
            format!("p{i}"),
            Geometry::Rect {
                x: i as f64 * 30.,
                y: 10.,
                width: 20.,
                height: 50.,
            },
            Color(10, 20, 30, 255),
        );
        m.fill_paint = Some(FillPaint::Pattern(pattern.clone()));
        s.marks.push(m);
    }
    let svg = render::to_svg(&s).unwrap();
    assert_eq!(svg.matches("<pattern ").count(), 1);
    let script = render::to_canvas_script(&s, "canvas").unwrap();
    assert_eq!(script.matches("ctx.createPattern(").count(), 1);
    assert_eq!(script.matches("ctx.fillStyle=pattern0").count(), 3);
    assert!(
        script.find("canvas scene pattern budget exceeded").unwrap()
            < script.find("canvas.width=").unwrap()
    );
}
#[test]
fn browser_dash_profile_rejects_native_saturation_and_normalizes_huge_offsets() {
    let mut s = ChartConfig::default().scene().unwrap();
    let mut m = mark();
    m.stroke_dash = Some(StrokeDash {
        segments: vec![5., 1e308],
        offset: 1e300,
    });
    s.marks.push(m);
    s.validate().unwrap();
    assert!(render::to_svg(&s).is_err());
    assert!(render::to_canvas_script(&s, "canvas").is_err());
    assert!(
        dash_geometry(
            &s.marks[0].geometry,
            s.marks[0].stroke_dash.as_ref().unwrap(),
            &Default::default()
        )
        .unwrap()
        .commands()
        .is_empty()
    );
    let normal = StrokeDash {
        segments: vec![5., 3.],
        offset: 1e300,
    }
    .native_normalized()
    .unwrap();
    assert!(normal.offset >= 0. && normal.offset < 8.);
}
