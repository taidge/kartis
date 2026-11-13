use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::{motion::*, *};
fn scene(geometry: Geometry) -> Scene {
    let mut s = ChartConfig::default().scene().unwrap();
    let mut m = Mark::new("shape", geometry, Color(30, 90, 160, 255));
    m.series = "data".into();
    s.marks.push(m);
    s
}
fn rect() -> Geometry {
    Geometry::Rect {
        x: 100.,
        y: 100.,
        width: 80.,
        height: 60.,
    }
}
fn circle() -> Geometry {
    Geometry::Circle {
        x: 300.,
        y: 200.,
        radius: 50.,
    }
}
fn options() -> PropertyMotionOptions {
    let mut options = PropertyMotionOptions::default();
    options.geometry.morphs.insert(
        "shape".into(),
        PathMorphOptions {
            samples: 32,
            ..Default::default()
        },
    );
    options
}
fn settle(m: &mut PropertyMotion) {
    for _ in 0..2000 {
        m.tick(0.016).unwrap().validate().unwrap();
        if m.is_settled() {
            return;
        }
    }
    panic!("motion did not settle");
}
fn points(g: &Geometry) -> Vec<P> {
    let Geometry::Path(path) = g else { panic!() };
    path.iter()
        .filter_map(|c| match c {
            C::Move(p) | C::Line(p) => Some(*p),
            _ => None,
        })
        .collect()
}
#[test]
fn default_crossfades_but_opted_id_morphs_one_sampled_contour_and_finishes_exactly() {
    let initial = scene(rect());
    let target = scene(circle());
    let mut default = PropertyMotion::new(initial.clone(), Default::default()).unwrap();
    default.retarget(target.clone()).unwrap();
    assert_eq!(default.stats().marks, 2);
    let mut morph = PropertyMotion::new(initial, options()).unwrap();
    morph.retarget(target.clone()).unwrap();
    assert_eq!(morph.stats().marks, 1);
    assert_eq!(morph.stats().exiting_marks, 0);
    assert_eq!(points(&morph.scene().marks[0].geometry).len(), 32);
    morph.tick(0.1).unwrap();
    let pts = points(&morph.scene().marks[0].geometry);
    assert!(pts.iter().any(|p| p.x > 180.));
    settle(&mut morph);
    assert_eq!(morph.scene(), &target);
    morph.retarget(scene(rect())).unwrap();
    settle(&mut morph);
    assert_eq!(morph.scene(), &scene(rect()));
}
#[test]
fn repeated_retarget_preserves_independent_sample_velocity_and_reduced_motion_snaps() {
    let mut m = PropertyMotion::new(scene(rect()), options()).unwrap();
    m.retarget(scene(circle())).unwrap();
    m.tick(0.1).unwrap();
    let before = m.mark_property("shape", "geometry.path.0.x").unwrap();
    m.retarget(scene(Geometry::Rect {
        x: 400.,
        y: 250.,
        width: 40.,
        height: 20.,
    }))
    .unwrap();
    let after = m.mark_property("shape", "geometry.path.0.x").unwrap();
    assert_eq!(before.value, after.value);
    assert_eq!(before.velocity, after.velocity);
    m.set_reduced_motion(true);
    assert!(m.is_settled());
    assert_eq!(m.scene(), m.target());
    assert_eq!(m.stats().marks, 1);
}
#[test]
fn custom_enter_and_exit_geometries_use_existing_physics_and_retire_after_presence() {
    let mut config = PropertyMotionOptions::default();
    config.geometry.enter.insert(
        "shape".into(),
        Geometry::Rect {
            x: 0.,
            y: 0.,
            width: 0.,
            height: 0.,
        },
    );
    config.geometry.exit.insert(
        "shape".into(),
        Geometry::Rect {
            x: 500.,
            y: 400.,
            width: 0.,
            height: 0.,
        },
    );
    let empty = ChartConfig::default().scene().unwrap();
    let target = scene(rect());
    let mut m = PropertyMotion::new(empty.clone(), config).unwrap();
    m.retarget(target.clone()).unwrap();
    assert_eq!(m.mark_property("shape", "geometry.x").unwrap().value, 0.);
    assert_eq!(m.mark_property("shape", "geometry.x").unwrap().target, 100.);
    m.tick(0.1).unwrap();
    assert!(m.mark_velocity("shape", "geometry.x").unwrap() > 0.);
    settle(&mut m);
    m.retarget(empty.clone()).unwrap();
    assert_eq!(m.stats().exiting_marks, 1);
    assert_eq!(m.mark_property("shape", "geometry.x").unwrap().target, 500.);
    assert!(m.scene().marks[0].label.is_empty());
    m.tick(0.1).unwrap();
    assert!(m.mark_property("shape", "geometry.x").unwrap().value > 100.);
    settle(&mut m);
    assert_eq!(m.scene(), &empty);
}
#[test]
fn custom_origins_can_cross_shapes_only_with_morph_consent() {
    let empty = ChartConfig::default().scene().unwrap();
    let mut opts = options();
    opts.geometry.enter.insert("shape".into(), rect());
    opts.geometry.exit.insert("shape".into(), rect());
    let mut m = PropertyMotion::new(empty.clone(), opts).unwrap();
    m.retarget(scene(circle())).unwrap();
    assert_eq!(m.stats().marks, 1);
    assert_eq!(points(&m.scene().marks[0].geometry).len(), 32);
    settle(&mut m);
    m.retarget(empty).unwrap();
    assert_eq!(m.stats().exiting_marks, 1);
    settle(&mut m);
    let mut opts = PropertyMotionOptions::default();
    opts.geometry.enter.insert("shape".into(), rect());
    let mut m = PropertyMotion::new(ChartConfig::default().scene().unwrap(), opts).unwrap();
    let before = m.scene().clone();
    assert!(m.retarget(scene(circle())).is_err());
    assert_eq!(m.scene(), &before);
}
#[test]
fn open_closed_changes_fade_and_multiple_contours_or_bad_budgets_reject_atomically() {
    let mut m = PropertyMotion::new(scene(rect()), options()).unwrap();
    let open = scene(Geometry::Path(vec![
        C::Move(P::new(0., 0.)),
        C::Line(P::new(50., 30.)),
    ]));
    m.retarget(open.clone()).unwrap();
    assert_eq!(m.stats().marks, 2);
    settle(&mut m);
    assert_eq!(m.scene(), &open);
    let before = m.scene().clone();
    let bad = scene(Geometry::Path(vec![
        C::Move(P::new(0., 0.)),
        C::Line(P::new(50., 30.)),
        C::Move(P::new(60., 60.)),
    ]));
    assert!(m.retarget(bad).is_err());
    assert_eq!(m.scene(), &before);
    let mut opts = options();
    opts.max_path_commands = 31;
    assert!(PropertyMotion::new(scene(rect()), opts).is_err());
    for samples in [0, 1, 257] {
        assert!(
            sampled_geometry(
                &rect(),
                &PathMorphOptions {
                    samples,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
}
#[test]
fn deterministic_cyclic_alignment_reduces_seam_displacement() {
    let square = Geometry::Path(vec![
        C::Move(P::new(0., 0.)),
        C::Line(P::new(100., 0.)),
        C::Line(P::new(100., 100.)),
        C::Line(P::new(0., 100.)),
        C::Close,
    ]);
    let rotated = Geometry::Path(vec![
        C::Move(P::new(110., 100.)),
        C::Line(P::new(10., 100.)),
        C::Line(P::new(10., 0.)),
        C::Line(P::new(110., 0.)),
        C::Close,
    ]);
    let mut opts = options();
    opts.geometry.morphs.get_mut("shape").unwrap().alignment = ClosedAlignment::Rotate;
    let mut a = PropertyMotion::new(scene(square.clone()), opts.clone()).unwrap();
    let mut b = PropertyMotion::new(scene(square), opts).unwrap();
    a.retarget(scene(rotated.clone())).unwrap();
    b.retarget(scene(rotated)).unwrap();
    for i in 0..32 {
        let p = a
            .mark_property("shape", &format!("geometry.path.{i}.x"))
            .unwrap();
        assert!((p.target - p.value - 10.).abs() < 1e-8);
    }
    a.tick(0.1).unwrap();
    b.tick(0.1).unwrap();
    assert_eq!(a.scene(), b.scene());
}
#[test]
fn cubic_sampling_keeps_collinear_backtracking_and_bounds_subdivision() {
    let path = Geometry::Path(vec![
        C::Move(P::new(0., 0.)),
        C::Cubic {
            first: P::new(300., 0.),
            second: P::new(-300., 0.),
            end: P::new(10., 0.),
        },
    ]);
    let sampled = sampled_geometry(&path, &PathMorphOptions::default()).unwrap();
    let pts = points(&sampled);
    assert!(pts.iter().any(|p| p.x > 50.) && pts.iter().any(|p| p.x < -50.));
    let mut m = PropertyMotion::new(scene(rect()), options()).unwrap();
    m.retarget(scene(circle())).unwrap();
    m.tick(0.1).unwrap();
    let before = m.scene().clone();
    let velocity = m.mark_velocity("shape", "geometry.path.0.x");
    assert!(m.tick(f64::NAN).is_err());
    assert_eq!(m.scene(), &before);
    assert_eq!(m.mark_velocity("shape", "geometry.path.0.x"), velocity);
}
#[test]
fn explicit_clock_consumes_custom_motion_via_existing_scene_motion_trait() {
    let m = PropertyMotion::new(scene(rect()), options()).unwrap();
    let mut clock = clock::ExplicitClock::from_motion(m).unwrap();
    clock.retarget(scene(circle())).unwrap();
    clock.step(0.1).unwrap();
    assert_eq!(clock.scene().marks.len(), 1);
    clock.set_reduced_motion(true);
    assert_eq!(clock.scene(), &scene(circle()));
}

#[test]
fn normalization_union_budget_rejection_keeps_active_tracks_and_custom_paint_ownership() {
    let mut initial = scene(rect());
    initial.labels.push(Label {
        x: 120.,
        y: 90.,
        text: "owned".into(),
        size: 12.,
        color: Color(0, 0, 0, 255),
        anchor: TextAnchor::Middle,
        rotation: 0.,
    });
    initial.label_series.insert(0, "data".into());
    initial.draw_order = vec![DrawItem::Label(0), DrawItem::Mark(0)];
    let mut opts = options();
    opts.max_path_commands = 40;
    let mut m = PropertyMotion::new(initial.clone(), opts).unwrap();
    let mut target = initial.clone();
    target.marks[0].geometry = circle();
    m.retarget(target.clone()).unwrap();
    m.tick(0.1).unwrap();
    assert_eq!(
        m.scene().label_series.get(&0).map(String::as_str),
        Some("data")
    );
    assert_eq!(
        m.scene().draw_order,
        vec![DrawItem::Label(0), DrawItem::Mark(0)]
    );
    let before = m.scene().clone();
    let velocity = m.mark_velocity("shape", "geometry.path.0.x");
    target.marks[0].geometry =
        Geometry::Path(vec![C::Move(P::new(0., 0.)), C::Line(P::new(10., 0.))]);
    assert!(m.retarget(target).is_err()); // Open/closed Fade union would have 65 commands.
    assert_eq!(m.scene(), &before);
    assert_eq!(m.mark_velocity("shape", "geometry.path.0.x"), velocity);
    assert!(
        sampled_geometry(
            &rect(),
            &PathMorphOptions {
                samples: 2,
                ..Default::default()
            }
        )
        .is_err()
    );
}

#[test]
fn revival_of_custom_exit_keeps_physical_momentum_and_retires_no_new_identity() {
    let mut opts = PropertyMotionOptions::default();
    opts.geometry.exit.insert(
        "shape".into(),
        Geometry::Rect {
            x: 500.,
            y: 100.,
            width: 0.,
            height: 0.,
        },
    );
    let mut m = PropertyMotion::new(scene(rect()), opts).unwrap();
    m.retarget(ChartConfig::default().scene().unwrap()).unwrap();
    m.tick(0.05).unwrap();
    let before = m.mark_property("shape", "geometry.x").unwrap();
    assert!(before.velocity > 0.);
    m.retarget(scene(rect())).unwrap();
    let after = m.mark_property("shape", "geometry.x").unwrap();
    assert_eq!(after.value, before.value);
    assert_eq!(after.velocity, before.velocity);
    assert_eq!(m.stats().marks, 1);
    assert_eq!(m.stats().exiting_marks, 0);
    settle(&mut m);
}
