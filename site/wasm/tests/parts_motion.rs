#[allow(dead_code)]
#[path = "../src/catalog.rs"]
mod catalog;

use glory::render::PathCommand::{Close, Cubic, Line, Move};
use kartis::motion::PropertyMotion;
use kartis::{Geometry, Scene};

fn sample(kind: &str, variant: &str, seed: u32) -> Scene {
    catalog::scene_with_variant(kind, 710., 420., seed, variant).unwrap()
}

fn assert_continuous(from: Scene, to: Scene) {
    let mut motion = PropertyMotion::new(from.clone(), Default::default()).unwrap();
    motion.retarget(to.clone()).unwrap();
    let frame = motion.tick(1. / 30.).unwrap();
    let moving = frame.marks.iter().any(|m| {
        let a = from.marks.iter().find(|a| a.id == m.id);
        let b = to.marks.iter().find(|b| b.id == m.id);
        a.zip(b).is_some_and(|(a, b)| {
            a.geometry != b.geometry && m.geometry != a.geometry && m.geometry != b.geometry
        })
    });
    assert!(moving, "{} has no intermediate geometry", to.title);
    assert_eq!(
        motion.stats().exiting_marks,
        0,
        "matching marks should morph in place"
    );
    motion.finish();
    assert_eq!(motion.scene(), &to);
}

#[test]
fn every_sector_preset_and_dataset_keeps_native_curves() {
    for kind in ["pie", "radial-bar", "polar-bar"] {
        let mut previous = sample(kind, "", 0);
        let start = catalog::entrance_scene(kind, "", 100, &previous);
        assert_continuous(start, previous.clone());
        for &(variant, _) in catalog::variants(kind) {
            for seed in 0..17 {
                let next = sample(kind, variant, seed);
                for mark in next.marks.iter().filter(|m| !m.series.is_empty()) {
                    let commands = mark.geometry.commands();
                    assert!(commands.iter().any(|c| matches!(c, Cubic { .. })));
                }
                if previous != next {
                    assert_continuous(previous, next.clone());
                }
                previous = next;
            }
        }
    }
}

#[test]
fn funnel_sides_morph_between_basis_and_linear() {
    let target = sample("funnel", "linear", 0);
    assert_continuous(sample("funnel", "vertical", 0), target.clone());
    assert_continuous(target, sample("funnel", "linear", 1));
}

#[test]
fn waffle_entrance_grows_every_cell_around_its_own_center() {
    let target = sample("waffle", "", 0);
    let start = catalog::entrance_scene("waffle", "", 100, &target);
    for (a, b) in start
        .marks
        .iter()
        .zip(&target.marks)
        .filter(|(m, _)| m.id.starts_with("waffle-cell:"))
    {
        let Geometry::Path(path) = &a.geometry else {
            panic!("expected rounded cell")
        };
        let Move(center) = path[0] else {
            panic!("missing cell center")
        };
        assert!(path.iter().all(|c| match c {
            Move(p) | Line(p) => *p == center,
            Cubic { first, second, end } => *first == center && *second == center && *end == center,
            Close => true,
        }));
        assert_eq!(a.opacity, b.opacity);
    }
    assert_continuous(start, target);
}
