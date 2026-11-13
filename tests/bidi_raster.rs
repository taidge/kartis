#![cfg(feature = "raster-bidi")]

use glory::{App, render::Props};
use kartis::{render::raster::*, *};
use std::cell::Cell;
use std::rc::Rc;

fn fonts(base: BidiBaseDirection) -> FontRegistry {
    let mut fonts = FontRegistry::new().with_bidi(base);
    for bytes in [
        include_bytes!("fonts/ABeeZee-Regular.ttf").as_slice(),
        include_bytes!("fonts/Lato-Regular.ttf").as_slice(),
        include_bytes!("fonts/KartisFixtureHebrew-Regular.ttf").as_slice(),
        include_bytes!("fonts/KartisFixtureArabic-Regular.ttf").as_slice(),
    ] {
        fonts.register(bytes, 0).unwrap();
    }
    fonts
}

fn scene(text: &str) -> Scene {
    let mut scene = ChartConfig {
        width: 420.,
        height: 180.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    scene.labels.push(Label {
        text: text.into(),
        x: 35.,
        y: 76.,
        size: 34.,
        color: Color(14, 45, 100, 255),
        anchor: TextAnchor::Start,
        rotation: 0.,
    });
    scene
}

#[test]
fn bidi_cpu_ink_and_vector_outlines_match_at_physical_anchors() {
    let fonts = fonts(BidiBaseDirection::Auto);
    let text = "abc אבג 123 السَّلَام";
    let mut scene = scene(text);
    let reference = render_raster(&scene, &fonts, &Default::default()).unwrap();
    let advance = fonts
        .measure(text, scene.labels[0].size, MissingGlyphPolicy::Error)
        .unwrap()
        .advance;
    for (anchor, offset) in [
        (TextAnchor::Middle, advance / 2.),
        (TextAnchor::End, advance),
    ] {
        scene.labels[0].anchor = anchor;
        scene.labels[0].x = 35. + offset;
        assert_eq!(
            render_raster(&scene, &fonts, &Default::default())
                .unwrap()
                .rgba,
            reference.rgba,
        );
    }

    scene.labels[0].x = 345.;
    scene.labels[0].y = 58.;
    scene.labels[0].rotation = -15.;
    let rotated = render_raster(&scene, &fonts, &Default::default()).unwrap();
    let outline = fonts
        .outline_label(&scene.labels[0], MissingGlyphPolicy::Error)
        .unwrap();
    assert_eq!(outline.advance, advance);
    let mut vector = scene.clone();
    let color = vector.labels.pop().unwrap().color;
    vector.marks.extend(
        outline
            .glyphs
            .into_iter()
            .enumerate()
            .map(|(i, commands)| Mark::new(format!("glyph:{i}"), Geometry::Path(commands), color)),
    );
    let vector = render_raster(&vector, &fonts, &Default::default()).unwrap();
    let ink = rotated
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[0] < 180 && p[2] > p[0])
        .count();
    assert!(ink > 1000, "rotated mixed-script glyph ink: {ink}");
    let difference = rotated
        .rgba
        .iter()
        .zip(&vector.rgba)
        .map(|(a, b)| u64::from(a.abs_diff(*b)))
        .sum::<u64>() as f64
        / rotated.rgba.len() as f64;
    assert!(
        difference < 0.08,
        "CPU quadratic vs vector cubic mean: {difference}"
    );
    assert!(!rotated.png().unwrap().is_empty());
}

