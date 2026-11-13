use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use glory::App;
use glory::render::{
    Event, GraphicPoint as P, GraphicStroke, NodeId, PathCommand as C, PlotPointer,
    PlotPointerDevice, PlotPointerPhase, PointerPhase, Props, Rect, StrokeCap,
};
use glory::style::PointerEvents;
use kartis::widget::chart_layout;
use kartis::*;

fn frame() -> SceneInteraction {
    let mut scene = ChartConfig {
        width: 200.,
        height: 120.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    let mut body = Mark::new(
        "body",
        Geometry::Rect {
            x: 40.,
            y: 60.,
            width: 40.,
            height: 40.,
        },
        Color(10, 20, 30, 0),
    );
    body.series = "x".into();
    body.label = "business".into();
    body.value = 5.;
    body.opacity = 0.;
    let mut line = Mark::new(
        "line",
        Geometry::Path(vec![C::Move(P::new(20., 20.)), C::Line(P::new(80., 20.))]),
        Color(0, 0, 0, 0),
    );
    line.series = "x".into();
    line.label.clear();
    line.fill = None;
    line.stroke = Some(GraphicStroke {
        width: 4.,
        color: Color(0, 0, 0, 255),
        cap: StrokeCap::Butt,
    });
    let mut outlier = Mark::new(
        "outlier",
        Geometry::Circle {
            x: 120.,
            y: 30.,
            radius: 5.,
        },
        Color(10, 20, 30, 255),
    );
    outlier.series = "x".into();
    outlier.label.clear();
    outlier.value = -123.; // Deliberately not the accepted original sidecar.
    scene.marks = vec![body, line, outlier];
    SceneInteraction {
        scene,
        plot: None,
        interaction: Some(InteractionBinding {
            sites: vec![InteractionSite {
                mark_id: "body".into(),
                series: "x".into(),
                label: "business".into(),
                value: Some(5.),
                formatted_value: None,
                color: Color(10, 20, 30, 0),
                position: [60., 80.],
            }],
            ink_owners: [
                ("line".into(), "body".into()),
                ("outlier".into(), "body".into()),
            ]
            .into(),
            part_data: [(
                "outlier".into(),
                InteractionPartDatum {
                    source_id: "raw-99".into(),
                    value: 99.,
                },
            )]
            .into(),
            picking_profile: PickingProfile::PaintedGeometry,
            ..Default::default()
        }),
    }
}
fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut pending = tree.roots().to_vec();
    let mut out = vec![];
    while let Some(id) = pending.pop() {
        let node = tree.node(id).unwrap();
        if node.descriptor.view().classes.iter().any(|s| s == class) {
            out.push(id);
        }
        pending.extend(node.children.iter().copied());
    }
    out
}
fn pointer(app: &App, node: NodeId, phase: PlotPointerPhase, x: f64, y: f64) {
    app.dispatch(
        node,
        Event::PlotPointer {
            from: node,
            pointer: PlotPointer {
                pointer_id: 7,
                device: PlotPointerDevice::Pen,
                phase,
                x,
                y,
                viewport: Rect {
                    width: 400.,
                    height: 240.,
                    ..Default::default()
                },
            },
        },
    )
    .unwrap();
}

