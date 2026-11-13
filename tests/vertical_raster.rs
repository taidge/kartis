#![cfg(feature = "raster")]

use glory::App;
use glory::render::Props;
use kartis::render::raster::*;
use kartis::*;

fn fonts() -> FontRegistry {
    let mut fonts = FontRegistry::new().with_writing_mode(RasterWritingMode::Vertical);
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
        text: text.into(),
        x: 200.,
        y: 35.,
        size: 36.,
        color: Color(14, 45, 100, 255),
        anchor: TextAnchor::Start,
        rotation: 0.,
    });
    scene
}

#[test]
fn upright_vertical_ink_anchors_and_rotated_outlines_share_geometry() {
    let fonts = fonts();
    let mut scene = scene("日本（漢字）。");
    let reference = render_raster(&scene, &fonts, &Default::default()).unwrap();
    let label = &scene.labels[0];
    let advance = fonts
        .measure(&label.text, label.size, MissingGlyphPolicy::Error)
        .unwrap()
        .advance;
    assert_eq!(advance, 252.);
    for (anchor, offset) in [
        (TextAnchor::Middle, advance / 2.),
        (TextAnchor::End, advance),
    ] {
        scene.labels[0].anchor = anchor;
        scene.labels[0].y = 35. + offset;
        assert_eq!(
            render_raster(&scene, &fonts, &Default::default())
                .unwrap()
                .rgba,
            reference.rgba
        );
    }
    let mut left = u32::MAX;
    let mut right = 0;
    let mut top = u32::MAX;
    let mut bottom = 0;
    for (index, pixel) in reference.rgba.as_chunks::<4>().0.iter().enumerate() {
        if pixel[0] < 140 {
            let (x, y) = (
                index as u32 % reference.width,
                index as u32 / reference.width,
            );
            left = left.min(x);
            right = right.max(x);
            top = top.min(y);
            bottom = bottom.max(y);
        }
    }
    assert!(
        left >= 180 && right < 220 && left < right,
        "horizontal bounds {left}..{right}"
    );
    assert!(
        top >= 35 && bottom > 250 && bottom < 287,
        "vertical bounds {top}..{bottom}"
    );

    scene.labels[0].rotation = 28.;
    let raster = render_raster(&scene, &fonts, &Default::default()).unwrap();
    let outline = fonts
        .outline_label(&scene.labels[0], MissingGlyphPolicy::Error)
        .unwrap();
    assert_eq!(outline.advance, advance);
    let color = scene.labels.pop().unwrap().color;
    scene.marks.extend(
        outline
            .glyphs
            .into_iter()
            .enumerate()
            .map(|(index, commands)| {
                Mark::new(format!("glyph:{index}"), Geometry::Path(commands), color)
            }),
    );
    let vector = render_raster(&scene, &fonts, &Default::default()).unwrap();
    let difference = raster
        .rgba
        .iter()
        .zip(&vector.rgba)
        .map(|(a, b)| u64::from(a.abs_diff(*b)))
        .sum::<u64>() as f64
        / raster.rgba.len() as f64;
    assert!(
        difference < 0.08,
        "CPU quadratic vs vector cubic mean: {difference}"
    );
}

#[test]
fn vertical_semantic_outline_keeps_upright_graphics_and_source_text() {
    let fonts = fonts();
    let scene = scene("日本。A");
    assert!(
        outline_labels(&scene, &fonts, &Default::default())
            .unwrap()
            .labels
            .is_empty()
    );
    let options = OutlineLabelOptions {
        rotated_only: false,
        ..Default::default()
    };
    let expected = outline_labels(&scene, &fonts, &options).unwrap();
    let app = App::memory();
    app.mount(chart(scene.clone()).outline_labels(fonts, options))
        .unwrap();

    let tree = app.tree();
    let mut pending: Vec<_> = tree.roots().iter().rev().copied().collect();
    let mut commands = Vec::new();
    let mut source = None;
    while let Some(id) = pending.pop() {
        let node = tree.node(id).unwrap();
        let view = node.descriptor.view();
        if view
            .classes
            .iter()
            .any(|class| class == "kartis-outline-glyph")
        {
            let Props::Graphic(glyph) = &view.props else {
                panic!("outline must use Glory Graphic")
            };
            glyph.validate().unwrap();
            commands.push(glyph.commands.clone());
        }
        if view
            .classes
            .iter()
            .any(|class| class == "kartis-outline-text")
        {
            let Props::Text(text) = &view.props else {
                panic!("source must use Glory Text")
            };
            source = Some(text.content.clone());
        }
        pending.extend(node.children.iter().rev().copied());
    }
    assert_eq!(commands, expected.labels[&0].glyphs);
    assert_eq!(source.as_deref(), Some(scene.labels[0].text.as_str()));
}

#[test]
fn vertical_fallback_preserves_graphemes_and_handles_line_separators() {
    use MissingGlyphPolicy::{Error, Notdef, Skip};
    let fonts = fonts();
    let measure = |text: &str, policy| fonts.measure(text, 36., policy);
    assert_eq!(
        measure("日\u{2028}本", Error).unwrap(),
        measure("日 本", Error).unwrap()
    );
    assert_eq!(
        measure("A\u{fe0f}", Error).unwrap(),
        measure("A", Error).unwrap()
    );
    assert!(measure("日\u{2067}本", Error).is_err());
    assert!(measure("日\u{7}本", Error).is_err());
    assert!(measure(&"日".repeat(MAX_LABEL_BYTES / 3 + 1), Error).is_err());
    assert!(measure("日界本", Error).is_err());
    assert_eq!(
        measure("日界本", Skip).unwrap(),
        measure("日本", Error).unwrap()
    );
    assert_eq!(measure("日界本", Notdef).unwrap().glyph_count, 3);

    let mut fallback = FontRegistry::new().with_writing_mode(RasterWritingMode::Vertical);
    fallback
        .register(include_bytes!("fonts/ABeeZee-Regular.ttf").as_slice(), 0)
        .unwrap();
    assert!(fallback.measure("A\u{323}B", 36., Error).is_err());
    assert_eq!(
        fallback.measure("A\u{323}B", 36., Skip).unwrap(),
        fallback.measure("B", 36., Error).unwrap()
    );
    fallback
        .register(include_bytes!("fonts/Lato-Regular.ttf").as_slice(), 0)
        .unwrap();
    assert!(fallback.measure("A\u{323}B", 36., Error).is_ok());
}

#[test]
fn horizontal_is_the_default_and_can_be_restored() {
    assert_eq!(
        FontRegistry::default().writing_mode(),
        RasterWritingMode::Horizontal
    );
    let vertical = fonts();
    assert_eq!(vertical.writing_mode(), RasterWritingMode::Vertical);
    let horizontal = vertical
        .clone()
        .with_writing_mode(RasterWritingMode::Horizontal);
    let measure =
        |fonts: &FontRegistry| fonts.measure("AA", 36., MissingGlyphPolicy::Error).unwrap();
    assert_eq!(measure(&vertical).advance, 72.);
    assert!(measure(&horizontal).advance < measure(&vertical).advance);

    #[cfg(feature = "raster-bidi")]
    assert_eq!(
        measure(&vertical.clone().with_bidi(BidiBaseDirection::RightToLeft)),
        measure(&vertical)
    );
}
