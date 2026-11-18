#[allow(dead_code)]
#[path = "../src/catalog.rs"]
mod catalog;

use glory::render::PathCommand::{Cubic, Line, Move};
use kartis::motion::PropertyMotion;
use kartis::{Geometry, Scene};

fn sample(kind: &str, variant: &str, seed: u32) -> Scene {
    catalog::scene_with_variant(kind, 730., 410., seed, variant).unwrap()
}

fn assert_retained_marks(from: &Scene, to: &Scene, context: &str) {
    let removed = from
        .marks
        .iter()
        .filter(|m| !to.marks.iter().any(|n| n.id == m.id))
        .count();
    let mut motion = PropertyMotion::new(from.clone(), Default::default()).unwrap();
    motion.retarget(to.clone()).unwrap();
    motion.tick(1. / 30.).unwrap().validate().unwrap();
    assert_eq!(
        motion.stats().exiting_marks,
        removed,
        "{context} retained a duplicate"
    );
    let mut remaining = (0..to.labels.len()).collect::<Vec<_>>();
    let mut lost_labels = 0;
    for (i, a) in from.labels.iter().enumerate() {
        if let Some(index) = remaining.iter().position(|&j| {
            let b = &to.labels[j];
            a.text == b.text
                && a.anchor == b.anchor
                && from.label_series.get(&i) == to.label_series.get(&j)
        }) {
            remaining.swap_remove(index);
        } else {
            lost_labels += 1;
        }
    }
    assert_eq!(
        motion.stats().exiting_labels,
        lost_labels,
        "{context} duplicated a retained label"
    );
}

#[test]
fn every_authored_layout_seed_and_independent_control_retains_common_marks() {
    for kind in [
        "tree",
        "network",
        "sankey",
        "treemap",
        "circle-packing",
        "icicle",
        "chord",
        "sunburst",
    ] {
        let layouts = catalog::variants(kind);
        for &(variant, _) in layouts {
            for seed in 0..17 {
                let current = sample(kind, variant, seed);
                let next = sample(kind, variant, (seed + 1) % 17);
                assert_retained_marks(&current, &next, &format!("{kind}/{variant}/seed#{seed}"));
            }
            let current = sample(kind, variant, 3);
            for &(other, _) in layouts {
                assert_retained_marks(
                    &current,
                    &sample(kind, other, 3),
                    &format!("{kind}/{variant}->{other}"),
                );
            }
            for (_, controls) in catalog::control_groups(kind).into_iter().take(3) {
                for control in controls {
                    for &(choice, _) in control.choices.iter().skip(1) {
                        let options = catalog::Settings(
                            [(control.key.to_string(), choice.to_string())].into(),
                        );
                        let configured = catalog::configured_sample(
                            kind,
                            (730., 410.),
                            3,
                            variant,
                            100,
                            Default::default(),
                            &options,
                        )
                        .unwrap()
                        .scene;
                        let context = format!("{kind}/{variant}/{}={choice}", control.key);
                        assert_retained_marks(&current, &configured, &context);
                        assert_retained_marks(&configured, &current, &context);
                    }
                }
            }
        }
    }
}

fn check_in_between(kind: &str, from: Scene, to: &Scene) {
    let mut motion = PropertyMotion::new(from.clone(), Default::default()).unwrap();
    motion.retarget(to.clone()).unwrap();
    let middle = motion.tick(1. / 30.).unwrap();
    middle.validate().unwrap();
    let moving = middle.marks.iter().any(|mark| {
        let old = from.marks.iter().find(|m| m.id == mark.id);
        let end = to.marks.iter().find(|m| m.id == mark.id);
        old.zip(end).is_some_and(|(a, b)| {
            a.geometry != b.geometry && mark.geometry != a.geometry && mark.geometry != b.geometry
        })
    });
    assert!(moving, "{kind} needs a real intermediate geometry");
    assert_eq!(
        motion.stats().exiting_marks,
        0,
        "{kind} has crossfading copies"
    );
    motion.finish();
    assert_eq!(motion.scene(), to);
}

