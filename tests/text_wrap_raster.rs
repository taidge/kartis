#![cfg(feature = "raster")]

use kartis::render::raster::*;
use kartis::*;

const REGULAR: &[u8] = include_bytes!("fonts/KartisFixtureTypography-Regular.ttf");
const BOLD: &[u8] = include_bytes!("fonts/KartisFixtureTypography-Bold.ttf");

fn fonts() -> FontRegistry {
    let mut fonts = FontRegistry::new();
    for (bytes, weight) in [(BOLD, 700.), (REGULAR, 400.)] {
        let id = fonts.register(bytes, 0).unwrap();
        fonts
            .bind_face(
                id,
                FaceDescriptor {
                    family: FontFamily::Named("WrapFixture".into()),
                    weight,
                },
            )
            .unwrap();
    }
    fonts
}
fn font(weight: f64) -> ResolvedFont {
    ResolvedFont {
        family: vec![FontFamily::Named("WrapFixture".into())],
        weight,
    }
}
fn label() -> Label {
    Label {
        x: 150.,
        y: 40.,
        text: "BBBB".into(),
        size: 20.,
        color: Color(20, 40, 90, 255),
        anchor: TextAnchor::Middle,
        rotation: 0.,
    }
}
fn options(width: f64) -> TextWrapOptions {
    TextWrapOptions {
        max_width: width,
        mode: TextWrapMode::Graphemes,
        ..Default::default()
    }
}

#[test]
fn real_bound_face_advances_control_breaks_and_metrics() {
    let fonts = fonts();
    let a = fonts
        .measure_with_font("BB", 20., &font(400.), MissingGlyphPolicy::Error)
        .unwrap()
        .advance;
    let b = fonts
        .measure_with_font("BB", 20., &font(700.), MissingGlyphPolicy::Error)
        .unwrap()
        .advance;
    assert!(a < b);
    let regular = fonts
        .wrap_label(
            &label(),
            &font(400.),
            &options((a + b) / 2.),
            MissingGlyphPolicy::Error,
        )
        .unwrap();
    let bold = fonts
        .wrap_label(
            &label(),
            &font(700.),
            &options((a + b) / 2.),
            MissingGlyphPolicy::Error,
        )
        .unwrap();
    assert_eq!(regular.labels.len(), 2);
    assert_eq!(bold.labels.len(), 4);
    for (line, measurement) in regular.labels.iter().zip(&regular.measurements) {
        let actual = fonts
            .measure_layout_with_font(
                &line.text,
                line.size,
                &font(400.),
                MissingGlyphPolicy::Error,
            )
            .unwrap();
        assert_eq!(measurement.metrics().advance, actual.advance);
        assert!(measurement.metrics().baseline_from_top > 0.);
        assert_eq!(measurement.request().font, font(400.));
        assert_eq!(
            measurement.metrics().ink_bounds.unwrap().width,
            actual.ink_bounds.unwrap().width
        );
    }
}

#[test]
fn scene_replacement_keeps_fonts_clips_ownership_and_real_exports() {
    let fonts = fonts();
    let mut scene = ChartConfig::default().scene().unwrap();
    let mut original = label();
    original.rotation = 90.;
    scene.labels.push(original.clone());
    scene.typography.labels.insert(
        0,
        FontOptions {
            family: Some(font(400.).family),
            weight: Some(400.),
        },
    );
    scene.label_series.insert(0, "source".into());
    scene
        .label_clips
        .insert(0, ClipPath::circle(150., 100., 150.));
    scene.draw_order.push(DrawItem::Label(0));
    let wrapped = fonts
        .wrap_scene_label(&mut scene, 0, &options(30.), MissingGlyphPolicy::Error)
        .unwrap();
    assert!(wrapped.labels.len() > 1);
    for (index, line) in scene.labels.iter().enumerate() {
        assert_eq!(scene.resolved_label_font(index), font(400.));
        assert_eq!(scene.label_series[&index], "source");
        assert!(scene.label_clips.contains_key(&index));
        assert!((line.x - (original.x - index as f64 * 24.)).abs() < 1e-8);
        assert!((line.y - original.y).abs() < 1e-8);
    }
    scene.validate().unwrap();
    assert_eq!(scene.draw_order.len(), scene.labels.len());
    let svg = render::to_svg(&scene).unwrap();
    assert!(svg.contains("WrapFixture"));
    assert!(
        render::to_canvas_script(&scene, "wrapped")
            .unwrap()
            .contains("WrapFixture")
    );
    let image = render_raster(&scene, &fonts, &RasterOptions::default()).unwrap();
    assert!(image.rgba.as_chunks::<4>().0.iter().any(|p| p[0] < 200));
    let outlined = scene_with_painted_labels(
        &scene,
        &fonts,
        &PaintedLabelOptions {
            rotated_only: false,
            ..Default::default()
        },
        "wrap",
    )
    .unwrap();
    let outlined_image =
        render_raster(&outlined, &FontRegistry::new(), &RasterOptions::default()).unwrap();
    assert!(
        image.rgba == outlined_image.rgba,
        "wrapped text and outline CPU output diverged"
    );
}

