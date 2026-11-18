#![allow(dead_code)]

#[path = "../src/catalog.rs"]
mod catalog;

use glory::render::PathCommand;
use kartis::motion::PropertyMotion;
use kartis::{Geometry, Scene};

fn scene(kind: &str, variant: &str, settings: &[(&str, &str)], seed: u32) -> Scene {
    let mut options = catalog::Settings::default();
    for &(key, value) in settings {
        options.0.insert(key.into(), value.into());
    }
    catalog::configured_sample(
        kind,
        (660., 440.),
        seed,
        variant,
        100,
        Default::default(),
        &options,
    )
    .unwrap()
    .scene
}

fn assert_morph(from: &Scene, to: &Scene, prefix: &str) {
    let mut motion = PropertyMotion::new(from.clone(), Default::default()).unwrap();
    motion.retarget(to.clone()).unwrap();
    let frame = motion.tick(0.06).unwrap();
    let mut changing = 0;
    for a in from.marks.iter().filter(|mark| mark.id.starts_with(prefix)) {
        let b = to.marks.iter().find(|m| m.id == a.id).unwrap();
        let m = frame.marks.iter().find(|m| m.id == a.id).unwrap();
        assert_eq!(a.opacity, m.opacity, "{} faded during morph", a.id);
        if a.geometry != b.geometry {
            assert_ne!(m.geometry, a.geometry);
            assert_ne!(m.geometry, b.geometry);
            changing += 1;
        }
    }
    assert!(changing > 0, "{prefix} had no intermediate geometry");
}

#[test]
fn all_five_line_and_area_presets_can_morph_in_place() {
    for kind in ["line", "area"] {
        let mut from = scene(kind, "monotone", &[], 0);
        for variant in ["linear", "step", "basis", "natural", "monotone"] {
            let to = scene(kind, variant, &[], 0);
            assert_morph(&from, &to, "line-");
            if kind == "area" {
                assert_morph(&from, &to, "area-");
            }
            let changed = scene(kind, variant, &[], 1);
            assert_morph(&to, &changed, "line-");
            if variant == "step" {
                for mark in to.marks.iter().filter(|m| m.id.starts_with("line-")) {
                    let commands = mark.geometry.commands();
                    let mut from = match commands[0] {
                        PathCommand::Move(p) => p,
                        _ => unreachable!(),
                    };
                    for cmd in commands {
                        if let PathCommand::Cubic { first, second, end } = cmd {
                            assert!(
                                [first, second, end].iter().all(|p| p.x == from.x)
                                    || [first, second, end].iter().all(|p| p.y == from.y)
                            );
                            from = end;
                        }
                    }
                }
            }
            from = to;
        }
    }
}

#[test]
fn bands_parallel_lines_and_bar_corners_keep_business_identity() {
    for (kind, from, options, to, changed, prefix) in [
        (
            "stream",
            "wiggle",
            vec![],
            "wiggle",
            vec![("curve", "linear")],
            "stream:area:",
        ),
        (
            "parallel-coordinates",
            "horizontal",
            vec![],
            "smooth",
            vec![],
            "parallel:",
        ),
        (
            "parallel-coordinates",
            "smooth",
            vec![],
            "vertical",
            vec![],
            "parallel:",
        ),
        (
            "bar",
            "horizontal",
            vec![],
            "horizontal",
            vec![("corners", "square")],
            "bar-",
        ),
        ("bump", "smooth", vec![], "linear", vec![], "bump-line:"),
        (
            "area-bump",
            "middle",
            vec![],
            "linear",
            vec![],
            "area-bump:",
        ),
    ] {
        assert_morph(
            &scene(kind, from, &options, 0),
            &scene(kind, to, &changed, 0),
            prefix,
        );
    }
}

#[test]
fn bars_enter_from_the_value_axis_and_radar_from_the_plot_center() {
    for kind in ["bar", "marimekko"] {
        for variant in [
            if kind == "bar" { "grouped" } else { "vertical" },
            "horizontal",
        ] {
            let to = scene(kind, variant, &[], 0);
            let start = catalog::entrance_scene(kind, variant, 100, &to);
            let prefix = if kind == "bar" { "bar-" } else { "mekko-" };
            assert_morph(&start, &to, prefix);
            for mark in start.marks.iter().filter(|m| m.id.starts_with(prefix)) {
                let points: Vec<_> = mark
                    .geometry
                    .commands()
                    .into_iter()
                    .filter_map(|cmd| match cmd {
                        PathCommand::Move(p)
                        | PathCommand::Line(p)
                        | PathCommand::Cubic { end: p, .. } => Some(p),
                        _ => None,
                    })
                    .collect();
                assert!(points.iter().all(|p| if variant == "horizontal" {
                    p.x == points[0].x
                } else {
                    p.y == points[0].y
                }));
            }
        }
    }
    for variant in ["default", "rotated"] {
        let target = scene("radar", variant, &[], 0);
        let start = catalog::entrance_scene("radar", variant, 100, &target);
        assert_morph(&start, &target, "series:");
        for mark in &start.marks {
            if let Geometry::Circle { x, y, radius } = mark.geometry {
                assert_eq!((x, y, radius), (340., 211., 0.));
            }
        }
    }
}

#[test]
fn all_cartesian_families_paint_intermediate_entrance_config_and_data_frames() {
    for (kind, preset, other) in [
        ("line", "monotone", "step"),
        ("area", "basis", "natural"),
        ("bar", "grouped", "stacked"),
        ("scatter", "small", "large"),
        ("radar", "default", "rotated"),
        ("stream", "wiggle", "silhouette"),
        ("bump", "smooth", "linear"),
        ("area-bump", "middle", "linear"),
        ("marimekko", "vertical", "horizontal"),
        ("parallel-coordinates", "vertical", "smooth"),
    ] {
        let first = scene(kind, preset, &[], 0);
        let entry = catalog::entrance_scene(kind, preset, 100, &first);
        let preset_change = scene(kind, other, &[], 0);
        let update = scene(kind, other, &[], 1);
        for (stage, from, to) in [
            ("entry", &entry, &first),
            ("config", &first, &preset_change),
            ("data", &preset_change, &update),
        ] {
            let mut spring = PropertyMotion::new(from.clone(), Default::default()).unwrap();
            spring.retarget(to.clone()).unwrap();
            let frame = spring.tick(0.08).unwrap();
            assert!(
                to.marks.iter().filter(|m| !m.series.is_empty()).any(|b| {
                    let Some(a) = from.marks.iter().find(|m| m.id == b.id) else {
                        return false;
                    };
                    let Some(m) = frame.marks.iter().find(|m| m.id == b.id) else {
                        return false;
                    };
                    (a.geometry != b.geometry
                        && m.geometry != a.geometry
                        && m.geometry != b.geometry)
                        || (a.clip != b.clip && m.clip != a.clip && m.clip != b.clip)
                }),
                "{kind} {stage} shows no shared mark in motion"
            );
            spring.finish();
            assert_eq!(
                &spring.scene().marks,
                &to.marks,
                "{kind} {stage} rest endpoint"
            );
        }
    }
}
