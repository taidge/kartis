use kartis::{annotations::ZoomTransform, surface::*, *};
#[test]
fn captured_brush_is_bounded_to_the_visible_viewport_after_zoom() {
    let mut m = SurfaceModel::new(scene(), SurfaceOptions::default()).unwrap();
    m.apply(SurfaceInput::Wheel {
        point: (100., 100.),
        delta: -2_f64.ln() / 0.002,
    })
    .unwrap();
    m.apply(SurfaceInput::Down {
        id: 1,
        point: (50., 50.),
        brush: true,
    })
    .unwrap();
    m.apply(SurfaceInput::Up {
        id: 1,
        point: (500., 400.),
    })
    .unwrap();
    let brush = m.transformed_brush().unwrap().unwrap();
    assert_eq!(
        (brush.x0, brush.y0, brush.x1, brush.y1),
        (50., 50., 400., 300.)
    );
}
fn scene() -> Scene {
    let c = ChartConfig {
        width: 400.,
        height: 300.,
        ..Default::default()
    };
    let mut s = c.scene().unwrap();
    for (id, series, x, y) in [
        ("a", "A", 100., 100.),
        ("b", "B", 200., 100.),
        ("c", "A", 100., 200.),
    ] {
        let mut m = Mark::new(id, Geometry::Circle { x, y, radius: 10. }, c.theme.color(0));
        m.series = series.into();
        m.value = x;
        s.marks.push(m);
    }
    s
}
#[test]
fn slices_skip_ineligible_clipped_sites_and_resize_reports_constrained_view() {
    let mut s = scene();
    s.marks.truncate(2);
    s.marks[0].clip = Some(ClipRect {
        x: 0.,
        y: 0.,
        width: 400.,
        height: 130.,
    });
    s.marks[1].geometry = Geometry::Circle {
        x: 120.,
        y: 200.,
        radius: 10.,
    };
    let mut m = SurfaceModel::new(
        s,
        SurfaceOptions {
            picking: Picking::SliceX { distance: 30. },
            ..Default::default()
        },
    )
    .unwrap();
    m.apply(SurfaceInput::Move {
        id: 1,
        point: (100., 200.),
    })
    .unwrap();
    assert_eq!(m.hovered(), ["b"]);
    m.apply(SurfaceInput::Wheel {
        point: (300., 200.),
        delta: -400.,
    })
    .unwrap();
    let old = m.view();
    let mut smaller = scene();
    smaller.width = 150.;
    smaller.height = 150.;
    let events = m.set_scene(smaller).unwrap();
    assert_ne!(m.view(), old);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, SurfaceEvent::View(_)))
    );
}
#[test]
fn geometry_selection_and_repeat_activation_use_current_datum() {
    let mut m = SurfaceModel::new(scene(), SurfaceOptions::default()).unwrap();
    assert_eq!(
        m.apply(SurfaceInput::Move {
            id: 1,
            point: (100., 100.)
        })
        .unwrap(),
        vec![SurfaceEvent::Hover(vec!["a".into()])]
    );
    for _ in 0..2 {
        m.apply(SurfaceInput::Down {
            id: 1,
            point: (100., 100.),
            brush: false,
        })
        .unwrap();
        let e = m
            .apply(SurfaceInput::Up {
                id: 1,
                point: (100., 100.),
            })
            .unwrap();
        assert!(e.contains(&SurfaceEvent::Activated("a".into())));
    }
    assert_eq!(m.selected(), Some("a"));
    let mut next = scene();
    next.marks[0].value = 999.;
    m.set_scene(next).unwrap();
    assert_eq!(m.datum("a").unwrap().value, 999.);
}
#[test]
fn zoom_anchor_pan_and_brush_share_the_same_inverse_coordinates() {
    let mut m = SurfaceModel::new(scene(), SurfaceOptions::default()).unwrap();
    m.apply(SurfaceInput::Wheel {
        point: (100., 100.),
        delta: -2_f64.ln() / 0.002,
    })
    .unwrap();
    assert!((m.view().scale - 2.).abs() < 1e-12);
    assert_eq!(m.view().unproject((100., 100.)).unwrap(), (100., 100.));
    m.apply(SurfaceInput::Down {
        id: 1,
        point: (100., 100.),
        brush: false,
    })
    .unwrap();
    m.apply(SurfaceInput::Move {
        id: 1,
        point: (130., 100.),
    })
    .unwrap();
    let e = m
        .apply(SurfaceInput::Up {
            id: 1,
            point: (130., 100.),
        })
        .unwrap();
    assert!(!e.iter().any(|e| matches!(e, SurfaceEvent::Activated(_))));
    m.apply(SurfaceInput::Move {
        id: 1,
        point: (130., 100.),
    })
    .unwrap();
    assert_eq!(m.hovered(), ["a"]);
    m.apply(SurfaceInput::Down {
        id: 2,
        point: (120., 90.),
        brush: true,
    })
    .unwrap();
    let e = m
        .apply(SurfaceInput::Up {
            id: 2,
            point: (140., 110.),
        })
        .unwrap();
    assert!(e.iter().any(
        |e| matches!(e,SurfaceEvent::Brush{ids,complete:true,..} if ids==&vec![String::from("a")])
    ));
    let b = m.transformed_brush().unwrap().unwrap();
    assert!((b.x0 - 120.).abs() < 1e-10 && (b.x1 - 140.).abs() < 1e-10);
}
#[test]
fn nearest_and_slice_picking_filter_hidden_series_and_respect_distance() {
    let mut m = SurfaceModel::new(
        scene(),
        SurfaceOptions {
            picking: Picking::Nearest { distance: 30. },
            ..Default::default()
        },
    )
    .unwrap();
    m.apply(SurfaceInput::Move {
        id: 1,
        point: (120., 100.),
    })
    .unwrap();
    assert_eq!(m.hovered(), ["a"]);
    m.apply(SurfaceInput::Move {
        id: 1,
        point: (140., 100.),
    })
    .unwrap();
    assert!(m.hovered().is_empty());
    let mut m = SurfaceModel::new(
        scene(),
        SurfaceOptions {
            picking: Picking::SliceX { distance: 10. },
            ..Default::default()
        },
    )
    .unwrap();
    m.apply(SurfaceInput::Move {
        id: 1,
        point: (104., 150.),
    })
    .unwrap();
    assert_eq!(m.hovered(), ["a", "c"]);
    m.set_hidden(["A".into()].into_iter().collect()).unwrap();
    assert!(m.hovered().is_empty());
    m.apply(SurfaceInput::Move {
        id: 1,
        point: (200., 150.),
    })
    .unwrap();
    assert_eq!(m.hovered(), ["b"]);
}
#[test]
fn keyboard_order_follows_paint_order_and_scene_retirement_reconciles() {
    let mut s = scene();
    s.draw_order = vec![DrawItem::Mark(2), DrawItem::Mark(0), DrawItem::Mark(1)];
    let mut m = SurfaceModel::new(s, SurfaceOptions::default()).unwrap();
    m.apply(SurfaceInput::First).unwrap();
    assert_eq!(m.selected(), Some("c"));
    m.apply(SurfaceInput::Next).unwrap();
    assert_eq!(m.selected(), Some("a"));
    m.apply(SurfaceInput::Previous).unwrap();
    assert_eq!(m.selected(), Some("c"));
    let next = m
        .scene()
        .without_series(&["A".into()].into_iter().collect());
    let e = m.set_scene(next).unwrap();
    assert!(e.contains(&SurfaceEvent::Selected(None)));
    m.apply(SurfaceInput::Last).unwrap();
    assert_eq!(m.selected(), Some("b"));
}
#[test]
fn invalid_inputs_and_sources_are_transactional_and_cancel_releases_drag() {
    let mut m = SurfaceModel::new(scene(), SurfaceOptions::default()).unwrap();
    m.apply(SurfaceInput::Move {
        id: 1,
        point: (100., 100.),
    })
    .unwrap();
    let before = m.hovered().to_vec();
    let view = m.view();
    assert!(
        m.apply(SurfaceInput::Wheel {
            point: (100., 100.),
            delta: f64::NAN
        })
        .is_err()
    );
    assert_eq!(m.hovered(), before);
    assert_eq!(m.view(), view);
    let mut bad = scene();
    bad.width = f64::NAN;
    assert!(m.set_scene(bad).is_err());
    assert_eq!(m.scene().width, 400.);
    assert!(
        m.set_hidden((0..257).map(|i| i.to_string()).collect())
            .is_err()
    );
    assert!(m.set_selected(Some("unknown".into())).is_err());
    m.apply(SurfaceInput::Down {
        id: 1,
        point: (100., 100.),
        brush: true,
    })
    .unwrap();
    m.apply(SurfaceInput::Cancel { id: 2 }).unwrap();
    assert!(m.dragging());
    m.cancel_drag();
    assert!(!m.dragging());
    assert_eq!(m.view(), ZoomTransform::default());
}
