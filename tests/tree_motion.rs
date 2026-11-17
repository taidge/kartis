use std::cell::Cell;

use kartis::*;

fn rows() -> Vec<HierarchyDatum> {
    vec![
        HierarchyDatum::new("a", Some("branch".into()), -0.),
        HierarchyDatum::new("r", None, -0.),
        HierarchyDatum::new("branch", Some("r".into()), 0.),
        HierarchyDatum::new("b", Some("r".into()), 5.),
        HierarchyDatum::new("c", Some("branch".into()), 3.),
    ]
}

fn build(data: &[HierarchyDatum], left: f64, rotation: f64) -> TreeEventFrame {
    let config = ChartConfig {
        width: 420.,
        height: 280.,
        margin: Margin {
            left,
            right: 24.,
            top: 20.,
            bottom: 20.,
        },
        ..Default::default()
    };
    build_at(data, config, rotation)
}

fn build_at(data: &[HierarchyDatum], config: ChartConfig, rotation: f64) -> TreeEventFrame {
    let mut options = TreeOptions::default();
    options.hierarchy.labels = true;
    options.style.labels.rotation = TreeLabelRotation::Fixed(rotation);
    let mut size = |_: &TreeNodeFunctionContext<'_>| Ok(14.);
    let mut color = |_: &TreeNodeFunctionContext<'_>| Ok(Color(50, 100, 150, 255));
    let mut width = |_: &TreeLinkFunctionContext<'_>| Ok(2.5);
    let layout = tree_layout_with_factories(
        &config,
        data,
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

fn close(actual: f64, wanted: f64) {
    assert!(
        (actual - wanted).abs() <= 1e-9,
        "physical value {actual:.16} != {wanted:.16}"
    );
}

// Independent closed-form solution for the chosen mass=1, k=120, damping=14.
// This deliberately does not use the production SpringValue as an oracle.
fn physical(x: f64, velocity: f64, target: f64, dt: f64) -> (f64, f64) {
    let alpha = 7.;
    let beta = 71_f64.sqrt();
    let decay = (-alpha * dt).exp();
    let (sin, cos) = (beta * dt).sin_cos();
    let displacement = x - target;
    (
        target + decay * (displacement * cos + (velocity + alpha * displacement) / beta * sin),
        decay * (velocity * cos - (alpha * velocity + 120. * displacement) / beta * sin),
    )
}

fn node<'a>(motion: &'a TreeMotion, id: &str) -> &'a TreeMotionNode {
    motion
        .frame()
        .nodes()
        .iter()
        .find(|n| n.key() == id)
        .unwrap()
}

fn label<'a>(motion: &'a TreeMotion, id: &str) -> &'a TreeMotionLabel {
    motion
        .frame()
        .labels()
        .iter()
        .find(|n| n.key() == id)
        .unwrap()
}

fn fingerprint(motion: &TreeMotion) -> String {
    format!("{:?}", motion.frame())
}

#[test]
fn initial_and_new_roles_enter_at_complete_targets_with_shared_source_data() {
    let frame = build(&rows(), 20., 0.);
    let mut motion = TreeMotion::new(frame.clone(), TreeMotionOptions::default()).unwrap();
    assert_eq!(motion.options().spring.mass, 1.);
    assert_eq!(motion.options().spring.stiffness, 120.);
    assert_eq!(motion.options().spring.damping, 14.);
    assert!(motion.is_settled());
    assert_eq!(motion.frame().target_serial(), 0);
    assert_eq!(motion.frame().sample_serial(), 0);
    assert_eq!(motion.frame().nodes().len(), 5);
    assert_eq!(motion.frame().links().len(), 4);
    assert_eq!(motion.frame().labels().len(), 5);
    for row in motion.frame().nodes() {
        assert_eq!(row.phase(), TreeMotionPhase::Live);
        assert_eq!([row.props().x, row.props().y], row.data().node().position);
        assert_eq!(row.props().diameter, 14.);
        assert_eq!(row.props().color, Color(50, 100, 150, 255));
        assert!(row.states().iter().all(|s| s.velocity == 0.));
        assert!(std::ptr::eq(row.data().frame().source(), frame.source()));
    }
    let reduced = vec![rows()[1].clone(), rows()[3].clone()];
    let other = TreeMotion::new(build(&reduced, 20., 0.), TreeMotionOptions::default()).unwrap();
    motion = other;
    motion.retarget(frame.clone()).unwrap();
    assert_eq!(node(&motion, "a").props().diameter, 14.);
    assert_eq!(node(&motion, "a").phase(), TreeMotionPhase::Live);
    assert_eq!(node(&motion, "a").states()[2].velocity, 0.);
    assert_eq!(
        node(&motion, "a").data().datum().value.to_bits(),
        (-0_f64).to_bits()
    );
}

