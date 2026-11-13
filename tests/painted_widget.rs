#![cfg(feature = "raster")]
use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use glory::App;
use glory::render::{NodeId, Props};
use glory::style::Display;
use kartis::render::raster::*;
use kartis::*;

fn fonts() -> FontRegistry {
    let mut fonts = FontRegistry::new();
    fonts
        .register(include_bytes!("fonts/ABeeZee-Regular.ttf").as_slice(), 0)
        .unwrap();
    fonts
}
fn scene() -> Scene {
    let mut scene = ChartConfig {
        width: 240.,
        height: 150.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    scene.labels.push(Label {
        text: "A".into(),
        x: 70.,
        y: 80.,
        size: 36.,
        color: Color(30, 100, 170, 255),
        anchor: TextAnchor::Middle,
        rotation: 24.,
    });
    scene.label_series.insert(0, "alpha".into());
    scene
}
fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut stack = tree.roots().iter().rev().copied().collect::<Vec<_>>();
    let mut out = vec![];
    while let Some(id) = stack.pop() {
        let node = tree.node(id).unwrap();
        if node.descriptor.view().classes.iter().any(|c| c == class) {
            out.push(id);
        }
        stack.extend(node.children.iter().rev().copied());
    }
    out
}
fn glyph(app: &App, id: NodeId) -> glory::render::GraphicProps {
    let tree = app.tree();
    let Props::Graphic(graphic) = tree.node(id).unwrap().descriptor.view().props else {
        panic!("expected graphic")
    };
    graphic.clone()
}
#[test]
fn painted_semantic_labels_retain_graphics_and_safe_text_through_updates_and_hiding() {
    let app = App::memory();
    let source = app.scope().cage(scene());
    let hidden = app.scope().cage(BTreeSet::new());
    app.mount(chart(source).hidden(hidden).painted_labels(
        fonts(),
        PaintedLabelOptions {
            rotated_only: false,
            ..Default::default()
        },
    ))
    .unwrap();
    let initial = nodes(&app, "kartis-painted-glyph");
    assert_eq!(initial.len(), 1);
    let drawing = glyph(&app, initial[0]);
    assert_eq!(drawing.fill, Some(Color(30, 100, 170, 255)));
    assert!(!drawing.commands.is_empty());
    let labels = nodes(&app, "kartis-painted-label");
    assert_eq!(labels.len(), 1);
    let text = nodes(&app, "kartis-outline-text");
    {
        let tree = app.tree();
        let Props::Text(value) = tree.node(text[0]).unwrap().descriptor.view().props else {
            panic!("safe text")
        };
        assert_eq!(value.content, "A");
    }
    source.revise(|scene| {
        scene.labels[0].text = "B".into();
        scene.labels[0].color = Color(120, 40, 80, 128);
        scene.labels[0].x += 20.;
    });
    app.flush().unwrap();
    assert_eq!(nodes(&app, "kartis-painted-glyph"), initial);
    let next = glyph(&app, initial[0]);
    assert_eq!(next.fill, Some(Color(120, 40, 80, 128)));
    assert_ne!(next.commands, drawing.commands);
    hidden.revise(|ids| {
        ids.insert("alpha".into());
    });
    app.flush().unwrap();
    assert_eq!(
        app.tree()
            .node(labels[0])
            .unwrap()
            .descriptor
            .view()
            .style
            .display,
        Some(Display::None)
    );
    hidden.revise(BTreeSet::clear);
    app.flush().unwrap();
    assert_eq!(nodes(&app, "kartis-painted-label"), labels);
    app.clear().unwrap();
    assert_eq!(app.stats().nodes, 0);
    assert_eq!(app.stats().handlers, 0);
}
#[test]
fn painted_label_budget_rejection_keeps_scene_layers_and_identity_until_retry() {
    let app = App::memory();
    let source = app.scope().cage(scene());
    let errors = Rc::new(RefCell::new(vec![]));
    let rejected = errors.clone();
    app.mount(
        chart(source)
            .painted_labels(
                fonts(),
                PaintedLabelOptions {
                    rotated_only: false,
                    max_glyphs: 1,
                    ..Default::default()
                },
            )
            .on_reject(move |e| rejected.borrow_mut().push(e.0)),
    )
    .unwrap();
    let ids = nodes(&app, "kartis-painted-glyph");
    let old = glyph(&app, ids[0]);
    let text = nodes(&app, "kartis-outline-text");
    source.revise(|scene| {
        scene.width = 280.;
        scene.labels[0].text = "AB".into();
        scene.labels[0].color = Color(230, 20, 40, 255);
    });
    app.flush().unwrap();
    assert!(!errors.borrow().is_empty());
    assert_eq!(nodes(&app, "kartis-painted-glyph"), ids);
    assert_eq!(glyph(&app, ids[0]), old);
    {
        let tree = app.tree();
        let Props::Text(value) = tree.node(text[0]).unwrap().descriptor.view().props else {
            panic!("safe text")
        };
        assert_eq!(value.content, "A");
    }
    source.revise(|scene| scene.labels[0].text = "B".into());
    app.flush().unwrap();
    assert_eq!(nodes(&app, "kartis-painted-glyph"), ids);
    assert_eq!(glyph(&app, ids[0]).width, 280.);
    assert_eq!(glyph(&app, ids[0]).fill, Some(Color(230, 20, 40, 255)));
}

