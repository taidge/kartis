use glory::{
    App,
    render::{NodeId, Props},
    style::Style,
};
use kartis::{motion::*, *};
use std::{cell::RefCell, rc::Rc};

fn font(name: &str, weight: f64) -> FontOptions {
    FontOptions {
        family: Some(vec![FontFamily::Named(name.into())]),
        weight: Some(weight),
    }
}
fn scene() -> Scene {
    let mut scene = ChartConfig::default().scene().unwrap();
    scene.typography.default = font("Chart face", 400.);
    scene.labels.push(Label {
        x: 120.,
        y: 100.,
        text: "AB".into(),
        color: Color(20, 40, 60, 255),
        size: 20.,
        rotation: 0.,
        anchor: TextAnchor::Start,
    });
    scene.legend.push(LegendItem {
        id: "alpha".into(),
        label: "Alpha".into(),
        color: Color(20, 40, 60, 255),
    });
    scene
}
fn nodes(app: &App) -> Vec<NodeId> {
    let tree = app.tree();
    let mut stack = tree.roots().to_vec();
    let mut out = vec![];
    while let Some(id) = stack.pop() {
        stack.extend(tree.node(id).unwrap().children.iter().copied());
        out.push(id);
    }
    out
}
fn text_node(app: &App, text: &str) -> NodeId {
    nodes(app).into_iter().find(|id| {
        let tree = app.tree();
        matches!(tree.node(*id).unwrap().descriptor.view().props, Props::Text(value) if value.content == text)
    }).unwrap()
}
fn class_node(app: &App, class: &str) -> NodeId {
    nodes(app)
        .into_iter()
        .find(|id| {
            app.tree()
                .node(*id)
                .unwrap()
                .descriptor
                .view()
                .classes
                .iter()
                .any(|value| value == class)
        })
        .unwrap()
}
fn style(app: &App, node: NodeId) -> Style {
    app.tree()
        .node(node)
        .unwrap()
        .descriptor
        .view()
        .style
        .clone()
}