#[test]
fn coordinate_spring_and_mid_flight_retarget_preserve_independent_physical_velocity() {
    let mut motion =
        TreeMotion::new(build(&rows(), 20., 0.), TreeMotionOptions::default()).unwrap();
    close(node(&motion, "r").props().x, 24.);
    motion.retarget(build(&rows(), 30., 0.)).unwrap();
    assert_eq!(motion.frame().target_serial(), 1);
    assert_eq!(motion.frame().sample_serial(), 0);
    motion.tick(0.1).unwrap();
    close(
        node(&motion, "r").states()[0].value,
        24. + 3.616041614657691,
    );
    close(node(&motion, "r").states()[0].velocity, 52.7848635081138);
    let first = node(&motion, "r").states()[0];
    let serial = motion.frame().sample_serial();
    motion.tick(0.).unwrap();
    assert_eq!(node(&motion, "r").states()[0], first);
    assert_eq!(motion.frame().sample_serial(), serial);
    motion.retarget(build(&rows(), 16., 0.)).unwrap();
    assert_eq!(node(&motion, "r").states()[0].value, first.value);
    assert_eq!(node(&motion, "r").states()[0].velocity, first.velocity);
    close(node(&motion, "r").states()[0].target, 20.);
    motion.tick(0.07).unwrap();
    close(
        node(&motion, "r").states()[0].value,
        24. + 4.168022874707477,
    );
    close(node(&motion, "r").states()[0].velocity, -25.0282065748095);
    assert!((node(&motion, "r").states()[0].value - (24. + 2.0333926424963975)).abs() > 2.);
}

#[test]
fn removed_node_shrinks_without_clamping_physics_and_reentry_retains_its_instance() {
    let original = build(&rows(), 20., 0.);
    let mut motion = TreeMotion::new(original.clone(), TreeMotionOptions::default()).unwrap();
    let instance = node(&motion, "a").instance();
    let reduced = rows()
        .into_iter()
        .filter(|r| r.id != "a")
        .collect::<Vec<_>>();
    motion.retarget(build(&reduced, 20., 0.)).unwrap();
    assert_eq!(node(&motion, "a").phase(), TreeMotionPhase::Leaving);
    assert!(std::ptr::eq(
        node(&motion, "a").data().frame().source(),
        original.source()
    ));
    assert_eq!(node(&motion, "a").states()[2].target, 0.);
    assert!(
        motion
            .frame()
            .links()
            .iter()
            .all(|r| r.key().target_id != "a")
    );
    assert!(motion.frame().labels().iter().all(|r| r.key() != "a"));
    motion.tick(0.4).unwrap();
    close(node(&motion, "a").states()[2].value, -0.9895980613717406);
    close(node(&motion, "a").states()[2].velocity, 2.7506869774702554);
    assert_eq!(node(&motion, "a").props().diameter, 0.);
    let physical = node(&motion, "a").states()[2];
    motion.retarget(original.clone()).unwrap();
    assert_eq!(node(&motion, "a").instance(), instance);
    assert_eq!(node(&motion, "a").phase(), TreeMotionPhase::Live);
    assert_eq!(node(&motion, "a").states()[2].value, physical.value);
    assert_eq!(node(&motion, "a").states()[2].velocity, physical.velocity);
    motion.finish().unwrap();
    assert_eq!(node(&motion, "a").props().diameter, 14.);
    motion.retarget(build(&reduced, 20., 0.)).unwrap();
    motion.finish().unwrap();
    assert!(motion.frame().nodes().iter().all(|r| r.key() != "a"));
    motion.retarget(original).unwrap();
    assert_ne!(node(&motion, "a").instance(), instance);
    assert_eq!(node(&motion, "a").props().diameter, 14.);
}

