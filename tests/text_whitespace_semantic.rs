use std::cell::RefCell;
use std::rc::Rc;

use glory::App;
use glory::render::{NodeId, Props};
use glory::style::WhiteSpace;
use kartis::*;

fn scene(text: &str, mode: LabelWhitespace) -> Scene {
    let mut scene = ChartConfig::default().scene().unwrap();
    scene.labels.push(Label {
        rotation: 0.,
        x: 40.,
        y: 60.,
        text: text.into(),
        size: 20.,
        color: Color(10, 20, 30, 255),
        anchor: TextAnchor::Start,
    });
    if mode != LabelWhitespace::Legacy {
        scene.typography.label_whitespace.insert(0, mode);
    }
    scene
}

fn text_nodes(app: &App) -> Vec<NodeId> {
    let tree = app.tree();
    let mut stack = tree.roots().to_vec();
    let mut output = Vec::new();
    while let Some(id) = stack.pop() {
        let node = tree.node(id).unwrap();
        if matches!(node.descriptor.view().props, Props::Text(_)) {
            output.push(id);
        }
        stack.extend(node.children.iter().copied());
    }
    output
}

fn measured(request: &SemanticTextRequest) -> Result<SemanticTextMeasurement> {
    request.measured(SemanticTextMetrics {
        advance: 100.,
        line_height: 24.,
        baseline_from_top: 18.,
        ink_bounds: Some(LayoutRect {
            x: 0.,
            y: -15.,
            width: 90.,
            height: 18.,
        }),
    })
}

fn assert_text(app: &App, expected: &str, whitespace: WhiteSpace) {
    let ids = text_nodes(app);
    let id = ids
        .into_iter()
        .find(|id| {
            let tree = app.tree();
            let node = tree.node(*id).unwrap();
            matches!(node.descriptor.view().props, Props::Text(value) if value.content == expected)
        })
        .expect("ordinary Text, not a glyph or image");
    assert_eq!(
        app.tree()
            .node(id)
            .unwrap()
            .descriptor
            .view()
            .style
            .white_space,
        Some(whitespace)
    );
}

#[test]
fn plain_and_measured_preserve_are_real_normalized_text() {
    for measured_mode in [false, true] {
        let app = App::memory();
        let widget = chart(scene("  A\t B\r\n  ", LabelWhitespace::Preserve));
        if measured_mode {
            app.mount(
                widget.semantic_text_measurer(SemanticTextProfile::default(), |request| {
                    assert_eq!(request.whitespace, LabelWhitespace::Preserve);
                    assert_eq!(request.text, "  A\t B\r\n  ");
                    measured(request)
                }),
            )
            .unwrap();
        } else {
            app.mount(widget).unwrap();
        }
        assert_text(&app, "  A  B    ", WhiteSpace::Pre);
    }
}

#[test]
fn mode_only_changes_remeasure_and_switch_plain_text_style_back() {
    let app = App::memory();
    let source = app.scope().cage(scene(" A  B ", LabelWhitespace::Legacy));
    let requests = Rc::new(RefCell::new(Vec::new()));
    let seen = requests.clone();
    app.mount(
        chart(source).semantic_text_measurer(SemanticTextProfile::default(), move |r| {
            seen.borrow_mut().push(r.clone());
            measured(r)
        }),
    )
    .unwrap();
    assert_text(&app, " A  B ", WhiteSpace::NoWrap);
    source.revise(|s| {
        s.typography
            .label_whitespace
            .insert(0, LabelWhitespace::Preserve);
    });
    app.flush().unwrap();
    assert_eq!(requests.borrow().len(), 2);
    assert_text(&app, " A  B ", WhiteSpace::Pre);
    source.revise(|s| {
        s.typography.label_whitespace.clear();
    });
    app.flush().unwrap();
    assert_eq!(requests.borrow().len(), 3);
    assert_text(&app, " A  B ", WhiteSpace::NoWrap);
}

#[test]
fn stale_mode_measurement_rejects_and_keeps_accepted_props() {
    let app = App::memory();
    let source = app.scope().cage(scene(" A  B ", LabelWhitespace::Legacy));
    let old = Rc::new(RefCell::new(None::<SemanticTextMeasurement>));
    let saved = old.clone();
    let errors = Rc::new(RefCell::new(Vec::new()));
    let rejected = errors.clone();
    app.mount(
        chart(source)
            .semantic_text_measurer(SemanticTextProfile::default(), move |r| {
                if let Some(value) = saved.borrow().as_ref() {
                    return Ok(value.clone());
                }
                let value = measured(r)?;
                *saved.borrow_mut() = Some(value.clone());
                Ok(value)
            })
            .on_reject(move |e| rejected.borrow_mut().push(e)),
    )
    .unwrap();
    source.revise(|s| {
        s.typography
            .label_whitespace
            .insert(0, LabelWhitespace::Preserve);
    });
    app.flush().unwrap();
    assert!(!errors.borrow().is_empty());
    assert_text(&app, " A  B ", WhiteSpace::NoWrap);
}

#[test]
fn request_mode_is_independent_of_text_and_revision() {
    let label = scene(" A  B ", LabelWhitespace::Legacy).labels.remove(0);
    let a = SemanticTextProfile::default()
        .request(&label, &ResolvedFont::default())
        .unwrap();
    let b = SemanticTextRequest::for_label_with_whitespace(
        &label,
        &ResolvedFont::default(),
        &SemanticTextProfile::default(),
        LabelWhitespace::Preserve,
    )
    .unwrap();
    assert_eq!(a.text, b.text);
    assert_eq!(a.revision, b.revision);
    assert_ne!(a, b);
    assert!(measured(&a).unwrap().validate_for(&b).is_err());
}

#[cfg(feature = "raster")]
#[test]
fn outlined_configuration_preserves_the_ordinary_text_fallback() {
    let app = App::memory();
    let mut fonts = render::raster::FontRegistry::new();
    fonts
        .register(include_bytes!("fonts/ABeeZee-Regular.ttf").as_slice(), 0)
        .unwrap();
    app.mount(
        chart(scene("  A\tB  ", LabelWhitespace::Preserve))
            .outline_labels(fonts, OutlineLabelOptions::default()),
    )
    .unwrap();
    assert_text(&app, "  A B  ", WhiteSpace::Pre);
}

#[test]
fn collision_without_whitespace_aware_metrics_is_rejected() {
    let app = App::memory();
    assert!(
        app.mount(
            chart(scene(" A  B ", LabelWhitespace::Preserve))
                .label_collision(LabelCollisionOptions::default())
        )
        .is_err()
    );
}
