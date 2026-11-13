use kartis::{surface::*, *};

fn scene() -> Scene {
    let mut scene = ChartConfig {
        width: 400.,
        height: 300.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    let mut mark = Mark::new(
        "point",
        Geometry::Circle {
            x: 200.,
            y: 150.,
            radius: 12.,
        },
        Color(60, 120, 180, 255),
    );
    mark.series = "series".into();
    mark.label = "point".into();
    scene.marks.push(mark);
    scene
}
fn model() -> SurfaceModel {
    SurfaceModel::new(scene(), SurfaceOptions::default()).unwrap()
}
fn start(m: &mut SurfaceModel) {
    m.apply(SurfaceInput::TouchStart {
        id: 1,
        point: (150., 150.),
    })
    .unwrap();
    m.apply(SurfaceInput::TouchStart {
        id: 2,
        point: (250., 150.),
    })
    .unwrap();
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}

#[test]
fn two_contacts_zoom_about_the_world_anchor_and_track_the_centroid() {
    let mut m = model();
    start(&mut m);
    assert!(m.pinching());
    assert_eq!(m.touch_count(), 2);
    m.apply(SurfaceInput::TouchMove {
        id: 1,
        point: (100., 150.),
    })
    .unwrap();
    m.apply(SurfaceInput::TouchMove {
        id: 2,
        point: (300., 150.),
    })
    .unwrap();
    close(m.view().scale, 2.);
    assert_eq!(m.view().project((200., 150.)).unwrap(), (200., 150.));
    m.apply(SurfaceInput::TouchMove {
        id: 1,
        point: (150., 150.),
    })
    .unwrap();
    m.apply(SurfaceInput::TouchMove {
        id: 2,
        point: (350., 150.),
    })
    .unwrap();
    close(m.view().scale, 2.);
    assert_eq!(m.view().project((200., 150.)).unwrap(), (250., 150.));
}
#[test]
fn lifting_one_contact_rebases_pan_and_never_activates_a_pinch_as_a_tap() {
    let mut m = model();
    start(&mut m);
    m.apply(SurfaceInput::TouchMove {
        id: 1,
        point: (100., 150.),
    })
    .unwrap();
    m.apply(SurfaceInput::TouchEnd {
        id: 2,
        point: (300., 150.),
    })
    .unwrap();
    let before = m.view();
    assert!(!m.pinching());
    assert_eq!(m.touch_count(), 1);
    m.apply(SurfaceInput::TouchMove {
        id: 1,
        point: (110., 150.),
    })
    .unwrap();
    close(m.view().translate_x, before.translate_x + 10.);
    let events = m
        .apply(SurfaceInput::TouchEnd {
            id: 1,
            point: (110., 150.),
        })
        .unwrap();
    assert!(
        !events
            .iter()
            .any(|event| matches!(event, SurfaceEvent::Activated(_)))
    );
    assert!(!m.dragging());
    assert_eq!(m.touch_count(), 0);
    let point = m.view().project((200., 150.)).unwrap();
    m.apply(SurfaceInput::TouchStart { id: 3, point }).unwrap();
    let events = m.apply(SurfaceInput::TouchEnd { id: 3, point }).unwrap();
    assert!(events.contains(&SurfaceEvent::Activated("point".into())));
}
#[test]
fn scale_limits_coincident_contacts_and_disabled_pan_are_consistent() {
    let mut m = SurfaceModel::new(
        scene(),
        SurfaceOptions {
            pan: false,
            max_scale: 2.,
            ..Default::default()
        },
    )
    .unwrap();
    start(&mut m);
    m.apply(SurfaceInput::TouchMove {
        id: 1,
        point: (-1000., 150.),
    })
    .unwrap();
    close(m.view().scale, 2.);
    assert_eq!(m.view().project((200., 150.)).unwrap(), (200., 150.));
    m.apply(SurfaceInput::TouchMove {
        id: 1,
        point: (250., 150.),
    })
    .unwrap();
    close(m.view().scale, 1.);
    m.cancel_drag();
    for id in [1, 2] {
        m.apply(SurfaceInput::TouchStart {
            id,
            point: (200., 150.),
        })
        .unwrap();
    }
    m.apply(SurfaceInput::TouchMove {
        id: 1,
        point: (150., 150.),
    })
    .unwrap();
    close(m.view().scale, 1.); // Establish a nondegenerate baseline first.
    m.apply(SurfaceInput::TouchMove {
        id: 2,
        point: (250., 150.),
    })
    .unwrap();
    close(m.view().scale, 2.);
}
#[test]
fn touch_cancellation_resize_and_clear_retire_gestures_without_latent_clicks() {
    let mut m = model();
    start(&mut m);
    m.apply(SurfaceInput::TouchCancel { id: 1 }).unwrap();
    assert_eq!(m.touch_count(), 1);
    assert!(!m.pinching());
    let events = m
        .apply(SurfaceInput::TouchEnd {
            id: 2,
            point: (250., 150.),
        })
        .unwrap();
    assert!(events.is_empty());
    start(&mut m);
    m.set_scene(scene()).unwrap();
    assert!(!m.dragging());
    assert_eq!(m.touch_count(), 0);
    let old = m.view();
    assert!(
        m.apply(SurfaceInput::TouchMove {
            id: 1,
            point: (50., 50.)
        })
        .unwrap()
        .is_empty()
    );
    assert_eq!(m.view(), old);
    start(&mut m);
    m.apply(SurfaceInput::Clear).unwrap();
    assert!(!m.dragging());
    assert_eq!(m.touch_count(), 0);
}
#[test]
fn duplicate_third_and_foreign_inputs_cannot_replace_contacts_or_the_mouse() {
    let mut m = model();
    start(&mut m);
    m.apply(SurfaceInput::TouchStart {
        id: 1,
        point: (0., 0.),
    })
    .unwrap();
    m.apply(SurfaceInput::TouchStart {
        id: 3,
        point: (0., 0.),
    })
    .unwrap();
    m.apply(SurfaceInput::Move {
        id: 50,
        point: (200., 150.),
    })
    .unwrap();
    m.apply(SurfaceInput::Wheel {
        point: (200., 150.),
        delta: -300.,
    })
    .unwrap();
    assert_eq!(m.touch_count(), 2);
    assert!(m.hovered().is_empty());
    close(m.view().scale, 1.);
    m.apply(SurfaceInput::TouchMove {
        id: 1,
        point: (100., 150.),
    })
    .unwrap();
    close(m.view().scale, 1.5);
    m.cancel_drag();
    m.apply(SurfaceInput::Down {
        id: 77,
        point: (50., 50.),
        brush: false,
    })
    .unwrap();
    m.apply(SurfaceInput::TouchStart {
        id: 1,
        point: (200., 150.),
    })
    .unwrap();
    assert_eq!(m.touch_count(), 0);
}
#[test]
fn disabled_pinch_keeps_the_first_touch_and_bad_coordinates_are_atomic() {
    let mut m = SurfaceModel::new(
        scene(),
        SurfaceOptions {
            pinch: false,
            ..Default::default()
        },
    )
    .unwrap();
    start(&mut m);
    assert_eq!(m.touch_count(), 1);
    assert!(!m.pinching());
    m.cancel_drag();
    m = model();
    start(&mut m);
    let view = m.view();
    assert!(
        m.apply(SurfaceInput::TouchMove {
            id: 1,
            point: (f64::NAN, 0.)
        })
        .is_err()
    );
    assert_eq!(m.view(), view);
    assert_eq!(m.touch_count(), 2);
    m.apply(SurfaceInput::TouchMove {
        id: 1,
        point: (100., 150.),
    })
    .unwrap();
    close(m.view().scale, 1.5);
}
