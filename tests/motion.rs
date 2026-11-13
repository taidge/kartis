use kartis::motion::*;
use kartis::*;
fn scene() -> Scene {
    let mut s = ChartConfig::default().scene().unwrap();
    let mut m = Mark::new(
        "common",
        Geometry::Rect {
            x: 10.,
            y: 20.,
            width: 30.,
            height: 40.,
        },
        Color(255, 0, 0, 255),
    );
    m.series = "series".into();
    m.clip = Some(ClipRect {
        x: 0.,
        y: 0.,
        width: 300.,
        height: 300.,
    });
    s.marks.push(m);
    s
}
fn target() -> Scene {
    let mut s = scene();
    s.marks[0].geometry = Geometry::Rect {
        x: 90.,
        y: 60.,
        width: 60.,
        height: 80.,
    };
    s.marks[0].fill = Some(Color(0, 0, 255, 255));
    s.marks[0].clip = Some(ClipRect {
        x: 10.,
        y: 20.,
        width: 290.,
        height: 280.,
    });
    s.width = 900.;
    s
}
fn motion() -> SpringMotion {
    SpringMotion::new(scene(), SpringConfig::default()).unwrap()
}
#[test]
fn compatible_geometry_colors_clip_and_resize_interpolate_and_finish_exactly() {
    let target = target();
    let mut m = motion();
    m.retarget(target.clone()).unwrap();
    assert!(!m.is_settled());
    m.tick(0.1).unwrap();
    let t = m.progress();
    assert!(t > 0. && t < 1.);
    let s = m.scene();
    s.validate().unwrap();
    assert_eq!(s.marks.len(), 1);
    assert_eq!(s.marks[0].id, "common");
    assert!((s.width - (800. + 100. * t)).abs() < 1e-9);
    let Geometry::Rect { x, .. } = s.marks[0].geometry else {
        panic!()
    };
    assert!((x - (10. + 80. * t)).abs() < 1e-9);
    assert!((s.marks[0].clip.unwrap().x - 10. * t).abs() < 1e-9);
    for _ in 0..1000 {
        m.tick(0.016).unwrap();
        if m.is_settled() {
            break;
        }
    }
    assert!(m.is_settled());
    assert_eq!(m.scene(), &target);
    assert_eq!(m.velocity(), 0.);
}
#[test]
fn enter_and_exit_fade_without_leaving_interactive_retired_data() {
    let mut initial = scene();
    let mut exit = initial.marks[0].clone();
    exit.id = "exit".into();
    initial.marks.push(exit);
    let mut target = target();
    let mut enter = target.marks[0].clone();
    enter.id = "enter".into();
    target.marks.push(enter);
    let mut m = SpringMotion::new(initial, SpringConfig::default()).unwrap();
    m.retarget(target.clone()).unwrap();
    assert_eq!(
        m.scene()
            .marks
            .iter()
            .find(|m| m.id == "enter")
            .unwrap()
            .opacity,
        0.
    );
    m.tick(0.1).unwrap();
    let t = m.progress();
    let s = m.scene();
    assert!((s.marks.iter().find(|m| m.id == "enter").unwrap().opacity - t).abs() < 1e-9);
    let exit = s.marks.iter().find(|m| m.id == "exit").unwrap();
    assert!(exit.label.is_empty());
    assert_eq!(exit.series, "series"); // Visibility ownership survives retirement.
    assert!((exit.opacity - (1. - t)).abs() < 1e-9);
    m.finish();
    assert_eq!(m.scene(), &target);
    assert!(m.is_settled());
}
#[test]
fn topology_changes_crossfade_distinct_valid_identities() {
    let mut target = scene();
    target.marks[0].geometry = Geometry::Circle {
        x: 70.,
        y: 80.,
        radius: 10.,
    };
    let mut m = motion();
    m.retarget(target.clone()).unwrap();
    assert_eq!(m.scene().marks.len(), 2);
    let ghost = m.scene().marks.iter().find(|m| m.id != "common").unwrap();
    assert!(ghost.id.starts_with("kartis-motion-exit-"));
    assert!(ghost.label.is_empty());
    for _ in 0..40 {
        m.tick(0.016).unwrap().validate().unwrap();
    }
    m.finish();
    assert_eq!(m.scene(), &target);
}
#[test]
fn analytic_spring_is_deterministic_under_dt_subdivision_in_all_damping_regimes() {
    for damping in [0., 10., 20., 50.] {
        let config = SpringConfig {
            stiffness: 100.,
            damping,
            ..Default::default()
        };
        let mut a = SpringMotion::new(scene(), config).unwrap();
        let mut b = SpringMotion::new(scene(), config).unwrap();
        a.retarget(target()).unwrap();
        b.retarget(target()).unwrap();
        a.tick(0.1).unwrap();
        for _ in 0..10 {
            b.tick(0.01).unwrap();
        }
        assert!(
            (a.progress() - b.progress()).abs() < 1e-10,
            "damping {damping}"
        );
        assert!((a.velocity() - b.velocity()).abs() < 1e-9);
    }
}
#[test]
fn overshoot_is_painted_with_valid_sizes_and_opacity_and_undamped_motion_stays_active() {
    let mut m = SpringMotion::new(
        scene(),
        SpringConfig {
            damping: 0.,
            ..Default::default()
        },
    )
    .unwrap();
    m.retarget(target()).unwrap();
    for _ in 0..300 {
        m.tick(0.016).unwrap().validate().unwrap();
        assert!((0. ..=1.).contains(&m.progress()));
    }
    assert!(!m.is_settled());
    m.finish();
    assert!(m.is_settled());
}
#[test]
fn rejected_clock_or_target_preserves_the_running_animation() {
    let mut m = motion();
    m.retarget(target()).unwrap();
    m.tick(0.1).unwrap();
    let before = m.scene().clone();
    let progress = m.progress();
    let velocity = m.velocity();
    for dt in [f64::NAN, f64::INFINITY, -1., 61.] {
        assert!(m.tick(dt).is_err());
        assert_eq!(m.scene(), &before);
        assert_eq!(m.progress(), progress);
        assert_eq!(m.velocity(), velocity);
    }
    let mut invalid = target();
    invalid.marks[0].opacity = -1.;
    assert!(m.retarget(invalid).is_err());
    assert_eq!(m.scene(), &before);
}
#[test]
fn retarget_uses_the_displayed_frame_and_reduced_motion_snaps() {
    let mut m = motion();
    m.retarget(target()).unwrap();
    m.tick(0.1).unwrap();
    let before = m.scene().marks[0].geometry.clone();
    let mut other = target();
    other.marks[0].geometry = Geometry::Rect {
        x: 140.,
        y: 20.,
        width: 10.,
        height: 10.,
    };
    m.retarget(other.clone()).unwrap();
    assert_eq!(m.scene().marks[0].geometry, before);
    m.set_reduced_motion(true);
    assert_eq!(m.scene(), &other);
    m.retarget(scene()).unwrap();
    assert!(m.is_settled());
    assert_eq!(m.scene(), &scene());
}
#[test]
fn changed_text_crossfades_instead_of_flashing_at_retarget() {
    let label = |text: &str| Label {
        rotation: 0.,
        x: 20.,
        y: 20.,
        text: text.into(),
        color: Color(0, 0, 0, 255),
        size: 12.,
        anchor: TextAnchor::Start,
    };
    let mut initial = scene();
    initial.labels.push(label("old"));
    let mut target = target();
    target.labels.push(label("new"));
    let mut m = SpringMotion::new(initial, SpringConfig::default()).unwrap();
    m.retarget(target).unwrap();
    assert_eq!(m.scene().labels[0].color.3, 255);
    assert_eq!(m.scene().labels[1].color.3, 0);
    m.tick(0.1).unwrap();
    assert!(
        m.scene()
            .labels
            .iter()
            .all(|l| l.color.3 > 0 && l.color.3 < 255)
    );
}
#[test]
fn transient_union_budgets_reject_atomically() {
    let mut initial = scene();
    let prototype = initial.marks[0].clone();
    initial.marks = (0..10_001)
        .map(|i| {
            let mut m = prototype.clone();
            m.id = format!("old{i}");
            m
        })
        .collect();
    let mut next = initial.clone();
    for (i, m) in next.marks.iter_mut().enumerate() {
        m.id = format!("new{i}");
    }
    let mut m = SpringMotion::new(initial.clone(), SpringConfig::default()).unwrap();
    assert!(m.retarget(next).is_err());
    assert_eq!(m.scene(), &initial);
    assert!(m.is_settled());
}
#[test]
fn invalid_spring_parameters_are_rejected() {
    for config in [
        SpringConfig {
            mass: 0.,
            ..Default::default()
        },
        SpringConfig {
            stiffness: f64::NAN,
            ..Default::default()
        },
        SpringConfig {
            damping: -1.,
            ..Default::default()
        },
        SpringConfig {
            position_epsilon: 0.,
            ..Default::default()
        },
    ] {
        assert!(SpringMotion::new(scene(), config).is_err());
    }
}

