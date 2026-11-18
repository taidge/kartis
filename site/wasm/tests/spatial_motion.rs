#[allow(dead_code)]
#[path = "../src/catalog.rs"]
mod catalog;

use glory::render::{GraphicPoint, PathCommand};
use kartis::motion::PropertyMotion;
use kartis::{Geometry, Mark, Scene};

fn sample(kind: &str, variant: &str, seed: u32, options: &[(&str, &str)]) -> Scene {
    let mut settings = catalog::Settings::default();
    for &(key, value) in options {
        settings.0.insert(key.into(), value.into());
    }
    catalog::configured_sample(
        kind,
        (630., 400.),
        seed,
        variant,
        100,
        Default::default(),
        &settings,
    )
    .unwrap()
    .scene
}

fn differs(a: &Mark, b: &Mark) -> bool {
    a.id != b.id
        || a.geometry != b.geometry
        || a.opacity != b.opacity
        || a.fill != b.fill
        || a.stroke != b.stroke
        || a.clip != b.clip
}

fn paint_changes(a: &Scene, b: &Scene) -> bool {
    a.marks.len() != b.marks.len() || a.marks.iter().zip(&b.marks).any(|(a, b)| differs(a, b))
}

fn verify_transition(kind: &str, event: &str, from: Scene, to: &Scene) {
    let mut motion = PropertyMotion::new(from.clone(), Default::default()).unwrap();
    motion.retarget(to.clone()).unwrap();
    let early = motion.tick(0.032).unwrap().clone();
    let middle = motion.tick(0.064).unwrap();
    middle.validate().unwrap();
    assert!(
        paint_changes(&from, &early),
        "{kind} {event}: unchanged first frame"
    );
    assert!(
        paint_changes(&early, middle),
        "{kind} {event}: missing intermediate paint"
    );
    assert!(
        paint_changes(middle, to),
        "{kind} {event}: jumped to target"
    );
    motion.finish();
    assert_eq!(motion.scene(), to, "{kind} {event} final target");
}

#[test]
fn spatial_and_statistical_families_move_on_entry_configuration_and_data() {
    for (kind, variant) in [
        ("heatmap", "spaced"),
        ("boxplot", "extrema"),
        ("bullet", "reverse"),
        ("calendar", "monday"),
        ("time-range", "sunday"),
        ("swarm", "vertical"),
        ("choropleth", "equirectangular"),
        ("geo-map", "equal-earth"),
        ("voronoi", "cells"),
        ("delaunay", "wire"),
        ("contour", "fine"),
        ("density", "broad"),
    ] {
        let target = sample(kind, "", 0, &[]);
        verify_transition(
            kind,
            "entry",
            catalog::entrance_scene(kind, "", 100, &target),
            &target,
        );
        let configured = sample(kind, variant, 0, &[]);
        verify_transition(kind, "config", target, &configured);
        verify_transition(kind, "data", configured, &sample(kind, variant, 1, &[]));
    }
}

#[test]
fn bullet_segments_share_the_domain_start_and_keep_full_track_thickness() {
    for (variant, vertical, reverse) in [
        ("", false, false),
        ("vertical", true, false),
        ("reverse", false, true),
    ] {
        let to = sample("bullet", variant, 0, &[]);
        let from = catalog::entrance_scene("bullet", variant, 100, &to);
        for (a, b) in from.marks.iter().zip(&to.marks) {
            if a.id.starts_with("bullet-axis") {
                assert_eq!(a, b);
            } else if let (
                Geometry::Rect {
                    x,
                    y,
                    width,
                    height,
                },
                Geometry::Rect {
                    width: w,
                    height: h,
                    ..
                },
            ) = (&a.geometry, &b.geometry)
            {
                let track = b.clip.unwrap();
                if vertical {
                    assert_eq!(
                        (*y, *height, width),
                        (track.y + track.height, 0., w),
                        "{}",
                        a.id
                    );
                } else {
                    assert_eq!(
                        (*x, *width, height),
                        (track.x + if reverse { track.width } else { 0. }, 0., h),
                        "{}",
                        a.id
                    );
                }
            }
        }
    }
}

