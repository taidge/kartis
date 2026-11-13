#![cfg(feature = "raster")]

use std::rc::Rc;

use glory::App;
use kartis::render::Canvas;
use kartis::render::raster::*;
use kartis::*;

const FONT: &[u8] = include_bytes!("fonts/KartisFixtureColrV0.ttf");
fn fonts(palette: u16) -> FontRegistry {
    registered(FONT, ColorGlyphMode::ColrV0 { palette })
}
fn registered(bytes: &[u8], mode: ColorGlyphMode) -> FontRegistry {
    let mut fonts = FontRegistry::new().with_color_glyphs(mode);
    fonts.register(bytes, 0).unwrap();
    fonts
}
fn scene(text: &str) -> Scene {
    let mut scene = ChartConfig {
        width: 240.,
        height: 180.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    scene.labels.push(Label {
        text: text.into(),
        x: 40.,
        y: 140.,
        size: 100.,
        color: Color(0, 255, 0, 128),
        anchor: TextAnchor::Start,
        rotation: 0.,
    });
    scene
}
fn pixel(image: &RenderedImage, x: usize, y: usize) -> &[u8] {
    let index = (y * image.width as usize + x) * 4;
    &image.rgba[index..index + 4]
}
fn options() -> PaintedLabelOptions {
    PaintedLabelOptions {
        rotated_only: false,
        ..Default::default()
    }
}

#[test]
fn colr_requires_opt_in_and_preserves_layer_order_palette_rgba_and_whitespace_indices() {
    let rejected = registered(FONT, ColorGlyphMode::Reject);
    assert_eq!(
        FontRegistry::default().color_glyphs(),
        ColorGlyphMode::Reject
    );
    assert!(render_raster(&scene("B"), &rejected, &Default::default()).is_err());
    assert!(
        rejected
            .painted_label(&scene("B").labels[0], MissingGlyphPolicy::Error)
            .is_err()
    );
    let fonts = fonts(0);
    assert!(
        fonts
            .outline_label(&scene("B").labels[0], MissingGlyphPolicy::Error)
            .is_err()
    );
    let painted = fonts
        .painted_label(&scene("B B").labels[0], MissingGlyphPolicy::Error)
        .unwrap();
    assert_eq!(painted.glyph_count, 3);
    assert_eq!(painted.advance, 300.);
    assert_eq!(painted.layers.len(), 8);
    for (group, glyph) in painted.layers.as_chunks::<4>().0.iter().zip([0, 2]) {
        for (index, layer) in group.iter().enumerate() {
            assert_eq!((layer.glyph, layer.layer), (glyph, index));
            assert!(!layer.commands.is_empty());
        }
        assert_eq!(
            group.iter().map(|layer| layer.color).collect::<Vec<_>>(),
            [
                Color(255, 0, 0, 255),
                Color(0, 0, 255, 128),
                Color(0, 255, 0, 128),
                Color(0, 0, 0, 0),
            ]
        );
        assert_eq!(
            group[0]
                .commands
                .iter()
                .filter(|command| matches!(command, glory::render::PathCommand::Move(_)))
                .count(),
            2,
            "ring contours must remain one winding-rule layer"
        );
    }
}

#[test]
fn analytic_colr_source_over_pixels_cover_holes_palette_switches_and_transparent_foreground() {
    let mut scene = scene("B");
    let image = render_raster(&scene, &fonts(0), &Default::default()).unwrap();
    for (x, y, expected) in [
        (50, 50, [255, 0, 0, 255]),
        (70, 80, [255, 255, 255, 255]),
        (100, 80, [127, 127, 255, 255]),
        (125, 80, [127, 0, 128, 255]),
        (50, 125, [127, 128, 0, 255]),
        (25, 25, [255, 255, 255, 255]),
    ] {
        assert_eq!(pixel(&image, x, y), expected, "palette0 at {x},{y}");
    }

    let alternate = render_raster(&scene, &fonts(1), &Default::default()).unwrap();
    for (x, y, expected) in [
        (50, 50, [0, 180, 0, 255]),
        (70, 80, [255, 255, 255, 255]),
        (100, 80, [255, 255, 127, 255]),
        (125, 80, [128, 218, 0, 255]),
        (50, 125, [0, 218, 0, 255]),
    ] {
        assert_eq!(pixel(&alternate, x, y), expected, "palette1 at {x},{y}");
    }

    scene.labels[0].color.3 = 0;
    let transparent = render_raster(&scene, &fonts(0), &Default::default()).unwrap();
    assert_eq!(pixel(&transparent, 50, 125), [255, 0, 0, 255]);
    assert_eq!(pixel(&transparent, 100, 80), [127, 127, 255, 255]);
    let lowered = painted_labels(&scene, &fonts(0), &options()).unwrap();
    assert_eq!(lowered.layer_count, 4);
    assert_eq!(lowered.labels[&0].layers[2].color, Color(0, 255, 0, 0));
}

#[test]
fn colored_ink_measurement_uses_painted_layers_and_excludes_transparent_extents() {
    let fonts = fonts(0);
    let mut scene = scene("B");
    let measured = fonts
        .measure_layout("B", 100., MissingGlyphPolicy::Error)
        .unwrap();
    assert_eq!(
        measured.ink_bounds.unwrap(),
        RasterTextBounds {
            x: 0.,
            y: -100.,
            width: 100.,
            height: 100.
        }
    );
    assert_eq!(
        fonts
            .measure_label(&scene.labels[0], MissingGlyphPolicy::Error)
            .unwrap()
            .unwrap(),
        RasterTextBounds {
            x: 40.,
            y: 40.,
            width: 100.,
            height: 100.
        }
    );
    scene.labels[0].anchor = TextAnchor::Middle;
    scene.labels[0].rotation = 90.;
    let measured = fonts
        .measure_label(&scene.labels[0], MissingGlyphPolicy::Error)
        .unwrap()
        .unwrap();
    assert!((measured.x - 40.).abs() < 1e-10);
    assert!((measured.y - 90.).abs() < 1e-10);
    assert!((measured.width - 100.).abs() < 1e-10 && (measured.height - 100.).abs() < 1e-10);
}

#[test]
fn cpu_matches_public_painted_geometry_through_rotation_mixed_shaping_and_clipping() {
    let vertical = fonts(0).with_vertical_layout(RasterVerticalLayout {
        orientation: RasterGlyphOrientation::Mixed,
        columns: RasterVerticalColumns::RightToLeft,
        column_advance_em: 1.5,
    });
    for (text, rotation) in [("B", 0.), ("\u{e000}", 0.), ("B\n\u{e000}", 27.)] {
        let mut scene = scene(text);
        scene.labels[0].size = 48.;
        scene.labels[0].x = 140.;
        scene.labels[0].y = 45.;
        scene.labels[0].rotation = rotation;
        scene
            .label_clips
            .insert(0, ClipPath::circle(135., 70., 48.));
        let raster = render_raster(&scene, &vertical, &Default::default()).unwrap();
        let text = vertical
            .painted_label(&scene.labels[0], MissingGlyphPolicy::Error)
            .unwrap();
        // U+E000 is upright and uses the actual OpenType `vert` substitution.
        if scene.labels[0].text == "\u{e000}" {
            assert_eq!(text.layers.len(), 1);
        }
        let clip = scene.label_clips.remove(&0).unwrap();
        scene.labels.clear();
        scene.marks = text
            .layers
            .into_iter()
            .enumerate()
            .map(|(index, layer)| {
                let mut mark = Mark::new(
                    format!("layer:{index}"),
                    Geometry::Path(layer.commands),
                    layer.color,
                );
                mark.clip_path = Some(Box::new(clip.clone()));
                mark
            })
            .collect();
        let vector = render_raster(&scene, &vertical, &Default::default()).unwrap();
        assert_eq!(raster.rgba, vector.rgba);
    }
    let mut canvas = RasterCanvas::new(fonts(0), Default::default()).unwrap();
    canvas.begin(240., 180., Color(255, 255, 255, 255)).unwrap();
    canvas
        .push_clip(ClipRect {
            x: 45.,
            y: 45.,
            width: 80.,
            height: 80.,
        })
        .unwrap();
    canvas.text(&scene("B").labels[0]).unwrap();
    canvas.pop_clip().unwrap();
    canvas.finish().unwrap();
    assert_eq!(pixel(canvas.frame().unwrap(), 42, 50), [255, 255, 255, 255]);
    assert_eq!(pixel(canvas.frame().unwrap(), 50, 50), [255, 0, 0, 255]);
}

#[test]
fn palette_metadata_and_colr_version_errors_preserve_the_last_frame() {
    for bytes in [
        include_bytes!("fonts/KartisFixtureColrBadPalette.ttf").as_slice(),
        include_bytes!("fonts/KartisFixtureColrTooManyLayers.ttf").as_slice(),
    ] {
        let fonts = registered(bytes, ColorGlyphMode::ColrV0 { palette: 0 });
        let app = App::memory();
        let scope = app.scope().child().unwrap();
        let surface =
            RasterSurface::mount(&scope, scene("B"), fonts.clone(), Default::default()).unwrap();
        let handle = surface.handle();
        let before = handle.snapshot().unwrap();
        let frame = handle.frame().unwrap();
        assert!(handle.set_scene(scene("B\u{e001}")).is_err());
        assert_eq!(handle.snapshot().unwrap(), before);
        assert!(Rc::ptr_eq(&handle.frame().unwrap(), &frame));
        assert!(painted_labels(&scene("B\u{e001}"), &fonts, &options()).is_err());
    }
    assert!(render_raster(&scene("B"), &fonts(2), &Default::default()).is_err());
    let version1 = registered(
        include_bytes!("fonts/KartisFixtureColrV1.ttf").as_slice(),
        ColorGlyphMode::ColrV0 { palette: 0 },
    );
    assert!(
        version1
            .painted_label(&scene("B").labels[0], MissingGlyphPolicy::Error)
            .is_err()
    );
}

#[test]
fn painted_lowering_and_cpu_operation_budgets_are_enforced() {
    let scene = scene("B");
    let fonts = fonts(0);
    let lowered = painted_labels(&scene, &fonts, &options()).unwrap();
    assert_eq!(lowered.glyph_count, 1);
    assert_eq!(lowered.layer_count, 4);
    assert!(
        painted_labels(
            &scene,
            &fonts,
            &PaintedLabelOptions {
                max_layers: 3,
                ..options()
            }
        )
        .is_err()
    );
    assert!(
        painted_labels(
            &scene,
            &fonts,
            &PaintedLabelOptions {
                max_commands: 1,
                ..options()
            }
        )
        .is_err()
    );
    assert!(
        painted_labels(
            &scene,
            &fonts,
            &PaintedLabelOptions {
                max_glyphs: 0,
                ..options()
            }
        )
        .is_err()
    );
    assert!(
        render_raster(
            &scene,
            &fonts,
            &RasterOptions {
                max_operations: 10,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        painted_labels(
            &scene,
            &fonts,
            &PaintedLabelOptions {
                rotated_only: true,
                ..Default::default()
            }
        )
        .unwrap()
        .labels
        .is_empty()
    );
}
