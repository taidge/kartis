use kartis::{surface::*, *};
fn scene(x: f64) -> Scene {
    let mut s = ChartConfig {
        width: 400.,
        height: 300.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    let mut mark = Mark::new(
        "p",
        Geometry::Circle {
            x,
            y: 130.,
            radius: 10.,
        },
        Color(40, 90, 140, 255),
    );
    mark.label = "P".into();
    mark.series = "s".into();
    s.marks.push(mark);
    s
}
fn plot() -> ClipRect {
    ClipRect {
        x: 50.,
        y: 30.,
        width: 300.,
        height: 200.,
    }
}
fn model() -> SurfaceModel {
    let mut m = SurfaceModel::new(scene(200.), SurfaceOptions::default()).unwrap();
    m.relayout(
        scene(200.),
        plot(),
        ZoomTransform::default(),
        [1., 20.],
        false,
    )
    .unwrap();
    m
}
#[test]
fn fixed_axes_relayout_picks_current_pixels_and_uses_a_plot_local_gesture_transform() {
    let mut m = model();
    assert!(
        m.apply(SurfaceInput::Wheel {
            point: (10., 10.),
            delta: -500.
        })
        .unwrap()
        .is_empty()
    );
    m.apply(SurfaceInput::Wheel {
        point: (200., 130.),
        delta: -2_f64.ln() / 0.002,
    })
    .unwrap();
    assert_eq!(m.view(), ZoomTransform::new(2., -150., -100.).unwrap());
    let view = m.view();
    m.relayout(scene(250.), plot(), view, [1., 20.], true)
        .unwrap();
    assert_eq!(m.paint_transform(), ZoomTransform::default());
    m.apply(SurfaceInput::Move {
        id: 1,
        point: (250., 130.),
    })
    .unwrap();
    assert_eq!(m.hovered(), ["p"]);
    m.apply(SurfaceInput::Down {
        id: 1,
        point: (10., 10.),
        brush: false,
    })
    .unwrap();
    assert!(!m.dragging());
}
#[test]
fn publishing_relaid_out_geometry_preserves_pan_and_pinch_baselines() {
    let mut m = model();
    let v = ZoomTransform::new(2., -150., -100.).unwrap();
    m.relayout(scene(200.), plot(), v, [1., 20.], false)
        .unwrap();
    m.apply(SurfaceInput::Down {
        id: 1,
        point: (200., 130.),
        brush: false,
    })
    .unwrap();
    m.apply(SurfaceInput::Move {
        id: 1,
        point: (210., 130.),
    })
    .unwrap();
    let a = m.view();
    m.relayout(scene(210.), plot(), a, [1., 20.], true).unwrap();
    assert!(m.dragging());
    m.apply(SurfaceInput::Move {
        id: 1,
        point: (220., 130.),
    })
    .unwrap();
    assert_eq!(m.view().translate_x, a.translate_x + 10.);
    m.cancel_drag();
    for (id, x) in [(1, 150.), (2, 250.)] {
        m.apply(SurfaceInput::TouchStart {
            id,
            point: (x, 130.),
        })
        .unwrap();
    }
    m.apply(SurfaceInput::TouchMove {
        id: 1,
        point: (140., 130.),
    })
    .unwrap();
    let a = m.view();
    m.relayout(scene(220.), plot(), a, [1., 20.], true).unwrap();
    assert!(m.pinching());
    m.apply(SurfaceInput::TouchMove {
        id: 2,
        point: (260., 130.),
    })
    .unwrap();
    assert!(m.view().scale > a.scale);
}
#[test]
fn fixed_axis_brush_uses_current_scene_pixels_and_clamps_to_plot_not_axes() {
    let mut m = model();
    m.relayout(
        scene(200.),
        plot(),
        ZoomTransform::new(2., -150., -100.).unwrap(),
        [1., 20.],
        false,
    )
    .unwrap();
    m.apply(SurfaceInput::Down {
        id: 1,
        point: (80., 80.),
        brush: true,
    })
    .unwrap();
    m.apply(SurfaceInput::Up {
        id: 1,
        point: (500., 500.),
    })
    .unwrap();
    let b = m.transformed_brush().unwrap().unwrap();
    assert_eq!((b.x0, b.y0, b.x1, b.y1), (80., 80., 350., 230.));
    let before = m.scene().clone();
    let view = m.view();
    assert!(
        m.relayout(
            scene(100.),
            ClipRect {
                width: 500.,
                ..plot()
            },
            view,
            [1., 20.],
            true
        )
        .is_err()
    );
    assert_eq!(m.scene(), &before);
    assert_eq!(m.view(), view);
}
