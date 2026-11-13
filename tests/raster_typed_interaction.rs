#![cfg(feature = "raster")]
use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use glory::{
    App,
    render::{NodeId, Props},
};
use kartis::{
    render::raster::FontRegistry,
    stack::StackOptions,
    surface::{Picking, SurfaceInput},
    *,
};
use std::{cell::Cell, rc::Rc};

fn stream(value: f64, second: bool) -> StreamLayout {
    let c = ChartConfig {
        width: 228.,
        height: 192.,
        margin: Margin {
            left: 11.,
            right: 17.,
            top: 13.,
            bottom: 19.,
        },
        ..Default::default()
    };
    let mut options = StreamLayoutOptions {
        stack: StackOptions::default(),
        curve: Curve::Linear,
        dots: Some(StreamDotOptions::default()),
        y_bounds: [Some(0.), Some(60.)],
        y_scale: ContinuousScaleOptions::default(),
        style: StreamLayerStyle {
            fill_opacity: 0.,
            ..Default::default()
        },
        ..Default::default()
    };
    options.axes.x.visible = false;
    options.axes.x.grid = false;
    options.axes.y.visible = false;
    options.axes.y.grid = false;
    let mut data = vec![BarSeries::new("A", vec![Some(10.), Some(value), Some(30.)])];
    if second {
        data.push(BarSeries::new("B", vec![Some(16.), None, Some(14.)]));
    }
    stream_layout_with_dot_factory(
        &c,
        &["first".into(), "middle".into(), "last".into()],
        &data,
        &options,
        |ctx| {
            let [x, y] = ctx.position;
            let mut geometry = ctx.inherit(Geometry::Path(vec![
                C::Move(P::new(x + 12., y - 6.)),
                C::Line(P::new(x + 24., y - 6.)),
                C::Line(P::new(x + 24., y + 6.)),
                C::Line(P::new(x + 12., y + 6.)),
                C::Close,
            ]));
            geometry.fill = None;
            geometry.stroke = Some(GraphicStroke {
                color: Color(165, 30, 91, 255),
                width: 3.,
                cap: StrokeCap::Butt,
            });
            Ok(geometry)
        },
    )
    .unwrap()
}

fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut pending = tree.roots().to_vec();
    let mut result = vec![];
    while let Some(id) = pending.pop() {
        let node = tree.node(id).unwrap();
        if node
            .descriptor
            .view()
            .classes
            .iter()
            .any(|name| name == class)
        {
            result.push(id);
        }
        pending.extend(node.children.iter().copied());
    }
    result.reverse();
    result
}