fn label(text: &str) -> Label {
    Label {
        rotation: 0.,
        x: 20.,
        y: 20.,
        text: text.into(),
        color: Color(0, 0, 0, 255),
        size: 12.,
        anchor: TextAnchor::Start,
    }
}
fn painted(scene: &Scene) -> Vec<String> {
    scene
        .paint_items()
        .filter_map(|item| match item {
            PaintItem::Mark(m) if m.opacity > 0. => Some(format!("M:{}", m.value)),
            PaintItem::Label(l) if l.color.3 > 0 => Some(format!("L:{}", l.text)),
            _ => None,
        })
        .collect()
}
#[test]
fn mixed_order_and_owned_labels_survive_compatible_interpolation() {
    let mut from = scene();
    from.labels.push(label("owned"));
    from.label_series.insert(0, "series".into());
    from.draw_order = vec![DrawItem::Label(0), DrawItem::Mark(0)];
    let mut target = from.clone();
    target.labels[0].x = 120.;
    target.marks[0].value = 55.;
    let mut motion = SpringMotion::new(from, SpringConfig::default()).unwrap();
    motion.retarget(target.clone()).unwrap();
    for _ in 0..30 {
        let frame = motion.tick(0.016).unwrap();
        frame.validate().unwrap();
        assert_eq!(frame.draw_items(), [DrawItem::Label(0), DrawItem::Mark(0)]);
        assert_eq!(
            frame.label_series.get(&0).map(String::as_str),
            Some("series")
        );
        assert_eq!(frame.labels.len(), 1);
    }
    motion.finish();
    assert_eq!(motion.scene(), &target);
}
#[test]
fn exits_keep_origin_anchor_order_and_label_ownership_after_slot_remapping() {
    let mut from = scene();
    from.marks[0].value = 1.;
    let mut exit = from.marks[0].clone();
    exit.id = "exit".into();
    exit.series = "departing".into();
    exit.value = 2.;
    from.marks.push(exit);
    from.labels = vec![label("axis"), label("exit caption")];
    from.label_series.insert(1, "departing".into());
    from.draw_order = vec![
        DrawItem::Label(1),
        DrawItem::Mark(0),
        DrawItem::Label(0),
        DrawItem::Mark(1),
    ];
    let initial = painted(&from);
    let mut target = from.clone();
    target.marks[1].id = "enter".into();
    target.marks[1].series = "arriving".into();
    target.marks[1].value = 3.;
    target.labels.truncate(1);
    target.label_series.clear();
    target.draw_order = vec![DrawItem::Mark(0), DrawItem::Label(0), DrawItem::Mark(1)];
    let mut motion = SpringMotion::new(from, SpringConfig::default()).unwrap();
    motion.retarget(target.clone()).unwrap();
    assert_eq!(painted(motion.scene()), initial);
    for _ in 0..30 {
        let frame = motion.tick(0.016).unwrap();
        frame.validate().unwrap();
        assert_eq!(
            frame.label_series.get(&0).map(String::as_str),
            Some("departing")
        );
        let filtered =
            frame.without_series(&std::collections::BTreeSet::from(["departing".into()]));
        filtered.validate().unwrap();
        assert!(!filtered.labels.iter().any(|l| l.text == "exit caption"));
    }
    motion.finish();
    assert_eq!(motion.scene(), &target);
}
#[test]
fn paint_order_permutations_crossfade_complete_snapshots_in_actual_order() {
    let mut from = scene();
    from.marks[0].value = 1.;
    let mut second = from.marks[0].clone();
    second.id = "second".into();
    second.value = 2.;
    from.marks.push(second);
    from.labels.push(label("owned"));
    from.label_series.insert(0, "series".into());
    from.draw_order = vec![DrawItem::Mark(0), DrawItem::Label(0), DrawItem::Mark(1)];
    let expected_start = painted(&from);
    let mut target = from.clone();
    target.draw_order = vec![DrawItem::Label(0), DrawItem::Mark(1), DrawItem::Mark(0)];
    let mut motion = SpringMotion::new(from, SpringConfig::default()).unwrap();
    motion.retarget(target.clone()).unwrap();
    assert_eq!(painted(motion.scene()), expected_start);
    motion.tick(0.1).unwrap();
    let frame = motion.scene();
    frame.validate().unwrap();
    assert_eq!(frame.marks.len(), 4);
    assert_eq!(frame.labels.len(), 2);
    assert_eq!(
        painted(frame),
        ["M:1", "L:owned", "M:2", "L:owned", "M:2", "M:1"]
    );
    assert_eq!(frame.label_series.get(&0), Some(&"series".to_string()));
    assert_eq!(frame.label_series.get(&1), Some(&"series".to_string()));
    motion.finish();
    assert_eq!(motion.scene(), &target);
}

