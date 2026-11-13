use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::*;
fn rectangle(x: f64, y: f64, w: f64, h: f64, reverse: bool) -> Vec<C> {
    let mut points = vec![
        P::new(x, y),
        P::new(x + w, y),
        P::new(x + w, y + h),
        P::new(x, y + h),
    ];
    if reverse {
        points.reverse();
    }
    let mut path = vec![C::Move(points[0])];
    path.extend(points.into_iter().skip(1).map(C::Line));
    path.push(C::Close);
    path
}
fn curved() -> ClipPath {
    ClipPath {
        geometry: Geometry::Path(vec![
            C::Move(P::new(0., 0.)),
            C::Cubic {
                first: P::new(10000., 0.),
                second: P::new(10000., 10000.),
                end: P::new(0., 10000.),
            },
            C::Close,
        ]),
        rule: ClipRule::NonZero,
    }
}
fn large() -> ClipPath {
    ClipPath {
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
    }
}

#[test]
fn nonzero_evenodd_holes_and_boundaries_match_contour_orientation() {
    for reverse in [false, true] {
        for rule in [ClipRule::NonZero, ClipRule::EvenOdd] {
            let path = rectangle(0., 0., 100., 100., false)
                .into_iter()
                .chain(rectangle(25., 25., 50., 50., reverse))
                .collect();
            let clip = ClipPath {
                geometry: Geometry::Path(path),
                rule,
            };
            clip.validate().unwrap();
            assert!(clip.contains(10., 50.));
            assert!(!clip.contains(-0.1, 50.));
            assert!(clip.contains(0., 50.));
            assert!(clip.contains(25., 50.));
            assert_eq!(
                clip.contains(50., 50.),
                rule == ClipRule::NonZero && !reverse
            );
        }
    }
    for geometry in [
        Geometry::Circle {
            x: 0.,
            y: 0.,
            radius: 10.,
        },
        Geometry::Rect {
            x: -10.,
            y: -10.,
            width: 20.,
            height: 20.,
        },
    ] {
        let clip = ClipPath {
            geometry,
            rule: ClipRule::EvenOdd,
        };
        clip.validate().unwrap();
        assert!(clip.contains(0., 0.));
        assert!(clip.contains(10., 0.));
        assert!(!clip.contains(10.01, 0.));
    }
    for geometry in [
        Geometry::Circle {
            x: 0.,
            y: 0.,
            radius: 0.,
        },
        Geometry::Rect {
            x: 0.,
            y: 0.,
            width: 0.,
            height: 20.,
        },
    ] {
        let clip = ClipPath {
            geometry,
            rule: ClipRule::NonZero,
        };
        clip.validate().unwrap();
        assert!(!clip.contains(0., 0.));
    }
}

#[test]
fn adaptive_cubic_picking_resolves_boundary_offsets_below_fixed48_sampling_error() {
    let clip = curved();
    let stats = clip.picking_stats().unwrap();
    assert!(stats.flattened_segments > 48);
    assert!(stats.max_error_bound <= CLIP_PICK_TOLERANCE / 2.);
    for t in [0.011_f64, 0.137, 0.289, 0.537, 0.917] {
        let x = 30000. * t * (1. - t);
        let y = 10000. * (3. * t * t - 2. * t * t * t);
        let dx = 30000. * (1. - 2. * t);
        let dy = 60000. * t * (1. - t);
        let length = dx.hypot(dy);
        let outward = [dy / length, -dx / length];
        assert!(clip.contains(x, y), "actual cubic boundary excluded at {t}");
        assert!(
            clip.contains(x - 0.1 * outward[0], y - 0.1 * outward[1]),
            "inside point excluded at {t}"
        );
        assert!(
            !clip.contains(x + 0.1 * outward[0], y + 0.1 * outward[1]),
            "outside point included at {t}"
        );
    }
    assert!(clip.contains(0., 5000.));
    assert!(!clip.contains(-0.1, 5000.));
}

#[test]
fn finite_segment_flatness_keeps_collinear_cubic_reversals() {
    let clip = ClipPath {
        geometry: Geometry::Path(vec![
            C::Move(P::new(0., 0.)),
            C::Cubic {
                first: P::new(100., 0.),
                second: P::new(-100., 0.),
                end: P::new(0., 0.),
            },
            C::Close,
        ]),
        rule: ClipRule::NonZero,
    };
    let stats = clip.picking_stats().unwrap();
    assert!(stats.flattened_segments > 4);
    assert!(clip.contains(20., 0.));
    assert!(clip.contains(-20., 0.));
    assert!(!clip.contains(50., 0.));
}

