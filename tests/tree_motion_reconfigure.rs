use kartis::motion::{PropertyState, SpringConfig, SpringValue};
use kartis::*;

fn gentle() -> SpringConfig {
    TreeMotionOptions::default().spring
}

fn changed() -> SpringConfig {
    SpringConfig {
        mass: 2.,
        stiffness: 180.,
        damping: 10.,
        ..gentle()
    }
}

// Independent underdamped closed-form solution, not the product solver.
fn physical(state: PropertyState, config: SpringConfig, dt: f64) -> (f64, f64) {
    let alpha = config.damping / (2. * config.mass);
    let omega_squared = config.stiffness / config.mass;
    let beta = (omega_squared - alpha * alpha).sqrt();
    assert!(beta.is_finite() && beta > 0.);
    let displacement = state.value - state.target;
    let decay = (-alpha * dt).exp();
    let (sin, cos) = (beta * dt).sin_cos();
    (
        state.target
            + decay * (displacement * cos + (state.velocity + alpha * displacement) / beta * sin),
        decay
            * (state.velocity * cos
                - (alpha * state.velocity + omega_squared * displacement) / beta * sin),
    )
}

fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
}

fn state(spring: &SpringValue) -> PropertyState {
    PropertyState {
        value: spring.value(),
        target: spring.target(),
        velocity: spring.velocity(),
    }
}

fn source() -> Vec<HierarchyDatum> {
    vec![
        HierarchyDatum::new("a", Some("r".into()), -0.),
        HierarchyDatum::new("r", None, 0.),
        HierarchyDatum::new("b", Some("r".into()), 3.),
    ]
}

fn capture(rows: &[HierarchyDatum], shifted: bool) -> TreeEventFrame {
    let mut config = ChartConfig::default();
    config.margin.left = if shifted { 50. } else { 20. };
    let mut options = TreeOptions::default();
    options.hierarchy.labels = true;
    options.style.labels.rotation = TreeLabelRotation::Fixed(if shifted { 90. } else { 0. });
    let mut size = |_: &TreeNodeFunctionContext<'_>| Ok(if shifted { 20. } else { 14. });
    let mut color = |_: &TreeNodeFunctionContext<'_>| {
        Ok(if shifted {
            Color(110, 40, 80, 200)
        } else {
            Color(50, 100, 150, 255)
        })
    };
    let mut width = |_: &TreeLinkFunctionContext<'_>| Ok(if shifted { 5. } else { 2.5 });
    let layout = tree_layout_with_factories(
        &config,
        rows,
        &options,
        &mut TreeFactories {
            node_size: Some(&mut size),
            node_color: Some(&mut color),
            link_thickness: Some(&mut width),
            ..Default::default()
        },
    )
    .unwrap();
    TreeEventFrame::new(config, layout).unwrap()
}

fn moving_with_exits() -> TreeMotion {
    let rows = source();
    let mut motion = TreeMotion::new(capture(&rows, false), TreeMotionOptions::default()).unwrap();
    motion.retarget(capture(&rows, true)).unwrap();
    motion.tick(0.1).unwrap();
    let remaining = rows
        .into_iter()
        .filter(|row| row.id != "a")
        .collect::<Vec<_>>();
    motion.retarget(capture(&remaining, true)).unwrap();
    assert!(
        motion
            .frame()
            .nodes()
            .iter()
            .any(|row| row.phase() == TreeMotionPhase::Leaving)
    );
    assert!(
        motion
            .frame()
            .links()
            .iter()
            .any(|row| row.phase() == TreeMotionPhase::Leaving)
    );
    assert!(
        motion
            .frame()
            .labels()
            .iter()
            .any(|row| row.phase() == TreeMotionPhase::Leaving)
    );
    motion
}