#[test]
fn real_fractional_plot_coordinates_route_parts_to_one_keyboard_site() {
    let app = App::memory();
    let physical = Rc::new(RefCell::new(vec![]));
    let logical = Rc::new(RefCell::new(vec![]));
    let seen = physical.clone();
    let hover = logical.clone();
    app.mount(
        chart_layout(frame())
            .highlight(false)
            .on_physical_hover(move |hit| seen.borrow_mut().push(hit))
            .on_hover(move |mark| hover.borrow_mut().push(mark.map(|m| m.id))),
    )
    .unwrap();
    let data = nodes(&app, "kartis-datum");
    assert_eq!(data.len(), 1);
    assert_eq!(
        app.tree()
            .node(data[0])
            .unwrap()
            .descriptor
            .view()
            .style
            .pointer_events,
        Some(PointerEvents::None)
    );
    app.dispatch(
        data[0],
        Event::Pointer {
            from: data[0],
            related: None,
            phase: PointerPhase::Enter,
        },
    )
    .unwrap();
    assert!(physical.borrow().is_empty());
    assert!(
        logical.borrow().is_empty(),
        "logical button bounds cannot bypass strict ink"
    );
    let plot = nodes(&app, "kartis-plot")[0];
    pointer(&app, plot, PlotPointerPhase::Move, 100.5, 40.);
    let first = physical.borrow()[0].clone().unwrap();
    assert_eq!(first.scene_point, [50.25, 20.]);
    assert_eq!(first.physical_mark_id, "line");
    assert_eq!(first.logical_site_id, "body");
    pointer(&app, plot, PlotPointerPhase::Move, 240., 60.);
    let second = physical.borrow()[1].clone().unwrap();
    assert_eq!(second.physical_mark_id, "outlier");
    assert_eq!(
        second.source_data.unwrap(),
        InteractionPartDatum {
            source_id: "raw-99".into(),
            value: 99.
        }
    );
    assert_eq!(*logical.borrow(), [Some("body".to_string())]);
    pointer(&app, plot, PlotPointerPhase::Move, 100., 45.);
    assert!(
        physical.borrow().last().unwrap().is_none(),
        "no legacy +2 stroke tolerance"
    );
    assert_eq!(logical.borrow().last(), Some(&None));
}

#[test]
fn group_zero_is_pointer_empty_but_retains_logical_keyboard_activation() {
    let app = App::memory();
    let mut input = frame();
    input.scene.opacity_groups.push(MarkOpacityGroup {
        id: "box".into(),
        members: vec!["body".into(), "line".into(), "outlier".into()],
        opacity: 0.,
    });
    let hits = Rc::new(RefCell::new(vec![]));
    let seen = hits.clone();
    let selected = Rc::new(RefCell::new(vec![]));
    let chosen = selected.clone();
    app.mount(
        chart_layout(input)
            .highlight(false)
            .on_physical_hover(move |hit| seen.borrow_mut().push(hit))
            .on_select(move |mark| chosen.borrow_mut().push(mark)),
    )
    .unwrap();
    let plot = nodes(&app, "kartis-plot")[0];
    pointer(&app, plot, PlotPointerPhase::Move, 100., 40.);
    assert!(hits.borrow().is_empty());
    let data = nodes(&app, "kartis-datum");
    assert_eq!(data.len(), 1);
    app.dispatch(data[0], Event::Activate).unwrap();
    assert_eq!(selected.borrow()[0].id, "body");
    assert_eq!(
        selected.borrow()[0].opacity,
        0.,
        "controls projection must not become source paint"
    );
}

