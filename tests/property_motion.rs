use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as Cmd, StrokeCap};
use kartis::motion::{FadeMode, PropertyMotion, PropertyMotionOptions, SpringConfig};
use kartis::*;
fn label(text: &str, x: f64, rotation: f64) -> Label {
    Label {
        x,
        y: 300.,
        text: text.into(),
        color: Color(20, 40, 60, 255),
        size: 12.,
        anchor: TextAnchor::Middle,
        rotation,
    }
}
fn one() -> Scene {
    let mut s = ChartConfig::default().scene().unwrap();
    let mut m = Mark::new(
        "rect",
        Geometry::Rect {
            x: 10.,
            y: 20.,
            width: 30.,
            height: 40.,
        },
        Color(10, 20, 30, 255),
    );
    m.series = "A".into();
    m.label = "Rectangle".into();
    s.marks.push(m);
    s
}

#[test]
fn unique_owned_labels_follow_a_moving_datum_across_insertions_and_deletions() {
    let mut initial = one();
    initial.labels = vec![
        label("old", 12., 0.),
        label("Total", 24., 0.),
        label("Total", 36., 0.),
    ];
    initial.label_series = [(0, "old".into()), (1, "A".into()), (2, "B".into())].into();
    let mut target = initial.clone();
    target.labels = vec![
        label("Total", 140., 0.),
        label("new", 90., 0.),
        label("Total", 20., 0.),
    ];
    target.label_series = [(0, "A".into()), (1, "new".into()), (2, "B".into())].into();

    let mut motion = PropertyMotion::new(initial, Default::default()).unwrap();
    motion.retarget(target).unwrap();
    assert_eq!(motion.stats().exiting_labels, 1);
    let first = motion.label_property(0, "x").unwrap();
    assert_eq!((first.value, first.target), (24., 140.));
    let second = motion.label_property(2, "x").unwrap();
    assert_eq!((second.value, second.target), (36., 20.));
}

#[test]
fn ambiguous_moving_labels_in_one_series_keep_their_slot() {
    let mut initial = one();
    initial.labels = vec![label("Total", 24., 0.), label("Total", 36., 0.)];
    initial.label_series = [(0, "A".into()), (1, "A".into())].into();
    let mut target = initial.clone();
    target.labels = vec![
        label("new", 90., 0.),
        label("Total", 140., 0.),
        label("Total", 20., 0.),
    ];
    target.label_series = [(0, "new".into()), (1, "A".into()), (2, "A".into())].into();

    let mut motion = PropertyMotion::new(initial, Default::default()).unwrap();
    motion.retarget(target).unwrap();
    assert_eq!(motion.stats().exiting_labels, 1);
    let retained = motion.label_property(1, "x").unwrap();
    assert_eq!((retained.value, retained.target), (36., 140.));
    assert_eq!(motion.label_property(2, "x").unwrap().value, 20.);
}