#[test]
fn scalar_configuration_preserves_motion_and_changes_only_future_dynamics() {
    let mut spring = SpringValue::new(0., gentle()).unwrap();
    spring.retarget(10.).unwrap();
    spring.tick(0.1).unwrap();
    close(spring.value(), 3.616041614657691);
    close(spring.velocity(), 52.7848635081138);
    let before = state(&spring);
    let settled = spring.is_settled();
    spring.set_config(changed()).unwrap();
    assert_eq!(state(&spring), before);
    assert_eq!(spring.is_settled(), settled);
    let expected = physical(before, changed(), 0.07);
    let old_config = physical(before, gentle(), 0.07);
    spring.tick(0.07).unwrap();
    close(spring.value(), expected.0);
    close(spring.velocity(), expected.1);
    assert!((spring.velocity() - old_config.1).abs() > 1.);
    let snapshot = format!("{spring:?}");
    let mut invalid = changed();
    invalid.mass = 0.;
    assert!(spring.set_config(invalid).is_err());
    assert_eq!(format!("{spring:?}"), snapshot);
}

#[test]
fn rest_threshold_changes_wait_for_a_nonzero_tick() {
    let strict = SpringConfig {
        position_epsilon: 1e-8,
        velocity_epsilon: 1e-8,
        ..gentle()
    };
    let mut spring = SpringValue::new(0., strict).unwrap();
    spring.retarget(0.001).unwrap();
    spring.tick(0.01).unwrap();
    assert!(!spring.is_settled());
    let before = state(&spring);
    spring
        .set_config(SpringConfig {
            position_epsilon: 0.1,
            velocity_epsilon: 0.1,
            ..strict
        })
        .unwrap();
    assert_eq!(state(&spring), before);
    assert!(!spring.is_settled());
    spring.tick(0.).unwrap();
    assert_eq!(state(&spring), before);
    assert!(!spring.is_settled());
    spring.tick(0.001).unwrap();
    assert!(spring.is_settled());
    assert_eq!(spring.value(), 0.001);
    assert_eq!(spring.velocity(), 0.);
    spring.set_config(changed()).unwrap();
    assert!(spring.is_settled());
}

fn check_channels(before: &[PropertyState], after: &[PropertyState], config: SpringConfig) {
    for (before, after) in before.iter().zip(after) {
        let expected = physical(*before, config, 0.03);
        close(after.value, expected.0);
        close(after.velocity, expected.1);
        assert_eq!(after.target, before.target);
    }
}

#[test]
fn tree_options_reconfigure_live_and_leaving_tracks_without_refreshing_frames() {
    let mut motion = moving_with_exits();
    let before = motion.frame().clone();
    let snapshot = format!("{before:?}");
    let mut options = motion.options().clone();
    options.spring = changed();
    options.max_nodes = before.nodes().len();
    options.max_links = before.links().len();
    options.max_labels = before.labels().len();
    motion.set_options(options.clone()).unwrap();
    assert_eq!(motion.options(), &options);
    assert_eq!(format!("{:?}", motion.frame()), snapshot);
    assert!(std::ptr::eq(motion.frame().nodes(), before.nodes()));
    assert!(std::ptr::eq(
        motion.target().source(),
        before.target().source()
    ));
    assert_eq!(motion.frame().target_serial(), before.target_serial());
    assert_eq!(motion.frame().sample_serial(), before.sample_serial());
    assert!(!motion.reduced_motion());
    motion.tick(0.03).unwrap();
    for (old, new) in before.nodes().iter().zip(motion.frame().nodes()) {
        assert_eq!(old.key(), new.key());
        assert_eq!(old.instance(), new.instance());
        assert_eq!(old.phase(), new.phase());
        assert!(std::ptr::eq(
            old.data().frame().source(),
            new.data().frame().source()
        ));
        assert_eq!(
            old.data().datum().value.to_bits(),
            new.data().datum().value.to_bits()
        );
        check_channels(old.states(), new.states(), changed());
    }
    for (old, new) in before.links().iter().zip(motion.frame().links()) {
        assert_eq!(old.key(), new.key());
        assert_eq!(old.instance(), new.instance());
        assert_eq!(old.phase(), new.phase());
        assert!(std::ptr::eq(
            old.data().frame().source(),
            new.data().frame().source()
        ));
        check_channels(old.states(), new.states(), changed());
    }
    for (old, new) in before.labels().iter().zip(motion.frame().labels()) {
        assert_eq!(old.key(), new.key());
        assert_eq!(old.instance(), new.instance());
        assert_eq!(old.phase(), new.phase());
        assert_eq!(old.data().label_index(), new.data().label_index());
        assert!(std::ptr::eq(
            old.data().frame().source(),
            new.data().frame().source()
        ));
        check_channels(old.states(), new.states(), changed());
    }
}