#[test]
fn invalid_atomic_layout_keeps_old_plot_controls_and_accepted_source_sidecar() {
    let app = App::memory();
    let layout = app.scope().cage(frame());
    let rejected = Rc::new(RefCell::new(vec![]));
    let errors = rejected.clone();
    let hits = Rc::new(RefCell::new(vec![]));
    let observed = hits.clone();
    app.mount(
        chart_layout(layout)
            .highlight(false)
            .on_reject(move |error| errors.borrow_mut().push(error))
            .on_physical_hover(move |hit| observed.borrow_mut().push(hit)),
    )
    .unwrap();
    let plot = nodes(&app, "kartis-plot")[0];
    pointer(&app, plot, PlotPointerPhase::Move, 240., 60.);
    let token = hits
        .borrow()
        .last()
        .unwrap()
        .as_ref()
        .unwrap()
        .frame_token
        .clone();
    let tree = format!("{:?}", app.tree());
    layout.revise(|frame| {
        frame.scene.marks[1].geometry = Geometry::Circle {
            x: 80.,
            y: 40.,
            radius: 4.,
        };
        frame
            .interaction
            .as_mut()
            .unwrap()
            .ink_owners
            .insert("line".into(), "unknown".into());
    });
    app.flush().unwrap();
    assert_eq!(rejected.borrow().len(), 1);
    assert_eq!(format!("{:?}", app.tree()), tree);
    pointer(&app, plot, PlotPointerPhase::Move, 240.5, 60.);
    let hit = hits.borrow().last().unwrap().clone().unwrap();
    assert_eq!(hit.frame_token, token);
    assert_eq!(hit.source_data.unwrap().value, 99.);
    layout.revise(|input| {
        *input = frame();
        input
            .interaction
            .as_mut()
            .unwrap()
            .part_data
            .get_mut("outlier")
            .unwrap()
            .value = 101.;
    });
    app.flush().unwrap();
    pointer(&app, plot, PlotPointerPhase::Move, 240., 60.);
    let hit = hits.borrow().last().unwrap().clone().unwrap();
    assert_ne!(hit.frame_token, token);
    assert_eq!(hit.source_data.unwrap().value, 101.);
}

#[test]
fn hidden_replacement_clears_physical_hover_and_nullable_label_is_not_zero() {
    let app = App::memory();
    let hidden = app.scope().cage(BTreeSet::new());
    let mut input = frame();
    input.interaction.as_mut().unwrap().sites[0].value = None;
    let physical = Rc::new(RefCell::new(vec![]));
    let seen = physical.clone();
    app.mount(
        chart_layout(input)
            .hidden(hidden)
            .highlight(false)
            .on_physical_hover(move |hit| seen.borrow_mut().push(hit)),
    )
    .unwrap();
    let data = nodes(&app, "kartis-datum")[0];
    assert!(
        matches!(&app.tree().node(data).unwrap().descriptor.view().props, Props::Button(button) if button.label == "business: missing")
    );
    let plot = nodes(&app, "kartis-plot")[0];
    pointer(&app, plot, PlotPointerPhase::Move, 100., 40.);
    hidden.revise(|hidden| {
        hidden.insert("x".into());
    });
    app.flush().unwrap();
    assert!(physical.borrow().last().unwrap().is_none());
    assert!(nodes(&app, "kartis-datum").is_empty());
    hidden.revise(BTreeSet::clear);
    app.flush().unwrap();
    assert_eq!(nodes(&app, "kartis-datum").len(), 1);
    pointer(&app, plot, PlotPointerPhase::Move, 100., 40.);
    assert!(physical.borrow().last().unwrap().is_some());
}

#[test]
fn legacy_dynamic_data_pointer_switch_keeps_keyboard_but_guards_stale_pointer() {
    let app = App::memory();
    let enabled = app.scope().cage(true);
    let mut scene = frame().scene;
    scene.marks.truncate(1);
    scene.marks[0].opacity = 1.;
    scene.marks[0].fill = Some(Color(10, 20, 30, 255));
    let hover = Rc::new(RefCell::new(vec![]));
    let seen = hover.clone();
    let select = Rc::new(RefCell::new(vec![]));
    let chosen = select.clone();
    app.mount(
        chart(scene)
            .data_control_pointer(enabled)
            .on_hover(move |mark| seen.borrow_mut().push(mark.map(|m| m.id)))
            .on_select(move |mark| chosen.borrow_mut().push(mark.id)),
    )
    .unwrap();
    let data = nodes(&app, "kartis-datum")[0];
    enabled.revise(|enabled| *enabled = false);
    app.flush().unwrap();
    app.dispatch(
        data,
        Event::Pointer {
            from: data,
            related: None,
            phase: PointerPhase::Enter,
        },
    )
    .unwrap();
    assert!(hover.borrow().is_empty());
    app.dispatch(data, Event::Activate).unwrap();
    assert_eq!(*select.borrow(), ["body"]);
    enabled.revise(|enabled| *enabled = true);
    app.flush().unwrap();
    app.dispatch(
        data,
        Event::Pointer {
            from: data,
            related: None,
            phase: PointerPhase::Enter,
        },
    )
    .unwrap();
    assert_eq!(*hover.borrow(), [Some("body".to_string())]);
}

