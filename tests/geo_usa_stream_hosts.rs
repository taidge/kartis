//! Real Glory Memory and native CPU hosts, with actual finite plot events and
//! publication transactions. These tests do not stand in for browser execution.
#[cfg(feature = "raster")]
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;

use glory::App;
use glory::render::{
    Event, GraphicStroke, NodeId, PathCommand, PlotPointer, PlotPointerDevice, PlotPointerPhase,
    Props, Rect, StrokeCap,
};
use kartis::*;

const INK: Color = Color(230, 20, 80, 255);
const NEXT_INK: Color = Color(20, 190, 240, 255);
const OWNER: &str = "geo-line:route";

fn config() -> ChartConfig {
    ChartConfig {
        width: 640.,
        height: 400.,
        margin: Margin {
            left: 20.,
            right: 20.,
            top: 20.,
            bottom: 20.,
        },
        ..Default::default()
    }
}
fn source(value: Option<f64>, shift: f64) -> Vec<GeoLineFeature> {
    let invisible = GeoLineFeature::new("invisible-before", vec![[1., 48.], [3., 49.]]);
    let mut route = GeoLineFeature::new("route", vec![]);
    route.label.clear();
    route.series.clear();
    route.value = value;
    route.lines = vec![
        vec![[-124. + shift, 38.], [-119. + shift, 38.]],
        vec![[-152. + shift, 61.], [-146. + shift, 61.]],
        vec![[-159. + shift, 21.], [-156. + shift, 21.]],
    ];
    vec![invisible, route]
}
fn map_source() -> Vec<GeoFeature> {
    [[-122., 38.], [-149., 61.], [-157., 21.]]
        .into_iter()
        .enumerate()
        .map(|(i, [x, y])| GeoFeature {
            id: format!("map-{i}"),
            label: format!("map region {i}"),
            value: Some(i as f64),
            polygons: vec![vec![vec![
                [x - 4., y - 2.],
                [x - 4., y + 2.],
                [x + 4., y + 2.],
                [x + 4., y - 2.],
                [x - 4., y - 2.],
            ]]],
        })
        .collect()
}
fn interior(g: &Geometry) -> [f64; 2] {
    let Geometry::Path(commands) = g else {
        panic!("line part must be a path")
    };
    let points = commands
        .iter()
        .map(|c| match c {
            PathCommand::Move(p) | PathCommand::Line(p) => [p.x, p.y],
            _ => panic!("line is open"),
        })
        .collect::<Vec<_>>();
    let segment = points
        .windows(2)
        .max_by(|a, b| {
            ((a[0][0] - a[1][0]).hypot(a[0][1] - a[1][1]))
                .total_cmp(&(b[0][0] - b[1][0]).hypot(b[0][1] - b[1][1]))
        })
        .unwrap();
    [
        (segment[0][0] + segment[1][0]) / 2.,
        (segment[0][1] + segment[1][1]) / 2.,
    ]
}
struct Authored {
    source: Vec<GeoLineFeature>,
    metadata: GeoUsaStreamMetadata,
    frame: SceneInteraction,
    grid_frame: SceneInteraction,
    probes: Vec<(GeoUsaRegion, String, [f64; 2])>,
}
fn authored(fit: GeoFit, value: Option<f64>, shift: f64, color: Color) -> Authored {
    let mut map = geographic_usa_map_layout(
        &config(),
        &map_source(),
        &GeoUsaMapOptions {
            fit,
            padding: 8.,
            fill: Some(Color(235, 238, 241, 255)),
            border: None,
            path: GeoPathOptions {
                precision: 0.02,
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    let graticule = map
        .add_usa_graticule(
            &GeoGraticuleOptions {
                step_major: [90., 45.],
                step_minor: [30., 20.],
                ..Default::default()
            },
            &GeoStreamOptions {
                style: GeoLineStyle {
                    stroke: GraphicStroke {
                        color: Color(130, 135, 140, 255),
                        width: 1.,
                        cap: StrokeCap::Butt,
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();
    let grid_binding = graticule.interaction_binding(&map.scene).unwrap();
    assert!(grid_binding.sites.is_empty());
    assert!(grid_binding.ink_owners.is_empty());
    let grid_frame = graticule.into_scene_interaction(map.scene.clone()).unwrap();
    let source = source(value, shift);
    let before = source.clone();
    let metadata = map
        .add_usa_lines(
            &source,
            &GeoStreamOptions {
                style: GeoLineStyle {
                    stroke: GraphicStroke {
                        color,
                        width: 8.,
                        cap: StrokeCap::Round,
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(source, before);
    assert_eq!(metadata.lines.len(), 2);
    assert_eq!(metadata.lines[0].source_index, 0);
    assert!(metadata.lines[0].parts.is_empty());
    let row = &metadata.lines[1];
    assert_eq!(row.source_index, 1);
    assert_eq!(row.source_series, "");
    assert_eq!(row.series, "route");
    assert_eq!(row.label, "");
    assert_eq!(row.value, value);
    assert_eq!(
        row.parts.iter().map(|p| p.region).collect::<Vec<_>>(),
        GeoUsaRegion::ALL
    );
    let probes = row
        .parts
        .iter()
        .map(|p| (p.region, p.mark_id.clone(), interior(&p.geometry)))
        .collect();
    let mut frame = metadata.clone().into_scene_interaction(map.scene).unwrap();
    let binding = frame.interaction.as_ref().unwrap();
    assert_eq!(binding.sites.len(), 1);
    assert_eq!(binding.ink_owners.len(), 2);
    assert_eq!(binding.sites[0].label, "route");
    assert_eq!(binding.sites[0].value, value);
    // A nullable accepted sidecar is valid independently of a live Mark scalar.
    // Exercise that host contract instead of accidentally passing because both
    // values happen to be zero. Metadata remains the original authored source.
    if value.is_none() {
        for mark in frame.scene.marks.iter_mut().filter(|m| m.series == "route") {
            mark.value = -913.;
        }
    }
    frame.validate().unwrap();
    Authored {
        source,
        metadata,
        frame,
        grid_frame,
        probes,
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
fn button(app: &App) -> String {
    let controls = nodes(app, "kartis-datum");
    assert_eq!(controls.len(), 1);
    match app
        .tree()
        .node(controls[0])
        .unwrap()
        .descriptor
        .view()
        .props
    {
        Props::Button(p) => p.label.clone(),
        _ => panic!("expected semantic datum button"),
    }
}
fn tooltip(app: &App) -> String {
    let tree = app.tree();
    nodes(app, "kartis-tooltip")
        .iter()
        .filter_map(|id| match tree.node(*id).unwrap().descriptor.view().props {
            Props::Text(p) => Some(p.content.clone()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}
fn memory_move(app: &App, p: [f64; 2]) {
    let plot = nodes(app, "kartis-plot")[0];
    app.dispatch(
        plot,
        Event::PlotPointer {
            from: plot,
            pointer: PlotPointer {
                pointer_id: 9,
                device: PlotPointerDevice::Pen,
                phase: PlotPointerPhase::Move,
                x: p[0] * 1.5,
                y: p[1] * 1.5,
                viewport: Rect {
                    width: 960.,
                    height: 600.,
                    ..Default::default()
                },
            },
        },
    )
    .unwrap();
    app.flush().unwrap();
}

#[test]
fn real_memory_plot_aliases_share_one_nullable_control_and_accept_atomic_source_updates() {
    for fit in [GeoFit::World, GeoFit::Data] {
        let app = App::memory();
        let old = authored(fit, None, 0., INK);
        let next = authored(fit, Some(42.), 0.7, NEXT_INK);
        let input = app.scope().cage(old.grid_frame.clone());
        let physical = Rc::new(RefCell::new(vec![]));
        let seen = physical.clone();
        let selected = Rc::new(RefCell::new(vec![]));
        let chosen = selected.clone();
        let rejected = Rc::new(RefCell::new(vec![]));
        let errors = rejected.clone();
        app.mount(
            chart_layout(input)
                .highlight(false)
                .tooltip_options(TooltipOptions {
                    missing: MissingValue::Placeholder("missing".into()),
                    ..Default::default()
                })
                .on_physical_hover(move |hit| seen.borrow_mut().push(hit))
                .on_select(move |mark| chosen.borrow_mut().push(mark.id))
                .on_reject(move |e| errors.borrow_mut().push(e)),
        )
        .unwrap();
        assert!(
            nodes(&app, "kartis-datum").is_empty(),
            "graticule must remain noninteractive on the map scene"
        );
        input.revise(|frame| *frame = old.frame.clone());
        app.flush().unwrap();
        assert_eq!(button(&app), "route: missing");
        for (_, physical_id, p) in &old.probes {
            memory_move(&app, *p);
            let hit = physical.borrow().last().unwrap().clone().unwrap();
            assert_eq!(hit.physical_mark_id, *physical_id);
            assert_eq!(hit.logical_site_id, OWNER);
            assert_eq!(hit.source_data, None);
            assert_eq!(tooltip(&app), "route: missing");
        }
        app.dispatch(nodes(&app, "kartis-datum")[0], Event::Activate)
            .unwrap();
        assert_eq!(*selected.borrow(), [OWNER]);
        let accepted_tree = format!("{:?}", app.tree());
        let accepted_token = physical
            .borrow()
            .last()
            .unwrap()
            .as_ref()
            .unwrap()
            .frame_token
            .clone();
        let mut bad = next.frame.clone();
        bad.interaction
            .as_mut()
            .unwrap()
            .ink_owners
            .insert(next.probes[1].1.clone(), "missing-owner".into());
        input.revise(|frame| *frame = bad);
        app.flush().unwrap();
        assert_eq!(rejected.borrow().len(), 1);
        assert_eq!(format!("{:?}", app.tree()), accepted_tree);
        memory_move(&app, old.probes[0].2);
        assert_eq!(
            physical
                .borrow()
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .frame_token,
            accepted_token
        );
        assert_eq!(tooltip(&app), "route: missing");
        input.revise(|frame| *frame = next.frame.clone());
        app.flush().unwrap();
        assert_eq!(button(&app), "route: 42");
        for (_, physical_id, p) in &next.probes {
            memory_move(&app, *p);
            let hit = physical.borrow().last().unwrap().clone().unwrap();
            assert_eq!(hit.logical_site_id, OWNER);
            assert_eq!(hit.physical_mark_id, *physical_id);
            assert_ne!(hit.frame_token, accepted_token);
            assert_eq!(tooltip(&app), "route: 42");
        }
        assert_eq!(old.source[1].value, None);
        assert_eq!(old.metadata.lines[1].value, None);
        assert_eq!(next.source[1].value, Some(42.));
        assert_eq!(next.metadata.lines[1].source_index, 1);
    }
}

#[test]
fn memory_metadata_only_nullable_zero_refreshes_stationary_tooltip_with_identical_scene() {
    use kartis::surface::{SurfaceModel, SurfaceOptions};
    let app = App::memory();
    let mut authored = authored(GeoFit::World, None, 0., INK);
    for mark in authored
        .frame
        .scene
        .marks
        .iter_mut()
        .filter(|m| m.series == "route")
    {
        mark.value = 0.;
    }
    let missing = authored.frame;
    let mut zero = missing.clone();
    let site = &mut zero.interaction.as_mut().unwrap().sites[0];
    site.value = Some(0.);
    site.formatted_value = Some("accepted-zero".into());
    missing.validate().unwrap();
    zero.validate().unwrap();
    assert_eq!(
        missing.scene, zero.scene,
        "source metadata is the only changed input"
    );
    let model = |frame: &SceneInteraction| {
        SurfaceModel::new_with_interaction(
            frame.scene.clone(),
            SurfaceOptions {
                plot: frame.plot,
                ..Default::default()
            },
            frame.interaction.clone(),
        )
        .unwrap()
    };
    assert_eq!(
        model(&missing).controls_scene(),
        model(&zero).controls_scene(),
        "equal scalar controls cannot invalidate the default typed tooltip by themselves"
    );
    let input = app.scope().cage(missing.clone());
    app.mount(
        chart_layout(input)
            .highlight(false)
            .tooltip_options(TooltipOptions {
                missing: MissingValue::Placeholder("missing".into()),
                ..Default::default()
            }),
    )
    .unwrap();
    memory_move(&app, authored.probes[1].2);
    assert_eq!(button(&app), "route: missing");
    assert_eq!(tooltip(&app), "route: missing");
    input.revise(|frame| *frame = zero.clone());
    app.flush().unwrap();
    assert_eq!(
        tooltip(&app),
        "route: accepted-zero",
        "stationary hover must observe the accepted source revision without another pointer event"
    );
    assert_eq!(button(&app), "route: 0");
    assert_eq!(missing.interaction.unwrap().sites[0].value, None);
    assert_eq!(zero.interaction.unwrap().sites[0].value, Some(0.));
}

#[cfg(feature = "raster")]
fn image_source(app: &App) -> (String, u64) {
    let images = nodes(app, "kartis-raster-surface-image");
    assert_eq!(images.len(), 1);
    match app.tree().node(images[0]).unwrap().descriptor.view().props {
        Props::Image(p) => (p.source.clone(), p.revision),
        _ => panic!("expected published image"),
    }
}
#[cfg(feature = "raster")]
fn pixel(image: &kartis::render::raster::RenderedImage, p: [f64; 2], dpr: f64, color: Color) {
    let x = (p[0] * dpr).floor() as usize;
    let y = (p[1] * dpr).floor() as usize;
    let i = (y * image.width as usize + x) * 4;
    let rgba = &image.rgba[i..i + 4];
    assert!(
        (rgba[0] as i16 - color.0 as i16).abs() <= 2
            && (rgba[1] as i16 - color.1 as i16).abs() <= 2
            && (rgba[2] as i16 - color.2 as i16).abs() <= 2
            && rgba[3] == 255,
        "DPR {dpr} real line pixel at {p:?}: {rgba:?}, expected {color:?}"
    );
}
#[cfg(feature = "raster")]
fn raster_move(
    handle: &RasterSurfaceHandle,
    p: [f64; 2],
    units: f64,
) -> surface::SurfacePhysicalHit {
    use kartis::surface::{SurfaceEvent, SurfaceInput};
    let bounds = RasterSurfaceBounds {
        x: 73.25 * units,
        y: 49.5 * units,
        width: 960. * units,
        height: 600. * units,
    };
    let point = (
        bounds.x + p[0] / 640. * bounds.width,
        bounds.y + p[1] / 400. * bounds.height,
    );
    handle
        .dispatch_host(SurfaceInput::Move { id: 3, point }, bounds)
        .unwrap()
        .into_iter()
        .find_map(|e| match e {
            SurfaceEvent::PhysicalHover(Some(hit)) => Some(hit),
            _ => None,
        })
        .expect("real native pointer must report physical line ink")
}

#[cfg(feature = "raster")]
#[test]
fn native_dpr_lines_keep_typed_source_and_controls_across_publisher_rejection_and_stop() {
    use kartis::render::raster::{FontRegistry, RasterOptions};
    for fit in [GeoFit::World, GeoFit::Data] {
        for dpr in [1., 2.] {
            let app = App::memory();
            let old = authored(fit, None, 0., INK);
            let next = authored(fit, Some(42.), 0.7, NEXT_INK);
            let reject = Rc::new(Cell::new(false));
            let failure = reject.clone();
            let publications = Rc::new(Cell::new(0));
            let published = publications.clone();
            let surface = RasterSurface::mount_layout_published(
                app.scope(),
                old.frame.clone(),
                FontRegistry::new(),
                RasterSurfaceOptions {
                    highlight: false,
                    raster: RasterOptions {
                        pixel_ratio: dpr,
                        ..Default::default()
                    },
                    tooltip: TooltipOptions {
                        missing: MissingValue::Placeholder("missing".into()),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                move |image| {
                    published.set(published.get() + 1);
                    if failure.get() {
                        Err(ChartError("reject USA line publication".into()))
                    } else {
                        image.data_url()
                    }
                },
            )
            .unwrap();
            app.mount(surface.widget()).unwrap();
            let handle = surface.handle();
            let controls = nodes(&app, "kartis-datum");
            assert_eq!(controls.len(), 1);
            assert_eq!(button(&app), "route: missing");
            let image = handle.frame().unwrap();
            assert_eq!(
                (image.width, image.height),
                ((640. * dpr) as u32, (400. * dpr) as u32)
            );
            let mut old_token = None;
            for (_, physical_id, p) in &old.probes {
                pixel(&image, *p, dpr, INK);
                let hit = raster_move(&handle, *p, 2.);
                assert_eq!(hit.physical_mark_id, *physical_id);
                assert_eq!(hit.logical_site_id, OWNER);
                assert_eq!(hit.source_data, None);
                old_token = Some(hit.frame_token);
                let active = handle.active_interaction().unwrap().unwrap();
                assert_eq!(active.ids, [OWNER]);
                assert_eq!(active.rows.len(), 1);
                assert_eq!(active.rows[0].value, None);
                assert_eq!(active.rows[0].label, "route");
                assert_eq!(active.rows[0].series, "route");
            }
            app.flush().unwrap();
            assert_eq!(tooltip(&app), "route: missing");
            let before = handle.snapshot().unwrap();
            let binding = handle.interaction_data().unwrap();
            let active = handle.active_interaction().unwrap();
            let accepted_image = handle.frame().unwrap();
            let accepted_source = image_source(&app);
            let tree = format!("{:?}", app.tree());
            let calls = publications.get();
            reject.set(true);
            assert!(handle.set_layout(next.frame.clone()).is_err());
            app.flush().unwrap();
            assert_eq!(
                publications.get(),
                calls + 1,
                "active publisher must reject an otherwise valid complete frame"
            );
            assert_eq!(handle.snapshot().unwrap(), before);
            assert_eq!(handle.interaction_data().unwrap(), binding);
            assert_eq!(handle.active_interaction().unwrap(), active);
            assert!(Rc::ptr_eq(&handle.frame().unwrap(), &accepted_image));
            assert_eq!(image_source(&app), accepted_source);
            assert_eq!(nodes(&app, "kartis-datum"), controls);
            assert_eq!(format!("{:?}", app.tree()), tree);
            assert_eq!(button(&app), "route: missing");
            let hit = raster_move(&handle, old.probes[0].2, 1.);
            assert_eq!(hit.frame_token, old_token.unwrap());
            assert_eq!(
                handle.active_interaction().unwrap().unwrap().rows[0].value,
                None
            );
            reject.set(false);
            handle.set_layout(next.frame.clone()).unwrap();
            app.flush().unwrap();
            assert_eq!(button(&app), "route: 42");
            assert_eq!(nodes(&app, "kartis-datum"), controls);
            assert_eq!(
                handle.interaction_data().unwrap().unwrap().sites[0].value,
                Some(42.)
            );
            let image = handle.frame().unwrap();
            assert!(!Rc::ptr_eq(&image, &accepted_image));
            assert_ne!(image_source(&app), accepted_source);
            for (_, physical_id, p) in &next.probes {
                pixel(&image, *p, dpr, NEXT_INK);
                let hit = raster_move(&handle, *p, 1.);
                assert_eq!(hit.logical_site_id, OWNER);
                assert_eq!(hit.physical_mark_id, *physical_id);
                let active = handle.active_interaction().unwrap().unwrap();
                assert_eq!(active.rows[0].value, Some(42.));
                assert_eq!(active.rows[0].label, "route");
            }
            app.flush().unwrap();
            assert_eq!(tooltip(&app), "route: 42");
            assert_eq!(old.source[1].value, None);
            assert_eq!(old.metadata.lines[1].value, None);
            assert_eq!(next.metadata.lines[1].source_index, 1);
            assert_eq!(next.metadata.lines[1].source_series, "");
            surface.stop();
            app.flush().unwrap();
            assert!(nodes(&app, "kartis-datum").is_empty());
            assert!(nodes(&app, "kartis-raster-surface-image").is_empty());
            assert!(!handle.is_active());
            assert!(handle.frame().is_err());
            assert!(handle.interaction_data().is_err());
        }
    }
}
