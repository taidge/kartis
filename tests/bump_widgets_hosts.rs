//! Actual Memory children and CPU publication, independently of React lifecycle.
use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::rc::Rc;

use glory::render::{Event, EventKind, NodeId, Props};
use glory::{App, Widget};
use kartis::*;
#[path = "../examples/shared/bump_widgets.rs"]
mod model;
fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut todo = tree.roots().to_vec();
    let mut result = vec![];
    while let Some(id) = todo.pop() {
        let node = tree.node(id).unwrap();
        if node.descriptor.view().classes.iter().any(|c| c == class) {
            result.push(id);
        }
        todo.extend(node.children.iter().copied());
    }
    result
}
fn tree(app: &App) -> String {
    let t = app.tree();
    let mut todo = t.roots().to_vec();
    let mut out = vec![];
    while let Some(id) = todo.pop() {
        let n = t.node(id).unwrap();
        out.push(format!("{id:?}:{n:?}"));
        todo.extend(n.children.iter().copied());
    }
    format!("{:?}:{out:?}", t.roots())
}
fn label(app: &App, id: NodeId) -> String {
    let t = app.tree();
    let Props::Button(p) = t.node(id).unwrap().descriptor.view().props else {
        panic!("Button expected")
    };
    p.label.clone()
}
fn child(
    ctx: PlotWidgetCompositionContext<BumpPointWidgetData>,
    drops: Rc<Cell<usize>>,
) -> impl Widget {
    let frame = ctx.frame;
    let class =
        frame.with(|f| format!("node-{}-{}", f.data.series_index, f.data.point.source_index));
    glory::ui::widget(move |scope| {
        let cleaned = drops.clone();
        scope.on_cleanup(move || cleaned.set(cleaned.get() + 1))?;
        let count = scope.cage(0u32);
        let focus = scope.cage(false);
        glory::ui::button()
            .class(class.clone())
            .label(scope.bond(move || {
                frame.with(|f| {
                    format!(
                        "{:?}|{:?}|original={}|x={}|count={}|focus={}",
                        f.data.point.raw_x,
                        f.data.point.rank,
                        f.data.series.data.len(),
                        f.frame.rect.x,
                        count.get(),
                        focus.get()
                    )
                })
            }))
            .on(EventKind::Activate, move |_| count.revise(|v| *v += 1))
            .on(EventKind::FocusChanged, move |e| {
                if let Event::FocusChanged(v) = e {
                    focus.revise(|f| *f = v)
                }
            })
            .show_in(scope)
    })
}

#[test]
fn actual_children_keep_state_and_owned_source_across_role_insertion_and_rejection() {
    let app = App::memory();
    let input = app.scope().cage(model::source(0, 600., 0).unwrap());
    let frame = app.scope().bond(move || input.with(|s| s.frame.clone()));
    let widgets = app
        .scope()
        .bond(move || input.with(|s| s.composition.clone()));
    let hidden = app.scope().cage(BTreeSet::new());
    let drops = Rc::new(Cell::new(0));
    let clean = drops.clone();
    let errors = Rc::new(RefCell::new(vec![]));
    let rejected = errors.clone();
    app.mount(
        chart_layout(frame)
            .legend(false)
            .tooltip(false)
            .highlight(false)
            .hidden(hidden)
            .on_reject(move |e| rejected.borrow_mut().push(e.0))
            .plot_widgets_composed(widgets, move |ctx| Ok(child(ctx, clean.clone()))),
    )
    .unwrap();
    assert!(nodes(&app, "node-0-2").is_empty());
    let id = nodes(&app, "node-0-0")[0];
    app.dispatch(id, Event::Activate).unwrap();
    app.dispatch(id, Event::FocusChanged(true)).unwrap();
    input.revise(|s| *s = model::source(1, 640., 0).unwrap());
    app.flush().unwrap();
    assert_eq!(nodes(&app, "node-0-0"), [id]);
    assert!(label(&app, id).contains("Some(3.5)"));
    assert!(label(&app, id).ends_with("count=1|focus=true"));
    assert_eq!(nodes(&app, "node-0-2").len(), 1);
    let zero = nodes(&app, "node-0-1")[0];
    assert!(label(&app, zero).contains("String(\"1\")|Some(-0.0)"));
    assert!(label(&app, id).contains("Number(1.0)|Some(3.5)"));
    hidden.revise(|h| {
        h.insert("north".into());
    });
    app.flush().unwrap();
    assert_eq!(nodes(&app, "node-0-0"), [id]);
    assert!(app.tree().inert(id));
    hidden.revise(BTreeSet::clear);
    app.flush().unwrap();
    assert!(!app.tree().inert(id));
    let accepted = input.get();
    for invalid in 0..3 {
        let before = tree(&app);
        let mut proposed = model::source(0, 600., 0).unwrap();
        match invalid {
            0 => {
                proposed.composition.widgets.data.pop_first();
            }
            1 => proposed.frame.scene.width = f64::NAN,
            _ => {
                proposed.composition.widgets.layout.slots[0].placement =
                    PlotWidgetPlacement::Rect(LayoutRect {
                        x: f64::NAN,
                        y: 0.,
                        width: 10.,
                        height: 10.,
                    })
            }
        }
        input.revise(|s| *s = proposed);
        app.flush().unwrap();
        assert_eq!(tree(&app), before);
        assert_eq!(nodes(&app, "node-0-0"), [id]);
        assert!(label(&app, id).contains("Some(3.5)"));
    }
    assert_eq!(errors.borrow().len(), 3);
    input.revise(|s| *s = accepted);
    app.flush().unwrap();
    input.revise(|s| *s = model::source(3, 640., 0).unwrap());
    app.flush().unwrap();
    assert_eq!(nodes(&app, "node-0-0"), [id]);
    app.clear().unwrap();
    assert_eq!(app.stats().nodes, 0);
    assert_eq!(drops.get(), 8);
}

