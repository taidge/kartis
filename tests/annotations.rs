use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as Cmd, StrokeCap};
use kartis::annotations::*;
use kartis::*;

fn scene() -> Scene {
    let mut scene = ChartConfig::default().scene().unwrap();
    let mut mark = Mark::new(
        "data",
        Geometry::Rect {
            x: 10.,
            y: 10.,
            width: 20.,
            height: 10.,
        },
        Color(10, 20, 30, 255),
    );
    mark.series = "series".into();
    scene.marks.push(mark);
    scene
}
fn stroke() -> GraphicStroke {
    GraphicStroke {
        color: Color(255, 0, 0, 255),
        width: 2.,
        cap: StrokeCap::Round,
    }
}
fn text(value: &str) -> Label {
    Label {
        rotation: 0.,
        x: 20.,
        y: 40.,
        text: value.into(),
        color: Color(0, 0, 0, 255),
        size: 12.,
        anchor: TextAnchor::Middle,
    }
}
fn line(id: &str) -> Annotation {
    Annotation::new(
        id,
        AnnotationKind::Line {
            from: (0., 0.),
            to: (100., 100.),
            stroke: stroke(),
        },
    )
}

#[test]
fn annotations_have_stable_identity_captions_and_explicit_layer_order() {
    let mut s = scene();
    s.labels.push(text("existing"));
    annotate(
        &mut s,
        &[
            line("line").label("Threshold").caption(text("Threshold")),
            Annotation::new("text", AnnotationKind::Text(text("Note"))),
        ],
        AnnotationLayer::Back,
    )
    .unwrap();
    assert_eq!(
        s.marks.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(),
        ["line", "text", "data"]
    );
    assert_eq!(
        s.labels.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(),
        ["Threshold", "Note", "existing"]
    );
    assert_eq!(s.marks[0].label, "Threshold");
    assert!(s.marks[1].fill.is_none());
    annotate(
        &mut s,
        &[Annotation::new(
            "circle",
            AnnotationKind::Circle {
                x: 30.,
                y: 30.,
                radius: 4.,
                fill: Some(Color(1, 2, 3, 255)),
                stroke: None,
            },
        )],
        AnnotationLayer::Front,
    )
    .unwrap();
    assert_eq!(s.marks.last().unwrap().id, "circle");
}
#[test]
fn annotation_rejection_is_atomic_for_invalid_geometry_and_duplicate_ids() {
    let mut s = scene();
    let before = s.clone();
    let bad = Annotation::new(
        "bad",
        AnnotationKind::Rect {
            x: 0.,
            y: 0.,
            width: f64::NAN,
            height: 1.,
            fill: None,
            stroke: Some(stroke()),
        },
    );
    assert!(annotate(&mut s, &[line("good"), bad], AnnotationLayer::Front).is_err());
    assert_eq!(s, before);
    assert!(annotate(&mut s, &[line("data")], AnnotationLayer::Back).is_err());
    assert_eq!(s, before);
    assert!(annotate(&mut s, &[line("same"), line("same")], AnnotationLayer::Back).is_err());
    assert_eq!(s, before);
}
#[test]
fn annotation_budgets_are_checked_before_publication() {
    let mut s = scene();
    let before = s.clone();
    assert!(
        annotate(
            &mut s,
            &vec![line("repeat"); MAX_ANNOTATIONS + 1],
            AnnotationLayer::Front
        )
        .is_err()
    );
    assert_eq!(s, before);
    let prototype = s.marks[0].clone();
    s.marks = (0..MAX_MARKS)
        .map(|i| {
            let mut m = prototype.clone();
            m.id = format!("m{i}");
            m
        })
        .collect();
    assert!(annotate(&mut s, &[line("one-more")], AnnotationLayer::Front).is_err());
    assert_eq!(s.marks.len(), MAX_MARKS);
    assert_eq!(s.marks[0].id, "m0");
}
#[test]
fn zoom_roundtrips_and_preserves_the_world_position_under_pointer() {
    let transform = ZoomTransform::new(2., 10., -20.).unwrap();
    let point = (30., 60.);
    let projected = transform.project(point).unwrap();
    assert_eq!(projected, (70., 100.));
    assert_eq!(transform.unproject(projected).unwrap(), point);
    let pointer = (100., 80.);
    let world = transform.unproject(pointer).unwrap();
    let zoomed = transform.zoom_at(3., pointer).unwrap();
    assert_eq!(zoomed.project(world).unwrap(), pointer);
    assert_eq!(
        transform.pan((10., 15.)).unwrap().project(point).unwrap(),
        (80., 115.)
    );
}
#[test]
fn zoom_rejects_nonfinite_input_and_arithmetic_overflow() {
    for scale in [0., -1., f64::NAN, f64::INFINITY, 1e-9, 1e9] {
        assert!(ZoomTransform::new(scale, 0., 0.).is_err());
    }
    let t = ZoomTransform::new(2., 0., 0.).unwrap();
    assert!(t.project((f64::MAX, 0.)).is_err());
    assert!(t.zoom_at(f64::MAX, (0., 0.)).is_err());
    let t = ZoomTransform::new(1., f64::MAX, 0.).unwrap();
    assert!(t.pan((f64::MAX, 0.)).is_err());
    assert!(t.unproject((-f64::MAX, 0.)).is_err());
    assert!(ZoomTransform::default().project((f64::NAN, 0.)).is_err());
    assert!(ZoomTransform::default().zoom_at(0., (0., 0.)).is_err());
}
fn limits() -> ZoomConstraints {
    ZoomConstraints {
        min_scale: 1.,
        max_scale: 8.,
        viewport_width: 100.,
        viewport_height: 100.,
        world: Brush::from_points((0., 0.), (100., 100.)).unwrap(),
    }
}
#[test]
fn pan_and_zoom_constraints_cover_the_viewport_and_clamp_scale() {
    let constraints = limits();
    let t = ZoomTransform::new(2., -20., -20.).unwrap();
    assert_eq!(
        t.pan_constrained((500., -500.), &constraints).unwrap(),
        ZoomTransform::new(2., 0., -100.).unwrap()
    );
    let zoom = t.zoom_constrained(2., (50., 50.), &constraints).unwrap();
    assert_eq!(
        zoom.unproject((50., 50.)).unwrap(),
        t.unproject((50., 50.)).unwrap()
    );
    assert_eq!(
        t.zoom_constrained(f64::MAX, (50., 50.), &constraints)
            .unwrap()
            .scale,
        8.
    );
    assert_eq!(
        t.zoom_constrained(f64::MIN_POSITIVE, (50., 50.), &constraints)
            .unwrap(),
        ZoomTransform::default()
    );
    let small = ZoomConstraints {
        world: Brush::from_points((10., 10.), (30., 30.)).unwrap(),
        ..constraints
    };
    assert_eq!(
        small.constrain(&ZoomTransform::default()).unwrap(),
        ZoomTransform::new(1., 30., 30.).unwrap()
    );
}
#[test]
fn invalid_constraints_are_rejected() {
    let invalid = ZoomConstraints {
        min_scale: 9.,
        max_scale: 2.,
        ..limits()
    };
    assert!(invalid.constrain(&ZoomTransform::default()).is_err());
    let invalid = ZoomConstraints {
        viewport_width: f64::INFINITY,
        ..limits()
    };
    assert!(invalid.constrain(&ZoomTransform::default()).is_err());
    let invalid = ZoomConstraints {
        world: Brush {
            x0: 20.,
            y0: 0.,
            x1: 10.,
            y1: 1.,
        },
        ..limits()
    };
    assert!(invalid.constrain(&ZoomTransform::default()).is_err());
}
#[test]
fn brush_normalizes_clips_and_keeps_disjoint_drags_empty() {
    let b = Brush::from_points((30., 40.), (-10., -20.)).unwrap();
    assert_eq!(
        b,
        Brush {
            x0: -10.,
            y0: -20.,
            x1: 30.,
            y1: 40.
        }
    );
    assert_eq!(
        b.clipped(20., 20.).unwrap(),
        Some(Brush {
            x0: 0.,
            y0: 0.,
            x1: 20.,
            y1: 20.
        })
    );
    assert!(
        Brush::from_points((101., 0.), (120., 10.))
            .unwrap()
            .clipped(100., 100.)
            .unwrap()
            .is_none()
    );
    assert!(
        Brush::from_points((2., 2.), (2., 2.))
            .unwrap()
            .contains((2., 2.))
    );
    assert!(Brush::from_points((f64::NAN, 0.), (1., 1.)).is_err());
    assert!(b.clipped(0., 100.).is_err());
}
#[test]
fn brush_selects_only_visible_data_centers_and_supports_view_transforms() {
    let mut s = scene();
    let mut hidden = s.marks[0].clone();
    hidden.id = "hidden".into();
    hidden.opacity = 0.;
    s.marks.push(hidden);
    annotate(
        &mut s,
        &[Annotation::new(
            "annotation",
            AnnotationKind::Circle {
                x: 20.,
                y: 15.,
                radius: 2.,
                fill: Some(Color(0, 0, 0, 255)),
                stroke: None,
            },
        )],
        AnnotationLayer::Front,
    )
    .unwrap();
    let b = Brush::from_points((19., 14.), (21., 16.)).unwrap();
    assert_eq!(b.select(&s).unwrap(), ["data"]);
    let view = Brush::from_points((49., 39.), (51., 41.)).unwrap();
    assert_eq!(
        view.select_transformed(&s, &ZoomTransform::new(2., 10., 10.).unwrap())
            .unwrap(),
        ["data"]
    );
}
#[test]
fn brush_uses_tight_cubic_extrema_instead_of_control_point_bounds() {
    let mut s = scene();
    s.marks.clear();
    let mut m = Mark::new(
        "arch",
        Geometry::Path(vec![
            Cmd::Move(P::new(0., 0.)),
            Cmd::Cubic {
                first: P::new(0., 100.),
                second: P::new(100., 100.),
                end: P::new(100., 0.),
            },
        ]),
        Color(0, 0, 0, 255),
    );
    m.series = "series".into();
    m.fill = None;
    m.stroke = Some(stroke());
    s.marks.push(m);
    let b = Brush::from_points((49., 37.), (51., 38.)).unwrap();
    assert_eq!(b.select(&s).unwrap(), ["arch"]);
    let control_box = Brush::from_points((49., 49.), (51., 51.)).unwrap();
    assert!(control_box.select(&s).unwrap().is_empty());
}