#[test]
fn each_family_moves_geometry_on_entry_layout_and_data_updates() {
    for (kind, variant) in [
        ("tree", "radial"),
        ("network", "compact"),
        ("sankey", "thin"),
        ("treemap", "squarify"),
        ("circle-packing", "shelf"),
        ("icicle", "flush"),
        ("sunburst", "flush"),
        ("chord", "wide"),
    ] {
        let base = sample(kind, "", 0);
        let start = catalog::entrance_scene(kind, "", 100, &base);
        for &i in base.label_series.keys() {
            assert_eq!(
                start.labels[i].color.3, 0,
                "{kind} label flashes before its node"
            );
        }
        check_in_between(kind, start, &base);
        let configured = sample(kind, variant, 0);
        check_in_between(kind, base, &configured);
        check_in_between(kind, configured, &sample(kind, variant, 1));
    }
}

#[test]
fn sector_and_ribbon_curves_survive_every_sample_seed() {
    for kind in ["sunburst", "chord"] {
        for seed in 0..17 {
            let goal = sample(kind, "", seed);
            let start = catalog::entrance_scene(kind, "", 100, &goal);
            for (a, b) in start.marks.iter().zip(&goal.marks) {
                assert_ne!(
                    a.geometry, b.geometry,
                    "{kind} {} only fades on entry",
                    a.id
                );
            }
            check_in_between(kind, goal, &sample(kind, "", (seed + 1) % 17));
        }
    }
}

#[test]
fn outline_controls_animate_the_stroke_without_replacing_the_fill() {
    for kind in ["sunburst", "icicle", "treemap", "circle-packing", "chord"] {
        let key = if kind == "chord" {
            "group-outline"
        } else {
            "node-outline"
        };
        let options = catalog::Settings([(key.to_owned(), "alternate".to_owned())].into());
        let from = sample(kind, "", 1);
        let goal = catalog::configured_sample(
            kind,
            (730., 410.),
            1,
            "",
            100,
            Default::default(),
            &options,
        )
        .unwrap()
        .scene;
        let mut animation = PropertyMotion::new(from.clone(), Default::default()).unwrap();
        animation.retarget(goal.clone()).unwrap();
        let middle = animation.tick(1. / 30.).unwrap();
        assert!(
            middle.marks.iter().any(|m| {
                let a = from.marks.iter().find(|a| a.id == m.id).unwrap();
                let b = goal.marks.iter().find(|b| b.id == m.id).unwrap();
                m.geometry == a.geometry
                    && m.fill == a.fill
                    && m.stroke != a.stroke
                    && m.stroke != b.stroke
            }),
            "{kind} should draw an intermediate outline"
        );
        assert_eq!(
            animation.stats().exiting_marks,
            0,
            "{kind} should retain every filled shape"
        );
    }
}

#[test]
fn sankey_flows_open_from_their_curved_centerline_and_tree_nodes_keep_their_color() {
    let goal = sample("sankey", "", 2);
    let start = catalog::entrance_scene("sankey", "", 100, &goal);
    for m in start.marks.iter().filter(|m| m.id.starts_with("flow:")) {
        let Geometry::Path(path) = &m.geometry else {
            panic!("flow must be a path")
        };
        let [
            Move(left),
            Cubic {
                first,
                second,
                end: right,
            },
            Line(return_right),
            Cubic {
                first: second_back,
                second: first_back,
                end: return_left,
            },
            _,
        ] = &path[..]
        else {
            panic!("flow must retain both cubic boundaries")
        };
        for (a, b) in [
            (left, return_left),
            (right, return_right),
            (first, first_back),
            (second, second_back),
        ] {
            assert!((a.x - b.x).hypot(a.y - b.y) < 1e-10);
        }
    }
    let before = sample("tree", "", 0);
    let after = sample("tree", "", 1);
    for node in before.marks.iter().filter(|m| m.id.starts_with("node:")) {
        let updated = after.marks.iter().find(|m| m.id == node.id).unwrap();
        assert_eq!(node.fill, updated.fill);
    }
}