#[test]
fn departing_link_and_label_continue_their_old_targets_and_provenance_without_fade() {
    let mut motion =
        TreeMotion::new(build(&rows(), 20., 0.), TreeMotionOptions::default()).unwrap();
    let moving = build(&rows(), 30., 90.);
    motion.retarget(moving.clone()).unwrap();
    motion.tick(0.1).unwrap();
    let old_label = label(&motion, "branch").states()[0];
    let survivors = rows()
        .into_iter()
        .filter(|r| r.id == "r" || r.id == "b")
        .collect::<Vec<_>>();
    motion.retarget(build(&survivors, 16., 0.)).unwrap();
    let link = motion
        .frame()
        .links()
        .iter()
        .find(|r| r.key().target_id == "branch")
        .unwrap();
    assert_eq!(link.phase(), TreeMotionPhase::Leaving);
    assert!(std::ptr::eq(link.data().frame().source(), moving.source()));
    assert_eq!(link.props().thickness, 2.5);
    assert_eq!(link.states()[4].target, 2.5);
    assert_eq!(link.states()[0].target, 34.);
    assert_eq!(label(&motion, "branch").phase(), TreeMotionPhase::Leaving);
    assert_eq!(label(&motion, "branch").states()[0], old_label);
    motion.tick(0.07).unwrap();
    let wanted = physical(24., 0., 34., 0.17);
    let link = motion
        .frame()
        .links()
        .iter()
        .find(|r| r.key().target_id == "branch")
        .unwrap();
    close(link.states()[0].value, wanted.0);
    close(link.states()[0].velocity, wanted.1);
    assert_eq!(link.props().thickness, 2.5);
    motion.finish().unwrap();
    assert!(
        motion
            .frame()
            .links()
            .iter()
            .all(|r| r.phase() == TreeMotionPhase::Live)
    );
    assert!(
        motion
            .frame()
            .labels()
            .iter()
            .all(|r| r.phase() == TreeMotionPhase::Live)
    );
    assert!(motion.is_settled());
}

#[test]
fn source_reordering_preserves_business_label_identity_and_rotation_takes_direct_numeric_path() {
    let original = build(&rows(), 20., 350.);
    let mut motion = TreeMotion::new(original, TreeMotionOptions::default()).unwrap();
    let old_instance = label(&motion, "a").instance();
    let old_slot = label(&motion, "a").data().label_index();
    let mut reordered = rows();
    reordered.reverse();
    let target = build(&reordered, 20., 10.);
    motion.retarget(target).unwrap();
    assert_eq!(label(&motion, "a").instance(), old_instance);
    assert_ne!(label(&motion, "a").data().label_index(), old_slot);
    assert_eq!(label(&motion, "a").data().datum().id, "a");
    assert_eq!(label(&motion, "a").states()[2].value, 350.);
    assert_eq!(label(&motion, "a").states()[2].target, 10.);
    motion.tick(0.1).unwrap();
    close(
        label(&motion, "a").states()[2].value,
        physical(350., 0., 10., 0.1).0,
    );
    assert!(label(&motion, "a").props().rotation < 250.);
}

#[test]
fn reparenting_keeps_node_identity_but_changes_the_structured_link_key() {
    let mut motion =
        TreeMotion::new(build(&rows(), 20., 0.), TreeMotionOptions::default()).unwrap();
    let original = node(&motion, "a").instance();
    let mut reparented = rows();
    reparented[0].parent = Some("r".into());
    motion.retarget(build(&reparented, 20., 0.)).unwrap();
    assert_eq!(node(&motion, "a").instance(), original);
    let link = motion
        .frame()
        .links()
        .iter()
        .find(|l| l.key().target_id == "a")
        .unwrap();
    assert_eq!(link.key().source_id, "r");
    assert_eq!(link.phase(), TreeMotionPhase::Live);
    assert!(
        motion
            .frame()
            .links()
            .iter()
            .all(|l| l.key().source_id != "branch" || l.key().target_id != "a")
    );
}

