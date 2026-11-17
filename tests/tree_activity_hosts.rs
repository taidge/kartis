//! Actual Memory activity proposals/retained children and CPU PNG rollback.
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use glory::render::{
    Event, EventKind, GraphicPoint as P, NodeId, PathCommand as C, PlotPointer, PlotPointerDevice,
    PlotPointerPhase, Props, Rect,
};
use glory::{App, Widget};
use kartis::*;

fn source(
    phase: u8,
    width: f64,
    revision: u64,
    paint_labels: bool,
) -> (
    SceneInteraction,
    PlotWidgetsComposition<TreeActivityWidgetData>,
) {
    let mut raw = vec![
        HierarchyDatum::new("a", Some("root".into()), if phase >= 3 { 4. } else { 2. }),
        HierarchyDatum::new("root", None, 0.),
        HierarchyDatum::new(
            "zero",
            Some("root".into()),
            if phase == 0 { -0. } else { 0. },
        ),
        HierarchyDatum::new("other", Some("root".into()), 5.),
    ];
    raw[2].label.clear();
    if phase == 2 {
        raw.rotate_left(1);
    }
    let config = ChartConfig {
        width,
        height: 260.,
        margin: Margin {
            left: 30.,
            right: 25.,
            top: 22.,
            bottom: 24.,
        },
        ..Default::default()
    };
    let mut options = TreeOptions::default();
    options.hierarchy.labels = paint_labels;
    let mut size = |c: &TreeNodeFunctionContext<'_>| Ok(if c.datum.id == "a" { 16. } else { 12. });
    let mut color = |c: &TreeNodeFunctionContext<'_>| {
        Ok(if c.datum.id == "other" {
            Color(180, 70, 90, 255)
        } else {
            Color(20, 90, 140, 255)
        })
    };
    let mut width = |_: &TreeLinkFunctionContext<'_>| Ok(3.);
    let mut linkcolor = |_: &TreeLinkFunctionContext<'_>| Ok(Color(230, 110, 20, 255));
    let mut label = |c: &TreeLabelFunctionContext<'_>| {
        Ok(format!(
            "function-{}-{}",
            c.datum.id,
            if phase >= 3 { "next" } else { "old" }
        ))
    };
    let focus_id = match phase {
        0 | 5 => None,
        2 | 4 => Some("other".to_string()),
        _ => Some("a".to_string()),
    };
    let mut active_size = |_: &TreeNodeModifierContext<'_>| Ok(24.);
    let mut inactive_size = |_: &TreeNodeModifierContext<'_>| Ok(4.);
    let mut active_width = |_: &TreeLinkModifierContext<'_>| Ok(5.);
    let mut inactive_width = |_: &TreeLinkModifierContext<'_>| Ok(1.);
    let mut modifiers = TreeActivityFactories::default();
    if phase >= 2 {
        modifiers = TreeActivityFactories {
            active_node_size: Some(&mut active_size),
            inactive_node_size: Some(&mut inactive_size),
            active_link_thickness: Some(&mut active_width),
            inactive_link_thickness: Some(&mut inactive_width),
        };
    }
    let mut layout = tree_layout_with_activity(
        &config,
        &raw,
        &options,
        &mut TreeFactories {
            node_size: Some(&mut size),
            node_color: Some(&mut color),
            link_thickness: Some(&mut width),
            link_color: Some(&mut linkcolor),
            label: Some(&mut label),
        },
        &TreeActivityOptions {
            focus_id,
            ..Default::default()
        },
        &mut modifiers,
    )
    .unwrap();
    // Legal current ink is separate from constructor metadata; original typed
    // source sign remains untouched. Compatibility Mark zero is authored +0.
    for mark in &mut layout.layout.scene.marks {
        if mark.value == 0. {
            mark.value = 0.;
        }
        if mark.id == "node:other" {
            mark.fill = Some(Color(180, 70, 90, 255));
        }
        if mark.id == "node:a" || mark.id == "link:a" {
            mark.geometry = Geometry::Path(vec![
                C::Move(P::new(205., 150.)),
                C::Line(P::new(225., 150.)),
                C::Line(P::new(215., 165.)),
                C::Close,
            ]);
            mark.fill = Some(Color(20, 90, 140, 255));
            mark.stroke = None;
        }
    }
    let widgets = tree_activity_widgets(&layout, revision, |context| {
        let selected = match context.tree {
            TreeWidgetContext::Node(c) => c.datum.id == "a",
            TreeWidgetContext::Link(c) => c.target_datum.id == "a",
        };
        Ok(selected.then_some(PlotWidgetPlacement::AtDatum {
            offset: [-10., -4.],
            measured: TextSize {
                width: 8.,
                height: 8.,
            },
        }))
    })
    .unwrap();
    (
        tree_activity_frame(&layout).unwrap(),
        PlotWidgetsComposition {
            replace_ink: widgets.data.keys().cloned().collect(),
            widgets,
        },
    )
}
fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut pending = tree.roots().to_vec();
    let mut ids = vec![];
    while let Some(id) = pending.pop() {
        let n = tree.node(id).unwrap();
        if n.descriptor.view().classes.iter().any(|c| c == class) {
            ids.push(id);
        }
        pending.extend(n.children.iter().copied());
    }
    ids
}
fn text(app: &App, id: NodeId) -> String {
    let tree = app.tree();
    let Props::Button(p) = tree.node(id).unwrap().descriptor.view().props else {
        panic!("real Button")
    };
    p.label.clone()
}
fn child(
    context: PlotWidgetCompositionContext<TreeActivityWidgetData>,
    calls: Rc<Cell<usize>>,
) -> Result<impl Widget> {
    calls.set(calls.get() + 1);
    let frame = context.frame;
    let built = context.revision;
    let link = frame.with(|f| matches!(f.data.tree, TreeWidgetData::Link(_)));
    if link && built == 90 {
        return Err(ChartError(
            "independent Tree-activity factory denied".into(),
        ));
    }
    let reject = link && built == 91;
    Ok(glory::ui::widget(move |scope| {
        let count = scope.cage(0u32);
        let focus = scope.cage(false);
        glory::ui::button().class(if link { "audit-tree-activity-link" } else { "audit-tree-activity-node" }).label(scope.bond(move || frame.with(|f| {
            let (source, position, index, role) = match &f.data.tree {
                TreeWidgetData::Node(d) => (&d.source, d.node.position, d.source_index, "node"),
                TreeWidgetData::Link(d) => (&d.source, d.link.position, d.link_index, "link"),
            };
            let labels = match &f.data.tree { TreeWidgetData::Node(d)=>&d.labels,TreeWidgetData::Link(d)=>&d.labels };
            if !labels.labels.is_empty(){assert!(labels.labels.iter().all(|l|l.text.starts_with("function-")));}
            let painted = labels.series.iter().find(|(_,owner)|owner.as_str()=="a").map(|(i,_)|labels.labels[*i].text.as_str()).unwrap_or("none");
            let zero = source.iter().find(|d| d.id == "zero").unwrap();
            let a = source.iter().find(|d| d.id == "a").unwrap();
            assert!((f.frame.rect.x+10.-position[0]).abs()<1e-9); assert!((f.frame.rect.y+4.-position[1]).abs()<1e-9);
            let diameter = match &f.data.tree { TreeWidgetData::Node(d)=>d.node.radius*2.,TreeWidgetData::Link(d)=>d.nodes[d.link.target_index].radius*2. };
            format!("role={role};zero={:016x};a={};index={index};x={:.3};built={built};paint={painted};active={:?};owner={:?};diameter={diameter};count={};focus={}",zero.value.to_bits(),a.value,position[0],f.data.is_active,f.data.activity.focus_id,count.get(),focus.get())
        }))).on(EventKind::Activate,move |_| count.revise(|n|*n+=1)).on(EventKind::FocusChanged,move |event| if let Event::FocusChanged(value)=event { focus.revise(|f|*f=value); }).show_in(scope)?;
        if reject {
            return Err(glory::render::Error::InvalidProps(
                "independent Tree-activity build denied",
            )
            .into());
        }
        Ok(())
    }))
}
fn same_controls(app: &App, expected: &[NodeId]) {
    let actual = nodes(app, "kartis-datum");
    assert_eq!(actual.len(), expected.len());
    assert!(expected.iter().all(|id| actual.contains(id)));
}