#[cfg(feature = "raster")]
#[test]
fn png_publication_rejection_preserves_binding_children_original_source_and_camera() {
    use kartis::render::raster::{FontRegistry, RasterOptions};
    for dpr in [1., 2.] {
        let app = App::memory();
        let input = app.scope().cage((
            model::source(0, 600., 0).unwrap(),
            RasterOptions {
                pixel_ratio: dpr,
                ..Default::default()
            },
        ));
        let frame = app.scope().bond(move || input.with(|s| s.0.frame.clone()));
        let widgets = app
            .scope()
            .bond(move || input.with(|s| s.0.composition.clone()));
        let options = app.scope().bond(move || input.with(|s| s.1.clone()));
        let fail = Rc::new(Cell::new(false));
        let refused = fail.clone();
        let errors = Rc::new(RefCell::new(vec![]));
        let rejected = errors.clone();
        let drops = Rc::new(Cell::new(0));
        let clean = drops.clone();
        app.mount(
            chart_layout(frame)
                .raster(FontRegistry::new(), options)
                .legend(false)
                .tooltip(false)
                .highlight(false)
                .on_reject(move |e| rejected.borrow_mut().push(e.0))
                .raster_image_source(move |image| {
                    if refused.get() {
                        Err(ChartError("Bump PNG refused".into()))
                    } else {
                        image.data_url()
                    }
                })
                .plot_widgets_composed(widgets, move |ctx| Ok(child(ctx, clean.clone()))),
        )
        .unwrap();
        let image = |app: &App| {
            let t = app.tree();
            let mut todo = t.roots().to_vec();
            while let Some(id) = todo.pop() {
                let n = t.node(id).unwrap();
                if let Props::Image(p) = n.descriptor.view().props {
                    return p.source.clone();
                }
                todo.extend(n.children.iter().copied());
            }
            panic!("Image expected")
        };
        let id = nodes(&app, "node-0-0")[0];
        app.dispatch(id, Event::Activate).unwrap();
        app.dispatch(id, Event::FocusChanged(true)).unwrap();
        let before = tree(&app);
        let png = image(&app);
        fail.set(true);
        input.revise(|s| {
            s.0 = model::source(1, 640., 0).unwrap();
            s.1.view = ZoomTransform::new(1.25, -10., -5.).unwrap();
        });
        app.flush().unwrap();
        assert_eq!(tree(&app), before);
        assert_eq!(image(&app), png);
        assert!(nodes(&app, "node-0-2").is_empty());
        assert!(label(&app, id).contains("Some(2.0)"));
        assert_eq!(errors.borrow().len(), 1);
        fail.set(false);
        input.revise(|s| s.0 = model::source(1, 640., 0).unwrap());
        app.flush().unwrap();
        assert_eq!(nodes(&app, "node-0-0"), [id]);
        assert!(label(&app, id).contains("Some(3.5)"));
        assert!(label(&app, id).ends_with("count=1|focus=true"));
        assert_eq!(nodes(&app, "node-0-2").len(), 1);
        assert_ne!(image(&app), png);
        app.clear().unwrap();
        assert_eq!(app.stats().nodes, 0);
    }
}