fn x_scene(x: f64) -> Scene {
    let mut s = scene();
    s.marks[0].geometry = Geometry::Rect {
        x,
        y: 20.,
        width: 30.,
        height: 40.,
    };
    s
}
#[test]
fn scene_retarget_projects_geometry_velocity_into_new_normalized_distance() {
    let mut motion = SpringMotion::new(x_scene(10.), SpringConfig::default()).unwrap();
    motion.retarget(x_scene(110.)).unwrap();
    motion.tick(0.1).unwrap();
    let velocity = 100. * motion.velocity();
    let Geometry::Rect { x, .. } = motion.scene().marks[0].geometry else {
        panic!()
    };
    motion.retarget(x_scene(210.)).unwrap();
    assert!((motion.velocity() - velocity / (210. - x)).abs() < 1e-10);
    let Geometry::Rect { x: after, .. } = motion.scene().marks[0].geometry else {
        panic!()
    };
    assert_eq!(after, x);
}
#[test]
fn scene_velocity_projection_is_explicit_and_topology_fades_reset_it() {
    let mut motion = SpringMotion::new(x_scene(10.), SpringConfig::default()).unwrap();
    assert_eq!(
        motion.retarget_policy(),
        RetargetPolicy::PreserveProjectedVelocity
    );
    motion.retarget(x_scene(110.)).unwrap();
    motion.tick(0.1).unwrap();
    assert!(motion.velocity() > 0.);
    motion
        .retarget_with_policy(x_scene(210.), RetargetPolicy::Reset)
        .unwrap();
    assert_eq!(motion.velocity(), 0.);
    motion.tick(0.1).unwrap();
    let mut circle = x_scene(210.);
    circle.marks[0].geometry = Geometry::Circle {
        x: 200.,
        y: 100.,
        radius: 20.,
    };
    motion.retarget(circle).unwrap();
    assert_eq!(motion.velocity(), 0.);
    motion.set_retarget_policy(RetargetPolicy::Reset);
    assert_eq!(motion.retarget_policy(), RetargetPolicy::Reset);
}
#[test]
fn physical_scalar_tracks_preserve_independent_velocity_across_reversing_targets() {
    let mut x = SpringValue::new(0., SpringConfig::default()).unwrap();
    let mut y = x.clone();
    x.retarget(100.).unwrap();
    y.retarget(200.).unwrap();
    x.tick(0.1).unwrap();
    y.tick(0.1).unwrap();
    let vx = x.velocity();
    let vy = y.velocity();
    let current = x.value();
    x.retarget(-100.).unwrap();
    y.retarget(400.).unwrap();
    assert_eq!(x.velocity(), vx);
    assert_eq!(y.velocity(), vy);
    assert!(x.tick(0.001).unwrap() > current); // Momentum initially opposes the new target.
    x.retarget_with_policy(-200., ScalarVelocityPolicy::Reset)
        .unwrap();
    assert_eq!(x.velocity(), 0.);
}
#[test]
fn scalar_spring_clock_reduced_motion_and_overflow_are_checked() {
    let mut scalar = SpringValue::new(0., SpringConfig::default()).unwrap();
    scalar.retarget(100.).unwrap();
    let mut split = scalar.clone();
    scalar.tick(0.1).unwrap();
    for _ in 0..10 {
        split.tick(0.01).unwrap();
    }
    assert!((scalar.value() - split.value()).abs() < 1e-9);
    assert!((scalar.velocity() - split.velocity()).abs() < 1e-9);
    let before = scalar.value();
    assert!(scalar.tick(f64::NAN).is_err());
    assert_eq!(scalar.value(), before);
    scalar.set_reduced_motion(true);
    assert_eq!(scalar.value(), 100.);
    scalar.retarget(250.).unwrap();
    assert_eq!(scalar.value(), 250.);
    assert!(scalar.is_settled());
    let mut extreme = SpringValue::new(f64::MAX, SpringConfig::default()).unwrap();
    assert!(extreme.retarget(-f64::MAX).is_err());
    assert_eq!(extreme.value(), f64::MAX);
    assert_eq!(extreme.target(), f64::MAX);
}

#[test]
fn label_rotation_interpolates_about_the_anchor_with_other_label_properties() {
    let mut from = ChartConfig::default().scene().unwrap();
    from.labels.push(Label {
        x: 10.,
        y: 20.,
        text: "tick".into(),
        rotation: -60.,
        color: Color(0, 0, 0, 255),
        size: 12.,
        anchor: TextAnchor::End,
    });
    let mut to = from.clone();
    to.labels[0].rotation = 30.;
    to.labels[0].x = 30.;
    let mut spring = SpringMotion::new(from, SpringConfig::default()).unwrap();
    spring.retarget(to).unwrap();
    spring.tick(0.06).unwrap();
    let progress = spring.progress();
    assert!(progress > 0. && progress < 1.);
    assert!((spring.scene().labels[0].rotation - (-60. + 90. * progress)).abs() < 1e-9);
    assert!((spring.scene().labels[0].x - (10. + 20. * progress)).abs() < 1e-9);
}