#[test]
fn invalid_time_and_transient_union_budget_reject_atomically() {
    let first = vec![
        HierarchyDatum::new("r", None, 0.),
        HierarchyDatum::new("a", Some("r".into()), 1.),
        HierarchyDatum::new("b", Some("r".into()), 2.),
    ];
    let replacement = vec![
        HierarchyDatum::new("r", None, 0.),
        HierarchyDatum::new("c", Some("r".into()), 3.),
        HierarchyDatum::new("d", Some("r".into()), 4.),
    ];
    let mut motion = TreeMotion::new(
        build(&first, 20., 0.),
        TreeMotionOptions {
            max_nodes: 3,
            ..Default::default()
        },
    )
    .unwrap();
    let prior = fingerprint(&motion);
    for dt in [-1., f64::NAN, f64::INFINITY, 60.0001] {
        assert!(motion.tick(dt).is_err());
        assert_eq!(fingerprint(&motion), prior);
    }
    assert!(motion.retarget(build(&replacement, 20., 0.)).is_err());
    assert_eq!(fingerprint(&motion), prior);
    assert!(motion.is_settled());
}

#[test]
fn reduced_motion_and_finish_clear_exits_and_publish_exact_target_values() {
    let mut motion =
        TreeMotion::new(build(&rows(), 20., 0.), TreeMotionOptions::default()).unwrap();
    let remaining = rows()
        .into_iter()
        .filter(|r| r.id != "a")
        .collect::<Vec<_>>();
    motion.retarget(build(&remaining, 30., 90.)).unwrap();
    motion.tick(0.1).unwrap();
    motion.set_reduced_motion(true).unwrap();
    assert!(motion.reduced_motion());
    assert!(motion.is_settled());
    assert!(
        motion
            .frame()
            .nodes()
            .iter()
            .all(|n| n.phase() == TreeMotionPhase::Live)
    );
    for row in motion.frame().nodes() {
        assert_eq!([row.props().x, row.props().y], row.data().node().position);
        assert!(
            row.states()
                .iter()
                .all(|s| s.value == s.target && s.velocity == 0.)
        );
    }
    motion.retarget(build(&rows(), 16., 10.)).unwrap();
    assert!(motion.is_settled());
    close(node(&motion, "r").props().x, 20.);
    motion.set_reduced_motion(false).unwrap();
    assert!(motion.is_settled());
}

#[test]
fn raw_only_zero_replacement_refreshes_the_target_pool_without_restarting_tracks() {
    let original = build(&rows(), 20., 0.);
    let mut motion = TreeMotion::new(original.clone(), TreeMotionOptions::default()).unwrap();
    let old = motion.frame().clone();
    let instance = node(&motion, "a").instance();
    let mut source = rows();
    source[0].value = 0.;
    let target = build(&source, 20., 0.);
    assert_eq!(original.scene(), target.scene());
    motion.retarget(target.clone()).unwrap();
    assert_eq!(motion.frame().target_serial(), 1);
    assert_eq!(motion.frame().sample_serial(), 0);
    assert!(motion.is_settled());
    assert_eq!(node(&motion, "a").instance(), instance);
    assert_eq!(
        node(&motion, "a").data().datum().value.to_bits(),
        0_f64.to_bits()
    );
    assert_eq!(
        old.nodes()
            .iter()
            .find(|n| n.key() == "a")
            .unwrap()
            .data()
            .datum()
            .value
            .to_bits(),
        (-0_f64).to_bits()
    );
    assert!(std::ptr::eq(
        node(&motion, "a").data().frame().source(),
        target.source()
    ));
    assert!(std::ptr::eq(old.target().source(), original.source()));
}