#[test]
fn missing_faces_vertical_and_work_failure_do_not_publish_partial_scene() {
    let mut scene = ChartConfig::default().scene().unwrap();
    scene.labels.push(label());
    scene.typography.labels.insert(
        0,
        FontOptions {
            family: Some(font(400.).family),
            weight: Some(400.),
        },
    );
    let before = scene.clone();
    assert!(
        FontRegistry::new()
            .wrap_scene_label(&mut scene, 0, &options(30.), MissingGlyphPolicy::Error)
            .is_err()
    );
    assert_eq!(before, scene);
    let vertical = fonts().with_writing_mode(RasterWritingMode::Vertical);
    assert!(
        vertical
            .wrap_scene_label(&mut scene, 0, &options(30.), MissingGlyphPolicy::Error)
            .is_err()
    );
    assert_eq!(before, scene);
    let low = TextWrapOptions {
        max_measurements: 1,
        ..options(30.)
    };
    assert!(
        fonts()
            .wrap_scene_label(&mut scene, 0, &low, MissingGlyphPolicy::Error)
            .is_err()
    );
    assert_eq!(before, scene);
}

#[test]
fn legacy_registration_order_matches_undeclared_scene_rendering() {
    let fonts = fonts();
    let mut scene = ChartConfig::default().scene().unwrap();
    scene.labels.push(label());
    let advance = fonts
        .measure("BB", 20., MissingGlyphPolicy::Error)
        .unwrap()
        .advance;
    let wrapped = fonts
        .wrap_scene_label(
            &mut scene,
            0,
            &options(advance + 0.01),
            MissingGlyphPolicy::Error,
        )
        .unwrap();
    assert_eq!(wrapped.labels.len(), 2);
    assert!(scene.typography.labels.is_empty());
    for m in wrapped.measurements {
        assert_eq!(m.metrics().advance, advance);
    }
}

#[test]
fn empty_unbound_registry_returns_error_even_for_empty_runs() {
    for text in ["", "   "] {
        let mut scene = ChartConfig::default().scene().unwrap();
        let mut value = label();
        value.text = text.into();
        scene.labels.push(value);
        let before = scene.clone();
        assert!(
            FontRegistry::new()
                .wrap_scene_label(&mut scene, 0, &options(30.), MissingGlyphPolicy::Error)
                .is_err()
        );
        assert_eq!(scene, before);
    }
}

#[cfg(feature = "raster-bidi")]
#[test]
fn bidi_lines_are_shaped_again_in_their_own_logical_order() {
    let mut fonts = FontRegistry::new().with_bidi(BidiBaseDirection::RightToLeft);
    let face = fonts
        .register(
            include_bytes!("fonts/KartisFixtureHebrew-Regular.ttf").as_slice(),
            0,
        )
        .unwrap();
    let family = FontFamily::Named("HebrewWrap".into());
    fonts
        .bind_face(
            face,
            FaceDescriptor {
                family: family.clone(),
                weight: 400.,
            },
        )
        .unwrap();
    let font = ResolvedFont {
        family: vec![family],
        weight: 400.,
    };
    let mut value = label();
    value.text = "אבגאבג".into();
    let width = fonts
        .measure_with_font("אב", 20., &font, MissingGlyphPolicy::Error)
        .unwrap()
        .advance
        + 0.01;
    let wrapped = fonts
        .wrap_label(&value, &font, &options(width), MissingGlyphPolicy::Error)
        .unwrap();
    assert!(wrapped.labels.len() > 1);
    assert_eq!(
        wrapped
            .labels
            .iter()
            .map(|l| l.text.as_str())
            .collect::<String>(),
        value.text
    );
    for (line, measurement) in wrapped.labels.iter().zip(wrapped.measurements) {
        assert_eq!(
            measurement.metrics().advance,
            fonts
                .measure_with_font(&line.text, line.size, &font, MissingGlyphPolicy::Error)
                .unwrap()
                .advance
        );
    }
}
