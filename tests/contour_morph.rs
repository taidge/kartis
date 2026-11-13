use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::motion::*;
use kartis::*;

fn scene(geometry: Geometry) -> Scene {
    let mut scene = ChartConfig {
        width: 240.,
        height: 200.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    let mut mark = Mark::new("shape", geometry, Color(30, 90, 160, 255));
    mark.series = "series".into();
    mark.label = "Donut".into();
    scene.marks.push(mark);
    scene
}
fn ring(radius: f64, reverse: bool) -> Vec<C> {
    let commands = Geometry::Circle {
        x: 120.,
        y: 100.,
        radius,
    }
    .commands();
    if !reverse {
        return commands;
    }
    let C::Move(start) = commands[0] else {
        unreachable!()
    };
    let mut cursor = start;
    let mut segments = Vec::new();
    for command in &commands[1..] {
        if let C::Cubic { first, second, end } = command {
            segments.push((cursor, *first, *second));
            cursor = *end;
        }
    }
    let mut result = vec![C::Move(start)];
    for (end, second, first) in segments.into_iter().rev() {
        result.push(C::Cubic { first, second, end });
    }
    result.push(C::Close);
    result
}
fn donut(outer: f64, inner: Option<f64>, reversed_order: bool) -> Geometry {
    let outer = ring(outer, false);
    let hole = inner.map(|radius| ring(radius, true)).unwrap_or_default();
    Geometry::Path(if reversed_order {
        hole.into_iter().chain(outer).collect()
    } else {
        outer.into_iter().chain(hole).collect()
    })
}
fn options() -> PropertyMotionOptions {
    let mut options = PropertyMotionOptions::default();
    options.geometry.contour_morphs.insert(
        "shape".into(),
        ContourMorphOptions {
            sampling: PathMorphOptions {
                samples: 96,
                tolerance: 0.1,
                alignment: ClosedAlignment::Rotate,
            },
            contours: vec![
                MorphContour {
                    key: "outer".into(),
                    closed: true,
                    collapse: [120., 100.],
                },
                MorphContour {
                    key: "hole".into(),
                    closed: true,
                    collapse: [120., 100.],
                },
            ],
        },
    );
    options
}
fn binding(keys: &[&str]) -> ContourBindings {
    ContourBindings::from([(
        "shape".into(),
        keys.iter().map(|key| (*key).into()).collect(),
    )])
}
fn settle(motion: &mut PropertyMotion) {
    for _ in 0..2000 {
        motion.tick(0.016).unwrap().validate().unwrap();
        if motion.is_settled() {
            return;
        }
    }
    panic!("motion failed to settle");
}
fn contours(geometry: &Geometry) -> Vec<Vec<P>> {
    let mut result = Vec::new();
    let Geometry::Path(path) = geometry else {
        panic!("expected path");
    };
    for command in path {
        match command {
            C::Move(p) => result.push(vec![*p]),
            C::Line(p) => result.last_mut().unwrap().push(*p),
            C::Close => {}
            _ => panic!("expected sampled contour"),
        }
    }
    result
}
fn area(points: &[P]) -> f64 {
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
        .map(|(a, b)| (a.x - 120.) * (b.y - 100.) - (b.x - 120.) * (a.y - 100.))
        .sum()
}

#[test]
fn explicit_reorder_reuses_scalar_tracks_velocity_winding_and_exact_target_order() {
    let mut motion = PropertyMotion::new(scene(donut(60., Some(25.), false)), options()).unwrap();
    let target = scene(donut(80., Some(35.), true));
    motion
        .retarget_with_contours(target.clone(), binding(&["hole", "outer"]))
        .unwrap();
    assert_eq!((motion.stats().marks, motion.stats().exiting_marks), (1, 0));
    motion.tick(0.08).unwrap();
    let sampled = contours(&motion.scene().marks[0].geometry);
    assert_eq!(sampled.iter().map(Vec::len).collect::<Vec<_>>(), [96, 96]);
    assert!(area(&sampled[0]) > 0. && area(&sampled[1]) < 0.);
    let before = motion.mark_property("shape", "geometry.path.0.x").unwrap();
    assert!(before.velocity != 0.);
    let next = scene(donut(70., Some(30.), false));
    motion.retarget(next.clone()).unwrap();
    let after = motion.mark_property("shape", "geometry.path.0.x").unwrap();
    assert_eq!(
        (after.value, after.velocity),
        (before.value, before.velocity)
    );
    settle(&mut motion);
    assert_eq!(motion.scene(), &next);
    motion
        .retarget_with_contours(target.clone(), binding(&["hole", "outer"]))
        .unwrap();
    settle(&mut motion);
    assert_eq!(motion.scene(), &target);
    // finish restored the reversed original path. Its accepted bindings must
    // survive, so a later target uses the same canonical key correspondence.
    motion.retarget(next.clone()).unwrap();
    assert_eq!(motion.stats().marks, 1);
    settle(&mut motion);
    assert_eq!(motion.scene(), &next);
}

#[test]
fn missing_contour_appearance_disappearance_and_revival_keep_one_mark() {
    let solid = scene(donut(60., None, false));
    let holed = scene(donut(70., Some(30.), false));
    let mut motion =
        PropertyMotion::new_with_contours(solid.clone(), options(), binding(&["outer"])).unwrap();
    motion.retarget(holed.clone()).unwrap();
    let initial = contours(&motion.scene().marks[0].geometry);
    assert!(initial[1].iter().all(|p| *p == P::new(120., 100.)));
    motion.tick(0.08).unwrap();
    assert!(area(&contours(&motion.scene().marks[0].geometry)[1]) < 0.);
    settle(&mut motion);
    motion
        .retarget_with_contours(solid.clone(), binding(&["outer"]))
        .unwrap();
    motion.tick(0.06).unwrap();
    let before = motion.mark_property("shape", "geometry.path.97.x").unwrap();
    motion.retarget(holed.clone()).unwrap();
    let after = motion.mark_property("shape", "geometry.path.97.x").unwrap();
    assert_eq!(
        (after.value, after.velocity),
        (before.value, before.velocity)
    );
    assert_eq!((motion.stats().marks, motion.stats().exiting_marks), (1, 0));
    settle(&mut motion);
    assert_eq!(motion.scene(), &holed);
    motion
        .retarget_with_contours(solid.clone(), binding(&["outer"]))
        .unwrap();
    settle(&mut motion);
    assert_eq!(motion.scene(), &solid);
    // The caller may explicitly collapse every declared contour.
    motion
        .retarget_with_contours(scene(Geometry::Path(vec![])), binding(&[]))
        .unwrap();
    settle(&mut motion);
    assert!(matches!(&motion.scene().marks[0].geometry, Geometry::Path(p) if p.is_empty()));
}

#[test]
fn bad_bindings_topology_winding_and_budget_rejections_are_transactional() {
    let initial = scene(donut(60., Some(25.), false));
    let target = scene(donut(80., Some(35.), false));
    let mut motion = PropertyMotion::new(initial.clone(), options()).unwrap();
    motion.retarget(target.clone()).unwrap();
    motion.tick(0.05).unwrap();
    let previous = motion.scene().clone();
    let velocity = motion.mark_velocity("shape", "geometry.path.0.x");
    for bindings in [
        binding(&["outer", "outer"]),
        binding(&["outer", "unknown"]),
        binding(&["outer"]),
        ContourBindings::from([("unknown-mark".into(), vec![])]),
    ] {
        assert!(
            motion
                .retarget_with_contours(target.clone(), bindings)
                .is_err()
        );
        assert_eq!(motion.scene(), &previous);
        assert_eq!(motion.mark_velocity("shape", "geometry.path.0.x"), velocity);
    }
    let mut wrong_winding = target.clone();
    wrong_winding.marks[0].geometry = Geometry::Path(
        ring(80., false)
            .into_iter()
            .chain(ring(35., false))
            .collect(),
    );
    assert!(motion.retarget(wrong_winding).is_err());
    let mut open_hole = ring(80., false);
    open_hole.extend([C::Move(P::new(120., 100.)), C::Line(P::new(130., 100.))]);
    assert!(motion.retarget(scene(Geometry::Path(open_hole))).is_err());
    assert_eq!(motion.scene(), &previous);
    assert_eq!(motion.mark_velocity("shape", "geometry.path.0.x"), velocity);
    let mut low = options();
    low.max_path_commands = 193; // two canonical closed slots require 194.
    assert!(PropertyMotion::new(initial.clone(), low).is_err());
    let mut union = options();
    union.max_path_commands = 194;
    let mut motion = PropertyMotion::new(initial, union).unwrap();
    let before = motion.scene().clone();
    let mut changed_paint = target;
    changed_paint.marks[0].stroke = Some(glory::render::GraphicStroke {
        color: Color(0, 0, 0, 255),
        width: 1.,
        cap: glory::render::StrokeCap::Butt,
    });
    assert!(motion.retarget(changed_paint).is_err()); // crossfade union is 388.
    assert_eq!(motion.scene(), &before);
}

#[test]
fn profile_limits_open_contours_and_reduced_motion_are_explicit() {
    let mut invalid = options();
    invalid
        .geometry
        .contour_morphs
        .get_mut("shape")
        .unwrap()
        .sampling
        .alignment = ClosedAlignment::RotateAndReverse;
    assert!(invalid.validate().is_err());
    let mut invalid = options();
    invalid
        .geometry
        .contour_morphs
        .get_mut("shape")
        .unwrap()
        .contours[1]
        .collapse[0] = f64::NAN;
    assert!(invalid.validate().is_err());
    let mut invalid = options();
    invalid
        .geometry
        .contour_morphs
        .get_mut("shape")
        .unwrap()
        .contours[1]
        .key = "outer".into();
    assert!(invalid.validate().is_err());
    let mut invalid = options();
    invalid
        .geometry
        .morphs
        .insert("shape".into(), PathMorphOptions::default());
    assert!(invalid.validate().is_err());
    let mut motion = PropertyMotion::new(scene(donut(60., Some(25.), false)), options()).unwrap();
    motion
        .retarget(scene(donut(80., Some(35.), false)))
        .unwrap();
    motion.tick(0.04).unwrap();
    motion.set_reduced_motion(true);
    assert!(motion.is_settled());
    assert_eq!(motion.scene(), motion.target());
    let reversed = scene(donut(70., Some(30.), true));
    motion
        .retarget_with_contours(reversed.clone(), binding(&["hole", "outer"]))
        .unwrap();
    assert_eq!(motion.scene(), &reversed);
    let old = motion.scene().clone();
    assert!(
        motion
            .retarget_with_contours(reversed, binding(&["unknown"]))
            .is_err()
    );
    assert_eq!(motion.scene(), &old);

    let profile = ContourMorphOptions {
        sampling: PathMorphOptions {
            samples: 8,
            ..Default::default()
        },
        contours: vec![MorphContour {
            key: "line".into(),
            closed: false,
            collapse: [0., 0.],
        }],
    };
    let path = Geometry::Path(vec![C::Move(P::new(0., 0.)), C::Line(P::new(10., 10.))]);
    let result = sampled_contours(&path, &profile, None).unwrap();
    let Geometry::Path(path) = result else {
        unreachable!()
    };
    assert_eq!(path.len(), 8);
    assert!(!path.iter().any(|c| matches!(c, C::Close)));
}

#[test]
fn fixed_slot_command_and_alignment_work_limits_reject_without_state_changes() {
    let profile = ContourMorphOptions {
        sampling: PathMorphOptions {
            samples: 256,
            alignment: ClosedAlignment::Rotate,
            ..Default::default()
        },
        contours: (0..32)
            .map(|i| MorphContour {
                key: format!("c{i}"),
                closed: true,
                collapse: [0., 0.],
            })
            .collect(),
    };
    let mut invalid = profile.clone();
    invalid.contours.push(MorphContour {
        key: "overflow".into(),
        closed: true,
        collapse: [0., 0.],
    });
    assert!(invalid.validate().is_err());
    let geometry = |offset: f64| {
        Geometry::Path(
            (0..32)
                .flat_map(|i| [C::Move(P::new(offset + i as f64, 50.)), C::Close])
                .collect(),
        )
    };
    let mut initial = scene(geometry(10.));
    let mut second = initial.marks[0].clone();
    second.id = "second".into();
    initial.marks.push(second);
    let mut opts = PropertyMotionOptions::default();
    opts.geometry
        .contour_morphs
        .insert("shape".into(), profile.clone());
    opts.geometry
        .contour_morphs
        .insert("second".into(), profile.clone());
    let mut motion = PropertyMotion::new(initial.clone(), opts).unwrap();
    let mut target = initial.clone();
    for mark in &mut target.marks {
        mark.geometry = geometry(20.);
    }
    // Two 32-slot profiles each need 32*256^2 cyclic comparisons. The shared
    // per-retarget 4m budget rejects the second before any state is published.
    assert!(
        motion
            .retarget(target)
            .unwrap_err()
            .0
            .contains("work budget")
    );
    assert_eq!(motion.scene(), &initial);
    assert_eq!(motion.target(), &initial);
    assert_eq!(motion.mark_velocity("shape", "geometry.path.0.x"), Some(0.));
    let mut one = initial.clone();
    one.marks.pop();
    let mut opts = PropertyMotionOptions::default();
    opts.geometry.contour_morphs.insert("shape".into(), profile);
    let mut motion = PropertyMotion::new(one.clone(), opts).unwrap();
    one.marks[0].geometry = geometry(20.);
    motion.retarget(one).unwrap(); // canonical references may contain 8,224 commands.
    assert_eq!(motion.scene().marks[0].geometry.commands().len(), 8224);
    let before = motion.scene().clone();
    let oversized = Geometry::Path(
        (0..8193)
            .map(|i| {
                if i == 0 {
                    C::Move(P::new(0., 0.))
                } else {
                    C::Line(P::new(i as f64, 0.))
                }
            })
            .collect(),
    );
    assert!(
        motion
            .retarget_with_contours(scene(oversized), binding(&["c0"]))
            .is_err()
    );
    assert_eq!(motion.scene(), &before);
}

#[test]
fn custom_enter_exit_geometries_and_mark_revival_use_multi_contour_tracks() {
    let mut opts = options();
    opts.geometry
        .enter
        .insert("shape".into(), donut(0., Some(0.), false));
    opts.geometry
        .exit
        .insert("shape".into(), donut(0., Some(0.), false));
    let mut empty = scene(donut(60., None, false));
    empty.marks.clear();
    let target = scene(donut(60., Some(25.), false));
    let mut motion = PropertyMotion::new(empty.clone(), opts).unwrap();
    motion.retarget(target.clone()).unwrap();
    assert!(
        contours(&motion.scene().marks[0].geometry)
            .iter()
            .flatten()
            .all(|p| *p == P::new(120., 100.))
    );
    settle(&mut motion);
    motion.retarget(empty).unwrap();
    motion.tick(0.05).unwrap();
    let before = motion.mark_property("shape", "geometry.path.0.x").unwrap();
    assert!(before.velocity < 0.);
    motion.retarget(target.clone()).unwrap();
    let after = motion.mark_property("shape", "geometry.path.0.x").unwrap();
    assert_eq!(
        (before.value, before.velocity),
        (after.value, after.velocity)
    );
    assert_eq!((motion.stats().marks, motion.stats().exiting_marks), (1, 0));
    settle(&mut motion);
    assert_eq!(motion.scene(), &target);
}

#[test]
fn shader_only_marks_retain_paint_and_fade_instead_of_interpolating_shader_styles() {
    let mut initial = scene(donut(60., Some(25.), false));
    let paint = FillPaint::Linear {
        start: [60., 100.],
        end: [180., 100.],
        stops: vec![
            GradientStop::new(0., Color(255, 0, 0, 255)),
            GradientStop::new(1., Color(0, 0, 255, 255)),
        ],
    };
    initial.marks[0].fill = None;
    initial.marks[0].fill_paint = Some(paint.clone());
    let mut changed = initial.clone();
    let FillPaint::Linear { end, .. } = changed.marks[0].fill_paint.as_mut().unwrap() else {
        unreachable!()
    };
    *end = [190., 110.];
    let mut motion = PropertyMotion::new(initial.clone(), options()).unwrap();
    motion.retarget(changed.clone()).unwrap();
    assert_eq!((motion.stats().marks, motion.stats().exiting_marks), (2, 1));
    assert!(
        motion
            .scene()
            .marks
            .iter()
            .any(|mark| mark.fill_paint == Some(paint.clone()))
    );
    settle(&mut motion);
    assert_eq!(motion.scene(), &changed);
    let mut empty = initial.clone();
    empty.marks.clear();
    motion.retarget(empty.clone()).unwrap();
    assert_eq!(motion.stats().exiting_marks, 1);
    settle(&mut motion);
    assert_eq!(motion.scene(), &empty);
    let mut spring = SpringMotion::new(initial, SpringConfig::default()).unwrap();
    spring.retarget(changed).unwrap();
    assert_eq!(spring.scene().marks.len(), 2);
}

#[cfg(feature = "raster")]
#[test]
fn donut_holes_survive_cpu_pixels_and_export_real_svg_browser_fixtures() {
    use kartis::render::raster::*;
    use kartis::render::to_svg;
    let mut motion = PropertyMotion::new(scene(donut(60., Some(25.), false)), options()).unwrap();
    let initial = motion.scene().clone();
    let target = scene(donut(80., Some(35.), true));
    motion
        .retarget_with_contours(target.clone(), binding(&["hole", "outer"]))
        .unwrap();
    motion.tick(0.08).unwrap();
    let middle = motion.scene().clone();
    settle(&mut motion);
    for (name, frame) in [("initial", initial), ("middle", middle), ("target", target)] {
        let image = render_raster(&frame, &FontRegistry::new(), &RasterOptions::default()).unwrap();
        let pixel = |x: usize, y: usize| &image.rgba[(y * 240 + x) * 4..(y * 240 + x) * 4 + 4];
        assert_eq!(
            pixel(120, 100),
            [255, 255, 255, 255],
            "{name}: hole must be empty"
        );
        assert_eq!(
            pixel(170, 100),
            [30, 90, 160, 255],
            "{name}: ring must be painted"
        );
        assert_eq!(pixel(5, 5), [255, 255, 255, 255]);
        let svg = to_svg(&frame).unwrap();
        assert_eq!(svg.matches('M').count(), 2);
        assert_eq!(svg.matches('Z').count(), 2);
        if let Some(directory) = std::env::var_os("KARTIS_MORPH_ARTIFACT_DIR") {
            let directory = std::path::PathBuf::from(directory);
            assert!(directory.is_absolute());
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(directory.join(format!("{name}.svg")), svg).unwrap();
            std::fs::write(directory.join(format!("{name}.png")), image.png().unwrap()).unwrap();
        }
    }
}