#[test]
fn semantic_font_only_updates_keep_nodes_and_resolve_role_overrides_and_default_reset() {
    let app = App::memory();
    let source = app.scope().cage(scene());
    let errors = Rc::new(RefCell::new(vec![]));
    let rejects = errors.clone();
    app.mount(
        chart(source)
            .legend_options(LegendOptions {
                text: PresentationText {
                    font: FontOptions {
                        weight: Some(600.),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            })
            .on_reject(move |error| rejects.borrow_mut().push(error)),
    )
    .unwrap();
    let text = text_node(&app, "AB");
    let legend = class_node(&app, "kartis-legend-item");
    let tooltip = class_node(&app, "kartis-tooltip");
    assert_eq!(style(&app, text).font_weight, Some(400.));
    assert_eq!(style(&app, legend).font_weight, Some(600.));
    source.revise(|scene| {
        scene.typography.default = font("Next face", 700.);
        scene.typography.tooltip = FontOptions {
            family: Some(vec![FontFamily::Mono]),
            weight: None,
        };
    });
    app.flush().unwrap();
    assert_eq!(text_node(&app, "AB"), text);
    assert_eq!(class_node(&app, "kartis-legend-item"), legend);
    assert_eq!(
        style(&app, text).font_family,
        Some(vec![FontFamily::Named("Next face".into())])
    );
    assert_eq!(style(&app, text).font_weight, Some(700.));
    assert_eq!(style(&app, legend).font_weight, Some(600.));
    assert_eq!(
        style(&app, legend).font_family,
        style(&app, text).font_family
    );
    assert_eq!(
        style(&app, tooltip).font_family,
        Some(vec![FontFamily::Mono])
    );
    assert_eq!(style(&app, tooltip).font_weight, Some(700.));
    let accepted = style(&app, text);
    source.revise(|scene| scene.typography.default.weight = Some(f64::NAN));
    app.flush().unwrap();
    assert!(!errors.borrow().is_empty());
    assert_eq!(style(&app, text), accepted);
    source.revise(|scene| scene.typography = SceneTypography::default());
    app.flush().unwrap();
    assert_eq!(style(&app, text).font_family, Some(vec![FontFamily::Sans]));
    assert_eq!(style(&app, text).font_weight, Some(400.));
    assert_eq!(text_node(&app, "AB"), text);
    app.clear().unwrap();
    assert_eq!(app.stats().nodes, 0);
    assert_eq!(app.stats().handlers, 0);
}

#[test]
fn actual_measurement_receives_resolved_role_font_and_rejection_keeps_the_whole_scene() {
    let app = App::memory();
    let source = app.scope().cage(scene());
    let calls = Rc::new(RefCell::new(vec![]));
    let measured = calls.clone();
    let errors = Rc::new(RefCell::new(vec![]));
    let rejects = errors.clone();
    app.mount(
        chart(source)
            .font_text_measurer(move |text, style, font| {
                assert_eq!(style.font.resolve(&ResolvedFont::default()), *font);
                measured.borrow_mut().push((text.to_owned(), font.clone()));
                if font.weight == 900. {
                    return Err(ChartError("face unavailable".into()));
                }
                Ok(TextSize {
                    width: if font.weight == 700. { 120. } else { 60. },
                    height: 20.,
                })
            })
            .legend_layout(LegendLayoutOptions {
                bounds: LayoutRect {
                    x: 0.,
                    y: 0.,
                    width: 500.,
                    height: 100.,
                },
                ..Default::default()
            })
            .on_reject(move |error| rejects.borrow_mut().push(error)),
    )
    .unwrap();
    assert!(calls.borrow().iter().any(|(_, font)| font.weight == 400.));
    let text = text_node(&app, "AB");
    let legend = class_node(&app, "kartis-legend-item");
    let before = style(&app, legend);
    source.revise(|scene| {
        scene.width = 900.;
        scene.typography.default.weight = Some(900.);
    });
    app.flush().unwrap();
    assert!(!errors.borrow().is_empty());
    assert_eq!(style(&app, legend), before);
    assert_eq!(style(&app, text).font_weight, Some(400.));
    source.revise(|scene| scene.typography.default.weight = Some(700.));
    app.flush().unwrap();
    assert_eq!(text_node(&app, "AB"), text);
    assert_eq!(style(&app, text).font_weight, Some(700.));
    assert_ne!(style(&app, legend).width, before.width);
    assert!(calls.borrow().iter().any(|(_, font)| font.weight == 700.
        && font.family == vec![FontFamily::Named("Chart face".into())]));
}

#[test]
fn crossfade_and_property_motion_keep_both_faces_and_discard_departed_font_slots() {
    let initial = scene();
    let mut target = initial.clone();
    target.typography.default.weight = Some(700.);
    let mut motion = SpringMotion::new(initial.clone(), SpringConfig::default()).unwrap();
    motion.retarget(target.clone()).unwrap();
    let frame = motion.tick(0.04).unwrap();
    assert_eq!(frame.labels.len(), 2);
    assert_eq!(frame.resolved_label_font(0).weight, 400.);
    assert_eq!(frame.resolved_label_font(1).weight, 700.);
    frame.validate().unwrap();
    let mut motion = PropertyMotion::new(initial, PropertyMotionOptions::default()).unwrap();
    motion.retarget(target.clone()).unwrap();
    let frame = motion.tick(0.04).unwrap();
    let mut weights = (0..frame.labels.len())
        .map(|index| frame.resolved_label_font(index).weight as u16)
        .collect::<Vec<_>>();
    weights.sort_unstable();
    assert_eq!(weights, [400, 700]);
    for _ in 0..400 {
        motion.tick(0.016).unwrap();
    }
    assert_eq!(motion.scene().labels.len(), 1);
    assert_eq!(motion.scene().resolved_label_font(0).weight, 700.);
    assert_eq!(motion.scene(), &target);
}

#[test]
fn prepended_annotation_remaps_label_fonts_and_keeps_its_own_inherited_font() {
    use kartis::annotations::*;
    let mut scene = scene();
    scene.typography.labels.insert(
        0,
        FontOptions {
            weight: Some(700.),
            ..Default::default()
        },
    );
    let note = Label {
        text: "note".into(),
        ..scene.labels[0].clone()
    };
    annotate(
        &mut scene,
        &[Annotation::new("note", AnnotationKind::Text(note))],
        AnnotationLayer::Back,
    )
    .unwrap();
    assert_eq!(scene.labels[0].text, "note");
    assert_eq!(scene.resolved_label_font(0).weight, 400.);
    assert_eq!(scene.resolved_label_font(1).weight, 700.);
    scene.validate().unwrap();
}
