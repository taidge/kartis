#![cfg(feature = "raster")]

use kartis::render::raster::*;
use kartis::*;

const REGULAR: &[u8] = include_bytes!("fonts/KartisFixtureTypography-Regular.ttf");
const BOLD: &[u8] = include_bytes!("fonts/KartisFixtureTypography-Bold.ttf");
const MISSING: MissingGlyphPolicy = MissingGlyphPolicy::Error;

fn family() -> FontFamily {
    FontFamily::Named("Test 'Typeface', \"quoted\"".into())
}
fn request(weight: f64) -> ResolvedFont {
    ResolvedFont {
        family: vec![family()],
        weight,
    }
}
fn options(weight: f64) -> FontOptions {
    FontOptions {
        family: Some(vec![family()]),
        weight: Some(weight),
    }
}
fn registry() -> FontRegistry {
    let mut fonts = FontRegistry::new();
    // Register bold first: explicit 400 selection cannot accidentally pass
    // through the old first-face registration fallback.
    let bold = fonts.register(BOLD, 0).unwrap();
    let regular = fonts.register(REGULAR, 0).unwrap();
    fonts
        .bind_face(
            bold,
            FaceDescriptor {
                family: family(),
                weight: 700.,
            },
        )
        .unwrap();
    fonts
        .bind_face(
            regular,
            FaceDescriptor {
                family: family(),
                weight: 400.,
            },
        )
        .unwrap();
    fonts
}
fn label(text: &str) -> Label {
    Label {
        x: 20.,
        y: 100.,
        text: text.into(),
        color: Color(0, 0, 0, 255),
        size: 100.,
        anchor: TextAnchor::Start,
        rotation: 0.,
    }
}
fn scene() -> Scene {
    let mut scene = ChartConfig {
        width: 300.,
        height: 250.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    scene.labels.push(label("B"));
    scene.typography.default = options(400.);
    scene
}
fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-6, "{actual} != {expected}");
}
fn pixel(image: &RenderedImage, x: usize, y: usize) -> &[u8] {
    let start = (y * image.width as usize + x) * 4;
    &image.rgba[start..start + 4]
}

#[test]
fn true_static_weights_have_independently_recorded_advances_and_visible_ink() {
    let fonts = registry();
    // Values come from independently re-read hmtx/glyf tables in
    // fixtures/typography-font-reference.json, generated with FontTools.
    for (bytes, weight, advance, width) in [(REGULAR, 400, 60., 40.), (BOLD, 700, 100., 80.)] {
        let face = ttf_parser::Face::parse(bytes, 0).unwrap();
        assert_eq!(face.weight().to_number(), weight);
        assert_eq!(face.units_per_em(), 1000);
        assert_eq!(
            face.glyph_hor_advance(face.glyph_index('A').unwrap()),
            Some((advance * 10.) as u16)
        );
        close(
            fonts
                .measure_with_font("A", 100., &request(f64::from(weight)), MISSING)
                .unwrap()
                .advance,
            advance,
        );
        let bounds = fonts
            .measure_label_with_font(&label("A"), &request(f64::from(weight)), MISSING)
            .unwrap()
            .unwrap();
        close(bounds.x, 22.);
        close(bounds.y, 30.);
        close(bounds.width, width);
        close(bounds.height, 70.);
    }
    close(fonts.measure("A", 100., MISSING).unwrap().advance, 100.);
    assert_ne!(REGULAR, BOLD);
}

#[test]
fn binding_checks_actual_weight_identity_and_rejection_preserves_existing_aliases() {
    let mut fonts = registry();
    for (id, weight) in [
        (FontId(0), 400.),
        (FontId(1), 700.),
        (FontId(99), 400.),
        (FontId(1), f64::NAN),
    ] {
        assert!(
            fonts
                .bind_face(
                    id,
                    FaceDescriptor {
                        family: FontFamily::Serif,
                        weight
                    }
                )
                .is_err()
        );
    }
    let duplicate = fonts.register(REGULAR, 0).unwrap();
    assert!(
        fonts
            .bind_face(
                duplicate,
                FaceDescriptor {
                    family: family(),
                    weight: 400.
                }
            )
            .is_err()
    );
    fonts
        .bind_face(
            FontId(1),
            FaceDescriptor {
                family: family(),
                weight: 400.,
            },
        )
        .unwrap();
    let mut folded = request(400.);
    folded.family = vec![FontFamily::Named("TEST 'TYPEFACE', \"QUOTED\"".into())];
    close(
        fonts
            .measure_with_font("A", 100., &folded, MISSING)
            .unwrap()
            .advance,
        60.,
    );
    for weight in [500., 400.5] {
        close(
            fonts
                .measure_with_font("A", 100., &request(weight), MISSING)
                .unwrap()
                .advance,
            60.,
        );
    }
}

