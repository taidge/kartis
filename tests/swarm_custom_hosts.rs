//! Actual Glory Memory controls and native raster publication, not a browser
//! substitute. Browsers have an independent trusted-input acceptance script.
use std::cell::RefCell;
use std::rc::Rc;

use glory::App;
use glory::render::{
    Event, GraphicPoint as P, NodeId, PathCommand as C, PlotPointer, PlotPointerDevice,
    PlotPointerPhase, Props, Rect,
};
use kartis::*;

fn build(phase: bool) -> SwarmLayout {
    candidate(phase, 0).unwrap()
}
fn candidate(phase: bool, invalid: u8) -> Result<SwarmLayout> {
    let mut data = vec![
        SwarmPoint::new("target", "G", if phase { 6. } else { 4. }),
        SwarmPoint::new("zero", "G", 8.),
    ];
    data[0].label = Some("Target <original>".into());
    data[1].size = Some(0.);
    if phase {
        data.reverse();
    }
    swarm_layout_with_node_factory(
        &ChartConfig {
            width: 320.,
            height: 180.,
            margin: Margin {
                left: 10.,
                right: 10.,
                top: 10.,
                bottom: 10.,
            },
            ..Default::default()
        },
        &data,
        &SwarmLayoutOptions {
            orientation: SwarmOrientation::Horizontal,
            value_domain: Some([0., 10.]),
            value_axis: None,
            group_axis: None,
            ..Default::default()
        },
        |ctx| {
            if invalid == 1 {
                return Err(ChartError("semantic factory rejection".into()));
            }
            if invalid == 2 {
                let mut ink = ctx.inherit(ctx.geometry.clone());
                ink.opacity = f64::NAN;
                return Ok(ink);
            }
            if ctx.datum.id == "zero" {
                return Ok(ctx.inherit(ctx.geometry.clone()));
            }
            let [x, y] = ctx.position;
            let mut ink = ctx.inherit(Geometry::Path(vec![
                C::Move(P::new(x + 20., y - 8.)),
                C::Line(P::new(x + 36., y - 8.)),
                C::Line(P::new(x + 36., y + 8.)),
                C::Line(P::new(x + 20., y + 8.)),
                C::Close,
            ]));
            ink.fill = Some(if phase {
                Color(190, 30, 80, 255)
            } else {
                Color(30, 90, 190, 255)
            });
            Ok(ink)
        },
    )
}
fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut pending = tree.roots().to_vec();
    let mut found = vec![];
    while let Some(id) = pending.pop() {
        let node = tree.node(id).unwrap();
        if node.descriptor.view().classes.iter().any(|c| c == class) {
            found.push(id);
        }
        pending.extend(node.children.iter().copied());
    }
    found
}
fn control(app: &App) -> NodeId {
    *nodes(app,"kartis-datum").iter().find(|id|matches!(app.tree().node(**id).unwrap().descriptor.view().props,Props::Button(p) if p.label.contains("Target <original>"))).unwrap()
}
fn pointer(app: &App, point: [f64; 2]) {
    let plot = nodes(app, "kartis-plot")[0];
    app.dispatch(
        plot,
        Event::PlotPointer {
            from: plot,
            pointer: PlotPointer {
                pointer_id: 4,
                device: PlotPointerDevice::Pen,
                phase: PlotPointerPhase::Move,
                x: point[0] * 2.,
                y: point[1] * 2.,
                viewport: Rect {
                    width: 640.,
                    height: 360.,
                    ..Default::default()
                },
            },
        },
    )
    .unwrap();
    app.flush().unwrap();
}