fn tooltip(app: &App) -> String {
    let ids = nodes(app, "kartis-tooltip");
    let tree = app.tree();
    ids.iter()
        .filter_map(|id| match tree.node(*id).unwrap().descriptor.view().props {
            Props::Text(text) => Some(text.content.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn native_raster_custom_path_preserves_original_anchor_and_rejects_publication_atomically() {
    let app = App::memory();
    let layout = stream(20., false);
    let sample = layout.layers[0].samples[1].clone();
    let reject = Rc::new(Cell::new(false));
    let failed = reject.clone();
    let surface = RasterSurface::mount_layout_published(
        app.scope(),
        layout.into_slice_interaction().unwrap(),
        FontRegistry::new(),
        RasterSurfaceOptions {
            highlight: false,
            ..Default::default()
        },
        move |image| {
            if failed.get() {
                Err(ChartError("reject stream frame".into()))
            } else {
                image.data_url()
            }
        },
    )
    .unwrap();
    app.mount(surface.widget()).unwrap();
    let handle = surface.handle();
    let [x, y] = sample.position;
    handle
        .dispatch(SurfaceInput::Move {
            id: 1,
            point: (x, y),
        })
        .unwrap();
    assert!(handle.snapshot().unwrap().hovered.is_empty());
    handle
        .dispatch_host(
            SurfaceInput::Move {
                id: 1,
                point: (80. + (x + 24.) * 2., 50. + y * 2.),
            },
            RasterSurfaceBounds {
                x: 80.,
                y: 50.,
                width: 456.,
                height: 384.,
            },
        )
        .unwrap();
    let active = handle.active_interaction().unwrap().unwrap();
    assert_eq!(active.ids, [sample.mark_id]);
    assert_eq!(active.anchor, sample.position);
    assert_eq!(active.rows[0].value, Some(20.));
    assert_eq!(active.rows[0].color, Color(165, 30, 91, 255));
    let before = handle.snapshot().unwrap();
    let binding = handle.interaction_data().unwrap();
    let image = handle.frame().unwrap();
    let controls = nodes(&app, "kartis-datum");
    let next = stream(24., false);
    let next_sample = next.layers[0].samples[1].clone();
    let mut next = next.into_slice_interaction().unwrap();
    next.plot.as_mut().unwrap().width = 190.;
    reject.set(true);
    assert!(handle.set_layout(next.clone()).is_err());
    app.flush().unwrap();
    assert_eq!(handle.snapshot().unwrap(), before);
    assert_eq!(handle.interaction_data().unwrap(), binding);
    assert!(Rc::ptr_eq(&image, &handle.frame().unwrap()));
    assert_eq!(nodes(&app, "kartis-datum"), controls);
    reject.set(false);
    handle.set_layout(next).unwrap();
    handle
        .dispatch(SurfaceInput::Move {
            id: 1,
            point: (next_sample.position[0] + 24., next_sample.position[1]),
        })
        .unwrap();
    let active = handle.active_interaction().unwrap().unwrap();
    assert_eq!(active.anchor, next_sample.position);
    assert_eq!(active.rows[0].value, Some(24.));
    surface.stop();
    app.flush().unwrap();
    assert!(!handle.is_active());
    assert!(handle.interaction_data().is_err());
    assert!(nodes(&app, "kartis-datum").is_empty());
}

#[test]
fn metadata_only_stream_slice_update_refreshes_tooltip_and_keeps_png_and_nullable_rows() {
    let app = App::memory();
    let layout = stream(20., true);
    let sample = layout.layers[0].samples[1].clone();
    let candidate = layout.into_slice_interaction().unwrap();
    let publications = Rc::new(Cell::new(0));
    let published = publications.clone();
    let surface = RasterSurface::mount_layout_published(
        app.scope(),
        candidate.clone(),
        FontRegistry::new(),
        RasterSurfaceOptions {
            interaction: surface::SurfaceOptions {
                picking: Picking::SliceX { distance: 1. },
                ..Default::default()
            },
            tooltip: TooltipOptions {
                missing: MissingValue::Placeholder("missing".into()),
                ..Default::default()
            },
            highlight: false,
            ..Default::default()
        },
        move |image| {
            published.set(published.get() + 1);
            image.data_url()
        },
    )
    .unwrap();
    app.mount(surface.widget()).unwrap();
    let handle = surface.handle();
    handle
        .dispatch(SurfaceInput::Move {
            id: 1,
            point: (sample.position[0], 14.),
        })
        .unwrap();
    app.flush().unwrap();
    let active = handle.active_interaction().unwrap().unwrap();
    assert_eq!(active.rows.len(), 2);
    assert!(
        active
            .rows
            .iter()
            .any(|row| row.series == "B" && row.value.is_none())
    );
    let old_tooltip = tooltip(&app);
    assert!(old_tooltip.contains("B · middle: missing") && old_tooltip.contains("A · middle: 20"));
    let controls = nodes(&app, "kartis-datum");
    let frame = handle.frame().unwrap();
    assert_eq!(publications.get(), 1);
    let mut changed = candidate;
    let site = changed
        .interaction
        .as_mut()
        .unwrap()
        .sites
        .iter_mut()
        .find(|site| site.mark_id == sample.mark_id)
        .unwrap();
    site.label = "updated source A".into();
    site.formatted_value = Some("twenty units".into());
    handle.set_layout(changed.clone()).unwrap();
    app.flush().unwrap();
    assert!(Rc::ptr_eq(&frame, &handle.frame().unwrap()));
    assert_eq!(publications.get(), 1);
    assert_eq!(nodes(&app, "kartis-datum"), controls);
    let updated = tooltip(&app);
    assert!(
        updated.contains("updated source A: twenty units"),
        "metadata-only tooltip stayed stale: {updated}"
    );
    assert!(updated.contains("B · middle: missing"));
    assert!(
        handle
            .active_interaction()
            .unwrap()
            .unwrap()
            .rows
            .iter()
            .any(|row| row.series == "B" && row.value.is_none())
    );
    let site = changed
        .interaction
        .as_mut()
        .unwrap()
        .sites
        .iter_mut()
        .find(|site| site.series == "B" && site.value.is_none())
        .unwrap();
    site.value = Some(0.);
    site.formatted_value = Some("zero units".into());
    handle.set_layout(changed).unwrap();
    app.flush().unwrap();
    assert!(Rc::ptr_eq(&frame, &handle.frame().unwrap()));
    assert_eq!(publications.get(), 1);
    assert_eq!(nodes(&app, "kartis-datum"), controls);
    let text = tooltip(&app);
    assert!(text.contains("B · middle: zero units") && !text.contains("B · middle: missing"));
    assert!(
        handle
            .active_interaction()
            .unwrap()
            .unwrap()
            .rows
            .iter()
            .any(|row| row.series == "B" && row.value == Some(0.))
    );
    let tree = app.tree();
    assert!(controls.iter().any(
        |id| matches!(tree.node(*id).unwrap().descriptor.view().props,
        Props::Button(button) if button.label == "B · middle: 0")
    ));
}
