use glory::render::{GraphicStroke, StrokeCap};
use kartis::{motion::*, *};
fn initial() -> Scene {
    let mut scene = ChartConfig::default().scene().unwrap();
    let mut mark = Mark::new(
        "datum",
        Geometry::Rect {
            x: 50.,
            y: 50.,
            width: 100.,
            height: 100.,
        },
        Color(20, 90, 160, 255),
    );
    mark.series = "data".into();
    mark.label = "Datum".into();
    scene.marks.push(mark);
    scene.labels.push(Label {
        x: 70.,
        y: 80.,
        text: "Label".into(),
        size: 12.,
        color: Color(20, 30, 40, 255),
        anchor: TextAnchor::Start,
        rotation: 0.,
    });
    scene.label_series.insert(0, "data".into());
    scene
        .label_clips
        .insert(0, ClipPath::circle(100., 100., 40.));
    scene
}
fn settle(motion: &mut PropertyMotion) {
    for _ in 0..2000 {
        motion.tick(0.016).unwrap();
        if motion.is_settled() {
            return;
        }
    }
    panic!("did not settle");
}

#[test]
fn changed_label_clip_is_topology_and_each_fading_slot_keeps_its_own_clip() {
    let original = initial();
    let mut target = original.clone();
    target
        .label_clips
        .insert(0, ClipPath::circle(110., 100., 30.));
    let mut motion = PropertyMotion::new(original.clone(), Default::default()).unwrap();
    motion.retarget(target.clone()).unwrap();
    motion.tick(0.05).unwrap();
    assert_eq!(
        (motion.stats().labels, motion.stats().exiting_labels),
        (2, 1)
    );
    let frame = motion.scene();
    assert_eq!(frame.label_clips.len(), 2);
    for expected in [
        original.label_clips.get(&0).unwrap(),
        target.label_clips.get(&0).unwrap(),
    ] {
        assert!(frame.label_clips.values().any(|clip| clip == expected));
    }
    for index in 0..frame.labels.len() {
        assert_eq!(
            frame.label_series.get(&index).map(String::as_str),
            Some("data")
        );
        assert!(frame.labels[index].color.3 > 0);
        assert!(frame.label_clips.contains_key(&index));
    }
    settle(&mut motion);
    assert_eq!(motion.scene(), &target);
    let mut legacy = SpringMotion::new(original.clone(), Default::default()).unwrap();
    legacy.retarget(target.clone()).unwrap();
    let frame = legacy.tick(0.05).unwrap();
    assert_eq!(frame.labels.len(), 2);
    assert_eq!(frame.label_clips.len(), 2);
    assert_eq!(frame.label_clips.get(&0), original.label_clips.get(&0));
    assert_eq!(frame.label_clips.get(&1), target.label_clips.get(&0));
    legacy.finish();
    assert_eq!(legacy.scene(), &target);
}

#[test]
fn identical_clip_reuses_label_velocity_and_clip_changes_reject_atomically_at_union_budget() {
    let original = initial();
    let mut target = original.clone();
    target.labels[0].x = 140.;
    let mut motion = PropertyMotion::new(original.clone(), Default::default()).unwrap();
    motion.retarget(target.clone()).unwrap();
    motion.tick(0.05).unwrap();
    let before = motion.label_property(0, "x").unwrap();
    target.labels[0].x = 40.;
    motion.retarget(target.clone()).unwrap();
    assert_eq!(motion.stats().labels, 1);
    let after = motion.label_property(0, "x").unwrap();
    assert_eq!(
        (before.value, before.velocity),
        (after.value, after.velocity)
    );
    assert_eq!(
        motion.scene().label_clips.get(&0),
        original.label_clips.get(&0)
    );
    let mut bounded = PropertyMotion::new(
        original.clone(),
        PropertyMotionOptions {
            max_transient_labels: 1,
            ..Default::default()
        },
    )
    .unwrap();
    bounded.retarget(target.clone()).unwrap();
    bounded.tick(0.05).unwrap();
    let old = bounded.scene().clone();
    let velocity = bounded.label_velocity(0, "x");
    target.label_clips.remove(&0);
    assert!(bounded.retarget(target).is_err());
    assert_eq!(bounded.scene(), &old);
    assert_eq!(bounded.label_velocity(0, "x"), velocity);
    assert_eq!(
        bounded.target().label_clips.get(&0),
        original.label_clips.get(&0)
    );
}

#[test]
fn mixed_order_snapshot_crossfade_remaps_every_original_and_target_clip() {
    let mut original = initial();
    let mut second = original.labels[0].clone();
    second.text = "Other".into();
    second.y = 120.;
    original.labels.push(second);
    original.label_series.insert(1, "other".into());
    original.label_clips.insert(
        1,
        ClipPath {
            geometry: Geometry::Rect {
                x: 20.,
                y: 90.,
                width: 100.,
                height: 50.,
            },
            rule: ClipRule::EvenOdd,
        },
    );
    original.draw_order = vec![DrawItem::Label(0), DrawItem::Mark(0), DrawItem::Label(1)];
    let mut target = original.clone();
    target.draw_order = vec![DrawItem::Label(1), DrawItem::Mark(0), DrawItem::Label(0)];
    let check = |frame: &Scene| {
        frame.validate().unwrap();
        assert_eq!(frame.labels.len(), 4);
        assert_eq!(frame.label_clips.len(), 4);
        for (index, label) in frame.labels.iter().enumerate() {
            let source = usize::from(label.text == "Other");
            assert_eq!(
                frame.label_clips.get(&index),
                original.label_clips.get(&source)
            );
            assert_eq!(
                frame.label_series.get(&index),
                original.label_series.get(&source)
            );
        }
        assert_eq!(frame.draw_order.len(), 6);
    };
    let mut motion = PropertyMotion::new(original.clone(), Default::default()).unwrap();
    motion.retarget(target.clone()).unwrap();
    check(motion.tick(0.05).unwrap());
    settle(&mut motion);
    assert_eq!(motion.scene(), &target);
    let mut legacy = SpringMotion::new(original.clone(), Default::default()).unwrap();
    legacy.retarget(target.clone()).unwrap();
    check(legacy.tick(0.05).unwrap());
    legacy.finish();
    assert_eq!(legacy.scene(), &target);
}

