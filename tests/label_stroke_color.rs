#![cfg(feature = "raster")]
//! Inactive label strokes must not change the existing color-glyph fill path.
use kartis::render::raster::{
    ColorGlyphMode, FontRegistry, MissingGlyphPolicy, RasterCanvas, RasterOptions, render_raster,
};
use kartis::*;
const FONT: &[u8] = include_bytes!("fonts/KartisFixtureColrV0.ttf");
fn fonts(mode: ColorGlyphMode) -> FontRegistry {
    let mut fonts = FontRegistry::new().with_color_glyphs(mode);
    fonts.register(FONT, 0).unwrap();
    fonts
}
fn source(rotation: f64, foreground_alpha: u8) -> Scene {
    let mut scene = ChartConfig {
        width: 240.,
        height: 180.,
        title: "Inactive glyph strokes".into(),
        ..Default::default()
    }
    .scene()
    .unwrap();
    scene.labels.push(Label {
        text: "B".into(),
        x: 40.,
        y: 140.,
        size: 100.,
        color: Color(0, 255, 0, foreground_alpha),
        anchor: TextAnchor::Start,
        rotation,
    });
    scene
}
fn exact_fill_control(stroke: LabelStroke) {
    for palette in [0, 1] {
        let fonts = fonts(ColorGlyphMode::ColrV0 { palette });
        for (rotation, foreground_alpha) in [(0., 128), (17., 0)] {
            let scene = source(rotation, foreground_alpha);
            let expected = render_raster(&scene, &fonts, &RasterOptions::default())
                .expect("explicitly allowed COLR v0 fill must render");
            assert!(
                expected
                    .rgba
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .any(|p| p[..3] != [255, 255, 255]),
                "the control must actually contain palette ink"
            );
            let mut candidate = scene.clone();
            candidate.typography.label_strokes.insert(0, stroke);
            candidate.validate().unwrap();
            let actual = render_raster(&candidate, &fonts, &RasterOptions::default())
                .expect("an inactive stroke must retain the already allowed color-glyph fill path");
            assert_eq!(
                (actual.width, actual.height),
                (expected.width, expected.height)
            );
            assert!(
                actual.rgba == expected.rgba,
                "inactive stroke must preserve every palette/foreground/antialias RGBA byte at palette{palette}, rotation{rotation}"
            );
        }
    }
}
#[test]
fn zero_width_label_stroke_keeps_colr_palette_fill_byte_identical() {
    exact_fill_control(LabelStroke {
        color: Color(230, 80, 10, 255),
        width: 0.,
    });
}
#[test]
fn transparent_label_stroke_keeps_colr_palette_fill_byte_identical() {
    exact_fill_control(LabelStroke {
        color: Color(230, 80, 10, 0),
        width: 7.,
    });
}
#[test]
fn inactive_strokes_keep_the_existing_color_policy_and_palette_errors() {
    let scene = source(0., 128);
    for mode in [
        ColorGlyphMode::Reject,
        ColorGlyphMode::ColrV0 { palette: 2 },
    ] {
        let fonts = fonts(mode);
        let expected = render_raster(&scene, &fonts, &RasterOptions::default()).unwrap_err();
        for stroke in [
            LabelStroke {
                color: Color(0, 0, 0, 255),
                width: 0.,
            },
            LabelStroke {
                color: Color(0, 0, 0, 0),
                width: 3.,
            },
        ] {
            let mut candidate = scene.clone();
            candidate.typography.label_strokes.insert(0, stroke);
            let actual = render_raster(&candidate, &fonts, &RasterOptions::default()).unwrap_err();
            assert_eq!(
                actual.0, expected.0,
                "inactive stroke must not mask the existing policy/palette error"
            );
        }
    }
}
#[test]
fn active_colr_stroke_remains_an_explicit_error_and_keeps_the_accepted_raster() {
    let fonts = fonts(ColorGlyphMode::ColrV0 { palette: 0 });
    let scene = source(0., 128);
    assert!(
        fonts
            .outline_label(&scene.labels[0], MissingGlyphPolicy::Error)
            .is_err(),
        "this fixture does not grant a monochrome outline for its color glyph"
    );
    let mut canvas = RasterCanvas::new(fonts, RasterOptions::default()).unwrap();
    let accepted = canvas.render(&scene).unwrap().clone();
    let mut candidate = scene.clone();
    candidate.typography.label_strokes.insert(
        0,
        LabelStroke {
            color: Color(0, 0, 0, 255),
            width: 4.,
        },
    );
    let error = canvas.render(&candidate).unwrap_err();
    assert!(error.0.contains("color glyph"), "{error}");
    assert_eq!(canvas.frame(), Some(&accepted));
}