#[test]
fn rejected_configuration_preserves_all_state_and_future_instance_allocation() {
    let mut motion = moving_with_exits();
    let mut control = motion.clone();
    let before = format!("{motion:?}");
    let options = motion.options().clone();
    let mut invalid = Vec::new();
    let mut candidate = options.clone();
    candidate.max_nodes = motion.frame().nodes().len() - 1;
    invalid.push(candidate);
    let mut candidate = options.clone();
    candidate.max_links = motion.frame().links().len() - 1;
    invalid.push(candidate);
    let mut candidate = options.clone();
    candidate.max_labels = motion.frame().labels().len() - 1;
    invalid.push(candidate);
    let mut candidate = options.clone();
    candidate.spring.damping = f64::NAN;
    invalid.push(candidate);
    let mut candidate = options;
    candidate.spring.velocity_epsilon = 0.;
    invalid.push(candidate);
    for candidate in invalid {
        assert!(motion.set_options(candidate).is_err());
        assert_eq!(format!("{motion:?}"), before);
    }
    motion.tick(0.03).unwrap();
    control.tick(0.03).unwrap();
    assert_eq!(format!("{motion:?}"), format!("{control:?}"));
    let mut rows = source();
    rows.push(HierarchyDatum::new("new", Some("r".into()), 7.));
    let target = capture(&rows, true);
    motion.retarget(target.clone()).unwrap();
    control.retarget(target).unwrap();
    assert_eq!(format!("{motion:?}"), format!("{control:?}"));
}

#[test]
fn changing_options_preserves_reduced_and_settled_state() {
    let mut motion = moving_with_exits();
    motion.set_reduced_motion(true).unwrap();
    let before = format!("{:?}", motion.frame());
    motion
        .set_options(TreeMotionOptions {
            spring: changed(),
            ..motion.options().clone()
        })
        .unwrap();
    assert!(motion.reduced_motion());
    assert!(motion.is_settled());
    assert_eq!(format!("{:?}", motion.frame()), before);
    motion.tick(0.1).unwrap();
    assert_eq!(format!("{:?}", motion.frame()), before);
}

#[test]
fn frame_equality_tracks_shared_snapshot_identity_not_layout_equality() {
    let rows = source();
    let mut motion = TreeMotion::new(capture(&rows, false), TreeMotionOptions::default()).unwrap();
    let initial = motion.frame().clone();
    assert_eq!(initial, initial.clone());
    motion
        .set_options(TreeMotionOptions {
            spring: changed(),
            ..motion.options().clone()
        })
        .unwrap();
    assert_eq!(motion.frame(), &initial);
    let mut replaced = rows;
    replaced[0].value = 0.;
    let target = capture(&replaced, false);
    assert_eq!(initial.target().scene(), target.scene());
    motion.retarget(target).unwrap();
    assert_ne!(motion.frame(), &initial);
    assert_eq!(motion.frame(), &motion.frame().clone());
    assert_eq!(
        initial.target().source()[0].value.to_bits(),
        (-0_f64).to_bits()
    );
    assert_eq!(motion.target().source()[0].value.to_bits(), 0_f64.to_bits());
}
