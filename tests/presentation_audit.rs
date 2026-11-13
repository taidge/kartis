use std::cell::Cell;
use std::collections::BTreeMap;
use std::rc::Rc;

use glory::App;
use glory::render::NodeId;
use glory::style::Display;
use kartis::*;

fn labels(app: &App) -> Vec<NodeId> {
    let tree = app.tree();
    let mut pending = tree.roots().iter().rev().copied().collect::<Vec<_>>();
    let mut found = Vec::new();
    while let Some(id) = pending.pop() {
        let node = tree.node(id).unwrap();
        if node
            .descriptor
            .view()
            .classes
            .iter()
            .any(|c| c == "kartis-label")
        {
            found.push(id);
        }
        pending.extend(node.children.iter().rev().copied());
    }
    found
}

#[test]
fn fully_transparent_labels_do_not_block_visible_labels_or_require_measurement() {
    let app = App::memory();
    let mut scene = ChartConfig {
        width: 240.,
        height: 160.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    for (text, alpha) in [("ghost", 0), ("visible", 255)] {
        scene.labels.push(Label {
            x: 100.,
            y: 100.,
            text: text.into(),
            size: 12.,
            color: Color(20, 30, 40, alpha),
            anchor: TextAnchor::Start,
            rotation: 0.,
        });
    }
    let source = app.scope().cage(scene);
    let measurements = Rc::new(Cell::new(0));
    let calls = measurements.clone();
    app.mount(
        chart(source)
            .text_measurer(move |_, _| {
                calls.set(calls.get() + 1);
                Ok(TextSize {
                    width: 40.,
                    height: 20.,
                })
            })
            .label_collision(LabelCollisionOptions {
                viewport: LayoutRect {
                    x: 0.,
                    y: 90.,
                    width: 240.,
                    height: 24.,
                },
                padding: 2.,
                ..Default::default()
            })
            .label_priorities(BTreeMap::from([(0, 10)])),
    )
    .unwrap();
    let ids = labels(&app);
    assert_eq!(ids.len(), 2);
    assert_ne!(
        app.tree()
            .node(ids[1])
            .unwrap()
            .descriptor
            .view()
            .style
            .display,
        Some(Display::None),
        "invisible text must not suppress a visible label"
    );
    assert_eq!(
        measurements.get(),
        1,
        "only visible text consumes measurement work"
    );
    source.revise(|scene| scene.labels[0].color.3 = 255);
    app.flush().unwrap();
    assert_eq!(
        app.tree()
            .node(ids[1])
            .unwrap()
            .descriptor
            .view()
            .style
            .display,
        Some(Display::None),
        "a visible higher-priority label must participate again"
    );
    source.revise(|scene| scene.labels[0].color.3 = 0);
    app.flush().unwrap();
    assert_ne!(
        app.tree()
            .node(ids[1])
            .unwrap()
            .descriptor
            .view()
            .style
            .display,
        Some(Display::None),
        "fading to transparent releases the collision footprint"
    );
    assert_eq!(measurements.get(), 4);
}