#[test]
fn touch_is_contact_input_and_does_not_leave_persistent_mouse_hover() {
    let app = App::memory();
    let hits = Rc::new(RefCell::new(vec![]));
    let seen = hits.clone();
    let selected = Rc::new(RefCell::new(vec![]));
    let chosen = selected.clone();
    app.mount(
        chart_layout(frame())
            .highlight(false)
            .on_physical_hover(move |hit| seen.borrow_mut().push(hit))
            .on_select(move |mark| chosen.borrow_mut().push(mark.id)),
    )
    .unwrap();
    let plot = nodes(&app, "kartis-plot")[0];
    let touch = |phase| {
        app.dispatch(
            plot,
            Event::PlotPointer {
                from: plot,
                pointer: PlotPointer {
                    pointer_id: 12,
                    device: PlotPointerDevice::Touch,
                    phase,
                    x: 100.,
                    y: 40.,
                    viewport: Rect {
                        width: 400.,
                        height: 240.,
                        ..Default::default()
                    },
                },
            },
        )
        .unwrap();
    };
    touch(PlotPointerPhase::Move);
    assert!(
        hits.borrow().is_empty(),
        "an unstarted touch is not mouse hover"
    );
    touch(PlotPointerPhase::Down);
    touch(PlotPointerPhase::Up);
    assert_eq!(*selected.borrow(), ["body"]);
    assert!(hits.borrow().iter().all(Option::is_none));
    touch(PlotPointerPhase::Down);
    touch(PlotPointerPhase::Cancel);
    touch(PlotPointerPhase::Up);
    assert_eq!(
        selected.borrow().len(),
        1,
        "cancelled contact cannot activate later"
    );
    pointer(&app, plot, PlotPointerPhase::Move, 100., 40.);
    assert!(
        hits.borrow().last().unwrap().is_some(),
        "touch cleanup leaves the mouse channel usable"
    );
}

#[test]
fn interactive_host_without_finite_coordinates_rejects_before_mount() {
    use glory::render::{self, Render};
    struct Unsupported(render::memory::Memory);
    impl Render for Unsupported {
        fn capabilities(&self) -> render::Capabilities {
            render::Capabilities {
                input: true,
                ..Default::default()
            }
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
            self.0.drain(inputs);
        }
    }
    let app = App::new(|runtime| Unsupported(render::memory::Memory::new(runtime)));
    assert!(app.mount(chart_layout(frame())).is_err());
    assert!(app.tree().roots().is_empty());
}

#[cfg(feature = "raster")]
mod raster {
    use std::cell::Cell;

    use glory::Widget;
    use kartis::render::raster::FontRegistry;
    use kartis::surface::SurfaceEvent;

    use super::*;

    fn mount(app: &App) -> RasterSurface {
        let installed = Rc::new(RefCell::new(None));
        let output = installed.clone();
        app.mount(glory::ui::widget(move |scope| {
            let surface = RasterSurface::mount_layout(
                scope,
                frame(),
                FontRegistry::new(),
                RasterSurfaceOptions {
                    highlight: false,
                    ..Default::default()
                },
            )
            .unwrap();
            let widget = surface.widget();
            *output.borrow_mut() = Some(surface);
            widget.show_in(scope)
        }))
        .unwrap();
        installed.borrow_mut().take().unwrap()
    }