#[test]
fn annotation_explicit_paint_order_groups_overlays_and_remaps_owned_labels() {
    let mut s = scene();
    s.labels.push(text("existing"));
    s.label_series.insert(0, "series".into());
    s.draw_order = vec![DrawItem::Label(0), DrawItem::Mark(0)];
    annotate(
        &mut s,
        &[
            line("under").caption(text("under-caption")),
            Annotation::new("note", AnnotationKind::Text(text("under-note"))),
        ],
        AnnotationLayer::Back,
    )
    .unwrap();
    assert_eq!(
        s.draw_order,
        vec![
            DrawItem::Mark(0),
            DrawItem::Label(0),
            DrawItem::Mark(1),
            DrawItem::Label(1),
            DrawItem::Label(2),
            DrawItem::Mark(2)
        ]
    );
    assert_eq!(s.label_series.get(&2).map(String::as_str), Some("series"));
    assert_eq!(s.label_series.len(), 1);
    annotate(
        &mut s,
        &[line("over").caption(text("over-caption"))],
        AnnotationLayer::Front,
    )
    .unwrap();
    assert_eq!(
        &s.draw_order[s.draw_order.len() - 2..],
        &[DrawItem::Mark(3), DrawItem::Label(3)]
    );
    assert_eq!(s.label_series.get(&2).map(String::as_str), Some("series"));
    let svg = kartis::render::to_svg(&s).unwrap();
    let under = svg.find("data-id=\"under\"").unwrap();
    let caption = svg.find(">under-caption</text>").unwrap();
    let existing = svg.find(">existing</text>").unwrap();
    let data = svg.find("data-id=\"data\"").unwrap();
    let over = svg.find("data-id=\"over\"").unwrap();
    assert!(under < caption && caption < existing && existing < data && data < over);
    let hidden = s.without_series(&std::collections::BTreeSet::from(["series".into()]));
    hidden.validate().unwrap();
    assert!(hidden.labels.iter().all(|l| l.text != "existing"));
    assert!(hidden.labels.iter().any(|l| l.text == "under-caption"));
}
#[test]
fn default_annotation_layers_preserve_default_order_and_label_ownership() {
    let mut s = scene();
    s.labels.push(text("owned"));
    s.label_series.insert(0, "series".into());
    annotate(
        &mut s,
        &[line("back").caption(text("shared"))],
        AnnotationLayer::Back,
    )
    .unwrap();
    assert!(s.draw_order.is_empty());
    assert_eq!(s.label_series.get(&1).map(String::as_str), Some("series"));
    annotate(
        &mut s,
        &[line("front").caption(text("another"))],
        AnnotationLayer::Front,
    )
    .unwrap();
    assert!(s.draw_order.is_empty());
    assert_eq!(s.label_series.get(&1).map(String::as_str), Some("series"));
}
#[test]
fn explicit_overlay_rejection_retains_order_and_ownership_transactionally() {
    let mut s = scene();
    s.labels.push(text("owned"));
    s.label_series.insert(0, "series".into());
    s.draw_order = vec![DrawItem::Label(0), DrawItem::Mark(0)];
    let before = s.clone();
    let mut invalid = text("invalid");
    invalid.x = f64::NAN;
    assert!(
        annotate(
            &mut s,
            &[line("valid").caption(invalid)],
            AnnotationLayer::Back
        )
        .is_err()
    );
    assert_eq!(s, before);
}
