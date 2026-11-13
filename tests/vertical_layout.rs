#![cfg(feature = "raster")]

use std::rc::Rc;

use glory::App;
use kartis::render::raster::*;
use kartis::*;

fn fonts(columns: RasterVerticalColumns) -> FontRegistry {
    let mut fonts = FontRegistry::new().with_vertical_layout(RasterVerticalLayout {
        orientation: RasterGlyphOrientation::Mixed,
        columns,
        column_advance_em: 1.5,
    });
    fonts
        .register(
            include_bytes!("fonts/KartisFixtureVertical-Regular.ttf").as_slice(),
            0,
        )
        .unwrap();
    fonts
}

fn scene(text: &str) -> Scene {
    let mut scene = ChartConfig {
        width: 400.,
        height: 340.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    scene.labels.push(Label {
        x: 240.,
        y: 45.,
        text: text.into(),
        size: 40.,
        color: Color(14, 45, 100, 255),
        anchor: TextAnchor::Start,
        rotation: 0.,
    });
    scene
}

#[test]
fn explicit_columns_preserve_empty_columns_and_crlf_and_report_two_dimensional_metrics() {
    let fonts = fonts(RasterVerticalColumns::RightToLeft);
    let metrics = fonts
        .measure_layout("日本AB\n（A）。", 40., MissingGlyphPolicy::Error)
        .unwrap();
    assert_eq!(metrics.columns.len(), 2);
    assert_eq!(metrics.columns[0].x, 0.);
    assert_eq!(metrics.columns[1].x, -60.);
    assert!((metrics.columns[0].advance - 130.6).abs() < 1e-10);
    assert!((metrics.columns[1].advance - 144.32).abs() < 1e-10);
    assert_eq!(
        metrics.advance_bounds,
        RasterTextBounds {
            x: -80.,
            y: 0.,
            width: 100.,
            height: metrics.advance
        }
    );
    assert_eq!(metrics.advance, metrics.columns[1].advance);
    assert_eq!(
        fonts
            .measure("日本AB\n（A）。", 40., MissingGlyphPolicy::Error)
            .unwrap()
            .advance,
        metrics.advance
    );

    let expected = fonts
        .measure_layout("日\n\n本\n", 40., MissingGlyphPolicy::Error)
        .unwrap();
    for text in [
        "日\r\n\r\n本\r\n",
        "日\r\r本\r",
        "日\u{c}\u{85}本\u{2029}",
        "日\u{2028}\u{2028}本\u{2028}",
    ] {
        assert_eq!(
            fonts
                .measure_layout(text, 40., MissingGlyphPolicy::Error)
                .unwrap(),
            expected
        );
    }
    assert_eq!(
        expected
            .columns
            .iter()
            .map(|column| column.advance)
            .collect::<Vec<_>>(),
        [40., 0., 40., 0.]
    );
    let empty = fonts
        .measure_layout("", 40., MissingGlyphPolicy::Error)
        .unwrap();
    assert!(empty.columns.is_empty() && empty.ink_bounds.is_none());
    assert_eq!(empty.advance_bounds, RasterTextBounds::default());
}

#[test]
fn mixed_columns_cpu_and_complete_outlines_have_matching_rotated_ink() {
    for columns in [
        RasterVerticalColumns::RightToLeft,
        RasterVerticalColumns::LeftToRight,
    ] {
        let fonts = fonts(columns);
        let mut scene = scene("日本AB\n（A）。\nA\u{301}B\nA\u{20dd}");
        scene.labels[0].x = if columns == RasterVerticalColumns::RightToLeft {
            240.
        } else {
            110.
        };
        for rotation in [0., 31.] {
            scene.labels[0].rotation = rotation;
            let raster = render_raster(&scene, &fonts, &Default::default()).unwrap();
            let outline = fonts
                .outline_label(&scene.labels[0], MissingGlyphPolicy::Error)
                .unwrap();
            let mut vector = scene.clone();
            let color = vector.labels.pop().unwrap().color;
            vector.marks.extend(
                outline
                    .glyphs
                    .into_iter()
                    .enumerate()
                    .map(|(index, commands)| {
                        Mark::new(format!("glyph:{index}"), Geometry::Path(commands), color)
                    }),
            );
            let vector = render_raster(&vector, &fonts, &Default::default()).unwrap();
            let difference = raster
                .rgba
                .iter()
                .zip(&vector.rgba)
                .map(|(a, b)| u64::from(a.abs_diff(*b)))
                .sum::<u64>() as f64
                / raster.rgba.len() as f64;
            assert!(
                difference < 0.08,
                "CPU quadratic vs vector cubic mean {difference}"
            );
        }
    }
}

#[test]
fn scene_ink_measurement_includes_bearings_anchor_rotation_and_sideways_glyphs() {
    let fonts = fonts(RasterVerticalColumns::RightToLeft);
    let mut scene = scene("日本AB\n（A）。");
    let local = fonts
        .measure_layout(&scene.labels[0].text, 40., MissingGlyphPolicy::Error)
        .unwrap();
    let bounds = fonts
        .measure_label(&scene.labels[0], MissingGlyphPolicy::Error)
        .unwrap()
        .unwrap();
    let ink = local.ink_bounds.unwrap();
    assert!((bounds.x - (ink.x + scene.labels[0].x)).abs() < 1e-10);
    assert!((bounds.y - (ink.y + scene.labels[0].y)).abs() < 1e-10);
    assert_eq!((bounds.width, bounds.height), (ink.width, ink.height));

    for (anchor, offset) in [
        (TextAnchor::Middle, local.advance / 2.),
        (TextAnchor::End, local.advance),
    ] {
        scene.labels[0].anchor = anchor;
        scene.labels[0].y = 45. + offset;
        let measured = fonts
            .measure_label(&scene.labels[0], MissingGlyphPolicy::Error)
            .unwrap()
            .unwrap();
        for (actual, expected) in [measured.x, measured.y, measured.width, measured.height]
            .into_iter()
            .zip([bounds.x, bounds.y, bounds.width, bounds.height])
        {
            assert!((actual - expected).abs() < 1e-10);
        }
    }
    scene.labels[0].rotation = 37.;
    let bounds = fonts
        .measure_label(&scene.labels[0], MissingGlyphPolicy::Error)
        .unwrap()
        .unwrap();
    let raster = render_raster(&scene, &fonts, &Default::default()).unwrap();
    let mut ink_pixels = 0;
    for (index, pixel) in raster.rgba.as_chunks::<4>().0.iter().enumerate() {
        if pixel[0] < 180 {
            let x = (index % raster.width as usize) as f64;
            let y = (index / raster.width as usize) as f64;
            assert!(x + 1. >= bounds.x && x <= bounds.x + bounds.width + 1.);
            assert!(y + 1. >= bounds.y && y <= bounds.y + bounds.height + 1.);
            ink_pixels += 1;
        }
    }
    assert!(ink_pixels > 1000);
    scene.labels[0].text = " \n ".into();
    assert_eq!(
        fonts
            .measure_label(&scene.labels[0], MissingGlyphPolicy::Error)
            .unwrap(),
        None
    );
    scene.labels[0].rotation = f64::NAN;
    assert!(
        fonts
            .measure_label(&scene.labels[0], MissingGlyphPolicy::Error)
            .is_err()
    );
}

#[test]
fn vertical_configuration_column_and_operation_budgets_reject_atomically() {
    let fonts = fonts(RasterVerticalColumns::RightToLeft);
    for spacing in [f64::NAN, f64::INFINITY, 0., 0.24, 16.01] {
        let invalid = fonts.clone().with_vertical_layout(RasterVerticalLayout {
            column_advance_em: spacing,
            ..fonts.vertical_layout()
        });
        assert!(invalid.measure("", 40., MissingGlyphPolicy::Error).is_err());
    }
    let maximum = format!("{}日", "\n".repeat(255));
    assert_eq!(
        fonts
            .measure_layout(&maximum, 40., MissingGlyphPolicy::Error)
            .unwrap()
            .columns
            .len(),
        256
    );
    let rejected = format!("{}日", "\n".repeat(256));
    assert!(
        fonts
            .measure_layout(&rejected, 40., MissingGlyphPolicy::Error)
            .is_err()
    );
    assert!(
        render_raster(
            &scene("日本AB"),
            &fonts,
            &RasterOptions {
                max_operations: 1,
                ..Default::default()
            }
        )
        .is_err()
    );

    let app = App::memory();
    let scope = app.scope().child().unwrap();
    let surface =
        RasterSurface::mount(&scope, scene("日本AB\n（A）。"), fonts, Default::default()).unwrap();
    let handle = surface.handle();
    let before = handle.snapshot().unwrap();
    let frame = handle.frame().unwrap();
    assert!(handle.set_scene(scene(&rejected)).is_err());
    assert_eq!(handle.snapshot().unwrap(), before);
    assert!(Rc::ptr_eq(&handle.frame().unwrap(), &frame));
    handle.set_scene(scene("日本AB\n日")).unwrap();
    assert_ne!(handle.frame().unwrap().rgba, frame.rgba);
}
