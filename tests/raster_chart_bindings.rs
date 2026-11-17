#![cfg(feature = "raster")]
//! Real Memory Image props and protocol injection into its physical finite
//! View, not OS pointer transport. Widgets remain siblings of the image View.
use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::rc::Rc;

use base64::Engine;
use glory::render::memory::{Memory, Subtree};
use glory::render::{
    self, Event, EventKind, NodeId, PlotPointer, PlotPointerDevice, PlotPointerPhase, Props, Rect,
    Render,
};
use glory::style::{Length, Profile};
use glory::{App, Widget};
use kartis::render::raster::{FontRegistry, RasterOptions};
use kartis::*;
#[path = "../examples/shared/raster_chart_bindings.rs"]
mod model;

struct NoGraphics(Memory);
impl Render for NoGraphics {
    fn preflight_place(
        &self,
        tree: &Memory,
        child: Subtree<'_>,
        _parent: Option<NodeId>,
    ) -> std::result::Result<(), render::Error> {
        let graphic = match child {
            Subtree::Plan(p) => p
                .nodes()
                .iter()
                .any(|n| matches!(n.descriptor.view().props, Props::Graphic(_))),
            Subtree::New(n) => matches!(n.props, Props::Graphic(_)),
            Subtree::Existing(id) => {
                matches!(tree.node(id)?.descriptor.view().props, Props::Graphic(_))
            }
        };
        if graphic {
            Err(render::Error::Unsupported(
                "test host accepts Image but forbids Graphic",
            ))
        } else {
            Ok(())
        }
    }
    fn capabilities(&self) -> render::Capabilities {
        self.0.capabilities()
    }
    fn style_profile(&self) -> Profile {
        self.0.style_profile()
    }
    fn supports_plot_pointer(&self) -> bool {
        self.0.supports_plot_pointer()
    }
    fn apply(
        &mut self,
        changes: &[render::Mutation],
    ) -> std::result::Result<render::Receipt, render::Error> {
        self.0.apply(changes)
    }
    fn request(
        &mut self,
        query: render::Query,
        token: render::QueryToken,
    ) -> std::result::Result<(), render::Error> {
        self.0.request(query, token)
    }
    fn drain(&mut self, inputs: &mut Vec<render::Input>) {
        self.0.drain(inputs)
    }
}
fn app() -> App {
    App::new(|runtime| NoGraphics(Memory::new(runtime)))
}
fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut queue = tree.roots().to_vec();
    let mut found = vec![];
    while let Some(id) = queue.pop() {
        let n = tree.node(id).unwrap();
        if n.descriptor.view().classes.iter().any(|v| v == class) {
            found.push(id);
        }
        queue.extend(n.children.iter().copied());
    }
    found
}
fn first(app: &App, class: &str) -> NodeId {
    nodes(app, class)[0]
}
fn label(app: &App, id: NodeId) -> String {
    let tree = app.tree();
    let Props::Button(p) = tree.node(id).unwrap().descriptor.view().props else {
        panic!("actual Button expected")
    };
    p.label.clone()
}
fn controls(app: &App) -> Vec<String> {
    let mut out = nodes(app, "kartis-datum")
        .into_iter()
        .map(|id| label(app, id))
        .collect::<Vec<_>>();
    out.sort();
    out
}
fn control(app: &App, prefix: &str) -> NodeId {
    nodes(app, "kartis-datum")
        .into_iter()
        .find(|id| label(app, *id).starts_with(prefix))
        .unwrap()
}
fn pressed(app: &App, id: NodeId) -> Option<bool> {
    app.tree()
        .node(id)
        .unwrap()
        .descriptor
        .view()
        .semantics
        .as_ref()
        .and_then(|s| s.pressed)
}
fn image_source(app: &App) -> String {
    let tree = app.tree();
    let Props::Image(p) = tree
        .node(first(app, "kartis-raster-image"))
        .unwrap()
        .descriptor
        .view()
        .props
    else {
        panic!("actual Image expected")
    };
    p.source.clone()
}
fn decode(source: &str) -> (u32, u32, Vec<u8>) {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(source.strip_prefix("data:image/png;base64,").unwrap())
        .unwrap();
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    let mut rgba = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut rgba).unwrap();
    assert_eq!(info.color_type, png::ColorType::Rgba);
    rgba.truncate(info.buffer_size());
    (info.width, info.height, rgba)
}
fn pixel(source: &str, point: [f64; 2], dpr: f64) -> [u8; 4] {
    let (width, _, rgba) = decode(source);
    let x = (point[0] * dpr).floor() as usize;
    let y = (point[1] * dpr).floor() as usize;
    rgba[4 * (y * width as usize + x)..][..4]
        .try_into()
        .unwrap()
}
fn projected(view: ZoomTransform, point: [f64; 2]) -> [f64; 2] {
    let p = view.project((point[0], point[1])).unwrap();
    [p.0, p.1]
}
fn pointer(
    app: &App,
    image: NodeId,
    phase: PlotPointerPhase,
    point: [f64; 2],
    view: ZoomTransform,
) {
    let p = projected(view, point);
    let plot = first(app, "kartis-raster-plot");
    assert!(app.scope().contains_node(plot, image).unwrap());
    // Finite event coordinates are relative to a nonuniformly scaled Image;
    // image DPR is intentionally independent of this host viewport.
    assert!(
        app.dispatch(
            plot,
            Event::PlotPointer {
                from: plot,
                pointer: PlotPointer {
                    pointer_id: 41,
                    device: PlotPointerDevice::Pen,
                    phase,
                    x: p[0] * 2.,
                    y: p[1] * 1.5,
                    viewport: Rect {
                        width: 480.,
                        height: 240.,
                        ..Default::default()
                    }
                }
            }
        )
        .unwrap()
    );
    app.flush().unwrap();
}
fn tooltip(app: &App) -> String {
    let tree = app.tree();
    nodes(app, "kartis-tooltip")
        .into_iter()
        .filter_map(|id| {
            if let Props::Text(p) = tree.node(id).unwrap().descriptor.view().props {
                Some(p.content.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
fn live_tree(app: &App) -> String {
    let tree = app.tree();
    let mut queue = tree.roots().to_vec();
    let mut out = vec![];
    while let Some(id) = queue.pop() {
        let n = tree.node(id).unwrap();
        out.push(format!("{id:?}:{n:?}"));
        queue.extend(n.children.iter().copied());
    }
    format!("{:?}:{out:?}", tree.roots())
}
fn child(
    frame: glory::Bond<TypedPlotWidgetFrame<model::Data>>,
    id: String,
    drops: Rc<Cell<usize>>,
) -> impl Widget {
    glory::ui::widget(move |scope| {
        let drops = drops.clone();
        scope.on_cleanup(move || drops.set(drops.get() + 1))?;
        let count = scope.cage(0usize);
        let focus = scope.cage(false);
        glory::ui::button()
            .class(format!("bound-widget-{id}"))
            .label(scope.bond(move || {
                frame.with(|f| {
                    format!(
                        "raw={:?}|note={}|live={}|count={}|focus={}",
                        f.data.value,
                        f.data.note,
                        f.frame.mark.value,
                        count.get(),
                        focus.get()
                    )
                })
            }))
            .on(EventKind::Activate, move |_| count.revise(|v| *v += 1))
            .on(EventKind::FocusChanged, move |event| {
                if let Event::FocusChanged(v) = event {
                    focus.revise(|old| *old = v);
                }
            })
            .show_in(scope)
    })
}

#[test]
fn image_protocol_resolves_aliases_nullable_controls_and_camera_once_without_graphics() {
    for dpr in [1., 2.] {
        for view in [
            ZoomTransform::default(),
            ZoomTransform::new(1.25, -12., -8.).unwrap(),
        ] {
            let app = app();
            let input = app.scope().cage(model::source(None, 0., 0, 0).unwrap());
            let scene = app.scope().bond(move || input.with(|p| p.frame.clone()));
            let widgets = app.scope().bond(move || input.with(|p| p.widgets.clone()));
            let hits = Rc::new(RefCell::new(vec![]));
            let observed = hits.clone();
            let selections = Rc::new(RefCell::new(vec![]));
            let selected = selections.clone();
            let drops = Rc::new(Cell::new(0));
            let cleaned = drops.clone();
            app.mount(
                chart_layout(scene)
                    .raster(
                        FontRegistry::new(),
                        RasterOptions {
                            pixel_ratio: dpr,
                            view,
                            ..Default::default()
                        },
                    )
                    .legend(false)
                    .highlight(false)
                    .tooltip_options(TooltipOptions {
                        missing: MissingValue::Placeholder("missing".into()),
                        ..Default::default()
                    })
                    .on_physical_hover(move |hit| observed.borrow_mut().push(hit))
                    .on_select(move |m| selected.borrow_mut().push(m.id))
                    .plot_widgets_with_data(widgets, move |ctx| {
                        Ok(child(ctx.frame, ctx.mark_id, cleaned.clone()))
                    }),
            )
            .unwrap();
            let image = first(&app, "kartis-raster-image");
            let widget = first(&app, "bound-widget-owner");
            assert!(nodes(&app, "kartis-mark").is_empty());
            assert_eq!(
                controls(&app),
                ["Nullable measurement: missing", "Other measurement: 7"]
            );
            let props = app.tree();
            let style = &props.node(image).unwrap().descriptor.view().style;
            assert_eq!(style.width, Some(Length::Percent(100.)));
            assert_eq!(style.height, Some(Length::Percent(100.)));
            let plot = first(&app, "kartis-raster-plot");
            assert!(
                !app.scope().contains_node(plot, widget).unwrap(),
                "live Widget is a sibling of the finite image View"
            );
            let plot_style = &props.node(plot).unwrap().descriptor.view().style;
            assert_eq!(plot_style.width, Some(Length::Px(240.)));
            assert_eq!(plot_style.height, Some(Length::Px(160.)));
            drop(props);
            let png = image_source(&app);
            let (width, height, _) = decode(&png);
            assert_eq!([width, height], [(240. * dpr) as u32, (160. * dpr) as u32]);
            assert_eq!(
                pixel(&png, projected(view, [110., 80.]), dpr),
                [20, 180, 100, 255]
            );
            let clip = first(&app, "kartis-plot-widget-clip");
            let tree = app.tree();
            let style = &tree.node(clip).unwrap().descriptor.view().style;
            assert_eq!(
                style.left,
                Some(Length::Px(24. * view.scale + view.translate_x))
            );
            assert_eq!(
                style.top,
                Some(Length::Px(26. * view.scale + view.translate_y))
            );
            assert_eq!(style.width, Some(Length::Px(32. * view.scale)));
            assert_eq!(style.height, Some(Length::Px(24. * view.scale)));
            drop(tree);
            assert!(label(&app, widget).contains("raw=None|note=0|live=-913"));
            app.dispatch(widget, Event::Activate).unwrap();
            app.dispatch(widget, Event::FocusChanged(true)).unwrap();
            pointer(&app, image, PlotPointerPhase::Move, [110., 80.], view);
            let first_hit = hits.borrow().last().unwrap().clone().unwrap();
            assert_eq!(first_hit.physical_mark_id, model::ALIAS);
            assert_eq!(first_hit.logical_site_id, model::OWNER);
            assert_eq!(first_hit.scene_point, [110., 80.]);
            assert!(first_hit.source_data.is_none());
            assert!(tooltip(&app).contains("missing"));
            assert!(!tooltip(&app).contains("-913"));
            pointer(&app, image, PlotPointerPhase::Move, [160., 105.], view);
            let hit = hits.borrow().last().unwrap().clone().unwrap();
            assert_eq!(hit.physical_mark_id, model::TAIL);
            assert_eq!(hit.logical_site_id, model::OWNER);
            assert_eq!(
                hit.source_data.unwrap(),
                InteractionPartDatum {
                    source_id: "raw-part-0".into(),
                    value: 99.
                }
            );
            pointer(&app, image, PlotPointerPhase::Down, [160., 105.], view);
            pointer(&app, image, PlotPointerPhase::Up, [160., 105.], view);
            assert_eq!(
                selections.borrow().last().map(String::as_str),
                Some(model::OWNER)
            );
            pointer(&app, image, PlotPointerPhase::Move, [110., 86.], view);
            assert!(
                hits.borrow().last().unwrap().is_none(),
                "camera-correct picking keeps exact stroke footprint"
            );
            input.revise(|p| *p = model::source(Some(0.), 0., 1, 0).unwrap());
            app.flush().unwrap();
            assert_eq!(first(&app, "bound-widget-owner"), widget);
            assert!(label(&app, widget).contains("raw=Some(0.0)|note=1|live=0|count=1|focus=true"));
            assert!(
                controls(&app)
                    .iter()
                    .any(|s| s == "Nullable measurement: 0")
            );
            app.clear().unwrap();
            assert_eq!(drops.get(), 1);
            assert_eq!(app.stats().nodes, 0);
        }
    }
}

#[derive(Clone)]
struct Input {
    source: model::Source,
    options: RasterOptions,
    hidden: BTreeSet<String>,
    selected: Option<String>,
}

#[test]
fn nonzero_plot_keeps_click_drag_threshold_in_visual_scene_units_at_both_zoom_scales() {
    for view in [
        ZoomTransform::new(2., -40., -40.).unwrap(),
        ZoomTransform::new(0.5, 40., 30.).unwrap(),
    ] {
        for visual_delta in [2., 4.] {
            let app = app();
            let mut source = model::source(None, 0., 0, 0).unwrap();
            source.frame.plot = Some(ClipRect {
                x: 20.,
                y: 20.,
                width: 200.,
                height: 120.,
            });
            let selected = Rc::new(RefCell::new(vec![]));
            let seen = selected.clone();
            app.mount(
                chart_layout(source.frame)
                    .raster(
                        FontRegistry::new(),
                        RasterOptions {
                            view,
                            ..Default::default()
                        },
                    )
                    .legend(false)
                    .highlight(false)
                    .on_select(move |mark| seen.borrow_mut().push(mark.id)),
            )
            .unwrap();
            let image = first(&app, "kartis-raster-image");
            pointer(&app, image, PlotPointerPhase::Down, [40., 40.], view);
            pointer(
                &app,
                image,
                PlotPointerPhase::Move,
                [40. + visual_delta / view.scale, 40.],
                view,
            );
            pointer(
                &app,
                image,
                PlotPointerPhase::Up,
                [40. + visual_delta / view.scale, 40.],
                view,
            );
            assert_eq!(
                selected.borrow().len(),
                usize::from(visual_delta < 3.),
                "absolute image camera must not scale the visual 3px gesture threshold: {view:?}/{visual_delta}"
            );
            app.clear().unwrap();
        }
    }
}

#[test]
fn publisher_error_retains_binding_png_options_controls_payload_and_local_focus() {
    for dpr in [1., 2.] {
        let app = app();
        let old_view = ZoomTransform::new(1.25, -12., -8.).unwrap();
        let input = app.scope().cage(Input {
            source: model::source(None, 0., 0, 0).unwrap(),
            options: RasterOptions {
                pixel_ratio: dpr,
                view: old_view,
                ..Default::default()
            },
            hidden: BTreeSet::new(),
            selected: Some(model::OWNER.into()),
        });
        let scene = app
            .scope()
            .bond(move || input.with(|p| p.source.frame.clone()));
        let widgets = app
            .scope()
            .bond(move || input.with(|p| p.source.widgets.clone()));
        let options = app.scope().bond(move || input.with(|p| p.options.clone()));
        let hidden = app.scope().bond(move || input.with(|p| p.hidden.clone()));
        let selected = app.scope().bond(move || input.with(|p| p.selected.clone()));
        let reject = Rc::new(Cell::new(false));
        let refusing = reject.clone();
        let publications = Rc::new(Cell::new(0));
        let called = publications.clone();
        let errors = Rc::new(RefCell::new(vec![]));
        let rejected = errors.clone();
        let hits = Rc::new(RefCell::new(vec![]));
        let hit = hits.clone();
        let drops = Rc::new(Cell::new(0));
        let cleaned = drops.clone();
        app.mount(
            chart_layout(scene)
                .raster(FontRegistry::new(), options)
                .legend(false)
                .highlight(false)
                .hidden(hidden)
                .selected(selected)
                .raster_image_source(move |image| {
                    called.set(called.get() + 1);
                    if refusing.get() {
                        Err(ChartError("typed image publication refused".into()))
                    } else {
                        image.data_url()
                    }
                })
                .on_reject(move |e| rejected.borrow_mut().push(e.0))
                .on_physical_hover(move |h| hit.borrow_mut().push(h))
                .plot_widgets_with_data(widgets, move |ctx| {
                    Ok(child(ctx.frame, ctx.mark_id, cleaned.clone()))
                }),
        )
        .unwrap();
        let image = first(&app, "kartis-raster-image");
        let widget = first(&app, "bound-widget-owner");
        app.dispatch(widget, Event::Activate).unwrap();
        app.dispatch(widget, Event::FocusChanged(true)).unwrap();
        app.flush().unwrap();
        assert_eq!(pressed(&app, control(&app, "Nullable")), Some(true));
        let before = live_tree(&app);
        let png = image_source(&app);
        let node_count = app.stats().nodes;
        let calls = publications.get();
        reject.set(true);
        input.revise(|p| {
            p.source = model::source(Some(0.), 28., 99, 0).unwrap();
            p.options.view = ZoomTransform::new(1.1, 14., 8.).unwrap();
            p.hidden.insert("other".into());
            p.selected = Some("other".into());
        });
        app.flush().unwrap();
        assert_eq!(errors.borrow().len(), 1);
        assert_eq!(publications.get(), calls + 1);
        assert_eq!(image_source(&app), png);
        assert_eq!(live_tree(&app), before);
        assert_eq!(app.stats().nodes, node_count);
        assert_eq!(first(&app, "bound-widget-owner"), widget);
        assert!(label(&app, widget).contains("raw=None|note=0|live=-913|count=1|focus=true"));
        pointer(&app, image, PlotPointerPhase::Move, [160., 105.], old_view);
        let accepted_hit = hits.borrow().last().unwrap().clone().unwrap();
        assert_eq!(accepted_hit.source_data.unwrap().value, 99.);
        reject.set(false);
        input.revise(|p| p.source.widgets.data.get_mut(model::OWNER).unwrap().note = 100);
        app.flush().unwrap();
        let next_view = ZoomTransform::new(1.1, 14., 8.).unwrap();
        assert_eq!(first(&app, "bound-widget-owner"), widget);
        assert!(label(&app, widget).contains("raw=Some(0.0)|note=100|live=0|count=1|focus=true"));
        assert_eq!(controls(&app), ["Nullable measurement: 0"]);
        pointer(&app, image, PlotPointerPhase::Move, [160., 105.], next_view);
        assert!(
            hits.borrow().last().unwrap().is_none(),
            "old physical alias position is no longer current"
        );
        pointer(&app, image, PlotPointerPhase::Move, [188., 105.], next_view);
        let next_hit = hits.borrow().last().unwrap().clone().unwrap();
        assert_eq!(next_hit.source_data.unwrap().value, 198.);
        assert_ne!(next_hit.frame_token, accepted_hit.frame_token);
        assert_eq!(
            pixel(&image_source(&app), projected(next_view, [200., 115.]), dpr),
            [255, 255, 255, 255],
            "accepted hidden state is painted with the new raster view"
        );
        assert_eq!(pressed(&app, control(&app, "Nullable")), Some(false));
        input.revise(|p| p.hidden.clear());
        app.flush().unwrap();
        assert_eq!(
            pressed(&app, control(&app, "Other")),
            Some(true),
            "the published candidate's controlled selection was retained while hidden"
        );
        input.revise(|p| {
            p.hidden.insert("route".into());
        });
        app.flush().unwrap();
        assert_eq!(first(&app, "bound-widget-owner"), widget);
        assert!(app.scope().is_inert(widget).unwrap());
        assert!(!app.dispatch(widget, Event::Activate).unwrap());
        assert_eq!(controls(&app), ["Other measurement: 7"]);
        input.revise(|p| p.hidden.clear());
        app.flush().unwrap();
        assert_eq!(first(&app, "bound-widget-owner"), widget);
        assert!(label(&app, widget).contains("count=1|focus=true"));
        app.clear().unwrap();
        assert_eq!(drops.get(), 1, "same-revision child cleans once");
    }
}

#[test]
fn view_only_commit_repicks_the_stationary_pointer_and_cancels_the_old_drag() {
    let app = app();
    let input = app.scope().cage(model::source(None, 0., 0, 0).unwrap());
    let frame = app.scope().bond(move || input.with(|p| p.frame.clone()));
    let options = app.scope().cage(RasterOptions::default());
    let hits = Rc::new(RefCell::new(vec![]));
    let observed = hits.clone();
    let selects = Rc::new(RefCell::new(vec![]));
    let chosen = selects.clone();
    app.mount(
        chart_layout(frame)
            .raster(FontRegistry::new(), options)
            .legend(false)
            .highlight(false)
            .on_physical_hover(move |hit| observed.borrow_mut().push(hit))
            .on_select(move |mark| chosen.borrow_mut().push(mark.id)),
    )
    .unwrap();
    let image = first(&app, "kartis-raster-image");
    pointer(
        &app,
        image,
        PlotPointerPhase::Move,
        [110., 80.],
        ZoomTransform::default(),
    );
    let old = hits.borrow().last().unwrap().clone().unwrap();
    pointer(
        &app,
        image,
        PlotPointerPhase::Down,
        [110., 80.],
        ZoomTransform::default(),
    );
    let next = ZoomTransform::new(1.25, -12., -8.).unwrap();
    options.revise(|o| o.view = next);
    app.flush().unwrap();
    assert!(
        hits.borrow().last().unwrap().is_none(),
        "view-only image publication must repick visual cursor without applying the camera twice"
    );
    assert_eq!(
        pixel(&image_source(&app), projected(next, [110., 80.]), 1.),
        [20, 180, 100, 255]
    );
    pointer(&app, image, PlotPointerPhase::Up, [110., 80.], next);
    assert!(
        selects.borrow().is_empty(),
        "a prior-view gesture cannot activate after image camera publication"
    );
    pointer(&app, image, PlotPointerPhase::Move, [110., 80.], next);
    let current = hits.borrow().last().unwrap().clone().unwrap();
    assert_ne!(current.frame_token, old.frame_token);
    assert_eq!(current.scene_point, [110., 80.]);
    pointer(&app, image, PlotPointerPhase::Down, [110., 80.], next);
    pointer(&app, image, PlotPointerPhase::Up, [110., 80.], next);
    assert_eq!(*selects.borrow(), [model::OWNER.to_owned()]);
    app.clear().unwrap();
}

#[test]
fn binding_only_missing_to_zero_updates_live_controls_and_tooltip_with_identical_scene_and_png() {
    let app = app();
    let mut initial = model::source(None, 0., 0, 0).unwrap();
    initial.frame.scene.marks[0].value = 0.;
    initial.frame.validate().unwrap();
    let input = app.scope().cage(initial);
    let frame = app.scope().bond(move || input.with(|p| p.frame.clone()));
    let widgets = app.scope().bond(move || input.with(|p| p.widgets.clone()));
    let drops = Rc::new(Cell::new(0));
    let cleaned = drops.clone();
    app.mount(
        chart_layout(frame)
            .raster(FontRegistry::new(), RasterOptions::default())
            .legend(false)
            .highlight(false)
            .tooltip_options(TooltipOptions {
                missing: MissingValue::Placeholder("missing".into()),
                ..Default::default()
            })
            .plot_widgets_with_data(widgets, move |ctx| {
                Ok(child(ctx.frame, ctx.mark_id, cleaned.clone()))
            }),
    )
    .unwrap();
    let image = first(&app, "kartis-raster-image");
    let widget = first(&app, "bound-widget-owner");
    pointer(
        &app,
        image,
        PlotPointerPhase::Move,
        [110., 80.],
        ZoomTransform::default(),
    );
    assert!(tooltip(&app).contains("missing"));
    let original_scene = input.with(|p| p.frame.scene.clone());
    let png = image_source(&app);
    input.revise(|p| {
        let site = &mut p.frame.interaction.as_mut().unwrap().sites[0];
        site.value = Some(0.);
        site.formatted_value = Some("0".into());
        p.widgets.data.get_mut(model::OWNER).unwrap().value = Some(0.);
    });
    app.flush().unwrap();
    assert_eq!(input.with(|p| p.frame.scene.clone()), original_scene);
    assert_eq!(image_source(&app), png);
    assert_eq!(first(&app, "bound-widget-owner"), widget);
    assert!(
        controls(&app)
            .iter()
            .any(|s| s == "Nullable measurement: 0")
    );
    assert_eq!(tooltip(&app), "Nullable measurement: 0");
    assert!(label(&app, widget).contains("raw=Some(0.0)"));
    app.clear().unwrap();
    assert_eq!(drops.get(), 1);
}

#[test]
fn strict_composition_omits_only_the_replaced_physical_alias_and_restores_it_atomically() {
    let app = app();
    let mut source = model::source(None, 0., 0, 0).unwrap();
    source.widgets.layout.slots.push(PlotWidgetSlot {
        mark_id: model::ALIAS.into(),
        revision: 0,
        placement: PlotWidgetPlacement::Rect(LayoutRect {
            x: 100.,
            y: 70.,
            width: 20.,
            height: 20.,
        }),
    });
    source.widgets.data.insert(
        model::ALIAS.into(),
        model::Data {
            value: None,
            note: 0,
        },
    );
    let input = app
        .scope()
        .cage((source, BTreeSet::from([model::ALIAS.to_owned()])));
    let frame = app.scope().bond(move || input.with(|p| p.0.frame.clone()));
    let packet = app.scope().bond(move || {
        input.with(|p| PlotWidgetsComposition {
            widgets: p.0.widgets.clone(),
            replace_ink: p.1.clone(),
        })
    });
    let hits = Rc::new(RefCell::new(vec![]));
    let observed = hits.clone();
    let drops = Rc::new(Cell::new(0));
    let cleaned = drops.clone();
    app.mount(
        chart_layout(frame)
            .raster(FontRegistry::new(), RasterOptions::default())
            .legend(false)
            .highlight(false)
            .tooltip_options(TooltipOptions {
                missing: MissingValue::Placeholder("missing".into()),
                ..Default::default()
            })
            .on_physical_hover(move |h| observed.borrow_mut().push(h))
            .plot_widgets_composed(packet, move |ctx| {
                Ok(child(ctx.frame, ctx.mark_id, cleaned.clone()))
            }),
    )
    .unwrap();
    let image = first(&app, "kartis-raster-image");
    let alias_widget = first(&app, "bound-widget-long-alias");
    assert_eq!(
        pixel(&image_source(&app), [110., 80.], 1.),
        [255, 255, 255, 255]
    );
    assert_eq!(controls(&app).len(), 2);
    pointer(
        &app,
        image,
        PlotPointerPhase::Move,
        [110., 80.],
        ZoomTransform::default(),
    );
    assert!(hits.borrow().last().is_none_or(|h| h.is_none()));
    pointer(
        &app,
        image,
        PlotPointerPhase::Move,
        [160., 105.],
        ZoomTransform::default(),
    );
    assert_eq!(
        hits.borrow()
            .last()
            .unwrap()
            .as_ref()
            .unwrap()
            .logical_site_id,
        model::OWNER
    );
    input.revise(|p| p.1.clear());
    app.flush().unwrap();
    assert_eq!(first(&app, "bound-widget-long-alias"), alias_widget);
    assert_eq!(
        pixel(&image_source(&app), [110., 80.], 1.),
        [20, 180, 100, 255]
    );
    pointer(
        &app,
        image,
        PlotPointerPhase::Move,
        [110., 80.],
        ZoomTransform::default(),
    );
    assert_eq!(
        hits.borrow()
            .last()
            .unwrap()
            .as_ref()
            .unwrap()
            .physical_mark_id,
        model::ALIAS
    );
    assert!(tooltip(&app).contains("missing"));
    app.clear().unwrap();
    assert_eq!(drops.get(), 2);
}

#[test]
fn accepted_source_change_from_a_hit_retires_its_old_physical_snapshot() {
    let app = app();
    let input = app.scope().cage(model::source(None, 0., 0, 0).unwrap());
    let frame = app.scope().bond(move || input.with(|p| p.frame.clone()));
    let hits = Rc::new(RefCell::new(vec![]));
    let observed = hits.clone();
    let changed = Rc::new(Cell::new(false));
    let once = changed.clone();
    app.mount(
        chart_layout(frame)
            .raster(FontRegistry::new(), RasterOptions::default())
            .legend(false)
            .highlight(false)
            .on_physical_hover(move |hit| {
                observed.borrow_mut().push(hit.clone());
                if hit.is_some() && !once.replace(true) {
                    input.revise(|p| *p = model::source(Some(0.), 28., 1, 0).unwrap());
                }
            }),
    )
    .unwrap();
    let image = first(&app, "kartis-raster-image");
    pointer(
        &app,
        image,
        PlotPointerPhase::Move,
        [160., 105.],
        ZoomTransform::default(),
    );
    let old = hits.borrow().iter().find_map(Clone::clone).unwrap();
    assert!(
        hits.borrow().last().unwrap().is_none(),
        "accepted new geometry clears the old physical hover"
    );
    pointer(
        &app,
        image,
        PlotPointerPhase::Move,
        [188., 105.],
        ZoomTransform::default(),
    );
    let next = hits.borrow().last().unwrap().clone().unwrap();
    assert_ne!(old.frame_token, next.frame_token);
    assert_eq!(next.source_data.unwrap().value, 100.);
    assert!(
        controls(&app)
            .iter()
            .any(|s| s == "Nullable measurement: 0")
    );
    app.clear().unwrap();
}

#[test]
fn cleanup_during_logical_hover_stops_remaining_physical_callbacks_and_source_work() {
    let app = app();
    let input = app.scope().cage(model::source(None, 0., 0, 0).unwrap());
    let frame = app.scope().bond(move || input.with(|p| p.frame.clone()));
    let owner = Rc::new(RefCell::new(None::<glory::ui::Mounted>));
    let stop = owner.clone();
    let hits = Rc::new(RefCell::new(vec![]));
    let observed = hits.clone();
    let publications = Rc::new(Cell::new(0));
    let called = publications.clone();
    let mounted = app
        .scope()
        .mount_with(|| {
            chart_layout(frame)
                .raster(FontRegistry::new(), RasterOptions::default())
                .legend(false)
                .highlight(false)
                .raster_image_source(move |image| {
                    called.set(called.get() + 1);
                    image.data_url()
                })
                .on_hover(move |m| {
                    if m.is_some() {
                        let mounted = stop.borrow_mut().take();
                        drop(mounted);
                    }
                })
                .on_physical_hover(move |hit| observed.borrow_mut().push(hit))
        })
        .unwrap();
    *owner.borrow_mut() = Some(mounted);
    let image = first(&app, "kartis-raster-image");
    pointer(
        &app,
        image,
        PlotPointerPhase::Move,
        [110., 80.],
        ZoomTransform::default(),
    );
    assert!(
        hits.borrow().is_empty(),
        "retired owner cancels remaining callback batch"
    );
    assert_eq!(app.stats().nodes, 0);
    let calls = publications.get();
    input.revise(|p| *p = model::source(Some(0.), 28., 4, 0).unwrap());
    app.flush().unwrap();
    assert_eq!(publications.get(), calls);
    assert_eq!(app.stats().nodes, 0);
}