#[test]
fn semantic_memory_keeps_source_controls_offset_ink_and_atomic_rejection() {
    let app = App::memory();
    let old = build(false);
    let next = build(true);
    let target = old.sites.iter().find(|s| s.node_id == "target").unwrap();
    let input = app
        .scope()
        .cage(old.clone().into_painted_scene_interaction().unwrap());
    let seen = Rc::new(RefCell::new(vec![]));
    let captured = seen.clone();
    let selected = Rc::new(RefCell::new(vec![]));
    let chosen = selected.clone();
    let rejects = Rc::new(RefCell::new(vec![]));
    let errors = rejects.clone();
    app.mount(
        chart_layout(input)
            .highlight(false)
            .on_hover(move |mark| captured.borrow_mut().push(mark))
            .on_select(move |mark| chosen.borrow_mut().push((mark.id, mark.value)))
            .on_reject(move |error| errors.borrow_mut().push(error)),
    )
    .unwrap_or_else(|error| {
        panic!(
            "semantic mount: {error:?}; rejected: {:?}",
            rejects.borrow()
        )
    });
    assert_eq!(nodes(&app, "kartis-datum").len(), 2);
    let button = control(&app);
    pointer(&app, target.position);
    assert!(seen.borrow().last().is_none_or(Option::is_none));
    pointer(&app, [target.position[0] + 28., target.position[1]]);
    let hit = seen.borrow().last().unwrap().clone().unwrap();
    assert_eq!(hit.id, target.mark_id);
    assert_eq!(hit.value, 4.);
    assert!(matches!(hit.geometry, Geometry::Path(_)));
    app.dispatch(button, Event::Activate).unwrap();
    assert_eq!(*selected.borrow(), [(target.mark_id.clone(), 4.)]);
    let accepted = format!("{:?}", app.tree());
    let accepted_input = input.get();
    for invalid in [1, 2] {
        let publish = || -> Result<()> {
            let proposed = candidate(true, invalid)?.into_painted_scene_interaction()?;
            input.revise(|frame| *frame = proposed);
            Ok(())
        };
        assert!(publish().is_err());
        app.flush().unwrap();
        assert_eq!(input.get(), accepted_input);
        assert_eq!(format!("{:?}", app.tree()), accepted);
        assert_eq!(control(&app), button);
    }
    let mut invalid = next.clone().into_painted_scene_interaction().unwrap();
    invalid.interaction.as_mut().unwrap().sites[0].position[0] += 1.;
    input.revise(|frame| *frame = invalid);
    app.flush().unwrap();
    assert_eq!(rejects.borrow().len(), 1);
    assert_eq!(format!("{:?}", app.tree()), accepted);
    input.revise(|frame| *frame = next.clone().into_painted_scene_interaction().unwrap());
    app.flush().unwrap();
    assert_eq!(control(&app), button);
    let target = next.sites.iter().find(|s| s.node_id == "target").unwrap();
    pointer(&app, [target.position[0] + 28., target.position[1]]);
    assert_eq!(seen.borrow().last().unwrap().as_ref().unwrap().value, 6.);
    app.dispatch(button, Event::Activate).unwrap();
    assert_eq!(
        selected.borrow().last(),
        Some(&(target.mark_id.clone(), 6.))
    );
    assert_eq!(next.metadata[target.source_index].raw_value, 6.);
}

#[cfg(feature = "raster")]
#[path = "../examples/shared/swarm_custom.rs"]
mod paint_model;

#[cfg(feature = "raster")]
#[test]
fn raster_real_gradient_offset_ink_and_controls_roll_back_with_publisher() {
    use std::cell::Cell;

    use kartis::render::raster::{FontRegistry, RasterOptions};
    for dpr in [1., 2.] {
        let app = App::memory();
        let old = paint_model::build(640., 360., false, 0, 0).unwrap();
        let next = paint_model::build(640., 360., false, 1, 0).unwrap();
        let rejecting = Rc::new(Cell::new(false));
        let reject = rejecting.clone();
        let surface = RasterSurface::mount_layout_published(
            app.scope(),
            old.clone().into(),
            FontRegistry::new(),
            RasterSurfaceOptions {
                highlight: false,
                raster: RasterOptions {
                    pixel_ratio: dpr,
                    ..Default::default()
                },
                ..Default::default()
            },
            move |image| {
                if reject.get() {
                    Err(ChartError("custom Swarm publisher rejection".into()))
                } else {
                    image.data_url()
                }
            },
        )
        .unwrap();
        app.mount(surface.widget()).unwrap();
        let handle = surface.handle();
        let controls = nodes(&app, "kartis-datum");
        assert_eq!(controls.len(), old.sites.len());
        let site = old.sites.iter().find(|s| s.node_id == "offset").unwrap();
        let p = paint_model::ink_probe(site);
        handle
            .dispatch(surface::SurfaceInput::Move {
                id: 4,
                point: (p[0], p[1]),
            })
            .unwrap();
        assert_eq!(
            handle.active_interaction().unwrap().unwrap().ids,
            std::slice::from_ref(&site.mark_id)
        );
        let image = handle.frame().unwrap();
        let i = ((p[1] * dpr).floor() as usize * image.width as usize
            + (p[0] * dpr).floor() as usize)
            * 4;
        assert_eq!(&image.rgba[i..i + 4], &[32, 100, 190, 255]);
        let accepted = handle.snapshot().unwrap();
        let accepted_binding = handle.interaction_data().unwrap();
        let tree = format!("{:?}", app.tree());
        rejecting.set(true);
        assert!(handle.set_layout(next.clone().into()).is_err());
        app.flush().unwrap();
        assert_eq!(handle.snapshot().unwrap(), accepted);
        assert_eq!(handle.interaction_data().unwrap(), accepted_binding);
        assert!(Rc::ptr_eq(&handle.frame().unwrap(), &image));
        assert_eq!(format!("{:?}", app.tree()), tree);
        assert_eq!(nodes(&app, "kartis-datum"), controls);
        rejecting.set(false);
        handle.set_layout(next.clone().into()).unwrap();
        app.flush().unwrap();
        assert_ne!(handle.frame().unwrap().rgba, image.rgba);
        assert_eq!(nodes(&app, "kartis-datum").len(), next.sites.len());
        let added = next.sites.iter().find(|s| s.node_id == "added").unwrap();
        assert!(
            handle
                .interaction_data()
                .unwrap()
                .unwrap()
                .sites
                .iter()
                .any(|s| s.mark_id == added.mark_id)
        );
        handle.stop();
        assert!(handle.snapshot().is_err());
    }
}
