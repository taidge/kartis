use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::rc::Rc;

use glory::App;
use glory::render::{NodeId, Props};
use glory::style::{Display, Length, Overflow, PointerEvents, Style};
use kartis::*;

fn scene() -> Scene {
    let mut scene = ChartConfig {
        width: 480.,
        height: 360.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    scene.labels.push(Label {
        x: 100.,
        y: 140.,
        size: 100.,
        text: "B".into(),
        rotation: 0.,
        color: Color(30, 90, 150, 128),
        anchor: TextAnchor::Start,
    });
    scene.typography.default.family = Some(vec![FontFamily::Named("Lato".into())]);
    scene.typography.default.weight = Some(700.);
    scene
}
fn rect(x: f64, y: f64, width: f64, height: f64) -> ClipPath {
    ClipPath {
        geometry: Geometry::Rect {
            x,
            y,
            width,
            height,
        },
        rule: ClipRule::NonZero,
    }
}
fn measured(request: &SemanticTextRequest) -> Result<SemanticTextMeasurement> {
    request.measured(SemanticTextMetrics {
        advance: 60.,
        line_height: 124.984375,
        baseline_from_top: 73.25,
        ink_bounds: Some(LayoutRect {
            x: -8.,
            y: -70.,
            width: 50.,
            height: 90.,
        }),
    })
}
fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut stack = tree.roots().iter().rev().copied().collect::<Vec<_>>();
    let mut out = vec![];
    while let Some(id) = stack.pop() {
        let n = tree.node(id).unwrap();
        if class.is_empty() || n.descriptor.view().classes.iter().any(|v| v == class) {
            out.push(id);
        }
        stack.extend(n.children.iter().rev().copied());
    }
    out
}
fn style(app: &App, id: NodeId) -> Style {
    app.tree().node(id).unwrap().descriptor.view().style.clone()
}
fn text(app: &App) -> Vec<String> {
    // Restrict to owned label subtrees. The independent tooltip live region
    // has its own empty Text; empty owned labels must still remain observable.
    let tree = app.tree();
    let mut out = Vec::new();
    for label in nodes(app, "kartis-label") {
        let mut stack = vec![label];
        while let Some(id) = stack.pop() {
            let node = tree.node(id).unwrap();
            if let Props::Text(text) = node.descriptor.view().props {
                out.push(text.content.clone());
            }
            stack.extend(node.children.iter().rev().copied());
        }
    }
    out
}
fn snapshot(app: &App) -> Vec<(NodeId, String)> {
    nodes(app, "")
        .into_iter()
        .map(|id| {
            (
                id,
                format!("{:?}", app.tree().node(id).unwrap().descriptor.view()),
            )
        })
        .collect()
}
fn clipped_style(app: &App, id: NodeId, expected: [f64; 4]) {
    let s = style(app, id);
    assert_eq!(
        [s.left, s.top, s.width, s.height],
        expected.map(|v| Some(Length::Px(v)))
    );
    assert_eq!(s.overflow_x, Some(Overflow::Hidden));
    assert_eq!(s.overflow_y, Some(Overflow::Hidden));
    assert_eq!(s.pointer_events, Some(PointerEvents::None));
    assert!(s.opacity.is_none());
}

