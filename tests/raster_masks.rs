#![cfg(feature = "raster")]
use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use kartis::render::Canvas;
use kartis::render::raster::*;
use kartis::*;
const RED: Color = Color(220, 30, 40, 255);
fn scene() -> Scene {
    ChartConfig {
        width: 100.,
        height: 100.,
        ..Default::default()
    }
    .scene()
    .unwrap()
}
fn rectangle(id: &str) -> Mark {
    Mark::new(
        id,
        Geometry::Rect {
            x: 0.,
            y: 0.,
            width: 100.,
            height: 100.,
        },
        RED,
    )
}
fn pixel(image: &RenderedImage, x: u32, y: u32) -> [u8; 4] {
    image.rgba[((y * image.width + x) * 4) as usize..((y * image.width + x) * 4 + 4) as usize]
        .try_into()
        .unwrap()
}
fn render(scene: &Scene) -> RenderedImage {
    render_raster(scene, &FontRegistry::new(), &RasterOptions::default()).unwrap()
}
fn ring(x: f64, y: f64, w: f64, h: f64) -> Vec<C> {
    vec![
        C::Move(P::new(x, y)),
        C::Line(P::new(x + w, y)),
        C::Line(P::new(x + w, y + h)),
        C::Line(P::new(x, y + h)),
        C::Close,
    ]
}
#[test]
fn evenodd_and_nonzero_masks_have_real_holes_and_rectangles_intersect_them() {
    let mut path = ring(10., 10., 80., 80.);
    path.extend(ring(35., 35., 30., 30.));
    for (rule, expected) in [
        (ClipRule::EvenOdd, [255, 255, 255, 255]),
        (ClipRule::NonZero, [220, 30, 40, 255]),
    ] {
        let mut scene = scene();
        let mut mark = rectangle("clipped");
        mark.clip_path = Some(
            ClipPath {
                geometry: Geometry::Path(path.clone()),
                rule,
            }
            .into(),
        );
        scene.marks.push(mark);
        let image = render(&scene);
        assert_eq!(pixel(&image, 50, 50), expected);
        assert_eq!(pixel(&image, 20, 50), [220, 30, 40, 255]);
        assert_eq!(pixel(&image, 5, 50), [255, 255, 255, 255]);
    }
    let mut scene = scene();
    let mut mark = rectangle("intersection");
    mark.clip = Some(ClipRect {
        x: 0.,
        y: 0.,
        width: 50.,
        height: 100.,
    });
    mark.clip_path = Some(ClipPath::circle(50., 50., 35.).into());
    scene.marks.push(mark);
    let image = render(&scene);
    for (x, expected) in [
        (10, [255, 255, 255, 255]),
        (30, [220, 30, 40, 255]),
        (49, [220, 30, 40, 255]),
        (70, [255, 255, 255, 255]),
    ] {
        assert_eq!(pixel(&image, x, 50), expected);
    }
}
#[test]
fn clipping_the_original_stroke_never_paints_a_synthetic_circle_boundary() {
    let mut scene = scene();
    let mut mark = Mark::new(
        "line",
        Geometry::Path(vec![C::Move(P::new(0., 50.)), C::Line(P::new(100., 50.))]),
        RED,
    );
    mark.fill = None;
    mark.stroke = Some(GraphicStroke {
        color: RED,
        width: 20.,
        cap: StrokeCap::Butt,
    });
    mark.clip_path = Some(ClipPath::circle(50., 50., 30.).into());
    scene.marks.push(mark);
    let image = render(&scene);
    assert_eq!(
        pixel(&image, 22, 50),
        [220, 30, 40, 255],
        "cut intersects the original stroke"
    );
    assert_eq!(
        pixel(&image, 50, 22),
        [255, 255, 255, 255],
        "no outline along the clip circle"
    );
    assert_eq!(pixel(&image, 15, 50), [255, 255, 255, 255]);
    assert_eq!(pixel(&image, 50, 35), [255, 255, 255, 255]);
}
#[test]
fn empty_masks_erase_fills_and_strokes_and_do_not_leak_to_following_marks() {
    for clip in [
        ClipPath::circle(50., 50., 0.),
        ClipPath {
            geometry: Geometry::Path(vec![]),
            rule: ClipRule::EvenOdd,
        },
    ] {
        let mut scene = scene();
        let mut first = rectangle("empty");
        first.clip_path = Some(clip.into());
        first.stroke = Some(GraphicStroke {
            color: RED,
            width: 15.,
            cap: StrokeCap::Round,
        });
        scene.marks.push(first);
        let mut following = Mark::new(
            "next",
            Geometry::Circle {
                x: 80.,
                y: 80.,
                radius: 5.,
            },
            Color(0, 100, 200, 255),
        );
        following.clip = Some(ClipRect {
            x: 0.,
            y: 0.,
            width: 100.,
            height: 100.,
        });
        scene.marks.push(following);
        let image = render(&scene);
        assert_eq!(pixel(&image, 50, 50), [255, 255, 255, 255]);
        assert_eq!(pixel(&image, 80, 80), [0, 100, 200, 255]);
    }
}
#[test]
fn transformed_label_masks_clip_font_outlines_and_preserve_inside_pixels() {
    let mut fonts = FontRegistry::new();
    fonts
        .register(include_bytes!("fonts/ABeeZee-Regular.ttf").as_slice(), 0)
        .unwrap();
    let mut original = scene();
    original.labels.push(Label {
        x: 10.,
        y: 65.,
        size: 50.,
        rotation: -20.,
        text: "MMMM".into(),
        color: RED,
        anchor: TextAnchor::Start,
    });
    let mut clipped = original.clone();
    clipped
        .label_clips
        .insert(0, ClipPath::circle(35., 45., 22.));
    let options = RasterOptions {
        pixel_ratio: 2.,
        view: ZoomTransform::new(1.5, -10., 5.).unwrap(),
        ..Default::default()
    };
    let full = render_raster(&original, &fonts, &options).unwrap();
    let cut = render_raster(&clipped, &fonts, &options).unwrap();
    let mut kept = 0;
    let mut erased = 0;
    for y in 0..cut.height {
        for x in 0..cut.width {
            let logical = [(f64::from(x) + 0.5) / 2., (f64::from(y) + 0.5) / 2.];
            let p = [(logical[0] + 10.) / 1.5, (logical[1] - 5.) / 1.5];
            let d = (p[0] - 35.).hypot(p[1] - 45.);
            if d < 21. {
                // tiny-skia's masked/unmasked AA compositors differ by at most
                // one RGBA quantization step on a partially covered glyph edge.
                assert!(
                    pixel(&cut, x, y)
                        .iter()
                        .zip(pixel(&full, x, y))
                        .all(|(a, b)| a.abs_diff(b) <= 1)
                );
                kept += usize::from(pixel(&cut, x, y)[0] != 255);
            }
            if d > 23. {
                assert_eq!(pixel(&cut, x, y), [255, 255, 255, 255]);
                erased += usize::from(pixel(&full, x, y)[0] != 255);
            }
        }
    }
    assert!(kept > 100 && erased > 100);
}
fn pattern(rotation: f64) -> PatternFill {
    PatternFill {
        shape: PatternShape::Lines { width: 4. },
        spacing: 8.,
        rotation,
        offset: [2.25, -1.5],
        foreground: Color(210, 40, 50, 128),
        background: Color(10, 20, 30, 255),
    }
}
#[test]
fn actual_pattern_shader_composes_phase_rotation_view_dpr_opacity_and_path_mask() {
    let mut scene = scene();
    let mut mark = rectangle("pattern");
    mark.fill = None;
    mark.fill_paint = Some(FillPaint::Pattern(pattern(37.)));
    mark.clip_path = Some(ClipPath::circle(45., 45., 35.).into());
    scene.marks.push(mark);
    let options = RasterOptions {
        pixel_ratio: 2.,
        view: ZoomTransform::new(2., -20., -10.).unwrap(),
        ..Default::default()
    };
    let image = render_raster(&scene, &FontRegistry::new(), &options).unwrap();
    let (sin, cos) = 37_f64.to_radians().sin_cos();
    let mut fore = 0;
    let mut back = 0;
    let mut erased = 0;
    for y in (0..200).step_by(3) {
        for x in (0..200).step_by(3) {
            let p = [
                ((f64::from(x) + 0.5) / 2. + 20.) / 2.,
                ((f64::from(y) + 0.5) / 2. + 10.) / 2.,
            ];
            let distance = (p[0] - 45.).hypot(p[1] - 45.);
            if distance > 36. {
                assert_eq!(pixel(&image, x, y), [255, 255, 255, 255]);
                erased += 1;
                continue;
            }
            if distance > 34. {
                continue;
            }
            let row = (-sin * (p[0] - 2.25) + cos * (p[1] + 1.5)).rem_euclid(8.);
            let d = row.min(8. - row);
            if (d - 2.).abs() < 0.8 {
                continue;
            }
            let expected = if d < 2. {
                fore += 1;
                [110_u8, 30, 40, 255]
            } else {
                back += 1;
                [10, 20, 30, 255]
            };
            let actual = pixel(&image, x, y);
            assert!(
                actual.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 1),
                "{x},{y} row={row}: {actual:?} vs {expected:?}"
            );
        }
    }
    assert!(fore > 100 && back > 100 && erased > 100);
    scene.marks[0].opacity = 0.5;
    let image = render_raster(&scene, &FontRegistry::new(), &options).unwrap();
    assert!(
        image
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[0] > 175 && p[0] < 185 && p[1] > 135 && p[1] < 150)
    );
}
#[test]
fn duplicate_path_clips_are_idempotent_and_restoring_nested_clips_is_balanced() {
    fn draw(duplicate: bool) -> RenderedImage {
        let mut canvas = RasterCanvas::new(FontRegistry::new(), RasterOptions::default()).unwrap();
        canvas.begin(100., 100., Color(255, 255, 255, 255)).unwrap();
        let clip = ClipPath::circle(50.3, 50.7, 30.2);
        canvas.push_clip_path(&clip).unwrap();
        if duplicate {
            canvas.push_clip_path(&clip).unwrap();
        }
        canvas.begin_path().unwrap();
        canvas.move_to(0., 0.).unwrap();
        canvas.line_to(100., 0.).unwrap();
        canvas.line_to(100., 100.).unwrap();
        canvas.line_to(0., 100.).unwrap();
        canvas.close_path().unwrap();
        canvas.fill(RED, 1.).unwrap();
        if duplicate {
            canvas.pop_clip_path().unwrap();
        }
        canvas.pop_clip_path().unwrap();
        canvas.finish().unwrap();
        canvas.frame().unwrap().clone()
    }
    assert_eq!(
        draw(false),
        draw(true),
        "identical clips must not square AA coverage"
    );
    let mut canvas = RasterCanvas::new(FontRegistry::new(), RasterOptions::default()).unwrap();
    let mut base = scene();
    base.marks.push(rectangle("base"));
    canvas.render(&base).unwrap();
    let before = canvas.frame().unwrap().clone();
    canvas.begin(100., 100., Color(255, 255, 255, 255)).unwrap();
    canvas
        .push_clip_path(&ClipPath::circle(50., 50., 30.))
        .unwrap();
    assert!(canvas.pop_clip().is_err());
    assert!(canvas.finish().is_err());
    assert_eq!(canvas.frame(), Some(&before));
    canvas.begin(100., 100., Color(255, 255, 255, 255)).unwrap();
    canvas
        .push_clip_path(&ClipPath::circle(50., 50., 30.))
        .unwrap();
    assert!(canvas.finish().is_err());
    assert_eq!(canvas.frame(), Some(&before));
}
#[test]
fn pattern_cache_reuses_identical_tiles_and_rejects_aggregate_pixels_atomically() {
    let mut canvas = RasterCanvas::new(
        FontRegistry::new(),
        RasterOptions {
            pixel_ratio: 2.,
            ..Default::default()
        },
    )
    .unwrap();
    let fill = PatternFill {
        spacing: 1024.,
        shape: PatternShape::Squares {
            size: 4.,
            stagger: false,
        },
        ..Default::default()
    };
    let mut repeated = scene();
    for i in 0..100 {
        let mut mark = Mark::new(
            i.to_string(),
            Geometry::Rect {
                x: 0.,
                y: 0.,
                width: 10.,
                height: 10.,
            },
            RED,
        );
        mark.fill_paint = Some(FillPaint::Pattern(fill.clone()));
        repeated.marks.push(mark);
    }
    canvas.render(&repeated).unwrap();
    let before = canvas.frame().unwrap().clone();
    let mut distinct = scene();
    for i in 0..5 {
        let mut mark = rectangle(&format!("different{i}"));
        let mut pattern = fill.clone();
        pattern.foreground = Color(i, 0, 0, 255);
        mark.fill_paint = Some(FillPaint::Pattern(pattern));
        distinct.marks.push(mark);
    }
    let error = canvas.render(&distinct).unwrap_err();
    assert!(error.0.contains("16M tile pixels"));
    assert_eq!(canvas.frame(), Some(&before));
    canvas.render(&repeated).unwrap();
    assert_eq!(
        canvas.frame(),
        Some(&before),
        "frame-local cache budgets reset after failure"
    );
}
#[test]
fn mask_operations_and_cache_entry_limits_reject_without_committing_partial_frame() {
    let mut base = scene();
    base.marks.push(rectangle("base"));
    let mut canvas = RasterCanvas::new(
        FontRegistry::new(),
        RasterOptions {
            max_operations: 50_000,
            ..Default::default()
        },
    )
    .unwrap();
    canvas.render(&base).unwrap();
    let before = canvas.frame().unwrap().clone();
    let mut costly = base.clone();
    costly.marks[0].clip_path = Some(ClipPath::circle(50., 50., 40.).into());
    assert!(
        canvas
            .render(&costly)
            .unwrap_err()
            .0
            .contains("operation budget")
    );
    assert_eq!(canvas.frame(), Some(&before));
    canvas.set_options(RasterOptions::default()).unwrap();
    let mut costly = scene();
    for i in 0..257 {
        let mut mark = rectangle(&format!("clip{i}"));
        mark.clip_path = Some(ClipPath::circle(50., 50., 30. + f64::from(i) * 0.001).into());
        costly.marks.push(mark);
    }
    assert!(
        canvas
            .render(&costly)
            .unwrap_err()
            .0
            .contains("mask cache budget")
    );
    assert_eq!(canvas.frame(), Some(&before));
    canvas.render(&base).unwrap();
    assert_eq!(canvas.frame(), Some(&before));
}
#[test]
fn dashed_zero_caps_use_source_tangents_and_do_not_double_composite_overlapping_alpha() {
    for cap in [StrokeCap::Round, StrokeCap::Square] {
        let mut scene = scene();
        let mut line = Mark::new(
            "dash",
            Geometry::Path(vec![C::Move(P::new(10., 50.)), C::Line(P::new(90., 50.))]),
            RED,
        );
        line.fill = None;
        line.stroke = Some(GraphicStroke {
            color: Color(255, 0, 0, 128),
            width: 10.,
            cap,
        });
        line.stroke_dash = Some(StrokeDash {
            segments: vec![0., 6., 8., 6.],
            offset: 0.,
        });
        scene.marks.push(line);
        let image = render(&scene);
        assert_eq!(
            pixel(&image, 13, 50),
            [255, 127, 127, 255],
            "zero cap and adjacent positive dash must paint as one stroke {cap:?}"
        );
    }
}
#[test]
fn zero_dash_round_and_tilted_square_caps_keep_gap_pixels_and_single_alpha() {
    for cap in [StrokeCap::Round, StrokeCap::Square] {
        let mut scene = scene();
        let mut mark = Mark::new(
            "tilted-dots",
            Geometry::Path(vec![C::Move(P::new(20., 20.)), C::Line(P::new(70., 70.))]),
            RED,
        );
        mark.fill = None;
        mark.stroke = Some(GraphicStroke {
            color: Color(255, 0, 0, 128),
            width: 10.,
            cap,
        });
        mark.stroke_dash = Some(StrokeDash {
            segments: vec![0., 20.],
            offset: 0.,
        });
        scene.marks.push(mark);
        let image = render(&scene);
        assert_eq!(pixel(&image, 20, 20), [255, 127, 127, 255]);
        assert_eq!(
            pixel(&image, 27, 27),
            [255, 255, 255, 255],
            "gap between zero dashes {cap:?}"
        );
        assert_eq!(
            pixel(&image, 24, 24),
            [255, 255, 255, 255],
            "tilted square corner must follow tangent"
        );
        if cap == StrokeCap::Square {
            assert_eq!(
                pixel(&image, 25, 20),
                [255, 127, 127, 255],
                "rotated cap extends beyond axis-aligned square"
            );
        } else {
            assert_eq!(pixel(&image, 25, 20), [255, 255, 255, 255]);
        }
    }
}
