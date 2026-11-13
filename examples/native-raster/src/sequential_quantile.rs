//! Headless Apace consumer paint, explicit PNG decode and host input bridge.
use super::{AnyResult, Provider, bounds, host_pointer, key, nodes, pointer, sample, save, settle};
use apace_dom::Document;
use apace_traits::events::UiEvent;
use glory::{
    Widget,
    render::{NodeId, PathCommand, Props},
};
use glory_apace::{
    Config, Raster,
    assets::{Bytes, Url},
    shell::WindowApp,
};
use kartis::{render::raster::*, *};
use keyboard_types::NamedKey;
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
    path::Path,
    rc::Rc,
    sync::Arc,
};

const LOW: Color = Color(20, 80, 150, 255);
const HIGH: Color = Color(240, 150, 40, 255);
const UNKNOWN: Color = Color(233, 17, 99, 255);
const VALUES: [Option<f64>; 6] = [Some(0.), Some(0.), Some(5.), Some(10.), Some(10.), None];
const DATES: [CalendarCivilDate; 6] = [
    (2023, 12, 29),
    (2023, 12, 30),
    (2023, 12, 31),
    (2024, 1, 1),
    (2024, 1, 2),
    (2024, 1, 3),
];

#[derive(Clone)]
struct Fixture {
    layout: SceneInteraction,
    scale: ColorScale,
    sites: Vec<InteractionSite>,
    sources: BTreeMap<String, usize>,
    legend: ColorLegendLayout,
}
fn options() -> ColorScaleOptions {
    ColorScaleOptions {
        kind: ColorScaleKind::SequentialQuantile,
        colors: vec![LOW, HIGH],
        unknown: UNKNOWN,
        ..Default::default()
    }
}
fn config(width: f64) -> ChartConfig {
    ChartConfig {
        width,
        height: 300.,
        margin: Margin {
            left: 20.,
            right: 20.,
            top: 12.,
            bottom: 88.,
        },
        ..Default::default()
    }
}
fn fixture(kind: u8, width: f64, updated: bool, hidden: bool) -> AnyResult<Fixture> {
    let config = config(width);
    let mut values = VALUES;
    if updated {
        values[2] = Some(7.);
    }
    let mut sources = BTreeMap::new();
    let (mut scene, scale, sites, anchors) = match kind {
        0 => {
            let data = values
                .iter()
                .enumerate()
                .map(|(i, v)| HeatCell {
                    row: if i < 3 { "early" } else { "late" }.into(),
                    column: format!("column-{}", i % 3),
                    value: *v,
                })
                .collect::<Vec<_>>();
            let layout = heatmap_layout(
                &config,
                &data,
                &HeatmapLayoutOptions {
                    colors: options(),
                    axes: HeatmapAxes::none(),
                    labels: false,
                    hidden_rows: if hidden {
                        BTreeSet::from(["early".into()])
                    } else {
                        BTreeSet::new()
                    },
                    ..Default::default()
                },
            )?;
            let sites = layout
                .cells
                .iter()
                .filter(|c| c.visible)
                .map(|c| {
                    let raw = c.tooltip_datum();
                    let i = data
                        .iter()
                        .position(|d| d.row == c.key.row && d.column == c.key.column)
                        .expect("original HeatMap source");
                    sources.insert(c.mark_id.clone(), i);
                    InteractionSite {
                        mark_id: c.mark_id.clone(),
                        series: raw.series.into(),
                        label: format!("HeatMap source-{i}"),
                        value: raw.value,
                        formatted_value: c.formatted_value.clone(),
                        color: c.color,
                        position: c.center.expect("visible HeatMap center"),
                    }
                })
                .collect::<Vec<_>>();
            (layout.scene, layout.color_scale, sites, BTreeMap::new())
        }
        1 => {
            let data = values
                .iter()
                .enumerate()
                .map(|(i, v)| CalendarCell::new(DATES[i], *v))
                .collect::<Vec<_>>();
            let layout = calendar_layout(
                &config,
                &data,
                &CalendarLayoutOptions {
                    colors: options(),
                    start: Some(DATES[0]),
                    end: Some(DATES[5]),
                    weekday_ticks: vec![],
                    weekday_space: 0.,
                    month_labels: false,
                    year_labels: Some(false),
                    year_legend: false,
                    day_border: None,
                    month_border: None,
                    hidden_years: if hidden {
                        BTreeSet::from([2023])
                    } else {
                        BTreeSet::new()
                    },
                    ..Default::default()
                },
            )?;
            let sites = layout
                .days
                .iter()
                .filter(|d| d.visible)
                .map(|d| {
                    let raw = d.tooltip_datum();
                    let i = DATES
                        .iter()
                        .position(|date| *date == d.date)
                        .expect("original Calendar source");
                    sources.insert(d.id.clone(), i);
                    InteractionSite {
                        mark_id: d.id.clone(),
                        series: format!("year:{}", d.year),
                        label: format!("Calendar source-{i}"),
                        value: raw.value,
                        formatted_value: d.formatted_value.clone(),
                        color: d.color,
                        position: [d.rect.x + d.rect.width / 2., d.rect.y + d.rect.height / 2.],
                    }
                })
                .collect::<Vec<_>>();
            (layout.scene, layout.color_scale, sites, BTreeMap::new())
        }
        2 => {
            let features = values
                .iter()
                .enumerate()
                .filter(|(i, _)| !hidden || *i >= 3)
                .map(|(i, v)| {
                    let x = -75. + i as f64 * 25.;
                    GeoFeature {
                        id: format!("region-{i}"),
                        label: format!("Region source-{i}"),
                        value: *v,
                        polygons: vec![vec![vec![
                            [x, 0.],
                            [x + 12., 0.],
                            [x + 12., 12.],
                            [x, 12.],
                            [x, 0.],
                        ]]],
                    }
                })
                .collect::<Vec<_>>();
            let layout = choropleth_with_color_scale(
                &config,
                &features,
                &GeoMapOptions {
                    fit: GeoFit::Data,
                    border: None,
                    ..Default::default()
                },
                &options(),
            )?;
            let mut sites = Vec::new();
            let mut anchors = BTreeMap::new();
            for mark in &layout.scene.marks {
                // The nullable source is authoritative; mark.value uses a legacy 0 placeholder.
                let raw = features
                    .iter()
                    .find(|f| f.id == mark.series)
                    .ok_or("missing original GeoFeature")?;
                let index = raw
                    .id
                    .strip_prefix("region-")
                    .ok_or("region identity")?
                    .parse::<usize>()?;
                let Geometry::Path(commands) = &mark.geometry else {
                    return Err("expected projected region path".into());
                };
                let points = commands
                    .iter()
                    .enumerate()
                    .filter_map(|(i, c)| match c {
                        PathCommand::Move(p) | PathCommand::Line(p) => Some((i, [p.x, p.y])),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                let position = [
                    points.iter().map(|p| p.1[0]).sum::<f64>() / points.len() as f64,
                    points.iter().map(|p| p.1[1]).sum::<f64>() / points.len() as f64,
                ];
                anchors.insert(
                    mark.id.clone(),
                    InteractionAnchor::path_points(
                        &mark.geometry,
                        position,
                        &points.iter().map(|p| p.0).collect::<Vec<_>>(),
                    )?,
                );
                sources.insert(mark.id.clone(), index);
                sites.push(InteractionSite {
                    mark_id: mark.id.clone(),
                    series: raw.id.clone(),
                    label: raw.label.clone(),
                    value: raw.value,
                    formatted_value: raw.value.map(|v| v.to_string()),
                    color: layout.color_scale.map(raw.value),
                    position,
                });
            }
            (layout.scene, layout.color_scale, sites, anchors)
        }
        _ => return Err("unsupported native color consumer".into()),
    };
    let legend = append_color_legend(
        &mut scene,
        &scale,
        &ColorLegendOptions {
            id: "native-sequential-quantile".into(),
            position: [30., 240.],
            length: 300.,
            thickness: 20.,
            steps: 2,
            tick_count: 0,
            ..Default::default()
        },
    )?;
    let layout = SceneInteraction {
        scene,
        interaction: Some(InteractionBinding {
            sites: sites.clone(),
            anchors,
            ..Default::default()
        }),
        plot: Some(ClipRect::plot(&config)),
    };
    layout.validate()?;
    Ok(Fixture {
        layout,
        scale,
        sites,
        sources,
        legend,
    })
}
fn source_site(f: &Fixture, index: usize) -> &InteractionSite {
    f.sites
        .iter()
        .find(|s| f.sources[&s.mark_id] == index)
        .expect("visible original source")
}
fn rgba(c: Color) -> [u8; 4] {
    [c.0, c.1, c.2, c.3]
}
fn move_to(
    app: &mut WindowApp,
    provider: &Provider,
    handle: &RasterSurfaceHandle,
    point: [f64; 2],
) -> AnyResult {
    let image = nodes(app.app(), "kartis-raster-surface-image")[0];
    let b = bounds(app, image)?;
    let s = handle.snapshot()?;
    host_pointer(
        app,
        handle,
        "move",
        b.x + point[0] / s.width * b.width,
        b.y + point[1] / s.height * b.height,
    )?;
    settle(app, provider)?;
    Ok(())
}
fn frame_pixels(
    app: &WindowApp,
    handle: &RasterSurfaceHandle,
    f: &Fixture,
    dpr: f64,
    first: bool,
) -> AnyResult<(Raster, Value)> {
    let image = nodes(app.app(), "kartis-raster-surface-image")[0];
    let b = bounds(app, image)?;
    let snapshot = handle.snapshot()?;
    let mut raster = Raster::new();
    raster.draw(app.renderer())?;
    let at = |p: [f64; 2]| {
        sample(
            &raster,
            ((b.x + p[0] / snapshot.width * b.width) * dpr).floor() as u32,
            ((b.y + p[1] / snapshot.height * b.height) * dpr).floor() as u32,
        )
    };
    let mut witnesses = Vec::new();
    for s in &f.sites {
        let actual = at(s.position);
        assert_eq!(
            actual,
            rgba(s.color),
            "actual Apace consumer center {}",
            s.label
        );
        witnesses.push(json!({"id":s.mark_id,"source":f.sources[&s.mark_id],"value":s.value,"position":s.position,"expected":rgba(s.color),"actual":actual}));
    }
    if first {
        let independent = [
            Color(75, 98, 123, 255),
            Color(75, 98, 123, 255),
            Color(130, 115, 95, 255),
            HIGH,
            HIGH,
            UNKNOWN,
        ];
        for (i, c) in independent.into_iter().enumerate() {
            assert_eq!(source_site(f, i).color, c);
        }
        assert_eq!(
            f.scale.sample_domain(),
            Some([0., 0., 5., 10., 10.].as_slice())
        );
        assert_eq!(
            f.legend.stops.iter().map(|s| s.offset).collect::<Vec<_>>(),
            vec![0., 0.5, 0.5, 1., 1.]
        );
        assert_eq!(at([179., 250.]), rgba(independent[0]));
        assert_eq!(at([180., 250.]), rgba(independent[2]));
        assert_eq!(at([329., 250.]), rgba(independent[2]));
        assert_eq!(at([330., 250.]), [255; 4]);
    } else {
        assert_eq!(
            at([80., 250.]),
            rgba(f.scale.map(Some(f.scale.value_at(1. / 6.)?)))
        );
        assert_eq!(
            at([290., 250.]),
            rgba(f.scale.map(Some(f.scale.value_at(13. / 15.)?)))
        );
    }
    let legend_points = if first {
        vec![[179., 250.], [180., 250.], [329., 250.], [330., 250.]]
    } else {
        vec![[80., 250.], [290., 250.]]
    };
    let legend_witnesses = legend_points
        .into_iter()
        .map(|point| json!({"position":point,"actual":at(point)}))
        .collect::<Vec<_>>();
    let legend_stops = f
        .legend
        .stops
        .iter()
        .map(|s| json!({"offset":s.offset,"value":s.value,"color":rgba(s.color)}))
        .collect::<Vec<_>>();
    Ok((
        raster,
        json!({"bounds":{"x":b.x,"y":b.y,"width":b.width,"height":b.height},"frameDimensions":[handle.frame()?.width,handle.frame()?.height],"revision":snapshot.revision,"sampleDomain":f.scale.sample_domain(),"domain":f.scale.domain(),"sites":witnesses,"legendWitnesses":legend_witnesses,"legendStops":legend_stops}),
    ))
}
fn typed_hover(
    app: &mut WindowApp,
    provider: &Provider,
    handle: &RasterSurfaceHandle,
    site: &InteractionSite,
) -> AnyResult {
    move_to(app, provider, handle, site.position)?;
    assert_eq!(
        handle.snapshot()?.hovered,
        std::slice::from_ref(&site.mark_id)
    );
    let active = handle.active_interaction()?.ok_or("typed hover absent")?;
    assert_eq!(active.rows[0].mark_id, site.mark_id);
    assert_eq!(active.rows[0].value, site.value);
    assert_eq!(active.rows[0].label, site.label);
    Ok(())
}
fn control(app: &WindowApp, label: &str) -> AnyResult<NodeId> {
    nodes(app.app(),"kartis-datum").into_iter().find(|id|matches!(&app.app().tree().node(*id).unwrap().descriptor.view().props,Props::Button(b)if b.label.starts_with(label))).ok_or_else(||format!("missing native datum control {label}").into())
}
fn assert_retained(
    handle: &RasterSurfaceHandle,
    before: &RasterSurfaceSnapshot,
    binding: &Option<InteractionBinding>,
    frame: &Rc<RenderedImage>,
) -> AnyResult {
    assert_eq!(&handle.snapshot()?, before);
    assert_eq!(&handle.interaction_data()?, binding);
    assert!(Rc::ptr_eq(frame, &handle.frame()?));
    Ok(())
}
fn rejected_legend(f: &Fixture, values: Vec<f64>) -> AnyResult {
    let scale = options().resolve(values.into_iter().map(Some))?;
    let mut scene = f.layout.scene.clone();
    let accepted = scene.clone();
    assert!(append_color_legend(&mut scene, &scale, &ColorLegendOptions::default()).is_err());
    assert_eq!(scene, accepted);
    Ok(())
}
pub(super) fn verify(output: &Path) -> AnyResult<Value> {
    let mut cases = Vec::new();
    for kind in 0..3 {
        for dpr in [1., 2.] {
            let first = fixture(kind, 400., false, false)?;
            let provider = Arc::new(Provider::default());
            provider.0.lock().unwrap().images.insert(
                "asset://raster/quantile-font.ttf".into(),
                Bytes::from_static(include_bytes!("../../../tests/fonts/ABeeZee-Regular.ttf")),
            );
            let mut app = WindowApp::with_config(Config {
                provider: Some(provider.clone()),
                base_url: Url::parse("asset://raster/")?,
                ..Default::default()
            });
            app.renderer()
                .set_viewport((760. * dpr) as u32, (900. * dpr) as u32, dpr as f32)?;
            app.renderer().set_css("@font-face{font-family:Fixture;src:url('asset://raster/quantile-font.ttf')}html,body{margin:0;background:white;font-family:Fixture}.quantile-case{padding:24px}.kartis-data{display:flex;gap:8px;flex-wrap:wrap}button{padding:8px;border:1px solid #aaa;background:#eee}");
            let guard = Rc::new(RefCell::new(None::<RasterSurface>));
            let installed = guard.clone();
            let publish = provider.clone();
            let reject = Rc::new(Cell::new(false));
            let fail = reject.clone();
            let initial = first.layout.clone();
            app.mount(
                glory::ui::view()
                    .class("quantile-case")
                    .fill(glory::ui::widget(move |scope| {
                        let publish = publish.clone();
                        let fail = fail.clone();
                        let surface = RasterSurface::mount_layout_published(
                            scope,
                            initial.clone(),
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
                                if fail.get() {
                                    Err(ChartError("quantile provider rejected candidate".into()))
                                } else {
                                    publish.publish(image)
                                }
                            },
                        )
                        .map_err(|_| glory::render::Error::InvalidProps("quantile native mount"))?;
                        let widget = surface.widget();
                        *installed.borrow_mut() = Some(surface);
                        widget.show_in(scope)
                    })),
            )?;
            settle(&mut app, &provider)?;
            let handle = guard
                .borrow()
                .as_ref()
                .ok_or("missing native guard")?
                .handle();
            let events = Rc::new(RefCell::new(Vec::new()));
            let observed = events.clone();
            let token = Rc::new(());
            let callback_weak = Rc::downgrade(&token);
            guard.borrow().as_ref().unwrap().on_event(move |batch| {
                let _keep = &token;
                observed.borrow_mut().extend(batch);
            })?;
            let image = nodes(app.app(), "kartis-raster-surface-image")[0];
            let host = bounds(&app, image)?;
            assert!(host.x >= 20. && host.y >= 20.);
            let physical = app.renderer().engine_id(image, glory_html::Part::Main)?;
            let decoded = app.renderer().inspect(|doc| {
                doc.get_node(physical)
                    .and_then(|n| n.element_data())
                    .and_then(|e| e.raster_image_data())
                    .map(|i| (i.width, i.height))
            });
            assert_eq!(
                decoded,
                Some((handle.frame()?.width, handle.frame()?.height))
            );
            let (raster, initial_pixels) = frame_pixels(&app, &handle, &first, dpr, true)?;
            let name = ["heatmap", "calendar", "choropleth"][kind as usize];
            let path = output.with_file_name(format!("sequential-quantile-{name}-dpr{dpr}.png"));
            save(&raster, &path)?;
            typed_hover(&mut app, &provider, &handle, source_site(&first, 0))?;
            typed_hover(&mut app, &provider, &handle, source_site(&first, 2))?;
            typed_hover(&mut app, &provider, &handle, source_site(&first, 5))?;
            assert_eq!(handle.active_interaction()?.unwrap().rows[0].value, None);
            let site = source_site(&first, 2);
            let control_label = &first
                .layout
                .scene
                .marks
                .iter()
                .find(|m| m.id == site.mark_id)
                .expect("current Scene control")
                .label;
            let button = control(&app, control_label)?;
            let b = bounds(&app, button)?;
            app.handle_ui_event(UiEvent::PointerDown(pointer(
                b.x + b.width / 2.,
                b.y + b.height / 2.,
                true,
            )));
            app.handle_ui_event(UiEvent::PointerUp(pointer(
                b.x + b.width / 2.,
                b.y + b.height / 2.,
                false,
            )));
            settle(&mut app, &provider)?;
            let before_enter = events.borrow().len();
            key(&mut app, NamedKey::Enter);
            settle(&mut app, &provider)?;
            assert_eq!(
                handle.snapshot()?.selected.as_deref(),
                Some(site.mark_id.as_str())
            );
            assert!(
                events.borrow()[before_enter..]
                    .iter()
                    .any(|e| matches!(e,surface::SurfaceEvent::Activated(id)if id==&site.mark_id))
            );
            let next = fixture(kind, 400., true, false)?;
            let before = handle.snapshot()?;
            let frame = handle.frame()?;
            let binding = handle.interaction_data()?;
            let requests = provider.0.lock().unwrap().fetched;
            assert!(options().resolve([Some(f64::NAN)]).is_err());
            rejected_legend(&first, (0..1024).map(|i| i as f64).collect())?;
            rejected_legend(&first, vec![0., 0.5, 0.50000001, 1.])?;
            assert_retained(&handle, &before, &binding, &frame)?;
            let mut invalid = next.layout.clone();
            invalid.scene.width = f64::NAN;
            assert!(handle.set_layout(invalid).is_err());
            assert_retained(&handle, &before, &binding, &frame)?;
            let mut invalid = next.layout.clone();
            invalid.interaction.as_mut().unwrap().sites[0].value = Some(f64::NAN);
            assert!(handle.set_layout(invalid).is_err());
            assert_retained(&handle, &before, &binding, &frame)?;
            reject.set(true);
            assert!(handle.set_layout(next.layout.clone()).is_err());
            settle(&mut app, &provider)?;
            assert_retained(&handle, &before, &binding, &frame)?;
            assert_eq!(provider.0.lock().unwrap().fetched, requests);
            reject.set(false);
            handle.set_layout(next.layout.clone())?;
            settle(&mut app, &provider)?;
            assert_eq!(handle.interaction_data()?, next.layout.interaction);
            typed_hover(&mut app, &provider, &handle, source_site(&next, 2))?;
            assert_eq!(
                handle.active_interaction()?.unwrap().rows[0].value,
                Some(7.)
            );
            let (raster, updated_pixels) = frame_pixels(&app, &handle, &next, dpr, false)?;
            let updated_path =
                output.with_file_name(format!("sequential-quantile-{name}-updated-dpr{dpr}.png"));
            save(&raster, &updated_path)?;
            let hidden = fixture(kind, 400., true, true)?;
            assert_eq!(hidden.scale.sample_domain(), Some([10., 10.].as_slice()));
            handle.set_layout(hidden.layout.clone())?;
            settle(&mut app, &provider)?;
            assert_eq!(handle.interaction_data()?, hidden.layout.interaction);
            assert_eq!(nodes(app.app(), "kartis-datum").len(), 3);
            assert!(!nodes(app.app(), "kartis-datum").contains(&button));
            typed_hover(&mut app, &provider, &handle, source_site(&hidden, 3))?;
            let (raster, hidden_pixels) = frame_pixels(&app, &handle, &hidden, dpr, false)?;
            let hidden_path =
                output.with_file_name(format!("sequential-quantile-{name}-hidden-dpr{dpr}.png"));
            save(&raster, &hidden_path)?;
            let resized = fixture(kind, 440., true, false)?;
            // Publication failure during resize must also retain geometry and binding.
            let before = handle.snapshot()?;
            let frame = handle.frame()?;
            let binding = handle.interaction_data()?;
            reject.set(true);
            assert!(handle.set_layout(resized.layout.clone()).is_err());
            settle(&mut app, &provider)?;
            assert_retained(&handle, &before, &binding, &frame)?;
            reject.set(false);
            handle.set_layout(resized.layout.clone())?;
            settle(&mut app, &provider)?;
            assert_eq!(bounds(&app, image)?.width, 440.);
            assert_eq!(handle.frame()?.width, (440. * dpr) as u32);
            assert_eq!(handle.interaction_data()?, resized.layout.interaction);
            typed_hover(&mut app, &provider, &handle, source_site(&resized, 2))?;
            let (raster, resized_pixels) = frame_pixels(&app, &handle, &resized, dpr, false)?;
            let resized_path =
                output.with_file_name(format!("sequential-quantile-{name}-resized-dpr{dpr}.png"));
            save(&raster, &resized_path)?;
            let fetched = provider.0.lock().unwrap().fetched;
            guard.borrow_mut().take();
            settle(&mut app, &provider)?;
            assert!(!handle.is_active());
            assert!(handle.frame().is_err());
            assert!(callback_weak.upgrade().is_none());
            assert!(nodes(app.app(), "kartis-raster-surface-image").is_empty());
            assert!(nodes(app.app(), "kartis-datum").is_empty());
            assert_eq!(provider.0.lock().unwrap().fetched, fetched);
            app.app().clear()?;
            settle(&mut app, &provider)?;
            assert_eq!(app.app().stats().nodes, 0);
            assert_eq!(app.app().stats().handlers, 0);
            drop(app);
            let storage = provider.0.lock().unwrap();
            assert!(storage.closed >= 1 && storage.jobs.is_empty() && storage.images.is_empty());
            cases.push(json!({"consumer":name,"dpr":dpr,"decoded":decoded,"provider":{"fetched":storage.fetched,"decoded":storage.decoded,"closed":storage.closed,"remainingAssets":storage.images.len(),"remainingJobs":storage.jobs.len()},"initial":initial_pixels,"updated":updated_pixels,"hidden":hidden_pixels,"resized":resized_pixels,"pngs":[path,updated_path,hidden_path,resized_path],"typedPointerValuesIncludingMissing":true,"intrinsicEnter":true,"hiddenRecomputedSampleDomain":[10.,10.],"invalidSourceLegendSceneBindingRetainRcFrame":true,"publisherUpdateAndResizeRecover":true,"currentBindingAfterResize":true,"guardCallbacksProviderAssetsClean":true}));
        }
    }
    let report = json!({"passed":true,"headlessApaceDocument":true,"osWindowOrPhysicalDevice":false,"pointerBridge":"Apace UiEvent + hit tree + image Bounds + explicit dispatch_host","keyboardBridge":"intrinsic typed datum button + Apace Enter","sampleValues":VALUES,"anchors":[rgba(LOW),rgba(HIGH)],"cases":cases});
    std::fs::write(
        output.with_file_name("sequential-quantile-native.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{report}");
    Ok(report)
}
#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "Requires explicit KARTIS_SEQUENTIAL_QUANTILE_NATIVE_ARTIFACTS directory"]
    fn actual_sequential_quantile() {
        let directory = std::path::PathBuf::from(
            std::env::var_os("KARTIS_SEQUENTIAL_QUANTILE_NATIVE_ARTIFACTS")
                .expect("explicit native artifact directory"),
        );
        std::fs::create_dir_all(&directory).unwrap();
        super::verify(&directory.join("sequential-quantile-focused.png")).unwrap();
    }
}