fn scene() -> Scene {
    let mut s = one();
    s.marks[0].stroke = Some(GraphicStroke {
        color: Color(5, 6, 7, 255),
        width: 2.,
        cap: StrokeCap::Round,
    });
    s.marks[0].opacity = 0.8;
    s.marks[0].clip = Some(ClipRect {
        x: 0.,
        y: 0.,
        width: 600.,
        height: 400.,
    });
    let mut circle = Mark::new(
        "circle",
        Geometry::Circle {
            x: 150.,
            y: 100.,
            radius: 20.,
        },
        Color(50, 60, 70, 255),
    );
    circle.series = "B".into();
    s.marks.push(circle);
    let mut path = Mark::new(
        "path",
        Geometry::Path(vec![
            Cmd::Move(P::new(200., 50.)),
            Cmd::Cubic {
                first: P::new(220., 20.),
                second: P::new(260., 80.),
                end: P::new(280., 50.),
            },
            Cmd::Line(P::new(300., 50.)),
            Cmd::Close,
        ]),
        Color(70, 80, 90, 255),
    );
    path.series = "C".into();
    s.marks.push(path);
    s.labels = vec![label("owned", 100., 0.), label("axis", 300., 350.)];
    s.label_series.insert(0, "A".into());
    s.draw_order = vec![
        DrawItem::Mark(0),
        DrawItem::Label(0),
        DrawItem::Mark(1),
        DrawItem::Mark(2),
        DrawItem::Label(1),
    ];
    s.validate().unwrap();
    s
}
fn target() -> Scene {
    let mut s = scene();
    s.width = 1000.;
    s.height = 500.;
    s.background = Color(100, 180, 240, 128);
    let m = &mut s.marks[0];
    m.geometry = Geometry::Rect {
        x: 110.,
        y: -40.,
        width: 70.,
        height: 80.,
    };
    m.fill = Some(Color(110, 120, 130, 128));
    m.stroke = Some(GraphicStroke {
        color: Color(105, 106, 107, 100),
        width: 6.,
        cap: StrokeCap::Round,
    });
    m.opacity = 0.2;
    m.clip = Some(ClipRect {
        x: 20.,
        y: 10.,
        width: 400.,
        height: 300.,
    });
    m.value = 42.;
    s.marks[1].geometry = Geometry::Circle {
        x: 200.,
        y: 160.,
        radius: 40.,
    };
    s.marks[1].fill = Some(Color(150, 160, 170, 255));
    s.marks[2].geometry = Geometry::Path(vec![
        Cmd::Move(P::new(220., 70.)),
        Cmd::Cubic {
            first: P::new(260., 10.),
            second: P::new(280., 120.),
            end: P::new(350., 70.),
        },
        Cmd::Line(P::new(370., 70.)),
        Cmd::Close,
    ]);
    s.labels[0].x = 200.;
    s.labels[0].y = 250.;
    s.labels[0].size = 24.;
    s.labels[0].rotation = 90.;
    s.labels[0].color = Color(100, 120, 180, 128);
    s.labels[1].rotation = -350.;
    s.validate().unwrap();
    s
}
#[test]
fn changing_springs_preserves_live_positions_and_velocities() {
    let mut slow = PropertyMotion::new(scene(), Default::default()).unwrap();
    slow.retarget(target()).unwrap();
    slow.tick(1. / 60.).unwrap();
    let mut fast = slow.clone();
    let state = fast.mark_property("rect", "geometry.x").unwrap();
    assert_ne!(state.velocity, 0.);
    let config = SpringConfig {
        stiffness: 400.,
        ..Default::default()
    };
    fast.set_springs(config, config).unwrap();
    assert_eq!(fast.mark_property("rect", "geometry.x").unwrap(), state);
    assert_eq!(fast.scene(), slow.scene());
    slow.tick(1. / 60.).unwrap();
    fast.tick(1. / 60.).unwrap();
    assert_ne!(
        fast.mark_velocity("rect", "geometry.x"),
        slow.mark_velocity("rect", "geometry.x")
    );
    fast.finish();
    assert_eq!(fast.scene(), &target());
}
fn mark<'a>(s: &'a Scene, id: &str) -> &'a Mark {
    s.marks.iter().find(|m| m.id == id).unwrap()
}
fn settle(m: &mut PropertyMotion) {
    for _ in 0..1000 {
        m.tick(0.016).unwrap().validate().unwrap();
        if m.is_settled() {
            return;
        }
    }
    panic!("property engine failed to settle");
}

#[test]
fn filtering_ticks_preserves_survivors_and_their_running_springs() {
    let mut all = ChartConfig::default().scene().unwrap();
    all.labels = vec![
        label("0", 10., 0.),
        label("0", 80., 0.),
        label("Mon", 120., 0.),
    ];
    let mut motion = PropertyMotion::new(all.clone(), Default::default()).unwrap();
    all.labels[2].x += 50.;
    motion.retarget(all.clone()).unwrap();
    motion.tick(0.05).unwrap();
    let moving = motion.label_property(2, "x").unwrap();
    assert_ne!(moving.velocity, 0.);
    let mut filtered = all.clone();
    filtered.labels.remove(0);
    motion.retarget(filtered).unwrap();
    assert_eq!(motion.stats().exiting_labels, 1);
    assert_eq!(motion.stats().labels, 3);
    assert_eq!(motion.label_property(1, "x").unwrap(), moving);
    assert_eq!(motion.label_property(0, "presence").unwrap().value, 1.);
    motion.tick(0.025).unwrap();
    let running = motion.label_property(1, "x").unwrap();
    motion.retarget(all.clone()).unwrap();
    assert_eq!(motion.stats().exiting_labels, 0);
    assert_eq!(motion.stats().labels, 3);
    assert_eq!(motion.label_property(2, "x").unwrap(), running);
    settle(&mut motion);
    assert_eq!(motion.scene(), &all);
}