#[test]
fn measured_three_anchors_keep_real_text_font_and_baseline_inside_actual_clip_boxes() {
    let mut frame = scene();
    frame.labels = [TextAnchor::Start, TextAnchor::Middle, TextAnchor::End]
        .into_iter()
        .map(|anchor| Label {
            anchor,
            ..frame.labels[0].clone()
        })
        .collect();
    for i in 0..3 {
        frame.label_clips.insert(i, rect(70., 60., 90., 50.));
    }
    frame.label_clips.get_mut(&1).unwrap().rule = ClipRule::EvenOdd;
    let labels = frame.labels.clone();
    let app = App::memory();
    app.mount(
        chart(frame)
            .legend(false)
            .data_controls(false)
            .semantic_text_measurer(
                SemanticTextProfile {
                    line_height: 1.25,
                    ..Default::default()
                },
                measured,
            ),
    )
    .unwrap();
    let clips = nodes(&app, "kartis-label-clip");
    let local = nodes(&app, "kartis-label");
    assert_eq!(clips.len(), 3);
    assert_eq!(local.len(), 3);
    assert_eq!(text(&app), ["B", "B", "B"]);
    for i in 0..3 {
        clipped_style(&app, clips[i], [70., 60., 90., 50.]);
        let s = style(&app, local[i]);
        assert_eq!(s.left, Some(Length::Px([30., 0., -30.][i])));
        assert_eq!(s.top, Some(Length::Px(6.75)));
        assert_eq!(s.width, Some(Length::Px(60.)));
        assert_eq!(s.height, Some(Length::Px(124.984375)));
        assert_eq!(s.pointer_events, Some(PointerEvents::None));
        let tree = app.tree();
        let child = tree.node(local[i]).unwrap().children[0];
        let child = tree.node(child).unwrap().descriptor.view();
        assert!(matches!(child.props, Props::Text(_)));
        assert_eq!(child.style.color, Some(labels[i].color));
        assert_eq!(
            child.style.font_family,
            Some(vec![FontFamily::Named("Lato".into())])
        );
        assert_eq!(child.style.font_weight, Some(700.));
        assert_eq!(child.style.font_size, Some(100.));
    }
    assert!(nodes(&app, "kartis-mark").is_empty());
}

#[test]
fn clip_map_only_updates_keep_label_nodes_and_measurement_then_hidden_owned_labels_restore() {
    let mut frame = scene();
    frame.label_series.insert(0, "alpha".into());
    frame.label_clips.insert(0, rect(70., 60., 90., 50.));
    let app = App::memory();
    let source = app.scope().cage(frame);
    let hidden = app.scope().cage(BTreeSet::<String>::new());
    let count = Rc::new(Cell::new(0));
    let calls = count.clone();
    app.mount(
        chart(source)
            .hidden(hidden)
            .legend(false)
            .data_controls(false)
            .semantic_text_measurer(
                SemanticTextProfile {
                    line_height: 1.25,
                    ..Default::default()
                },
                move |request| {
                    calls.set(calls.get() + 1);
                    measured(request)
                },
            ),
    )
    .unwrap();
    let clip = nodes(&app, "kartis-label-clip")[0];
    let label = nodes(&app, "kartis-label")[0];
    let originals = source.get_untracked().labels;
    source.revise(|s| {
        s.label_clips.insert(0, rect(80., 50., 70., 40.));
    });
    app.flush().unwrap();
    assert_eq!(source.get_untracked().labels, originals);
    assert_eq!(nodes(&app, "kartis-label-clip"), [clip]);
    assert_eq!(nodes(&app, "kartis-label"), [label]);
    assert_eq!(count.get(), 1);
    clipped_style(&app, clip, [80., 50., 70., 40.]);
    assert_eq!(style(&app, label).left, Some(Length::Px(20.)));
    assert_eq!(style(&app, label).top, Some(Length::Px(16.75)));
    hidden.revise(|h| {
        h.insert("alpha".into());
    });
    app.flush().unwrap();
    assert_eq!(style(&app, clip).display, Some(Display::None));
    hidden.revise(|h| h.clear());
    app.flush().unwrap();
    assert_ne!(style(&app, clip).display, Some(Display::None));
    assert_eq!(text(&app), ["B"]);
    assert_eq!(source.get_untracked().labels, originals);
    // Ownership alone changes; a hidden unrelated row must not hide this label.
    hidden.revise(|h| {
        h.insert("beta".into());
    });
    source.revise(|s| {
        s.label_series.insert(0, "beta".into());
    });
    app.flush().unwrap();
    assert_eq!(style(&app, clip).display, Some(Display::None));
    source.revise(|s| s.label_series.clear());
    app.flush().unwrap();
    assert_ne!(style(&app, clip).display, Some(Display::None));
}