#[test]
fn structural_parent_child_keys_do_not_alias_delimiter_containing_business_ids() {
    let rows = vec![
        HierarchyDatum::new("r", None, 0.),
        HierarchyDatum::new("a:b", Some("r".into()), 0.),
        HierarchyDatum::new("c", Some("a:b".into()), 1.),
        HierarchyDatum::new("a", Some("r".into()), 0.),
        HierarchyDatum::new("b:c", Some("a".into()), 2.),
    ];
    let motion = TreeMotion::new(build(&rows, 20., 0.), TreeMotionOptions::default()).unwrap();
    let first = motion
        .frame()
        .links()
        .iter()
        .find(|l| l.key().source_id == "a:b" && l.key().target_id == "c")
        .unwrap();
    let second = motion
        .frame()
        .links()
        .iter()
        .find(|l| l.key().source_id == "a" && l.key().target_id == "b:c")
        .unwrap();
    assert_ne!(first.key(), second.key());
    assert_ne!(first.instance(), second.instance());
    assert_eq!(motion.frame().links().len(), 4);
}

#[test]
fn coordinate_presentation_clamping_does_not_change_an_independent_diameter_track() {
    let small = build(&rows(), 20., 0.);
    let mut config = small.config().clone();
    config.width = 16384.;
    config.margin.left = 16000.;
    let large = build_at(&rows(), config, 0.);
    let mut options = TreeMotionOptions::default();
    options.spring.damping = 0.;
    let mut motion = TreeMotion::new(small.clone(), options).unwrap();
    let half_period = std::f64::consts::PI / 120_f64.sqrt();
    let mut observed_clamp = false;
    for turn in 0..160 {
        motion
            .retarget(if turn % 2 == 0 {
                large.clone()
            } else {
                small.clone()
            })
            .unwrap();
        motion.tick(half_period).unwrap();
        let root = node(&motion, "r");
        assert_eq!(root.states()[2].value, 14.);
        assert_eq!(root.props().diameter, 14.);
        if root.states()[0].value.abs() > 1_000_000. {
            assert_eq!(root.props().x.abs(), 1_000_000.);
            observed_clamp = true;
            break;
        }
    }
    assert!(
        observed_clamp,
        "independent resonance must exercise the presentation bound"
    );
}

#[test]
fn reading_and_ticking_does_not_replay_factories_or_claim_unowned_scene_annotations() {
    let count = Cell::new(0);
    let config = ChartConfig::default();
    let data = rows();
    let mut options = TreeOptions::default();
    options.hierarchy.labels = true;
    options.style.links.enabled = false;
    options.style.nodes.fill = TreeNodeFill::None;
    let mut size = |_: &TreeNodeFunctionContext<'_>| {
        count.set(count.get() + 1);
        Ok(0.)
    };
    let mut layout = tree_layout_with_factories(
        &config,
        &data,
        &options,
        &mut TreeFactories {
            node_size: Some(&mut size),
            ..Default::default()
        },
    )
    .unwrap();
    let index = layout.scene.labels.len();
    let mut annotation = layout.scene.labels[0].clone();
    annotation.text = "unowned annotation".into();
    layout.scene.labels.push(annotation);
    if !layout.scene.draw_order.is_empty() {
        layout.scene.draw_order.push(DrawItem::Label(index));
    }
    let frame = TreeEventFrame::new(config, layout).unwrap();
    assert_eq!(count.get(), data.len());
    let mut motion = TreeMotion::new(frame.clone(), TreeMotionOptions::default()).unwrap();
    assert_eq!(motion.frame().nodes().len(), data.len());
    assert_eq!(motion.frame().links().len(), data.len() - 1);
    assert_eq!(motion.frame().labels().len(), data.len());
    assert_eq!(motion.target().scene().labels.len(), data.len() + 1);
    for row in motion.frame().links() {
        assert!(row.data().mark().is_none());
    }
    for _ in 0..10 {
        motion.tick(0.1).unwrap();
        let _ = motion.frame().clone();
    }
    assert_eq!(count.get(), data.len());
    assert!(std::ptr::eq(motion.target().source(), frame.source()));
}