fn pointer(app: &App, plot: NodeId, phase: PlotPointerPhase, point: [f64; 2]) {
    app.dispatch(
        plot,
        Event::PlotPointer {
            from: plot,
            pointer: PlotPointer {
                pointer_id: 19,
                device: PlotPointerDevice::Mouse,
                phase,
                x: point[0],
                y: point[1],
                viewport: Rect {
                    x: 0.,
                    y: 0.,
                    width: 450.,
                    height: 260.,
                },
            },
        },
    )
    .unwrap();
}

#[test]
fn activity_metadata_and_paint_update_retained_real_children_with_focus_and_hover_proposals() {
    let initial = source(0, 400., 8, true);
    let metadata = source(1, 400., 8, true);
    assert_eq!(initial.0, metadata.0);
    assert_eq!(
        render::to_svg(&initial.0.scene).unwrap(),
        render::to_svg(&metadata.0.scene).unwrap()
    );
    assert_eq!(initial.1.widgets.layout, metadata.1.widgets.layout);
    assert_ne!(initial.1.widgets, metadata.1.widgets);
    let app = App::memory();
    let input = app.scope().cage(initial);
    let frame = app.scope().bond(move || input.with(|s| s.0.clone()));
    let packet = app.scope().bond(move || input.with(|s| s.1.clone()));
    let calls = Rc::new(Cell::new(0));
    let counted = calls.clone();
    let errors = Rc::new(RefCell::new(vec![]));
    let rejected = errors.clone();
    let hovers = Rc::new(RefCell::new(vec![]));
    let observed_hover = hovers.clone();
    let focuses = Rc::new(RefCell::new(vec![]));
    let observed_focus = focuses.clone();
    app.mount(
        chart_layout(frame)
            .legend(false)
            .tooltip(false)
            .highlight(false)
            .on_hover(move |mark| observed_hover.borrow_mut().push(mark.map(|m| m.id)))
            .on_focus(move |mark| observed_focus.borrow_mut().push(mark.map(|m| m.id)))
            .on_reject(move |e| rejected.borrow_mut().push(e.0))
            .plot_widgets_composed(packet, move |c| child(c, counted.clone())),
    )
    .unwrap();
    let link = nodes(&app, "audit-tree-activity-link")[0];
    let node = nodes(&app, "audit-tree-activity-node")[0];
    let controls = nodes(&app, "kartis-datum");
    assert_eq!(
        controls.len(),
        3,
        "empty-label zero and decorative link do not invent controls"
    );
    assert_eq!(calls.get(), 2);
    app.dispatch(link, Event::Activate).unwrap();
    app.dispatch(link, Event::FocusChanged(true)).unwrap();
    assert!(text(&app, link).contains("zero=8000000000000000;"));
    assert!(text(&app, node).contains("active=None;owner=None;diameter=16;"));
    input.revise(|old| *old = metadata);
    app.flush().unwrap();
    for id in [link, node] {
        assert!(text(&app, id).contains("zero=0000000000000000;"));
        assert!(text(&app, id).contains("active=Some(true);owner=Some(\"a\");"));
    }
    assert!(text(&app, link).ends_with("count=1;focus=true"));
    assert_eq!(calls.get(), 2);
    input.revise(|old| *old = source(2, 420., 8, true));
    app.flush().unwrap();
    assert_eq!(nodes(&app, "audit-tree-activity-link"), [link]);
    assert_eq!(nodes(&app, "audit-tree-activity-node"), [node]);
    same_controls(&app, &controls);
    assert!(text(&app, link).ends_with("count=1;focus=true"));
    let old = [text(&app, link), text(&app, node)];
    let node_count = app.stats().nodes;
    for revision in [90, 91] {
        input.revise(|s| *s = source(3, 460., revision, true));
        app.flush().unwrap();
        assert_eq!([text(&app, link), text(&app, node)], old);
        assert_eq!(app.stats().nodes, node_count);
        same_controls(&app, &controls);
    }
    let mut missing = source(3, 460., 8, true);
    missing.1.widgets.data.remove("link:a");
    input.revise(|s| *s = missing);
    app.flush().unwrap();
    assert_eq!([text(&app, link), text(&app, node)], old);
    assert_eq!(errors.borrow().len(), 3);
    input.revise(|s| *s = source(3, 450., 8, true));
    app.flush().unwrap();
    assert!(text(&app, link).contains("a=4;"));
    assert!(text(&app, node).contains("a=4;"));
    assert!(text(&app, node).contains("paint=function-a-next;"));
    assert!(text(&app, node).contains("active=Some(true);owner=Some(\"a\");diameter=24;"));
    assert!(text(&app, link).ends_with("count=1;focus=true"));
    assert_eq!(
        calls.get(),
        4,
        "two retained children plus two rejected revision attempts"
    );
    same_controls(&app, &controls);
    let other = controls
        .iter()
        .copied()
        .find(|&id| text(&app, id).starts_with("other:"))
        .unwrap();
    app.dispatch(other, Event::FocusChanged(true)).unwrap();
    assert_eq!(focuses.borrow().last(), Some(&Some("node:other".into())));
    // This is the application's explicit accepted publication of a real host
    // proposal, not a claim that borrowed factory closures become retained.
    input.revise(|s| *s = source(4, 450., 8, true));
    app.flush().unwrap();
    assert!(text(&app, node).contains("active=Some(false);owner=Some(\"other\");diameter=4;"));
    let hovered = source(4, 450., 8, true)
        .0
        .interaction
        .unwrap()
        .sites
        .into_iter()
        .find(|s| s.mark_id == "node:other")
        .unwrap()
        .position;
    let plot = nodes(&app, "kartis-plot")[0];
    pointer(&app, plot, PlotPointerPhase::Move, hovered);
    assert_eq!(hovers.borrow().last(), Some(&Some("node:other".into())));
    pointer(&app, plot, PlotPointerPhase::Leave, [0., 0.]);
    assert_eq!(hovers.borrow().last(), Some(&None));
    app.dispatch(other, Event::FocusChanged(false)).unwrap();
    assert_eq!(focuses.borrow().last(), Some(&None));
    input.revise(|s| *s = source(5, 450., 8, true));
    app.flush().unwrap();
    assert!(text(&app, node).contains("active=None;owner=None;diameter=16;"));
    assert_eq!(nodes(&app, "audit-tree-activity-node"), [node]);
    assert_eq!(nodes(&app, "audit-tree-activity-link"), [link]);
    assert_eq!(
        calls.get(),
        4,
        "focus/hover/clear reuse both accepted children"
    );
    same_controls(&app, &controls);
    app.clear().unwrap();
    assert_eq!(app.stats().nodes, 0);
}

