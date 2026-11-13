#![cfg(feature = "raster")]
use std::cell::Cell;
use std::collections::BTreeSet;
use std::rc::Rc;

use glory::App;
use glory::render::{GraphicPoint, NodeId, PathCommand, Props};
use glory::style::{Display, Length};
use kartis::render::raster::*;
use kartis::*;
fn fonts() -> FontRegistry {
    let mut fonts = FontRegistry::new();
    fonts
        .register(include_bytes!("fonts/Lato-Regular.ttf").as_slice(), 0)
        .unwrap();
    fonts
}
fn label(text: &str) -> Label {
    Label {
        x: 100.,
        y: 80.,
        text: text.into(),
        color: Color(10, 20, 30, 255),
        size: 30.,
        anchor: TextAnchor::Start,
        rotation: 0.,
    }
}
fn scene() -> Scene {
    let mut s = ChartConfig {
        width: 300.,
        height: 200.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    let mut m = Mark::new(
        "datum",
        Geometry::Circle {
            x: 20.,
            y: 20.,
            radius: 5.,
        },
        Color(20, 80, 150, 255),
    );
    m.series = "a".into();
    m.label = "Datum".into();
    s.marks.push(m);
    s.labels.push(Label {
        rotation: 35.,
        ..label("AV fi O")
    });
    s.labels.push(Label {
        x: 60.,
        y: 150.,
        text: "Shared axis".into(),
        ..label("")
    });
    s.label_series.insert(0, "a".into());
    s.legend.push(LegendItem {
        id: "a".into(),
        label: "A".into(),
        color: Color(20, 80, 150, 255),
    });
    s.draw_order = vec![DrawItem::Label(1), DrawItem::Mark(0), DrawItem::Label(0)];
    s
}
fn points(commands: &[PathCommand]) -> Vec<GraphicPoint> {
    commands
        .iter()
        .flat_map(|c| match c {
            PathCommand::Move(p) | PathCommand::Line(p) => vec![*p],
            PathCommand::Cubic { first, second, end } => vec![*first, *second, *end],
            PathCommand::Close => vec![],
        })
        .collect()
}
fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut stack = tree.roots().iter().rev().copied().collect::<Vec<_>>();
    let mut out = Vec::new();
    while let Some(id) = stack.pop() {
        let n = tree.node(id).unwrap();
        if n.descriptor.view().classes.iter().any(|c| c == class) {
            out.push(id);
        }
        stack.extend(n.children.iter().rev().copied());
    }
    out
}
#[test]
fn exact_font_outlines_shape_ligatures_kerning_combining_and_rotate_about_baseline() {
    let fonts = fonts();
    let plain = fonts
        .outline_label(&label("AV fi e\u{301}"), MissingGlyphPolicy::Error)
        .unwrap();
    assert!(plain.advance > 0.);
    assert_eq!(
        fonts
            .measure("AV fi e\u{301}", 30., MissingGlyphPolicy::Error)
            .unwrap()
            .advance,
        plain.advance
    );
    assert_eq!(
        fonts
            .outline_label(&label("fi"), MissingGlyphPolicy::Error)
            .unwrap()
            .glyphs
            .len(),
        1
    );
    let rotated = fonts
        .outline_label(
            &Label {
                rotation: 90.,
                ..label("AV fi e\u{301}")
            },
            MissingGlyphPolicy::Error,
        )
        .unwrap();
    for (a, b) in plain.glyphs.iter().zip(&rotated.glyphs) {
        assert_eq!(a.len(), b.len());
        for (p, q) in points(a).iter().zip(points(b)) {
            assert!((q.x - (100. - (p.y - 80.))).abs() < 1e-8);
            assert!((q.y - (80. + (p.x - 100.))).abs() < 1e-8);
        }
    }
    let middle = fonts
        .outline_label(
            &Label {
                anchor: TextAnchor::Middle,
                ..label("AV fi e\u{301}")
            },
            MissingGlyphPolicy::Error,
        )
        .unwrap();
    let a = points(&plain.glyphs[0])[0];
    let b = points(&middle.glyphs[0])[0];
    assert!((a.x - b.x - plain.advance / 2.).abs() < 1e-8);
}
#[test]
fn whole_glyph_paths_preserve_holes_and_scene_metadata_ownership_and_strata() {
    let fonts = fonts();
    let o = fonts
        .outline_label(&label("O"), MissingGlyphPolicy::Error)
        .unwrap();
    assert_eq!(o.glyphs.len(), 1);
    assert!(
        o.glyphs[0]
            .iter()
            .filter(|c| matches!(c, PathCommand::Move(_)))
            .count()
            >= 2
    );
    assert!(
        o.glyphs[0]
            .iter()
            .filter(|c| matches!(c, PathCommand::Close))
            .count()
            >= 2
    );
    let s = scene();
    let original = s.clone();
    let outlined = outline_labels(&s, &fonts, &Default::default()).unwrap();
    assert_eq!(s, original);
    assert!(outlined.labels.contains_key(&0) && !outlined.labels.contains_key(&1));
    assert!(outlined.command_count > 0);
    assert_eq!(s.draw_items(), original.draw_items());
    for glyph in &outlined.labels[&0].glyphs {
        glory::render::GraphicProps {
            commands: glyph.clone(),
            fill: Some(Color(10, 20, 30, 255)),
            ..Default::default()
        }
        .validate()
        .unwrap();
    }
}
#[test]
fn semantic_chart_paints_real_outlines_with_accessible_safe_text_at_original_order() {
    let app = App::memory();
    let hidden = app.scope().cage(BTreeSet::new());
    app.mount(
        chart(scene())
            .outline_labels(fonts(), Default::default())
            .hidden(hidden),
    )
    .unwrap();
    assert!(!nodes(&app, "kartis-outline-glyph").is_empty());
    assert_eq!(nodes(&app, "kartis-datum").len(), 1);
    let wrappers = nodes(&app, "kartis-paint-item");
    let tree = app.tree();
    assert!(
        tree.node(wrappers[0])
            .unwrap()
            .descriptor
            .view()
            .classes
            .contains(&"kartis-outline-label".into())
    );
    assert!(
        tree.node(wrappers[1])
            .unwrap()
            .descriptor
            .view()
            .classes
            .contains(&"kartis-mark-clip".into())
    );
    let outlined = nodes(&app, "kartis-outline-label")[1];
    assert_eq!(
        tree.node(outlined).unwrap().descriptor.view().style.width,
        Some(Length::Px(300.))
    );
    drop(tree);
    hidden.revise(|h| {
        h.insert("a".into());
    });
    app.flush().unwrap();
    assert_eq!(
        app.tree()
            .node(outlined)
            .unwrap()
            .descriptor
            .view()
            .style
            .display,
        Some(Display::None)
    );
    assert_ne!(
        app.tree()
            .node(nodes(&app, "kartis-outline-label")[0])
            .unwrap()
            .descriptor
            .view()
            .style
            .display,
        Some(Display::None)
    );
    let mut s = scene();
    s.labels[0].text = "<safe O>".into();
    let html = glory::ssr::render(chart(s).outline_labels(fonts(), Default::default())).unwrap();
    assert!(html.contains("kartis-outline-glyph"));
    assert!(html.contains("&lt;safe O&gt;"));
    assert!(!html.contains("<safe O>"));
}
#[test]
fn reactive_rotation_and_text_topology_changes_are_live_and_rejected_shaping_retains_scene() {
    let app = App::memory();
    let mut s = scene();
    s.labels[0].rotation = 0.;
    let source = app.scope().cage(s);
    let rejected = Rc::new(Cell::new(0));
    let r = rejected.clone();
    app.mount(
        chart(source)
            .outline_labels(fonts(), Default::default())
            .on_reject(move |_| r.set(r.get() + 1)),
    )
    .unwrap();
    let wrapper = nodes(&app, "kartis-outline-label")[1];
    assert!(nodes(&app, "kartis-outline-glyph").is_empty());
    source.revise(|s| s.labels[0].rotation = 45.);
    app.flush().unwrap();
    assert!(!nodes(&app, "kartis-outline-glyph").is_empty());
    assert_eq!(nodes(&app, "kartis-outline-label")[1], wrapper);
    let first = nodes(&app, "kartis-outline-glyph")[0];
    let before = app
        .tree()
        .node(first)
        .unwrap()
        .descriptor
        .view()
        .props
        .clone();
    source.revise(|s| {
        s.labels[0].text = "缺字".into();
        s.marks[0].value = 999.;
    });
    app.flush().unwrap();
    assert_eq!(rejected.get(), 1);
    assert_eq!(
        app.tree().node(first).unwrap().descriptor.view().props,
        &before
    );
    let tree = app.tree();
    let Props::Button(data) = tree
        .node(nodes(&app, "kartis-datum")[0])
        .unwrap()
        .descriptor
        .view()
        .props
    else {
        panic!("datum")
    };
    assert!(!data.label.contains("999"));
    drop(tree);
    source.revise(|s| {
        s.labels[0].text = "O".into();
        s.labels[0].rotation = -30.;
    });
    app.flush().unwrap();
    assert_eq!(nodes(&app, "kartis-outline-glyph").len(), 1);
    source.revise(|s| s.labels[0].rotation = 0.);
    app.flush().unwrap();
    assert!(nodes(&app, "kartis-outline-glyph").is_empty());
    app.clear().unwrap();
    source.revise(|s| s.labels[0].rotation = 30.);
    app.flush().unwrap();
    assert_eq!(app.stats().nodes, 0);
    assert_eq!(app.stats().handlers, 0);
}
#[test]
fn resource_missing_glyph_and_coordinate_profiles_reject_boundedly_and_transparencies_need_no_font()
{
    let s = scene();
    assert!(
        outline_labels(
            &s,
            &fonts(),
            &OutlineLabelOptions {
                max_commands: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(outline_labels(&s, &FontRegistry::new(), &Default::default()).is_err());
    assert!(
        fonts()
            .outline_label(&label("אב"), MissingGlyphPolicy::Error)
            .is_err()
    );
    assert!(
        fonts()
            .outline_label(
                &Label {
                    rotation: f64::NAN,
                    ..label("O")
                },
                MissingGlyphPolicy::Error
            )
            .is_err()
    );
    assert!(
        fonts()
            .outline_label(
                &Label {
                    x: 1e6,
                    anchor: TextAnchor::Start,
                    ..label("O")
                },
                MissingGlyphPolicy::Error
            )
            .is_err()
    );
    let mut transparent = s.clone();
    transparent.labels[0].color.3 = 0;
    assert_eq!(
        outline_labels(&transparent, &FontRegistry::new(), &Default::default())
            .unwrap()
            .glyph_count,
        0
    );
    let app = App::memory();
    assert!(app.mount(chart(s.clone())).is_err());
    assert!(
        app.mount(chart(s).outline_labels(
            fonts(),
            OutlineLabelOptions {
                max_glyphs: 1,
                ..Default::default()
            }
        ))
        .is_err()
    );
    assert_eq!(app.stats().nodes, 0);
}
#[test]
fn semantic_gradient_fill_admission_is_explicit_but_external_plot_is_allowed() {
    let mut scene = scene();
    scene.labels.clear();
    scene.label_series.clear();
    scene.draw_order.clear();
    scene.marks[0].fill_paint = Some(FillPaint::Linear {
        start: [0., 0.],
        end: [100., 0.],
        stops: vec![
            GradientStop::new(0., Color(0, 0, 0, 255)),
            GradientStop::new(1., Color(255, 255, 255, 255)),
        ],
    });
    let app = App::memory();
    assert!(app.mount(chart(scene.clone())).is_err());
    app.mount(chart(scene).plot(false)).unwrap();
}