#[test]
fn semantic_colr_layers_preserve_palette_alpha_holes_and_transparent_foreground() {
    let mut fonts = FontRegistry::new().with_color_glyphs(ColorGlyphMode::ColrV0 { palette: 0 });
    fonts
        .register(
            include_bytes!("fonts/KartisFixtureColrV0.ttf").as_slice(),
            0,
        )
        .unwrap();
    let mut scene = scene();
    scene.labels[0] = Label {
        text: "B".into(),
        x: 40.,
        y: 140.,
        size: 100.,
        color: Color(0, 255, 0, 0),
        anchor: TextAnchor::Start,
        rotation: 0.,
    };
    let app = App::memory();
    let source = app.scope().cage(scene.clone());
    app.mount(chart(source).painted_labels(
        fonts.clone(),
        PaintedLabelOptions {
            rotated_only: false,
            ..Default::default()
        },
    ))
    .unwrap();
    let ids = nodes(&app, "kartis-painted-glyph");
    assert_eq!(ids.len(), 4);
    let drawings = ids.iter().map(|id| glyph(&app, *id)).collect::<Vec<_>>();
    assert_eq!(
        drawings.iter().map(|g| g.fill.unwrap()).collect::<Vec<_>>(),
        [
            Color(255, 0, 0, 255),
            Color(0, 0, 255, 128),
            Color(0, 255, 0, 0),
            Color(0, 0, 0, 0)
        ]
    );
    let semantic_pixels = |scene: &Scene| {
        let mut projected = scene.clone();
        projected.labels.clear();
        projected.label_series.clear();
        projected.draw_order.clear();
        projected.marks.clear();
        for (i, id) in ids.iter().enumerate() {
            let graphic = glyph(&app, *id);
            let mut mark = Mark::new(
                format!("glyph-{i}"),
                Geometry::Path(graphic.commands),
                graphic.fill.unwrap(),
            );
            mark.label.clear();
            projected.marks.push(mark);
        }
        render_raster(&projected, &FontRegistry::new(), &Default::default()).unwrap()
    };
    let expected = render_raster(&scene, &fonts, &Default::default()).unwrap();
    assert_eq!(
        semantic_pixels(&scene).rgba,
        expected.rgba,
        "semantic layer paths must paint the same font ink as CPU text"
    );
    let pixel = |image: &RenderedImage, x: usize, y: usize| {
        let start = (y * image.width as usize + x) * 4;
        image.rgba[start..start + 4].to_vec()
    };
    assert_eq!(pixel(&expected, 50, 50), [255, 0, 0, 255]);
    assert_eq!(pixel(&expected, 70, 80), [255, 255, 255, 255]);
    assert_eq!(
        pixel(&expected, 50, 125),
        [255, 0, 0, 255],
        "transparent foreground leaves the palette ring visible"
    );
    source.revise(|scene| scene.labels[0].color = Color(0, 255, 0, 128));
    app.flush().unwrap();
    assert_eq!(nodes(&app, "kartis-painted-glyph"), ids);
    assert_eq!(glyph(&app, ids[0]).fill, Some(Color(255, 0, 0, 255)));
    assert_eq!(glyph(&app, ids[2]).fill, Some(Color(0, 255, 0, 128)));
    scene.labels[0].color = Color(0, 255, 0, 128);
    let next = render_raster(&scene, &fonts, &Default::default()).unwrap();
    assert_eq!(semantic_pixels(&scene).rgba, next.rgba);
    assert_eq!(pixel(&next, 50, 125), [127, 128, 0, 255]);
}

#[test]
fn painted_font_bytes_and_reactive_effects_retire_with_the_widget_scope() {
    let bytes: std::sync::Arc<[u8]> = include_bytes!("fonts/KartisFixtureColrV0.ttf")
        .as_slice()
        .into();
    let weak = std::sync::Arc::downgrade(&bytes);
    let mut fonts = FontRegistry::new().with_color_glyphs(ColorGlyphMode::ColrV0 { palette: 0 });
    fonts.register(bytes, 0).unwrap();
    let app = App::memory();
    let mut initial = scene();
    initial.labels[0].text = "B".into();
    let source = app.scope().cage(initial);
    app.mount(chart(source).painted_labels(fonts, PaintedLabelOptions::default()))
        .unwrap();
    assert!(weak.upgrade().is_some());
    app.clear().unwrap();
    assert!(
        weak.upgrade().is_none(),
        "registered font bytes survived owner cleanup"
    );
    source.revise(|scene| scene.labels[0].text = "missing 🐱".into());
    app.flush().unwrap();
    assert_eq!(app.stats().nodes, 0);
    assert_eq!(app.stats().handlers, 0);
}