#[cfg(feature = "raster")]
fn image(app: &App) -> String {
    let tree = app.tree();
    let id = nodes(app, "kartis-raster-image")[0];
    let Props::Image(p) = tree.node(id).unwrap().descriptor.view().props else {
        panic!("Image")
    };
    p.source.clone()
}
#[cfg(feature = "raster")]
fn pixel(uri: &str, point: [usize; 2]) -> [u8; 4] {
    use base64::Engine;
    let png = base64::engine::general_purpose::STANDARD
        .decode(uri.split_once(',').unwrap().1)
        .unwrap();
    let mut r = png::Decoder::new(std::io::Cursor::new(png))
        .read_info()
        .unwrap();
    let mut bytes = vec![0; r.output_buffer_size().unwrap()];
    let info = r.next_frame(&mut bytes).unwrap();
    let offset = 4 * (point[1] * info.width as usize + point[0]);
    bytes[offset..offset + 4].try_into().unwrap()
}

#[cfg(feature = "raster")]
#[test]
fn cpu_activity_publication_rejection_keeps_original_typed_frame_and_actual_png_atomic() {
    use kartis::render::raster::{FontRegistry, RasterOptions};
    for dpr in [1., 2.] {
        let initial = source(0, 400., 8, false);
        let other = initial
            .0
            .interaction
            .as_ref()
            .unwrap()
            .sites
            .iter()
            .find(|s| s.mark_id == "node:other")
            .unwrap()
            .position;
        let app = App::memory();
        let input = app.scope().cage(initial);
        let frame = app.scope().bond(move || input.with(|s| s.0.clone()));
        let packet = app.scope().bond(move || input.with(|s| s.1.clone()));
        let calls = Rc::new(Cell::new(0));
        let counted = calls.clone();
        let deny = Rc::new(Cell::new(false));
        let denied = deny.clone();
        let publications = Rc::new(Cell::new(0));
        let published = publications.clone();
        let errors = Rc::new(RefCell::new(vec![]));
        let rejected = errors.clone();
        app.mount(
            chart_layout(frame)
                .legend(false)
                .tooltip(false)
                .raster(
                    FontRegistry::new(),
                    RasterOptions {
                        pixel_ratio: dpr,
                        ..Default::default()
                    },
                )
                .raster_image_source(move |png| {
                    published.set(published.get() + 1);
                    if denied.get() {
                        Err(ChartError("independent Tree-activity PNG denied".into()))
                    } else {
                        png.data_url()
                    }
                })
                .on_reject(move |e| rejected.borrow_mut().push(e.0))
                .plot_widgets_composed(packet, move |c| child(c, counted.clone())),
        )
        .unwrap();
        let link = nodes(&app, "audit-tree-activity-link")[0];
        let node = nodes(&app, "audit-tree-activity-node")[0];
        let controls = nodes(&app, "kartis-datum");
        app.dispatch(link, Event::Activate).unwrap();
        app.dispatch(link, Event::FocusChanged(true)).unwrap();
        let old = [text(&app, link), text(&app, node)];
        let old_png = image(&app);
        assert_eq!(
            pixel(
                &old_png,
                [(other[0] * dpr) as usize, (other[1] * dpr) as usize]
            ),
            [180, 70, 90, 255]
        );
        let count = publications.get();
        let tree_count = app.stats().nodes;
        deny.set(true);
        input.revise(|s| *s = source(3, 450., 8, false));
        app.flush().unwrap();
        assert_eq!(publications.get(), count + 1);
        assert_eq!(image(&app), old_png);
        assert_eq!([text(&app, link), text(&app, node)], old);
        assert_eq!(app.stats().nodes, tree_count);
        same_controls(&app, &controls);
        assert_eq!(errors.borrow().len(), 1);
        deny.set(false);
        input.revise(|s| *s = source(3, 460., 8, false));
        app.flush().unwrap();
        assert_ne!(image(&app), old_png);
        assert!(text(&app, link).contains("a=4;"));
        assert!(text(&app, node).contains("a=4;"));
        assert!(text(&app, node).contains("active=Some(true);owner=Some(\"a\");diameter=24;"));
        let updated = source(3, 460., 8, false)
            .0
            .interaction
            .unwrap()
            .sites
            .into_iter()
            .find(|s| s.mark_id == "node:other")
            .unwrap()
            .position;
        assert_eq!(
            pixel(
                &image(&app),
                [(updated[0] * dpr) as usize, (updated[1] * dpr) as usize]
            ),
            [180, 70, 90, 255]
        );
        assert_eq!(
            pixel(
                &image(&app),
                [
                    ((updated[0] + 4.) * dpr) as usize,
                    (updated[1] * dpr) as usize
                ]
            ),
            [255; 4],
            "inactive source node really shrank to radius two"
        );
        assert!(text(&app, link).ends_with("count=1;focus=true"));
        assert_eq!(calls.get(), 2);
        same_controls(&app, &controls);
        app.clear().unwrap();
        assert_eq!(app.stats().nodes, 0);
    }
}