#[test]
fn all_properties_tween_and_finish_to_the_exact_target() {
    let initial = scene();
    let final_scene = target();
    let mut m = PropertyMotion::new(initial, Default::default()).unwrap();
    assert!(m.is_settled());
    m.retarget(final_scene.clone()).unwrap();
    m.tick(0.1).unwrap();
    m.scene().validate().unwrap();
    for (id, path) in [
        ("rect", "geometry.x"),
        ("rect", "geometry.y"),
        ("rect", "geometry.width"),
        ("rect", "fill.r"),
        ("rect", "stroke.g"),
        ("rect", "stroke.width"),
        ("rect", "opacity"),
        ("rect", "clip.width"),
        ("circle", "geometry.radius"),
        ("path", "geometry.path.1.first.x"),
    ] {
        assert!(m.mark_velocity(id, path).unwrap().abs() > 0., "{id}.{path}");
    }
    for path in ["x", "y", "size", "rotation", "color.b"] {
        assert!(
            m.label_velocity(0, path).unwrap().abs() > 0.,
            "label {path}"
        );
    }
    assert!(m.frame_property("width").unwrap().velocity > 0.);
    assert!(m.frame_property("background.r").unwrap().velocity < 0.);
    assert_eq!(mark(m.scene(), "rect").value, 42.); // Target data metadata, not a geometric spring.
    settle(&mut m);
    assert_eq!(m.scene(), &final_scene);
    assert_eq!(m.stats().exiting_marks, 0);
    assert_eq!(m.stats().exiting_labels, 0);
}
#[test]
fn retarget_preserves_each_independent_velocity_instead_of_one_projected_scalar() {
    let mut m = PropertyMotion::new(scene(), Default::default()).unwrap();
    m.retarget(target()).unwrap();
    m.tick(0.1).unwrap();
    let paths = [
        ("rect", "geometry.x"),
        ("rect", "geometry.y"),
        ("rect", "geometry.width"),
        ("rect", "fill.r"),
        ("rect", "stroke.width"),
        ("rect", "opacity"),
        ("rect", "clip.width"),
        ("circle", "geometry.radius"),
        ("path", "geometry.path.1.first.x"),
    ];
    let before: Vec<_> = paths
        .iter()
        .map(|(id, path)| m.mark_property(id, path).unwrap())
        .collect();
    let labels: Vec<_> = ["x", "y", "size", "rotation", "color.r"]
        .iter()
        .map(|path| m.label_property(0, path).unwrap())
        .collect();
    let frame = m.frame_property("width").unwrap();
    let mut next = target();
    next.marks[0].geometry = Geometry::Rect {
        x: -100.,
        y: 70.,
        width: 5.,
        height: 20.,
    };
    next.marks[0].fill = Some(Color(0, 1, 2, 255));
    next.marks[0].opacity = 0.95;
    next.marks[0].clip.as_mut().unwrap().width = 550.;
    next.labels[0].x = 50.;
    next.labels[0].rotation = -45.;
    next.width = 700.;
    m.retarget(next).unwrap();
    for ((id, path), before) in paths.iter().zip(before) {
        let after = m.mark_property(id, path).unwrap();
        assert_eq!(after.value, before.value);
        assert_eq!(after.velocity, before.velocity, "{id}.{path}");
    }
    for (path, before) in ["x", "y", "size", "rotation", "color.r"].iter().zip(labels) {
        assert_eq!(m.label_property(0, path).unwrap().velocity, before.velocity);
    }
    assert_eq!(m.frame_property("width").unwrap().velocity, frame.velocity);
    let x = m.mark_property("rect", "geometry.x").unwrap().value;
    m.tick(0.001).unwrap();
    assert!(m.mark_property("rect", "geometry.x").unwrap().value > x); // Momentum opposes the reversed target initially.
}
#[test]
fn targeting_the_current_painted_frame_keeps_active_momentum() {
    let mut final_scene = one();
    final_scene.marks[0].geometry = Geometry::Rect {
        x: 110.,
        y: 20.,
        width: 30.,
        height: 40.,
    };
    let mut m = PropertyMotion::new(one(), Default::default()).unwrap();
    m.retarget(final_scene).unwrap();
    m.tick(0.1).unwrap();
    let before = m.mark_property("rect", "geometry.x").unwrap();
    let current = m.scene().clone();
    m.retarget(current).unwrap();
    assert_eq!(m.mark_velocity("rect", "geometry.x"), Some(before.velocity));
    assert!(!m.is_settled());
    m.tick(0.001).unwrap();
    assert!(m.mark_property("rect", "geometry.x").unwrap().value > before.value);
}
#[test]
fn ownership_changes_are_label_topology_and_do_not_reuse_old_data_velocity() {
    let mut m = PropertyMotion::new(scene(), Default::default()).unwrap();
    m.retarget(target()).unwrap();
    m.tick(0.1).unwrap();
    assert!(m.label_velocity(0, "x").unwrap() > 0.);
    let mut next = target();
    next.label_series.insert(0, "B".into());
    m.retarget(next.clone()).unwrap();
    assert_eq!(m.label_velocity(0, "x"), Some(0.));
    assert_eq!(m.stats().exiting_labels, 1);
    m.tick(0.1).unwrap();
    let s = m.scene();
    s.validate().unwrap();
    assert!(s.label_series.values().any(|owner| owner == "A"));
    assert!(s.label_series.values().any(|owner| owner == "B"));
    let filtered = s.without_series(&std::collections::BTreeSet::from(["A".into()]));
    filtered.validate().unwrap();
    assert_eq!(
        filtered.labels.iter().filter(|l| l.text == "owned").count(),
        1
    );
    settle(&mut m);
    assert_eq!(m.scene(), &next);
}
#[test]
fn topology_mismatch_crossfades_unique_ids_and_keeps_departing_physics() {
    let mut m = PropertyMotion::new(scene(), Default::default()).unwrap();
    m.retarget(target()).unwrap();
    m.tick(0.1).unwrap();
    let velocity = m.mark_velocity("rect", "geometry.x").unwrap();
    let mut next = target();
    next.marks[0].geometry = Geometry::Circle {
        x: 120.,
        y: 80.,
        radius: 20.,
    };
    m.retarget(next.clone()).unwrap();
    assert!(matches!(
        mark(m.scene(), "rect").geometry,
        Geometry::Circle { .. }
    ));
    assert_eq!(mark(m.scene(), "rect").opacity, 0.);
    let ghost = m
        .scene()
        .marks
        .iter()
        .find(|m| m.id.starts_with("kartis-property-ghost-"))
        .unwrap();
    assert!(ghost.label.is_empty());
    assert_eq!(ghost.series, "A");
    assert_eq!(m.mark_velocity(&ghost.id, "geometry.x"), Some(velocity));
    assert_eq!(m.mark_velocity("rect", "geometry.x"), Some(0.));
    assert_eq!(m.stats().exiting_marks, 1);
    settle(&mut m);
    assert_eq!(m.scene(), &next);
}
#[test]
fn user_ids_cannot_accidentally_reuse_internal_ghost_physics() {
    let mut m = PropertyMotion::new(one(), Default::default()).unwrap();
    let mut next = one();
    next.marks[0].geometry = Geometry::Circle {
        x: 100.,
        y: 100.,
        radius: 20.,
    };
    m.retarget(next.clone()).unwrap();
    let ghost = m
        .scene()
        .marks
        .iter()
        .find(|m| m.id != "rect")
        .unwrap()
        .id
        .clone();
    let mut user = Mark::new(
        ghost.clone(),
        Geometry::Rect {
            x: 500.,
            y: 20.,
            width: 10.,
            height: 10.,
        },
        Color(1, 2, 3, 255),
    );
    user.label = "user datum".into();
    next.marks.push(user);
    m.retarget(next).unwrap();
    m.scene().validate().unwrap();
    assert_eq!(mark(m.scene(), &ghost).label, "user datum");
    assert_eq!(m.mark_velocity(&ghost, "geometry.x"), Some(0.));
    assert!(
        m.scene()
            .marks
            .iter()
            .filter(|m| m.label.is_empty())
            .count()
            >= 1
    );
}
#[test]
fn enters_exits_and_revival_keep_presence_velocity_and_retire_inert_tracks() {
    let initial = one();
    let mut second = initial.clone();
    let mut added = initial.marks[0].clone();
    added.id = "enter".into();
    second.marks.push(added);
    let mut m = PropertyMotion::new(initial.clone(), Default::default()).unwrap();
    m.retarget(second.clone()).unwrap();
    assert_eq!(mark(m.scene(), "enter").opacity, 0.);
    m.tick(0.1).unwrap();
    let velocity = m.mark_velocity("enter", "presence").unwrap();
    assert!(velocity > 0.);
    m.retarget(initial).unwrap();
    assert_eq!(m.mark_velocity("enter", "presence"), Some(velocity));
    assert!(mark(m.scene(), "enter").label.is_empty());
    m.tick(0.02).unwrap();
    let velocity = m.mark_velocity("enter", "presence").unwrap();
    m.retarget(second.clone()).unwrap();
    assert_eq!(m.mark_velocity("enter", "presence"), Some(velocity));
    assert!(!mark(m.scene(), "enter").label.is_empty());
    settle(&mut m);
    assert_eq!(m.scene(), &second);
}
#[test]
fn immediate_modes_replace_topology_and_membership_without_transient_union() {
    let options = PropertyMotionOptions {
        enter: FadeMode::Immediate,
        exit: FadeMode::Immediate,
        topology: FadeMode::Immediate,
        ..Default::default()
    };
    let mut m = PropertyMotion::new(one(), options).unwrap();
    let mut target = one();
    target.marks[0].geometry = Geometry::Circle {
        x: 80.,
        y: 80.,
        radius: 10.,
    };
    m.retarget(target.clone()).unwrap();
    assert!(m.is_settled());
    assert_eq!(m.scene(), &target);
    target.marks[0].id = "new".into();
    m.retarget(target.clone()).unwrap();
    assert_eq!(m.stats().marks, 1);
    assert_eq!(m.scene(), &target);
}
#[test]
fn raw_overshoot_is_not_written_back_by_paint_constraints() {
    let options = PropertyMotionOptions {
        spring: SpringConfig {
            mass: 1.,
            stiffness: 100.,
            damping: 0.,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut initial = scene();
    if let Geometry::Rect { width, .. } = &mut initial.marks[0].geometry {
        *width = 100.;
    }
    let mut target = initial.clone();
    if let Geometry::Rect { width, .. } = &mut target.marks[0].geometry {
        *width = 1.;
    }
    target.marks[0].opacity = 0.;
    target.marks[0].fill = Some(Color(0, 0, 0, 0));
    target.marks[0].stroke.as_mut().unwrap().width = 0.000001;
    target.marks[0].clip.as_mut().unwrap().width = 1.;
    target.labels[0].size = 1.;
    target.labels[1].rotation = -350.;
    let mut m = PropertyMotion::new(initial, options).unwrap();
    m.retarget(target.clone()).unwrap();
    m.tick(0.25).unwrap();
    let raw = m.mark_property("rect", "geometry.width").unwrap();
    assert!(raw.value < 0. && raw.velocity.abs() > 1.);
    let Geometry::Rect { width, .. } = mark(m.scene(), "rect").geometry else {
        panic!()
    };
    assert_eq!(width, 0.);
    assert_eq!(mark(m.scene(), "rect").opacity, 0.);
    assert!(m.mark_property("rect", "opacity").unwrap().value < 0.);
    assert_eq!(m.scene().labels[0].size, 1.);
    m.scene().validate().unwrap();
    target.marks[0].opacity = 0.4;
    m.retarget(target).unwrap();
    assert_eq!(
        m.mark_property("rect", "geometry.width").unwrap().value,
        raw.value
    );
    assert_eq!(
        m.mark_velocity("rect", "geometry.width"),
        Some(raw.velocity)
    );
    for _ in 0..120 {
        m.tick(0.016).unwrap().validate().unwrap();
    }
}
#[test]
fn shortest_rotation_crosses_full_turn_without_paint_sticking_at_360() {
    let mut initial = one();
    initial.labels.push(label("rotate", 100., 350.));
    let mut target = initial.clone();
    target.labels[0].rotation = -350.;
    let mut m = PropertyMotion::new(initial, Default::default()).unwrap();
    m.retarget(target.clone()).unwrap();
    assert_eq!(m.label_property(0, "rotation").unwrap().target, 370.);
    let mut crossed = false;
    for _ in 0..100 {
        m.tick(0.016).unwrap().validate().unwrap();
        let raw = m.label_property(0, "rotation").unwrap().value;
        if raw > 360. {
            crossed = true;
            let paint = m.scene().labels[0].rotation;
            assert!(paint < 0.);
            assert!(((raw - paint) / 360.).round() * 360. - (raw - paint) < 1e-6);
            assert_ne!(paint, 360.);
        }
    }
    assert!(crossed);
    settle(&mut m);
    assert_eq!(m.scene(), &target);
}
#[test]
fn invalid_input_and_transient_budget_rejection_are_transactional() {
    let options = PropertyMotionOptions {
        max_transient_marks: 1,
        ..Default::default()
    };
    let mut m = PropertyMotion::new(one(), options).unwrap();
    let mut target = one();
    target.marks[0].geometry = Geometry::Rect {
        x: 100.,
        y: 20.,
        width: 30.,
        height: 40.,
    };
    m.retarget(target.clone()).unwrap();
    m.tick(0.1).unwrap();
    let before = m.scene().clone();
    let velocity = m.mark_velocity("rect", "geometry.x");
    target.marks[0].geometry = Geometry::Circle {
        x: 80.,
        y: 80.,
        radius: 10.,
    };
    assert!(m.retarget(target).is_err());
    assert_eq!(m.scene(), &before);
    assert_eq!(m.mark_velocity("rect", "geometry.x"), velocity);
    for dt in [f64::NAN, -1., 61.] {
        assert!(m.tick(dt).is_err());
        assert_eq!(m.scene(), &before);
    }
    let mut bad = one();
    bad.labels.push(label("bad", 100., f64::NAN));
    assert!(m.retarget(bad).is_err());
    assert_eq!(m.scene(), &before);
}
#[test]
fn mixed_order_permutations_keep_primary_velocity_and_crossfade_ordered_snapshots() {
    let mut m = PropertyMotion::new(scene(), Default::default()).unwrap();
    m.retarget(target()).unwrap();
    m.tick(0.1).unwrap();
    let velocity = m.mark_velocity("rect", "geometry.x");
    let mut next = target();
    next.draw_order.reverse();
    m.retarget(next.clone()).unwrap();
    assert_eq!(m.mark_velocity("rect", "geometry.x"), velocity);
    assert_eq!(m.stats().exiting_marks, 3);
    assert_eq!(m.stats().exiting_labels, 2);
    m.scene().validate().unwrap();
    let kinds: Vec<_> = m
        .scene()
        .paint_items()
        .map(|item| match item {
            PaintItem::Mark(_) => 'M',
            PaintItem::Label(_) => 'L',
        })
        .collect();
    assert_eq!(kinds, ['M', 'L', 'M', 'M', 'L', 'L', 'M', 'M', 'L', 'M']);
    settle(&mut m);
    assert_eq!(m.scene(), &next);
}
#[test]
fn reduced_motion_finishes_all_properties_and_cancels_exit_tracks() {
    let mut m = PropertyMotion::new(scene(), Default::default()).unwrap();
    m.retarget(target()).unwrap();
    m.tick(0.1).unwrap();
    let mut next = target();
    next.marks.remove(2);
    next.draw_order.clear();
    m.retarget(next.clone()).unwrap();
    assert!(m.stats().exiting_marks > 0);
    m.set_reduced_motion(true);
    assert!(m.is_settled());
    assert_eq!(m.scene(), &next);
    assert_eq!(m.stats().exiting_marks, 0);
    m.retarget(scene()).unwrap();
    assert!(m.is_settled());
    assert_eq!(m.scene(), &scene());
}
#[test]
fn property_engine_runs_through_the_generic_explicit_clock() {
    let motion = PropertyMotion::new(one(), Default::default()).unwrap();
    let mut clock = kartis::clock::ExplicitClock::from_motion(motion).unwrap();
    let mut target = one();
    target.marks[0].geometry = Geometry::Rect {
        x: 110.,
        y: 20.,
        width: 30.,
        height: 40.,
    };
    clock.retarget(target.clone()).unwrap();
    clock.step(0.1).unwrap();
    let Geometry::Rect { x, .. } = mark(clock.scene(), "rect").geometry else {
        panic!()
    };
    assert!(x > 10. && x < 110.);
    for _ in 0..1000 {
        clock.step(0.016).unwrap();
        if clock.is_settled() {
            break;
        }
    }
    assert!(clock.is_settled());
    assert_eq!(clock.scene(), &target);
    assert_eq!(clock.pending_frames(), 0);
}

#[test]
fn physical_property_tracks_are_deterministic_under_clock_subdivision() {
    for damping in [0., 10., 20., 50.] {
        let options = PropertyMotionOptions {
            spring: SpringConfig {
                stiffness: 100.,
                damping,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut a = PropertyMotion::new(scene(), options.clone()).unwrap();
        let mut b = PropertyMotion::new(scene(), options).unwrap();
        a.retarget(target()).unwrap();
        b.retarget(target()).unwrap();
        a.tick(0.1).unwrap();
        for _ in 0..10 {
            b.tick(0.01).unwrap();
        }
        for (id, path) in [
            ("rect", "geometry.x"),
            ("rect", "fill.r"),
            ("rect", "clip.width"),
            ("path", "geometry.path.1.second.y"),
        ] {
            let a = a.mark_property(id, path).unwrap();
            let b = b.mark_property(id, path).unwrap();
            assert!((a.value - b.value).abs() < 1e-9);
            assert!((a.velocity - b.velocity).abs() < 1e-8);
        }
        assert!(
            (a.label_velocity(1, "rotation").unwrap() - b.label_velocity(1, "rotation").unwrap())
                .abs()
                < 1e-8
        );
    }
}
#[test]
fn label_and_path_union_budgets_reject_before_publishing_changes() {
    let mut initial = one();
    initial.labels.push(label("old", 100., 0.));
    let options = PropertyMotionOptions {
        max_transient_labels: 1,
        ..Default::default()
    };
    let mut labels = PropertyMotion::new(initial.clone(), options).unwrap();
    let mut next = initial.clone();
    next.labels[0].text = "new".into();
    assert!(labels.retarget(next).is_err());
    assert_eq!(labels.scene(), &initial);
    assert!(labels.is_settled());
    let mut initial = one();
    initial.marks[0].geometry = Geometry::Path(vec![
        Cmd::Move(P::new(0., 0.)),
        Cmd::Line(P::new(10., 10.)),
        Cmd::Close,
    ]);
    let options = PropertyMotionOptions {
        max_path_commands: 5,
        ..Default::default()
    };
    let mut paths = PropertyMotion::new(initial.clone(), options).unwrap();
    let mut next = initial.clone();
    next.marks[0].geometry = Geometry::Path(vec![
        Cmd::Move(P::new(0., 0.)),
        Cmd::Line(P::new(10., 10.)),
        Cmd::Line(P::new(20., 0.)),
        Cmd::Close,
    ]);
    assert!(paths.retarget(next).is_err());
    assert_eq!(paths.scene(), &initial);
}
#[test]
fn option_validation_and_literal_rotation_policy_are_explicit() {
    assert!(
        PropertyMotionOptions {
            spring: SpringConfig {
                mass: 0.,
                ..Default::default()
            },
            ..Default::default()
        }
        .validate()
        .is_err()
    );
    assert!(
        PropertyMotionOptions {
            max_transient_marks: MAX_MARKS + 1,
            ..Default::default()
        }
        .validate()
        .is_err()
    );
    let mut initial = one();
    initial.labels.push(label("turn", 100., 350.));
    let mut target = initial.clone();
    target.labels[0].rotation = -350.;
    let options = PropertyMotionOptions {
        shortest_rotation: false,
        ..Default::default()
    };
    let mut motion = PropertyMotion::new(initial, options).unwrap();
    motion.retarget(target).unwrap();
    assert_eq!(motion.label_property(0, "rotation").unwrap().target, -350.);
}