#[test]
fn same_font_selection_drives_cpu_outline_painted_and_rotation_aware_measurement() {
    let fonts = registry();
    for weight in [400., 700.] {
        let mut scene = scene();
        scene.typography.default = options(weight);
        scene.labels[0].rotation = 15.;
        let original = scene.clone();
        let measured = fonts
            .measure_label_with_font(&scene.labels[0], &request(weight), MISSING)
            .unwrap()
            .unwrap();
        let painted = painted_labels(&scene, &fonts, &PaintedLabelOptions::default()).unwrap();
        let outlined = outline_labels(&scene, &fonts, &OutlineLabelOptions::default()).unwrap();
        assert_eq!(
            painted.labels[&0].layers[0].commands,
            outlined.labels[&0].glyphs[0]
        );
        let converted =
            scene_with_painted_labels(&scene, &fonts, &PaintedLabelOptions::default(), "font-test")
                .unwrap();
        assert_eq!(scene, original);
        assert!(converted.labels.is_empty());
        let cpu = render_raster(&scene, &fonts, &Default::default()).unwrap();
        let paths =
            render_raster(&converted, &FontRegistry::default(), &Default::default()).unwrap();
        assert_eq!(cpu.rgba, paths.rgba);
        let black = cpu
            .rgba
            .as_chunks::<4>()
            .0
            .iter()
            .enumerate()
            .filter(|(_, p)| p[0] < 100)
            .collect::<Vec<_>>();
        assert!(black.len() > 500);
        for (index, _) in black {
            let x = (index % cpu.width as usize) as f64 + 0.5;
            let y = (index / cpu.width as usize) as f64 + 0.5;
            assert!(x >= measured.x - 1. && x <= measured.x + measured.width + 1.);
            assert!(y >= measured.y - 1. && y <= measured.y + measured.height + 1.);
        }
    }
}

#[test]
fn per_label_weight_and_remaining_label_font_survive_text_to_path_remapping() {
    let fonts = registry();
    let mut scene = scene();
    scene.typography.labels.insert(
        0,
        FontOptions {
            weight: Some(700.),
            ..Default::default()
        },
    );
    let mut second = label("B");
    second.y = 200.;
    second.rotation = 10.;
    scene.labels.push(second);
    let image = render_raster(&scene, &fonts, &Default::default()).unwrap();
    assert_eq!(pixel(&image, 100, 80), [0, 0, 0, 255]); // bold-only right stem
    let converted = scene_with_painted_labels(
        &scene,
        &fonts,
        &PaintedLabelOptions {
            rotated_only: true,
            ..Default::default()
        },
        "remaining",
    )
    .unwrap();
    assert_eq!(converted.labels.len(), 1);
    assert_eq!(converted.resolved_label_font(0).weight, 700.);
    assert_eq!(converted.typography.labels.len(), 1);
    assert_eq!(
        render_raster(&converted, &fonts, &Default::default())
            .unwrap()
            .rgba,
        image.rgba
    );
}

#[test]
fn missing_binding_rejects_before_frame_commit_and_retry_uses_original_registry() {
    let fonts = registry();
    let valid = scene();
    let mut renderer = RasterCanvas::new(fonts.clone(), RasterOptions::default()).unwrap();
    let accepted = renderer.render(&valid).unwrap().clone();
    let mut invalid = valid.clone();
    invalid.typography.labels.insert(
        0,
        FontOptions {
            family: Some(vec![FontFamily::Named("unbound face".into())]),
            ..Default::default()
        },
    );
    assert!(renderer.render(&invalid).is_err());
    assert_eq!(renderer.frame(), Some(&accepted));
    invalid.typography.default.family = Some(vec![FontFamily::Sans]);
    invalid.typography.labels.clear();
    assert!(renderer.render(&invalid).is_err());
    assert_eq!(renderer.frame(), Some(&accepted));
    assert_eq!(renderer.render(&valid).unwrap(), &accepted);
    assert!(painted_labels(&invalid, &fonts, &Default::default()).is_err());
    invalid.labels[0].rotation = 30.;
    assert!(outline_labels(&invalid, &fonts, &Default::default()).is_err());
}