#[test]
fn adding_and_removing_only_the_clip_map_rebuilds_the_wrapper_without_changing_plain_text_style() {
    let app = App::memory();
    let source = app.scope().cage(scene());
    app.mount(chart(source).legend(false).data_controls(false))
        .unwrap();
    assert!(nodes(&app, "kartis-label-clip").is_empty());
    let plain = style(&app, nodes(&app, "kartis-label")[0]);
    let plain_text = text(&app);
    source.revise(|s| {
        s.label_clips.insert(0, rect(7., 11., 30., 40.));
    });
    app.flush().unwrap();
    assert_eq!(nodes(&app, "kartis-label-clip").len(), 1);
    assert_eq!(text(&app), plain_text);
    let local = style(&app, nodes(&app, "kartis-label")[0]);
    assert_eq!(local.left, Some(Length::Px(93.)));
    assert_eq!(local.top, Some(Length::Px(49.)));
    source.revise(|s| s.label_clips.clear());
    app.flush().unwrap();
    assert!(nodes(&app, "kartis-label-clip").is_empty());
    assert_eq!(style(&app, nodes(&app, "kartis-label")[0]), plain);
    assert_eq!(text(&app), plain_text);
    let html = glory::ssr::render(chart(scene()).legend(false).data_controls(false)).unwrap();
    assert!(!html.contains("kartis-label-clip"));
    assert!(html.contains("kartis-label kartis-paint-item"));
}

#[test]
fn zero_and_negative_origin_rectangles_keep_source_text_and_exact_overflow_geometry() {
    for bounds in [
        [-20., -30., 0., 80.],
        [-20., -30., 80., 0.],
        [-20., -30., 80., 90.],
        [470., 350., 30., 40.],
    ] {
        let mut s = scene();
        s.label_clips
            .insert(0, rect(bounds[0], bounds[1], bounds[2], bounds[3]));
        let original = s.labels.clone();
        s.validate().unwrap();
        let app = App::memory();
        app.mount(chart(s.clone()).legend(false).data_controls(false))
            .unwrap();
        let clip = nodes(&app, "kartis-label-clip")[0];
        clipped_style(&app, clip, bounds);
        let label = style(&app, nodes(&app, "kartis-label")[0]);
        assert_eq!(label.left, Some(Length::Px(100. - bounds[0])));
        assert_eq!(label.top, Some(Length::Px(60. - bounds[1])));
        assert_eq!(text(&app), ["B"]);
        assert_eq!(s.labels, original);
        let html = glory::ssr::render(chart(s).legend(false).data_controls(false)).unwrap();
        assert!(html.contains("kartis-label-clip"));
        assert!(html.contains("overflow-x:hidden"));
        assert!(html.contains('B'));
    }
}

#[test]
fn nonrectangular_invalid_and_rotated_proposals_reject_before_any_accepted_frame_change() {
    let mut valid = scene();
    valid.label_clips.insert(0, rect(70., 60., 90., 50.));
    let app = App::memory();
    let source = app.scope().cage(valid.clone());
    let errors = Rc::new(RefCell::new(vec![]));
    let captured = errors.clone();
    app.mount(
        chart(source)
            .legend(false)
            .data_controls(false)
            .on_reject(move |e| captured.borrow_mut().push(e.0)),
    )
    .unwrap();
    let accepted = snapshot(&app);
    let circle = ClipPath {
        geometry: Geometry::Circle {
            x: 100.,
            y: 140.,
            radius: 30.,
        },
        rule: ClipRule::NonZero,
    };
    let path = ClipPath {
        geometry: Geometry::Path(vec![
            glory::render::PathCommand::Move(glory::render::GraphicPoint::new(0., 0.)),
            glory::render::PathCommand::Line(glory::render::GraphicPoint::new(200., 0.)),
            glory::render::PathCommand::Line(glory::render::GraphicPoint::new(0., 200.)),
            glory::render::PathCommand::Close,
        ]),
        rule: ClipRule::EvenOdd,
    };
    for clip in [circle.clone(), path] {
        source.revise(|s| {
            s.labels[0].text = "must not publish".into();
            s.label_clips.insert(0, clip.clone());
        });
        app.flush().unwrap();
        assert_eq!(snapshot(&app), accepted);
        let mut initial = valid.clone();
        initial.label_clips.insert(0, clip);
        assert!(glory::ssr::render(chart(initial)).is_err());
    }
    source.revise(|s| {
        *s = valid.clone();
        s.labels[0].rotation = 30.;
    });
    app.flush().unwrap();
    assert_eq!(snapshot(&app), accepted);
    source.revise(|s| {
        *s = valid.clone();
        s.label_clips.insert(0, rect(70., 60., -1., 50.));
    });
    app.flush().unwrap();
    assert_eq!(snapshot(&app), accepted);
    assert_eq!(errors.borrow().len(), 4);
    source.revise(|s| {
        *s = valid;
        s.labels[0].text = "accepted after repair".into();
    });
    app.flush().unwrap();
    assert_eq!(text(&app), ["accepted after repair"]);
}