#[test]
fn mark_clip_and_dash_changes_fade_with_original_metadata_and_reduced_motion_finishes_exactly() {
    let mut original = initial();
    original.marks[0].stroke = Some(GraphicStroke {
        color: Color(0, 0, 0, 255),
        width: 2.,
        cap: StrokeCap::Round,
    });
    original.marks[0].clip_path = Some(Box::new(ClipPath::circle(100., 100., 40.)));
    for changed in [0, 1, 2] {
        let mut target = original.clone();
        match changed {
            0 => target.marks[0].clip_path = None,
            1 => target.marks[0].clip_path = Some(Box::new(ClipPath::circle(100., 100., 30.))),
            _ => {
                target.marks[0].stroke_dash = Some(StrokeDash {
                    segments: vec![4., 2.],
                    offset: -1.,
                })
            }
        }
        let mut motion = PropertyMotion::new(original.clone(), Default::default()).unwrap();
        motion.retarget(target.clone()).unwrap();
        assert_eq!((motion.stats().marks, motion.stats().exiting_marks), (2, 1));
        assert!(
            motion
                .scene()
                .marks
                .iter()
                .any(|mark| mark.clip_path == original.marks[0].clip_path
                    && mark.stroke_dash == original.marks[0].stroke_dash)
        );
        motion.set_reduced_motion(true);
        assert_eq!(motion.scene(), &target);
        assert_eq!(motion.stats().marks, 1);
        let mut legacy = SpringMotion::new(original.clone(), Default::default()).unwrap();
        legacy.retarget(target.clone()).unwrap();
        assert_eq!(legacy.scene().marks.len(), 2);
        legacy.set_reduced_motion(true);
        assert_eq!(legacy.scene(), &target);
        let mut bounded = PropertyMotion::new(
            original.clone(),
            PropertyMotionOptions {
                max_transient_marks: 1,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(bounded.retarget(target).is_err());
        assert_eq!(bounded.scene(), &original);
    }
    let mut target = original.clone();
    target.marks[0].geometry = Geometry::Rect {
        x: 80.,
        y: 50.,
        width: 100.,
        height: 100.,
    };
    let mut motion = PropertyMotion::new(original, Default::default()).unwrap();
    motion.retarget(target).unwrap();
    assert_eq!(
        motion.stats().marks,
        1,
        "equal clip/dash definitions remain compatible"
    );
}

#[test]
fn adaptive_mask_union_cost_rejects_crossfade_transactionally_but_reduced_motion_can_snap() {
    use glory::render::{GraphicPoint as P, PathCommand as C};
    let mask = ClipPath {
        geometry: Geometry::Path(vec![
            C::Move(P::new(-1e6, 0.)),
            C::Cubic {
                first: P::new(-1e6, -1e6),
                second: P::new(1e6, -1e6),
                end: P::new(1e6, 0.),
            },
            C::Cubic {
                first: P::new(1e6, 1e6),
                second: P::new(-1e6, 1e6),
                end: P::new(-1e6, 0.),
            },
            C::Close,
        ]),
        rule: ClipRule::NonZero,
    };
    let cost = mask.picking_stats().unwrap().flattened_segments;
    let count = 200_000 / cost;
    let mut scene = ChartConfig::default().scene().unwrap();
    for i in 0..count {
        scene.labels.push(Label {
            x: 10.,
            y: 20.,
            size: 12.,
            text: format!("label{i}"),
            color: Color(0, 0, 0, 255),
            anchor: TextAnchor::Start,
            rotation: 0.,
        });
        scene.label_clips.insert(i, mask.clone());
    }
    scene.validate().unwrap();
    let mut target = scene.clone();
    target.label_clips.get_mut(&0).unwrap().rule = ClipRule::EvenOdd;
    target.validate().unwrap();
    let mut motion = PropertyMotion::new(scene.clone(), Default::default()).unwrap();
    assert!(
        motion
            .retarget(target.clone())
            .unwrap_err()
            .0
            .contains("picking")
    );
    assert_eq!(motion.scene(), &scene);
    assert_eq!(motion.target(), &scene);
    motion.set_reduced_motion(true);
    motion.retarget(target.clone()).unwrap();
    assert_eq!(motion.scene(), &target);
    let mut legacy = SpringMotion::new(scene.clone(), Default::default()).unwrap();
    assert!(legacy.retarget(target).is_err());
    assert_eq!(legacy.scene(), &scene);
}
