use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use glory::App;
use glory::render::{Event, NodeId, Props};
use kartis::surface::{SurfaceModel, SurfaceOptions};
use kartis::*;

fn scene() -> Scene {
    let mut scene = ChartConfig::default().scene().unwrap();
    let mut mark = Mark::new(
        "a",
        Geometry::Rect {
            x: 20.,
            y: 20.,
            width: 40.,
            height: 40.,
        },
        Color(30, 90, 150, 255),
    );
    mark.label = "Revenue".into();
    mark.series = "series".into();
    scene.marks.push(mark);
    scene
}
fn button(app: &App) -> (NodeId, String) {
    let tree = app.tree();
    let mut stack = tree.roots().to_vec();
    while let Some(id) = stack.pop() {
        let node = tree.node(id).unwrap();
        if node
            .descriptor
            .view()
            .classes
            .iter()
            .any(|class| class == "kartis-datum")
        {
            let Props::Button(props) = node.descriptor.view().props else {
                panic!("expected button")
            };
            return (id, props.label.clone());
        }
        stack.extend(node.children.iter().copied());
    }
    panic!("missing datum button")
}

#[test]
fn labels_follow_stable_ids_and_reject_atomically_with_the_scene() {
    let app = App::memory();
    let source = app.scope().cage(scene());
    let labels = app
        .scope()
        .cage(BTreeMap::from([("a".into(), "Revenue: missing".into())]));
    let selections = Rc::new(RefCell::new(Vec::new()));
    let selected = selections.clone();
    let errors = Rc::new(RefCell::new(Vec::new()));
    let rejected = errors.clone();
    app.mount(
        chart(source)
            .data_control_labels(labels)
            .on_select(move |mark| selected.borrow_mut().push(mark.value))
            .on_reject(move |error| rejected.borrow_mut().push(error)),
    )
    .unwrap();
    let (id, label) = button(&app);
    assert_eq!(label, "Revenue: missing");
    source.revise(|scene| scene.marks[0].value = 42.);
    labels.revise(|labels| {
        labels.insert("a".into(), "x".repeat(2049));
    });
    app.flush().unwrap();
    assert_eq!(button(&app), (id, "Revenue: missing".into()));
    app.dispatch(id, Event::Activate).unwrap();
    assert_eq!(*selections.borrow(), [0.]);
    assert!(!errors.borrow().is_empty());
    labels.revise(|labels| {
        labels.insert("a".into(), "Revenue: pending".into());
    });
    app.flush().unwrap();
    assert_eq!(button(&app), (id, "Revenue: pending".into()));
    app.dispatch(id, Event::Activate).unwrap();
    assert_eq!(*selections.borrow(), [0., 42.]);
    labels.revise(BTreeMap::clear);
    app.flush().unwrap();
    assert_eq!(button(&app), (id, "Revenue: 42".into()));
}

#[test]
fn label_inputs_obey_runtime_ownership() {
    let app = App::memory();
    let other = App::memory();
    let labels = other.scope().cage(BTreeMap::new());
    assert!(
        app.mount(chart(scene()).data_control_labels(labels))
            .is_err()
    );
}

#[test]
fn nullable_bound_controls_distinguish_missing_from_actual_zero() {
    let mut scene = scene();
    let mut second = scene.marks[0].clone();
    second.id = "b".into();
    second.label = "Zero".into();
    second.geometry = Geometry::Rect {
        x: 80.,
        y: 20.,
        width: 40.,
        height: 40.,
    };
    scene.marks.push(second);
    let binding = InteractionBinding {
        sites: vec![
            InteractionSite {
                mark_id: "a".into(),
                series: "series".into(),
                label: "Revenue".into(),
                value: None,
                formatted_value: None,
                color: Color(30, 90, 150, 255),
                position: [40., 40.],
            },
            InteractionSite {
                mark_id: "b".into(),
                series: "series".into(),
                label: "Zero".into(),
                value: Some(0.),
                formatted_value: None,
                color: Color(30, 90, 150, 255),
                position: [100., 40.],
            },
        ],
        ..Default::default()
    };
    let model = SurfaceModel::new_with_interaction(scene, SurfaceOptions::default(), Some(binding))
        .unwrap();
    assert_eq!(
        model.data_control_labels(),
        BTreeMap::from([("a".into(), "Revenue: missing".into())])
    );
}