#[test]
fn malformed_open_nonfinite_and_oversized_masks_reject_before_picking() {
    let invalid = [
        Geometry::Path(vec![C::Line(P::new(0., 0.))]),
        Geometry::Path(vec![C::Close]),
        Geometry::Path(vec![C::Move(P::new(0., 0.)), C::Line(P::new(1., 1.))]),
        Geometry::Path(vec![
            C::Move(P::new(0., 0.)),
            C::Move(P::new(1., 1.)),
            C::Close,
        ]),
        Geometry::Path(vec![
            C::Move(P::new(0., 0.)),
            C::Cubic {
                first: P::new(f64::NAN, 0.),
                second: P::new(1., 1.),
                end: P::new(0., 0.),
            },
            C::Close,
        ]),
        Geometry::Circle {
            x: 1e6,
            y: 0.,
            radius: 1.,
        },
        Geometry::Rect {
            x: 0.,
            y: 0.,
            width: -1.,
            height: 1.,
        },
        Geometry::Path(vec![C::Move(P::new(0., 0.)); MAX_CLIP_COMMANDS + 1]),
    ];
    for geometry in invalid {
        let clip = ClipPath {
            geometry,
            rule: ClipRule::EvenOdd,
        };
        assert!(clip.validate().is_err());
        assert!(!clip.contains(0., 0.));
    }
    assert!(!curved().contains(f64::NAN, 0.));
    assert!(!curved().contains(0., f64::INFINITY));
}

#[test]
fn per_mask_and_combined_scene_adaptive_workload_are_pre_admitted() {
    let clip = large();
    let cost = clip.picking_stats().unwrap();
    assert!(cost.flattened_segments > 1000 && cost.flattened_segments <= MAX_CLIP_PICK_SEGMENTS);
    let Geometry::Path(path) = &clip.geometry else {
        unreachable!()
    };
    let too_many = MAX_CLIP_PICK_SEGMENTS / cost.flattened_segments + 1;
    let over = ClipPath {
        geometry: Geometry::Path(
            path.iter()
                .copied()
                .cycle()
                .take(path.len() * too_many)
                .collect(),
        ),
        rule: ClipRule::NonZero,
    };
    assert!(over.command_count() <= MAX_CLIP_COMMANDS);
    assert!(over.validate().unwrap_err().0.contains("picking"));
    assert!(!over.contains(0., 0.));
    let mut scene = ChartConfig::default().scene().unwrap();
    let count = 200_000 / cost.flattened_segments + 1;
    for i in 0..count {
        if i % 2 == 0 {
            let mut mark = Mark::new(
                format!("clip{i}"),
                Geometry::Rect {
                    x: 0.,
                    y: 0.,
                    width: 20.,
                    height: 20.,
                },
                Color(0, 0, 0, 255),
            );
            mark.clip_path = Some(Box::new(clip.clone()));
            scene.marks.push(mark);
        } else {
            scene.label_clips.insert(scene.labels.len(), clip.clone());
            scene.labels.push(Label {
                x: 0.,
                y: 12.,
                size: 12.,
                text: "Label".into(),
                anchor: TextAnchor::Start,
                color: Color(0, 0, 0, 255),
                rotation: 0.,
            });
        }
    }
    assert!(
        scene.validate().unwrap_err().0.contains("picking"),
        "source-command budget alone cannot bound expanded cubic work"
    );
    let before = scene.clone();
    assert!(render::to_svg(&scene).is_err());
    assert_eq!(scene, before);
}

#[test]
fn svg_ids_preserve_exact_identity_without_cloning_or_hashing_large_paths() {
    fn id(clip: ClipPath) -> String {
        let mut scene = ChartConfig::default().scene().unwrap();
        let mut mark = Mark::new(
            "m",
            Geometry::Rect {
                x: 0.,
                y: 0.,
                width: 10.,
                height: 10.,
            },
            Color(0, 0, 0, 255),
        );
        mark.clip_path = Some(Box::new(clip));
        scene.marks.push(mark);
        let svg = render::to_svg(&scene).unwrap();
        let start = svg.find("<clipPath id=\"").unwrap() + 14;
        svg[start..].split('"').next().unwrap().into()
    }
    let regular = ClipPath {
        geometry: Geometry::Rect {
            x: 0.,
            y: 0.,
            width: 1.,
            height: 1.,
        },
        rule: ClipRule::NonZero,
    };
    let negative_zero = ClipPath {
        geometry: Geometry::Rect {
            x: -0.,
            y: -0.,
            width: 1.,
            height: 1.,
        },
        rule: ClipRule::NonZero,
    };
    assert_eq!(id(regular.clone()), id(negative_zero));
    assert_ne!(
        id(regular.clone()),
        id(ClipPath {
            rule: ClipRule::EvenOdd,
            ..regular
        })
    );
    let mut path = vec![C::Move(P::new(0., 0.))];
    path.extend((0..MAX_CLIP_COMMANDS - 2).map(|_| C::Cubic {
        first: P::new(0., 0.),
        second: P::new(0., 0.),
        end: P::new(0., 0.),
    }));
    path.push(C::Close);
    let large = ClipPath {
        geometry: Geometry::Path(path),
        rule: ClipRule::NonZero,
    };
    large.validate().unwrap();
    let first = id(large.clone());
    assert!(first.len() < 459_000);
    assert_eq!(first, id(large));
}
