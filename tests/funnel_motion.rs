use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::motion::SpringConfig;
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 500.,
        height: 360.,
        title: "Motion / 漏斗".into(),
        ..Default::default()
    }
}
fn frame(value: f64, color: Color) -> FunnelSvgFrame {
    let mut datum = FunnelDatum::new("a", value);
    datum.color = Some(color);
    FunnelSvgFrame::new(
        &config(),
        &[datum, FunnelDatum::new(1., 3.)],
        &Default::default(),
    )
    .unwrap()
}
fn edited(points: Vec<C>, color: Color) -> FunnelSvgFrame {
    FunnelSvgFrame::with_stage_factory(
        &config(),
        &[FunnelDatum::new("a", 6.)],
        &Default::default(),
        &mut |context| {
            let mut geometry = context.inherit(Geometry::Path(points.clone()));
            geometry.body.fill = Some(color);
            geometry.body.fill_paint = None;
            geometry.retain_profile_border = false;
            Ok(geometry)
        },
    )
    .unwrap()
}
fn path(x: f64) -> Vec<C> {
    vec![
        C::Move(P::new(x, 20.)),
        C::Line(P::new(x + 10., 30.)),
        C::Close,
    ]
}
fn first_x(scene: &Scene, id: &str) -> f64 {
    match &scene.marks.iter().find(|m| m.id == id).unwrap().geometry {
        Geometry::Path(p) => match p[0] {
            C::Move(p) => p.x,
            _ => panic!("Move expected"),
        },
        _ => panic!("Path expected"),
    }
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
// Independent solution of x''+26x'+170(x-target)=0, zero initial velocity.
fn progress(t: f64) -> f64 {
    1. - (-13. * t).exp() * (t.cos() + 13. * t.sin())
}

#[test]
fn paths_use_actual_edited_ink_and_previous_target_on_interruption() {
    let a = edited(path(20.), Color(10, 20, 30, 255));
    let b = edited(path(120.), Color(110, 120, 130, 255));
    let c = edited(path(220.), Color(210, 220, 230, 255));
    let id = a.parts()[0].mark_id.clone();
    let mut model = FunnelMotion::new(a.clone(), Default::default()).unwrap();
    assert_eq!(model.options().spring, SpringConfig::default());
    model.retarget(b.clone()).unwrap();
    near(first_x(model.frame().scene(), &id), 20.);
    model.tick(0.05).unwrap();
    near(
        first_x(model.frame().scene(), &id),
        20. + 100. * progress(0.05),
    );
    let color_state = model
        .frame()
        .mark_property(&id, FunnelMotionMarkChannel::FillRed)
        .unwrap();
    assert!(color_state.velocity > 0.);
    model.retarget(c.clone()).unwrap();
    // This visible jump to B is upstream's previous-target string semantics.
    near(first_x(model.frame().scene(), &id), 120.);
    let next_color = model
        .frame()
        .mark_property(&id, FunnelMotionMarkChannel::FillRed)
        .unwrap();
    near(next_color.value, color_state.value);
    near(next_color.velocity, color_state.velocity);
    assert_eq!(model.frame().path_progress(&id).unwrap().value, 0.);
    assert_eq!(model.target(), &c);
    assert_eq!(
        a.scene()
            .marks
            .iter()
            .find(|m| m.id == id)
            .unwrap()
            .geometry,
        Geometry::Path(path(20.))
    );
}

#[test]
fn d3_numeric_ordinal_matching_uses_target_topology_and_unmatched_constants() {
    let a = edited(
        vec![C::Move(P::new(10., 20.)), C::Line(P::new(30., 40.))],
        Color(0, 0, 0, 255),
    );
    let b = edited(
        vec![
            C::Move(P::new(100., 200.)),
            C::Cubic {
                first: P::new(300., 400.),
                second: P::new(500., 600.),
                end: P::new(700., 800.),
            },
            C::Close,
        ],
        Color(0, 0, 0, 255),
    );
    let mut m = FunnelMotion::new(a, Default::default()).unwrap();
    m.retarget(b).unwrap();
    let body = m.frame().parts()[0].body_mark_index.unwrap();
    assert_eq!(
        m.frame().scene().marks[body].geometry,
        Geometry::Path(vec![
            C::Move(P::new(10., 20.)),
            C::Cubic {
                first: P::new(30., 40.),
                second: P::new(500., 600.),
                end: P::new(700., 800.)
            },
            C::Close
        ])
    );
    m.tick(0.1).unwrap();
    let p = progress(0.1);
    if let Geometry::Path(path) = &m.frame().scene().marks[body].geometry {
        if let C::Cubic { first, second, end } = path[1] {
            near(first.x, 30. + 270. * p);
            near(first.y, 40. + 360. * p);
            assert_eq!(second, P::new(500., 600.));
            assert_eq!(end, P::new(700., 800.));
        } else {
            panic!();
        }
    } else {
        panic!();
    }
}

#[test]
fn equal_geometry_new_capture_refreshes_data_without_resetting_path_progress() {
    let a = edited(path(10.), Color(10, 20, 30, 255));
    let b = edited(path(100.), Color(30, 40, 50, 255));
    let id = a.parts()[0].mark_id.clone();
    let mut m = FunnelMotion::new(a, Default::default()).unwrap();
    m.retarget(b.clone()).unwrap();
    m.tick(0.04).unwrap();
    let state = m.frame().path_progress(&id).unwrap();
    let serial = m.frame().sample_serial();
    let same = edited(path(100.), Color(30, 40, 50, 255));
    assert_ne!(b, same);
    m.retarget(same.clone()).unwrap();
    assert_eq!(m.target(), &same);
    assert_eq!(m.frame().target_serial(), 2);
    assert_eq!(m.frame().sample_serial(), serial);
    assert_eq!(m.frame().path_progress(&id), Some(state));
}

#[test]
fn labels_separators_new_and_removed_roles_keep_target_slot_order() {
    let a = frame(7., Color(20, 30, 40, 255));
    let removed_id = a.parts()[0].mark_id.clone();
    let mut c = config();
    c.width = 700.;
    let b = FunnelSvgFrame::new(
        &c,
        &[FunnelDatum::new(1., 6.), FunnelDatum::new("new", 2.)],
        &Default::default(),
    )
    .unwrap();
    let mut m = FunnelMotion::new(a, Default::default()).unwrap();
    m.retarget(b.clone()).unwrap();
    assert_eq!(m.frame().parts().len(), 2);
    assert_eq!(m.frame().parts()[0].key, CategoryKey::from(1.));
    assert_eq!(m.frame().scene().draw_items(), b.scene().draw_items());
    let new = m.frame().parts()[1].body_mark_index.unwrap();
    assert_eq!(
        m.frame().scene().marks[new],
        *b.body_mark(1).unwrap().unwrap()
    );
    assert!(m.frame().scene().marks.iter().all(|x| x.id != removed_id));
    let slot = m.frame().labels()[0].label_index;
    let x = m
        .frame()
        .label_property(slot, FunnelMotionLabelChannel::X)
        .unwrap();
    assert_ne!(x.value, x.target);
    let separator = m
        .frame()
        .separators()
        .iter()
        .find(|s| s.key.part_id == Some(CategoryKey::from(1.)))
        .unwrap();
    assert_eq!(
        m.frame()
            .geometry_states(&m.frame().scene().marks[separator.mark_index.unwrap()].id)
            .unwrap()
            .len(),
        4
    );
}

#[test]
fn invalid_tick_budget_and_invalid_sample_do_not_mutate_accepted_snapshot() {
    let a = edited(path(10.), Color(0, 0, 0, 255));
    let b = edited(path(20.), Color(255, 0, 0, 255));
    let mut m = FunnelMotion::new(a, Default::default()).unwrap();
    m.retarget(b).unwrap();
    let old = m.frame().clone();
    for dt in [f64::NAN, -0.1, 61.] {
        assert!(m.tick(dt).is_err());
        assert_eq!(m.frame(), &old);
    }
    assert!(
        m.set_options(FunnelMotionOptions {
            max_marks: 0,
            ..Default::default()
        })
        .is_err()
    );
    assert_eq!(m.frame(), &old);
    m.tick(0.).unwrap();
    assert_eq!(m.frame(), &old);
    m.finish().unwrap();
    assert!(m.is_settled());
    let settled = m.frame().clone();
    m.tick(1.).unwrap();
    assert_eq!(m.frame(), &settled);
}

#[test]
fn reduced_motion_finishes_and_new_targets_remain_immediate_then_resume() {
    let mut m = FunnelMotion::new(frame(7., Color(0, 0, 0, 255)), Default::default()).unwrap();
    m.retarget(frame(2., Color(255, 0, 0, 255))).unwrap();
    m.set_reduced_motion(true).unwrap();
    assert!(m.is_settled());
    let b = frame(5., Color(0, 255, 0, 255));
    m.retarget(b.clone()).unwrap();
    assert_eq!(m.frame().scene(), b.scene());
    m.set_reduced_motion(false).unwrap();
    assert!(m.is_settled());
    m.retarget(frame(8., Color(0, 0, 255, 255))).unwrap();
    assert!(!m.is_settled());
}

#[test]
fn negative_physical_border_width_has_no_hairline_and_recovers_with_velocity() {
    let source = [FunnelDatum::new("a", 8.)];
    let a = FunnelSvgFrame::new(
        &config(),
        &source,
        &FunnelLayoutOptions {
            border_width: 100.,
            ..Default::default()
        },
    )
    .unwrap();
    let b = FunnelSvgFrame::new(
        &config(),
        &source,
        &FunnelLayoutOptions {
            border_width: 1.,
            ..Default::default()
        },
    )
    .unwrap();
    let mut m = FunnelMotion::new(
        a,
        FunnelMotionOptions {
            spring: SpringConfig {
                damping: 0.,
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    m.retarget(b).unwrap();
    let index = m.frame().parts()[0].border_mark_index.unwrap();
    let id = m.frame().scene().marks[index].id.clone();
    let half_period = std::f64::consts::PI / 170f64.sqrt();
    m.tick(half_period).unwrap();
    near(
        m.frame()
            .mark_property(&id, FunnelMotionMarkChannel::StrokeWidth)
            .unwrap()
            .value,
        -98.,
    );
    assert!(m.frame().scene().marks[index].stroke.is_none());
    m.tick(half_period).unwrap();
    near(
        m.frame()
            .mark_property(&id, FunnelMotionMarkChannel::StrokeWidth)
            .unwrap()
            .value,
        100.,
    );
    near(m.frame().scene().marks[index].stroke.unwrap().width, 100.);
    assert!(!m.is_settled());
}

fn annotated(base: &FunnelSvgFrame, x: f64) -> FunnelSvgFrame {
    let mut s = FunnelAnnotationSpec::new(
        FunnelAnnotationMatch::All,
        FunnelAnnotationKind::Rect,
        FunnelAnnotationNote::Text("note".into()),
        FunnelAnnotationPosition::Relative(x),
        FunnelAnnotationPosition::Relative(-20.),
    );
    s.offset = 3.;
    base.with_annotations(&[s], &mut Default::default(), &Default::default())
        .unwrap()
}
#[test]
fn annotation_link_progress_and_note_coordinates_animate_without_factories() {
    let a = annotated(&frame(8., Color(0, 0, 0, 255)), 10.);
    let b = annotated(&frame(3., Color(0, 0, 0, 255)), 80.);
    let base_labels = b.annotation_source().scene().labels.len();
    let target_note = b.scene().labels[base_labels].x;
    let previous_note = a.scene().labels[a.annotation_source().scene().labels.len()].x;
    let mut m = FunnelMotion::new(a, Default::default()).unwrap();
    m.retarget(b.clone()).unwrap();
    near(m.frame().scene().labels[base_labels].x, previous_note);
    m.tick(0.1).unwrap();
    near(
        m.frame().scene().labels[base_labels].x,
        previous_note + (target_note - previous_note) * progress(0.1),
    );
    let annbase = b.annotation_source().scene().marks.len();
    let link = &m.frame().scene().marks[annbase];
    assert!(m.frame().path_progress(&link.id).is_some());
    let shape = &m.frame().scene().marks[annbase + 1];
    assert_eq!(m.frame().geometry_states(&shape.id).unwrap().len(), 4);
    m.finish().unwrap();
    assert_eq!(m.frame().scene(), b.scene());
}

#[test]
fn plain_frame_send_sync_and_extras_are_static_target_metadata() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<FunnelSvgFrame>();
    send_sync::<FunnelMotionFrame>();
    let mut layout =
        funnel_layout(&config(), &[FunnelDatum::new("a", 8.)], &Default::default()).unwrap();
    layout.scene.labels.push(Label {
        x: 10.,
        y: 20.,
        text: "extra".into(),
        color: Color(4, 5, 6, 255),
        size: 12.,
        anchor: TextAnchor::Start,
        rotation: 0.,
    });
    let b = FunnelSvgFrame::from_layout(layout).unwrap();
    let index = b.scene().labels.len() - 1;
    let mut m = FunnelMotion::new(frame(2., Color(2, 3, 4, 255)), Default::default()).unwrap();
    m.retarget(b.clone()).unwrap();
    m.tick(0.07).unwrap();
    assert_eq!(m.frame().scene().labels[index], b.scene().labels[index]);
    assert!(
        m.frame()
            .label_property(index, FunnelMotionLabelChannel::X)
            .is_none()
    );
}