#[test]
fn boxplot_starts_at_each_groups_median_in_both_orientations() {
    for variant in ["", "horizontal"] {
        let target = sample("boxplot", variant, 0, &[]);
        let from = catalog::entrance_scene("boxplot", variant, 100, &target);
        for median in target.marks.iter().filter(|m| m.id.ends_with(":median")) {
            let body = from
                .marks
                .iter()
                .find(|m| Some(m.id.as_str()) == median.id.strip_suffix(":median"))
                .unwrap();
            let Geometry::Path(commands) = &median.geometry else {
                panic!()
            };
            let PathCommand::Move(p) = commands[0] else {
                panic!()
            };
            let Geometry::Rect {
                x,
                y,
                width,
                height,
            } = body.geometry
            else {
                panic!()
            };
            if variant == "horizontal" {
                assert_eq!((x, width), (p.x, 0.));
                assert!(height > 0.);
            } else {
                assert_eq!((y, height), (p.y, 0.));
                assert!(width > 0.);
            }
        }
        assert!(target.marks.iter().any(|m| m.id.contains(":outlier:")));
    }
}

#[test]
fn voronoi_cells_open_at_their_sites_and_square_days_keep_rounded_corner_slots() {
    let target = sample("voronoi", "", 0, &[]);
    let from = catalog::entrance_scene("voronoi", "", 100, &target);
    for cell in from.marks.iter().filter(|m| m.id.starts_with("cell:")) {
        let id = cell.id.strip_prefix("cell:").unwrap();
        let site = target
            .marks
            .iter()
            .find(|m| m.id.strip_prefix("point:") == Some(id))
            .unwrap();
        let Geometry::Circle { x, y, .. } = site.geometry else {
            panic!()
        };
        let Geometry::Path(commands) = &cell.geometry else {
            panic!()
        };
        assert!(commands.iter().all(|cmd| match cmd {
            PathCommand::Move(p) | PathCommand::Line(p) => *p == GraphicPoint::new(x, y),
            PathCommand::Close => true,
            _ => false,
        }));
    }
    for variant in ["", "vertical"] {
        let round = sample("time-range", variant, 1, &[]);
        let square = sample("time-range", variant, 1, &[("dayCorners", "square")]);
        let mut motion = PropertyMotion::new(round.clone(), Default::default()).unwrap();
        motion.retarget(square.clone()).unwrap();
        let frame = motion.tick(0.045).unwrap().clone();
        assert_eq!(motion.stats().exiting_marks, 0);
        let middle = frame.marks.iter().find(|m| m.id == "2026-01-01").unwrap();
        for scene in [&round, &square] {
            let day = scene.marks.iter().find(|m| m.id == middle.id).unwrap();
            assert_ne!(middle.geometry, day.geometry);
            assert_eq!(middle.opacity, day.opacity);
        }
    }
}

#[test]
fn geographic_values_preserve_borders_and_contours_resolve_their_interior_curves() {
    for kind in ["choropleth", "geo-map"] {
        let initial = sample(kind, "", 0, &[]);
        let updated = sample(kind, "", 1, &[]);
        let boundaries: Vec<_> = initial
            .marks
            .iter()
            .filter(|m| m.id.starts_with("map:"))
            .collect();
        assert_eq!(boundaries.len(), 175);
        for a in boundaries {
            let b = updated.marks.iter().find(|m| m.id == a.id).unwrap();
            assert_eq!(a.geometry, b.geometry);
            if kind == "geo-map" {
                assert!(
                    !differs(a, b),
                    "{} changes paint on a symbol-only update",
                    a.id
                );
            }
        }
    }
    let contour = sample("contour", "", 0, &[]);
    let isoline = contour.marks.iter().find(|m| m.value == 40.).unwrap();
    let Geometry::Path(commands) = &isoline.geometry else {
        panic!()
    };
    let mut last = None;
    let mut segments = 0;
    for cmd in commands {
        match *cmd {
            PathCommand::Move(p) => last = Some(p),
            PathCommand::Line(p) => {
                let q = last.unwrap();
                assert!(
                    (q.x - p.x).hypot(q.y - p.y) < 14.,
                    "visible angular step at {q:?}"
                );
                last = Some(p);
                segments += 1;
            }
            PathCommand::Close => last = None,
            PathCommand::Cubic { .. } => panic!("contour crossings must stay on the scalar grid"),
        }
    }
    assert!(
        segments >= 50,
        "the 40-unit isoline has only {segments} measured threshold crossings"
    );
}