#[test]
fn explicit_family_order_uses_same_weight_coverage_without_implicit_system_lookup() {
    let mut fonts = registry();
    let bee = fonts
        .register(include_bytes!("fonts/ABeeZee-Regular.ttf").as_slice(), 0)
        .unwrap();
    fonts
        .bind_face(
            bee,
            FaceDescriptor {
                family: FontFamily::Sans,
                weight: 400.,
            },
        )
        .unwrap();
    let fallback = ResolvedFont {
        family: vec![family(), FontFamily::Sans],
        weight: 400.,
    };
    let mut single = FontRegistry::new();
    single
        .register(include_bytes!("fonts/ABeeZee-Regular.ttf").as_slice(), 0)
        .unwrap();
    let bee_advance = single.measure("C", 100., MISSING).unwrap().advance;
    close(
        fonts
            .measure_with_font("AC", 100., &fallback, MISSING)
            .unwrap()
            .advance,
        60. + bee_advance,
    );
    let reversed = ResolvedFont {
        family: vec![FontFamily::Sans, family()],
        weight: 400.,
    };
    close(
        fonts
            .measure_with_font("A", 100., &reversed, MISSING)
            .unwrap()
            .advance,
        single.measure("A", 100., MISSING).unwrap().advance,
    );
    let unbound = ResolvedFont {
        family: vec![FontFamily::Mono, family(), FontFamily::Fantasy],
        weight: 400.,
    };
    close(
        fonts
            .measure_with_font("A", 100., &unbound, MISSING)
            .unwrap()
            .advance,
        60.,
    );
    assert!(
        fonts
            .measure_with_font("C", 100., &request(400.), MISSING)
            .is_err()
    );
}

#[test]
fn css_weight_search_uses_real_static_faces_at_fractional_and_400_500_boundaries() {
    let fonts = registry();
    for (weight, advance) in [
        (1., 60.),
        (300., 60.),
        (399.9, 60.),
        (400., 60.),
        (400.5, 60.),
        (450., 60.),
        (500., 60.),
        (500.1, 100.),
        (600., 100.),
        (700., 100.),
        (1000., 100.),
    ] {
        close(
            fonts
                .measure_with_font("A", 100., &request(weight), MISSING)
                .unwrap()
                .advance,
            advance,
        );
        let mut scene = scene();
        scene.typography.default = options(weight);
        let converted =
            scene_with_painted_labels(&scene, &fonts, &Default::default(), "weight-match").unwrap();
        assert_eq!(
            render_raster(&scene, &fonts, &Default::default())
                .unwrap()
                .rgba,
            render_raster(&converted, &FontRegistry::new(), &Default::default())
                .unwrap()
                .rgba
        );
    }
}

#[test]
fn face_selection_preserves_actual_vertical_substitution_and_colr_palette_layers() {
    let fonts = registry().with_writing_mode(RasterWritingMode::Vertical);
    let mut reference = FontRegistry::new().with_writing_mode(RasterWritingMode::Vertical);
    reference.register(REGULAR, 0).unwrap();
    let vertical = fonts
        .painted_label_with_font(&label("A"), &request(400.), MISSING)
        .unwrap();
    assert_eq!(
        vertical,
        reference.painted_label(&label("A"), MISSING).unwrap()
    );
    close(vertical.advance, 100.);
    assert_eq!(
        fonts
            .measure_layout_with_font("A", 100., &request(400.), MISSING)
            .unwrap(),
        reference.measure_layout("A", 100., MISSING).unwrap()
    );

    let bytes = include_bytes!("fonts/KartisFixtureColrV0.ttf").as_slice();
    let mut color = FontRegistry::new().with_color_glyphs(ColorGlyphMode::ColrV0 { palette: 1 });
    let id = color.register(bytes, 0).unwrap();
    color
        .bind_face(
            id,
            FaceDescriptor {
                family: family(),
                weight: 400.,
            },
        )
        .unwrap();
    let mut label = label("B");
    label.color.3 = 0;
    let selected = color
        .painted_label_with_font(&label, &request(400.), MISSING)
        .unwrap();
    assert_eq!(selected, color.painted_label(&label, MISSING).unwrap());
    assert_eq!(selected.layers.len(), 4);
    assert!(selected.layers.iter().any(|layer| layer.color.3 != 0));
}

#[cfg(feature = "raster-bidi")]
#[test]
fn family_selection_preserves_bidi_order_anchor_and_combining_shape_profile() {
    let fonts = registry().with_bidi(BidiBaseDirection::Auto);
    let mut reference = FontRegistry::new().with_bidi(BidiBaseDirection::Auto);
    reference.register(REGULAR, 0).unwrap();
    let mut label = label("A\u{301} אב B");
    label.anchor = TextAnchor::Middle;
    label.rotation = 20.;
    assert_eq!(
        fonts
            .painted_label_with_font(&label, &request(400.), MISSING)
            .unwrap(),
        reference.painted_label(&label, MISSING).unwrap()
    );
    assert_eq!(
        fonts
            .measure_label_with_font(&label, &request(400.), MISSING)
            .unwrap(),
        reference.measure_label(&label, MISSING).unwrap()
    );
}