#[test]
fn semantic_graphics_preserve_source_text_and_previous_outlined_frame_on_error() {
    let app = App::memory();
    let scene = scene("אבג (123) abc سلام");
    let fonts = fonts(BidiBaseDirection::RightToLeft);
    let options = OutlineLabelOptions {
        rotated_only: false,
        ..Default::default()
    };
    let expected = outline_labels(&scene, &fonts, &options).unwrap();
    let source = app.scope().cage(scene.clone());
    let rejected = Rc::new(Cell::new(0));
    let seen = rejected.clone();
    app.mount(
        chart(source)
            .outline_labels(fonts, options)
            .on_reject(move |_| seen.set(seen.get() + 1)),
    )
    .unwrap();
    let painted = || {
        let tree = app.tree();
        let mut pending: Vec<_> = tree.roots().iter().rev().copied().collect();
        let mut commands = Vec::new();
        let mut accessible = None;
        while let Some(id) = pending.pop() {
            let node = tree.node(id).unwrap();
            let view = node.descriptor.view();
            if view.classes.iter().any(|c| c == "kartis-outline-glyph") {
                let Props::Graphic(glyph) = &view.props else {
                    panic!("outline label must use actual Glory Graphic")
                };
                glyph.validate().unwrap();
                commands.push(glyph.commands.clone());
            }
            if view.classes.iter().any(|c| c == "kartis-outline-text") {
                let Props::Text(text) = &view.props else {
                    panic!("outline label must preserve source text")
                };
                accessible = Some(text.clone());
            }
            pending.extend(node.children.iter().rev().copied());
        }
        (commands, accessible)
    };
    let before = painted();
    assert_eq!(before.0, expected.labels[&0].glyphs);
    assert_eq!(before.1.as_ref().unwrap().content, scene.labels[0].text);
    source.revise(|scene| scene.labels[0].text = "ابج界".into());
    app.flush().unwrap();
    assert_eq!(rejected.get(), 1);
    assert_eq!(painted(), before);
}

#[test]
fn bidi_controls_and_grapheme_missing_policies_are_explicit() {
    use MissingGlyphPolicy::{Error, Notdef, Skip};
    let fonts = fonts(BidiBaseDirection::Auto);
    let measure = |text: &str, policy| fonts.measure(text, 30., policy);
    assert_eq!(
        measure("A\u{2029}אב", Error).unwrap(),
        measure("A אב", Error).unwrap()
    );
    assert_eq!(
        measure("\u{2067}\u{200f}\u{2069}", Error)
            .unwrap()
            .glyph_count,
        0
    );
    assert_eq!(
        measure("x\u{fe0f}", Error).unwrap(),
        measure("x", Error).unwrap()
    );
    assert!(measure("x\u{7}", Error).is_err());
    assert!(measure(&"א".repeat(MAX_LABEL_BYTES / 2 + 1), Error).is_err());

    let mut first = FontRegistry::new().with_bidi(BidiBaseDirection::Auto);
    first
        .register(include_bytes!("fonts/ABeeZee-Regular.ttf").as_slice(), 0)
        .unwrap();
    assert!(first.measure("A\u{323}B", 30., Error).is_err());
    assert_eq!(
        first.measure("A\u{323}B", 30., Skip).unwrap(),
        first.measure("B", 30., Error).unwrap(),
    );
    assert!(first.measure("A\u{323}B", 30., Notdef).unwrap().glyph_count > 1);
    assert!(measure("A\u{323}B", Error).is_ok());
    // The Arabic fixture lacks U+0635. One context span fails or skips as a
    // unit, rather than breaking adjacent connected letters across faces.
    assert!(measure("سصلام", Error).is_err());
    assert_eq!(measure("سصلام", Skip).unwrap().glyph_count, 0);
    assert!(measure("سصلام", Notdef).unwrap().glyph_count > 0);
    assert!(
        measure("سلام", Error).unwrap().glyph_count
            < measure("س\u{200c}لام", Error).unwrap().glyph_count
            || fonts
                .outline_label(&scene("سلام").labels[0], Error)
                .unwrap()
                .glyphs
                != fonts
                    .outline_label(&scene("س\u{200c}لام").labels[0], Error)
                    .unwrap()
                    .glyphs
    );
}

#[test]
fn raster_surface_preserves_frame_when_bidi_shaping_rejects() {
    let app = App::memory();
    let scope = app.scope().child().unwrap();
    let surface = RasterSurface::mount(
        &scope,
        scene("abc אבג"),
        fonts(BidiBaseDirection::Auto),
        Default::default(),
    )
    .unwrap();
    let handle = surface.handle();
    let before = handle.snapshot().unwrap();
    let pixels = handle.frame().unwrap();
    assert!(handle.set_scene(scene("سلام界")).is_err());
    assert_eq!(handle.snapshot().unwrap(), before);
    assert!(Rc::ptr_eq(&handle.frame().unwrap(), &pixels));
    handle.set_scene(scene("سلام")).unwrap();
    assert_ne!(handle.frame().unwrap().rgba, pixels.rgba);
}