    #[test]
    fn raster_actual_image_view_routes_parts_and_dynamic_profile_retires_interest() {
        let app = App::memory();
        let surface = mount(&app);
        let events = Rc::new(RefCell::new(vec![]));
        let seen = events.clone();
        surface
            .on_event(move |batch| seen.borrow_mut().extend(batch))
            .unwrap();
        let handle = surface.handle();
        let image = nodes(&app, "kartis-raster-brush-image-host")[0];
        assert_eq!(nodes(&app, "kartis-datum").len(), 1);
        pointer(&app, image, PlotPointerPhase::Move, 240., 60.);
        assert_eq!(handle.snapshot().unwrap().hovered, ["body"]);
        let hit = events
            .borrow()
            .iter()
            .find_map(|event| match event {
                SurfaceEvent::PhysicalHover(Some(hit)) => Some(hit.clone()),
                _ => None,
            })
            .unwrap();
        assert_eq!(hit.physical_mark_id, "outlier");
        assert_eq!(hit.source_data.unwrap().value, 99.);
        let mut legacy = frame();
        let binding = legacy.interaction.as_mut().unwrap();
        binding.picking_profile = PickingProfile::LegacyGeometry;
        binding.ink_owners.clear();
        binding.part_data.clear();
        handle.set_layout(legacy).unwrap();
        app.flush().unwrap();
        assert!(
            !app.dispatch(
                image,
                Event::PlotPointer {
                    from: image,
                    pointer: PlotPointer {
                        pointer_id: 7,
                        device: PlotPointerDevice::Mouse,
                        phase: PlotPointerPhase::Move,
                        x: 100.,
                        y: 40.,
                        viewport: Rect {
                            width: 400.,
                            height: 240.,
                            ..Default::default()
                        },
                    }
                }
            )
            .unwrap(),
            "legacy image must not retain strict coordinate interest"
        );
        handle.set_layout(frame()).unwrap();
        app.flush().unwrap();
        pointer(&app, image, PlotPointerPhase::Move, 100., 40.);
        assert_eq!(handle.snapshot().unwrap().hovered, ["body"]);
        let before = handle.snapshot().unwrap();
        let pixels = handle.frame().unwrap();
        let mut invalid = frame();
        invalid
            .interaction
            .as_mut()
            .unwrap()
            .ink_owners
            .insert("line".into(), "unknown".into());
        assert!(handle.set_layout(invalid).is_err());
        assert_eq!(handle.snapshot().unwrap(), before);
        assert!(Rc::ptr_eq(&pixels, &handle.frame().unwrap()));
        drop(surface);
        app.flush().unwrap();
        assert!(handle.snapshot().is_err());
    }

    #[test]
    fn strict_editable_brush_rejects_before_image_publication() {
        let app = App::memory();
        let count = Rc::new(Cell::new(0));
        let published = count.clone();
        let result = RasterSurface::mount_layout_published(
            app.scope(),
            frame(),
            FontRegistry::new(),
            RasterSurfaceOptions {
                editable_brush: Some(EditableBrushOptions::default()),
                ..Default::default()
            },
            move |image| {
                published.set(published.get() + 1);
                image.data_url()
            },
        );
        assert!(result.is_err());
        assert_eq!(count.get(), 0);
    }

    #[test]
    fn raster_touch_cancels_contacts_without_persistent_hover() {
        let app = App::memory();
        let surface = mount(&app);
        let handle = surface.handle();
        let image = nodes(&app, "kartis-raster-brush-image-host")[0];
        for phase in [PlotPointerPhase::Down, PlotPointerPhase::Cancel] {
            app.dispatch(
                image,
                Event::PlotPointer {
                    from: image,
                    pointer: PlotPointer {
                        pointer_id: 5,
                        device: PlotPointerDevice::Touch,
                        phase,
                        x: 100.,
                        y: 40.,
                        viewport: Rect {
                            width: 400.,
                            height: 240.,
                            ..Default::default()
                        },
                    },
                },
            )
            .unwrap();
        }
        let snapshot = handle.snapshot().unwrap();
        assert_eq!(snapshot.touches, 0);
        assert!(!snapshot.dragging);
        assert!(snapshot.hovered.is_empty());
    }
}