#[test]
fn public_bar_default_label_clipping_mounts_and_preserves_all_labels_and_rectangles() {
    let config = ChartConfig {
        width: 420.,
        height: 240.,
        ..Default::default()
    };
    let categories = ["A".into(), "B".into()];
    let data = [BarSeries::new("sales", vec![Some(4.), Some(8.)])];
    let mut style = BarStyleOptions::default();
    style.labels.enabled = true;
    let layout =
        bar_layout_with_style(&config, &categories, &data, &BarOptions::default(), &style).unwrap();
    assert_eq!(layout.scene.label_clips.len(), 2);
    assert!(style.labels.clip_to_plot);
    let scene = layout.scene;
    let texts = scene
        .labels
        .iter()
        .map(|l| l.text.clone())
        .collect::<Vec<_>>();
    let app = App::memory();
    app.mount(chart(scene.clone()).legend(false).data_controls(false))
        .unwrap();
    assert_eq!(nodes(&app, "kartis-label-clip").len(), 2);
    assert!(texts.iter().all(|s| text(&app).contains(s)));
    assert!(glory::ssr::render(chart(scene)).is_ok());
}

#[cfg(feature = "raster")]
#[test]
fn rotated_outlined_and_painted_glyphs_keep_original_commands_under_the_same_rect_clip() {
    use kartis::render::raster::FontRegistry;
    let mut fonts = FontRegistry::new();
    let face = fonts
        .register(include_bytes!("fonts/Lato-Regular.ttf").as_slice(), 0)
        .unwrap();
    fonts
        .bind_face(
            face,
            kartis::render::raster::FaceDescriptor {
                family: FontFamily::Named("Lato".into()),
                weight: 400.,
            },
        )
        .unwrap();
    for painted in [false, true] {
        let mut frame = scene();
        frame.labels[0].rotation = 30.;
        frame.labels[0].size = 30.;
        let app = App::memory();
        let source = app.scope().cage(frame);
        let chart = chart(source).legend(false).data_controls(false);
        let chart = if painted {
            chart.painted_labels(fonts.clone(), Default::default())
        } else {
            chart.outline_labels(fonts.clone(), Default::default())
        };
        app.mount(chart).unwrap();
        let glyphs = nodes(&app, "kartis-outline-glyph");
        assert!(!glyphs.is_empty());
        let drawings = glyphs
            .iter()
            .map(|id| {
                format!(
                    "{:?}",
                    app.tree().node(*id).unwrap().descriptor.view().props
                )
            })
            .collect::<Vec<_>>();
        source.revise(|s| {
            s.label_clips.insert(0, rect(70., 60., 90., 50.));
        });
        app.flush().unwrap();
        clipped_style(
            &app,
            nodes(&app, "kartis-label-clip")[0],
            [70., 60., 90., 50.],
        );
        let label = style(&app, nodes(&app, "kartis-label")[0]);
        assert_eq!(label.left, Some(Length::Px(-70.)));
        assert_eq!(label.top, Some(Length::Px(-60.)));
        assert_eq!(text(&app), ["B"]);
        let after = nodes(&app, "kartis-outline-glyph")
            .iter()
            .map(|id| {
                format!(
                    "{:?}",
                    app.tree().node(*id).unwrap().descriptor.view().props
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(after, drawings);
        for glyph in nodes(&app, "kartis-outline-glyph") {
            assert_eq!(style(&app, glyph).left, Some(Length::Px(0.)));
            assert_eq!(style(&app, glyph).top, Some(Length::Px(0.)));
        }
    }
}
