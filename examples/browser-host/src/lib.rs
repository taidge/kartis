//! Two real application-owned BrowserChart surfaces with independent lifetimes.
#[path = "../../shared/geo_conic.rs"]
pub mod conic_model;
#[cfg(target_arch = "wasm32")]
mod browser {
    mod area_bump;
    mod area_bump_labels;
    mod bar_styles;
    mod blend_motion;
    mod blends;
    mod calendar_layout;
    mod clips;
    mod colors;
    mod conic;
    mod contour_density;
    mod contours;
    mod delaunay;
    mod density;
    mod editable_brush;
    mod flow_layouts;
    mod geo_glyphs;
    mod geo_inverse;
    mod geo_stream;
    mod glyphs;
    mod heatmap;
    mod hierarchy_focus;
    mod layout_models;
    mod line_crosshair;
    mod line_custom_points;
    mod marimekko;
    mod navigation;
    mod network_drag;
    mod network_glyphs;
    mod nonlinear_colors;
    mod paints;
    mod parallel;
    mod pie;
    mod pie_layout;
    mod piecewise_scales;
    mod plot_widgets;
    mod radar;
    mod radar_custom_points;
    mod radar_slices;
    mod radial_labels;
    mod relations;
    mod sankey_styles;
    mod scale_profiles;
    mod scatter;
    mod sequential_quantile;
    mod stacked_radial;
    mod statistics;
    mod styled_line;
    mod styles_extended;
    mod svg_isolation;
    mod svg_transaction_probe;
    mod swarm;
    mod text_baseline;
    mod tree;
    mod typography;
    use glory::render::{GraphicPoint, GraphicStroke, PathCommand, StrokeCap};
    use kartis::{
        browser::{
            BrowserAnimation, BrowserChart, BrowserEvent, BrowserHandle, BrowserOptions,
            BrowserRenderer, BrowserViewportLayout,
        },
        surface::{SurfaceEvent, SurfaceInput},
        *,
    };
    use serde_json::{Value, json};
    use std::{
        cell::{Cell, RefCell},
        collections::BTreeMap,
        rc::Rc,
    };
    use wasm_bindgen::prelude::*;
    struct Entry {
        chart: Option<BrowserChart>,
        handle: BrowserHandle,
        events: Rc<RefCell<Vec<Value>>>,
        phase: Rc<Cell<u32>>,
        layouts: Rc<Cell<u32>>,
        replacement_layouts: Rc<Cell<u32>>,
        stop_hover: Rc<Cell<bool>>,
        stop_resize: Rc<Cell<bool>>,
        viewport_calls: Rc<Cell<u32>>,
        viewport_replacements: Rc<Cell<u32>>,
        viewport_mode: Rc<Cell<u8>>,
    }
    thread_local! {static ENTRIES:RefCell<BTreeMap<String,Entry>>=const{RefCell::new(BTreeMap::new())};}
    fn js(error: impl std::fmt::Display) -> JsValue {
        error.to_string().into()
    }
    fn scene(width: f64, height: f64, phase: u32) -> Scene {
        let c = ChartConfig {
            width,
            height,
            margin: Margin {
                top: height * 0.05,
                right: width * 0.05,
                bottom: height * 0.10,
                left: width * 0.05,
            },
            title: "Interactive browser plot".into(),
            ..Default::default()
        };
        let mut s = c.scene().expect("valid fixture dimensions");
        for (id, x, value, color) in [
            (
                "alpha",
                0.14 + 0.20 * f64::from(phase),
                10. + 30. * f64::from(phase),
                Color(65, 120, 185, 255),
            ),
            ("beta", 0.56, 20., Color(232, 139, 48, 255)),
        ] {
            let mut mark = Mark::new(
                format!("datum-{id}"),
                Geometry::Rect {
                    x: x * width,
                    y: height * 0.25,
                    width: width * 0.20,
                    height: height * 0.35,
                },
                color,
            );
            mark.series = id.into();
            mark.label = id.into();
            mark.value = value;
            mark.clip = Some(ClipRect {
                x: width * 0.05,
                y: height * 0.10,
                width: width * 0.90,
                height: height * 0.80,
            });
            s.marks.push(mark);
            s.legend.push(LegendItem {
                id: id.into(),
                label: id.into(),
                color,
            });
            s.label_series.insert(s.labels.len(), id.into());
            s.labels.push(Label {
                rotation: if phase == 0 { -15. } else { 30. },
                x: (x + 0.10) * width,
                y: height * 0.82,
                text: format!("{id} caption"),
                color: Color(38, 50, 68, 255),
                size: 13.,
                anchor: TextAnchor::Middle,
            });
        }
        s.labels.push(Label {
            rotation: 0.,
            x: width / 2.,
            y: 20.,
            text: "Shared axis".into(),
            color: Color(38, 50, 68, 255),
            size: 13.,
            anchor: TextAnchor::Middle,
        });
        let commands = (0..=600)
            .map(|i| {
                let p = GraphicPoint::new(width * f64::from(i) / 600., height * 0.95);
                if i == 0 {
                    PathCommand::Move(p)
                } else {
                    PathCommand::Line(p)
                }
            })
            .collect();
        let mut long = Mark::new(
            "long-path",
            Geometry::Path(commands),
            Color(140, 160, 180, 255),
        );
        long.label.clear();
        long.fill = None;
        long.stroke = Some(GraphicStroke {
            color: Color(140, 160, 180, 255),
            width: 1.,
            cap: StrokeCap::Butt,
        });
        s.marks.push(long);
        s.draw_order = vec![
            DrawItem::Mark(2),
            DrawItem::Mark(0),
            DrawItem::Label(0),
            DrawItem::Mark(1),
            DrawItem::Label(1),
            DrawItem::Label(2),
        ];
        s.validate().expect("fixture scene");
        s
    }
    fn event_value(event: BrowserEvent) -> Value {
        match event {
            BrowserEvent::Focus(id) => json!({"type":"focus","id":id}),
            BrowserEvent::Resized { width, height } => {
                json!({"type":"resize","width":width,"height":height})
            }
            BrowserEvent::Error(error) => json!({"type":"error","message":error.0}),
            BrowserEvent::LegendActivated(event) => json!({
                "type":"legend", "id":event.item.id, "label":event.item.label,
                "role":format!("{:?}",event.role),
            }),
            BrowserEvent::Interaction(event) => match event {
                SurfaceEvent::Hover(ids) => json!({"type":"hover","ids":ids}),
                SurfaceEvent::PhysicalHover(hit) => json!({
                    "type":"physicalHover",
                    "hit":hit.map(|hit|json!({
                        "physicalId":hit.physical_mark_id,
                        "logicalId":hit.logical_site_id,
                        "scenePoint":hit.scene_point,
                        "sourceData":hit.source_data.map(|source|json!({"sourceId":source.source_id,"value":source.value})),
                    })),
                }),
                SurfaceEvent::Selected(id) => json!({"type":"selected","id":id}),
                SurfaceEvent::Activated(id) => json!({"type":"activated","id":id}),
                SurfaceEvent::View(view) => {
                    json!({"type":"view","scale":view.scale,"x":view.translate_x,"y":view.translate_y})
                }
                SurfaceEvent::Brush {
                    bounds,
                    ids,
                    complete,
                } => {
                    json!({"type":"brush","ids":ids,"complete":complete,"bounds":[bounds.x0,bounds.y0,bounds.x1,bounds.y1]})
                }
                SurfaceEvent::EditableBrush { event, ids } => json!({
                    "type":"editableBrush", "ids":ids,
                    "phase":format!("{:?}",event.phase),
                    "source":format!("{:?}",event.source),
                    "mode":format!("{:?}",event.mode),
                    "bounds":event.selection.map(|b|[b.x0,b.y0,b.x1,b.y1]),
                }),
            },
        }
    }
    fn viewport_plot(width: f64, height: f64) -> ClipRect {
        ClipRect {
            x: 56.,
            y: 24.,
            width: width - 80.,
            height: height - 66.,
        }
    }
    fn viewport_controller(width: f64, height: f64) -> Result<DomainViewport> {
        DomainViewport::new(
            ViewportAxes {
                x: ViewportAxis::new([0., 100.], ScaleKind::Linear),
                left: Some(ViewportAxis::new([0., 200.], ScaleKind::Linear)),
                right: None,
            },
            viewport_plot(width, height),
            ViewportLimits {
                min_zoom: 1.,
                max_zoom: 100.,
                bounded: true,
            },
        )
    }
    fn viewport_scene(request: &BrowserViewportLayout) -> Result<Scene> {
        let config = ChartConfig {
            width: request.width,
            height: request.height,
            margin: Margin {
                left: request.plot.x,
                top: request.plot.y,
                right: request.width - request.plot.x - request.plot.width,
                bottom: request.height - request.plot.y - request.plot.height,
            },
            title: "Fixed axes domain viewport".into(),
            ..Default::default()
        };
        scatter(
            &config,
            &[
                Series::new(
                    "alpha",
                    vec![
                        Point::new(50., 100.),
                        Point::new(40., 80.),
                        Point::new(60., 120.),
                    ],
                ),
                Series::new("beta", vec![Point::new(55., 90.)]),
            ],
            &ScatterOptions {
                radius: 8.,
                x_domain: Some(request.domains.x),
                y_domain: request.domains.left,
                hidden: request.hidden.clone(),
                ..Default::default()
            },
        )
    }
    fn install_viewport_binding(id: &str, replacement: bool) -> std::result::Result<(), JsValue> {
        with_entry(id, |entry| {
            let snapshot = entry.handle.snapshot().map_err(js)?;
            let controller = match entry.handle.viewport_controller().map_err(js)? {
                Some(controller) => controller,
                None => {
                    let mut controller =
                        viewport_controller(snapshot.width, snapshot.height).map_err(js)?;
                    controller
                        .apply(ViewportRequest::Transform {
                            transform: snapshot.view,
                            axes: ViewportSelection::Both,
                        })
                        .map_err(js)?;
                    controller
                }
            };
            let calls = entry.viewport_calls.clone();
            let replacements = entry.viewport_replacements.clone();
            let mode = entry.viewport_mode.clone();
            let handle = entry.handle.clone();
            let key = id.to_owned();
            entry
                .chart
                .as_ref()
                .ok_or_else(|| js("chart dropped"))?
                .on_viewport(controller, ViewportSelection::Both, move |request| {
                    calls.set(calls.get() + 1);
                    if replacement {
                        replacements.set(replacements.get() + 1);
                    }
                    match mode.get() {
                        1 => return Err(ChartError("fixture viewport layout rejection".into())),
                        2 => {
                            mode.set(0);
                            handle.stop();
                        }
                        3 | 4 => {
                            let fail = mode.get() == 3;
                            mode.set(0);
                            install_viewport_binding(&key, true)
                                .map_err(|e| ChartError(format!("replacement failed: {e:?}")))?;
                            if fail {
                                return Err(ChartError("stale viewport callback failure".into()));
                            }
                            let mut rejected = viewport_scene(&request)?;
                            for mark in &mut rejected.marks {
                                if !mark.label.is_empty() {
                                    mark.value = 999.;
                                    mark.label = "stale callback result".into();
                                }
                            }
                            return Ok(rejected);
                        }
                        _ => {}
                    }
                    viewport_scene(&request)
                })
                .map_err(js)
        })
    }
    fn record(id: &str, events: &Rc<RefCell<Vec<Value>>>, event: Value) {
        let mut log = events.borrow_mut();
        if log.len() == 256 {
            log.remove(0);
        }
        log.push(event);
        if let Some(element) = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id(&format!("{id}-log")))
        {
            element.set_text_content(Some(
                &serde_json::to_string_pretty(&*log).expect("event JSON"),
            ));
        }
    }
    fn mount(
        id: &str,
        animation: BrowserAnimation,
        layout_mode: u8,
    ) -> std::result::Result<Entry, JsValue> {
        let renderer = match id {
            "svg" => BrowserRenderer::Svg,
            "canvas" => BrowserRenderer::Canvas,
            _ => return Err(js("unknown renderer")),
        };
        let root = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id(&format!("{id}-root")))
            .ok_or_else(|| js("missing host root"))?;
        let (legend_options, mut tooltip_options) = if layout_mode == 4 {
            (
                LegendOptions {
                    direction: LegendDirection::Vertical,
                    wrap: false,
                    gap: 11.,
                    item_gap: 7.,
                    align: PresentationAlign::End,
                    item_width: Some(220.),
                    symbol: LegendSymbol::Circle,
                    size: 16.,
                    symbol_color: Some(Color(180, 30, 90, 255)),
                    text: PresentationText {
                        color: Some(Color(20, 40, 80, 255)),
                        size: 17.,
                        weight: 650.,
                        line_height: 1.7,
                        font: FontOptions::default(),
                    },
                    series_labels: BTreeMap::from([
                        ("alpha".into(), "Alpha <safe>".into()),
                        ("beta".into(), "Beta & safe".into()),
                    ]),
                    label_prefix: "[".into(),
                    label_suffix: "]".into(),
                },
                TooltipOptions {
                    heading: Some("<b>Safe tooltip</b>".into()),
                    lines: vec![
                        vec![
                            TooltipPart::Series,
                            TooltipPart::Text(" = ".into()),
                            TooltipPart::Value,
                        ],
                        vec![TooltipPart::Text("id: ".into()), TooltipPart::Id],
                    ],
                    value_format: ValueFormat::Fixed(2),
                    series_labels: BTreeMap::from([("alpha".into(), "Alpha <safe>".into())]),
                    padding: 13.,
                    background: Some(Color(12, 34, 56, 255)),
                    text: PresentationText {
                        color: Some(Color(230, 240, 250, 255)),
                        size: 16.,
                        weight: 600.,
                        line_height: 1.8,
                        font: FontOptions::default(),
                    },
                    ..Default::default()
                },
            )
        } else {
            (
                LegendOptions::default(),
                BrowserOptions::default().tooltip_options,
            )
        };
        if layout_mode >= 5 {
            tooltip_options.heading = Some("<b>Automatic safe tooltip</b>".into());
            tooltip_options.lines = vec![
                vec![TooltipPart::Label],
                vec![TooltipPart::Text("Value: ".into()), TooltipPart::Value],
                vec![TooltipPart::Text(
                    "Measured browser text wraps inside the plot viewport.".into(),
                )],
            ];
            tooltip_options.value_format = ValueFormat::Fixed(2);
            if layout_mode == 7 {
                tooltip_options.heading = Some(
                    (0..40)
                        .map(|i| format!("Line{i} <em>safe</em>"))
                        .collect::<Vec<_>>()
                        .join("\n"),
                );
            }
        }
        let tooltip_layout = if layout_mode == 6 {
            Some(TooltipLayoutOptions {
                viewport: LayoutRect {
                    x: 0.,
                    y: 0.,
                    width: 2000.,
                    height: 260.,
                },
                preferred: vec![TooltipAnchor::BottomRight],
                gap: 8.,
                forbidden: vec![LayoutRect {
                    x: 0.,
                    y: 130.,
                    width: 2000.,
                    height: 130.,
                }],
                ..Default::default()
            })
        } else {
            None
        };
        let initial = if layout_mode >= 5 {
            tooltip_layout_scene(640., 260., layout_mode)
        } else {
            scene(640., 260., 0)
        };
        let chart = BrowserChart::mount(
            root,
            initial,
            BrowserOptions {
                renderer,
                animation,
                height: Some(260.),
                legend_options,
                tooltip_options,
                tooltip_layout,
                ..Default::default()
            },
        )
        .map_err(js)?;
        let handle = chart.handle();
        let events = Rc::new(RefCell::new(Vec::new()));
        let phase = Rc::new(Cell::new(0));
        let layouts = Rc::new(Cell::new(0));
        let stop_hover = Rc::new(Cell::new(false));
        let stop_resize = Rc::new(Cell::new(false));
        let callback_handle = handle.clone();
        let callback_events = events.clone();
        let callback_stop = stop_hover.clone();
        let callback_id = id.to_string();
        chart.on_event(move|event|{let should_stop=matches!(&event,BrowserEvent::Interaction(SurfaceEvent::Hover(ids))if !ids.is_empty())&&callback_stop.replace(false);record(&callback_id,&callback_events,event_value(event));if should_stop{callback_handle.stop();}});
        let layout_phase = phase.clone();
        let layout_count = layouts.clone();
        let layout_stop = stop_resize.clone();
        let layout_handle = handle.clone();
        if layout_mode != 0 && layout_mode != 4 {
            chart.on_resize(move |width, height| {
                layout_count.set(layout_count.get() + 1);
                if layout_mode == 2 {
                    return Err(ChartError("fixture layout rejection".into()));
                }
                if layout_stop.replace(false) {
                    layout_handle.stop();
                }
                Ok(if layout_mode >= 5 {
                    tooltip_layout_scene(width, height, layout_mode)
                } else {
                    scene(width, height, layout_phase.get())
                })
            });
        }
        Ok(Entry {
            chart: Some(chart),
            handle,
            events,
            phase,
            layouts,
            replacement_layouts: Rc::new(Cell::new(0)),
            stop_hover,
            stop_resize,
            viewport_calls: Rc::new(Cell::new(0)),
            viewport_replacements: Rc::new(Cell::new(0)),
            viewport_mode: Rc::new(Cell::new(0)),
        })
    }
    fn tooltip_layout_scene(width: f64, height: f64, mode: u8) -> Scene {
        let mut s = ChartConfig {
            width,
            height,
            title: "Tooltip automatic placement".into(),
            ..Default::default()
        }
        .scene()
        .expect("fixture dimensions");
        for (id, x, y) in [
            ("corner-br", 0.95, 0.95),
            ("corner-tl", 0.05, 0.05),
            ("center", 0.45, 0.40),
        ] {
            let mut mark = Mark::new(
                id,
                Geometry::Circle {
                    x: width * x,
                    y: height * y,
                    radius: 10.,
                },
                Color(65, 120, 185, 255),
            );
            mark.series = "alpha".into();
            mark.label = format!("{id} <safe> mode{mode}");
            mark.value = 12.5;
            s.marks.push(mark);
        }
        s.legend.push(LegendItem {
            id: "alpha".into(),
            label: "Safe <legend>".into(),
            color: Color(65, 120, 185, 255),
        });
        s
    }
    #[wasm_bindgen]
    pub fn remount_tooltip_layout(id: &str, mode: u8) -> std::result::Result<(), JsValue> {
        if mode > 2 {
            return Err(js("invalid tooltip layout mode"));
        }
        let previous = ENTRIES.with(|entries| entries.borrow_mut().remove(id));
        drop(previous);
        let entry = mount(id, BrowserAnimation::default(), mode + 5)?;
        ENTRIES.with(|entries| entries.borrow_mut().insert(id.into(), entry));
        Ok(())
    }
    fn with_entry<T>(
        id: &str,
        read: impl FnOnce(&Entry) -> std::result::Result<T, JsValue>,
    ) -> std::result::Result<T, JsValue> {
        ENTRIES.with(|entries| {
            let entries = entries.borrow();
            read(entries.get(id).ok_or_else(|| js("unknown fixture"))?)
        })
    }
    #[wasm_bindgen(start)]
    pub fn start() -> std::result::Result<(), JsValue> {
        remount("svg")?;
        remount("canvas")
    }
    #[wasm_bindgen]
    pub fn remount(id: &str) -> std::result::Result<(), JsValue> {
        let previous = ENTRIES.with(|entries| entries.borrow_mut().remove(id));
        drop(previous);
        let entry = mount(id, BrowserAnimation::default(), 1)?;
        ENTRIES.with(|entries| entries.borrow_mut().insert(id.into(), entry));
        Ok(())
    }
    #[wasm_bindgen]
    pub fn svg_patch_case(case: u8) -> std::result::Result<(), JsValue> {
        with_entry("svg", |entry| {
            let size = entry.handle.snapshot().map_err(js)?;
            let mut frame = scene(size.width, size.height, 0);
            if case == 1 || case == 2 {
                frame.marks[0].clip = Some(ClipRect {
                    x: 0.,
                    y: 0.,
                    width: size.width * 0.45,
                    height: size.height,
                });
                frame.marks[0].stroke = Some(GraphicStroke {
                    color: Color(20, 40, 60, 255),
                    width: 3.,
                    cap: StrokeCap::Square,
                });
                frame.labels[0].text = "<safe & retained>".into();
                frame.labels[0].rotation = 0.;
                frame.draw_order = (0..frame.labels.len())
                    .map(DrawItem::Label)
                    .chain((0..frame.marks.len()).rev().map(DrawItem::Mark))
                    .collect();
            }
            if case == 2 {
                frame = frame.without_series(&std::collections::BTreeSet::from(["beta".into()]));
            }
            if case == 3 {
                frame.marks[0].clip = None;
            }
            if case == 4 {
                frame.marks[0].geometry = Geometry::Circle {
                    x: size.width * 0.24,
                    y: size.height * 0.425,
                    radius: 20.,
                };
                frame.marks[0].fill = None;
                frame.marks[0].stroke = Some(GraphicStroke {
                    color: Color(60, 90, 140, 255),
                    width: 3.,
                    cap: StrokeCap::Round,
                });
            }
            if case > 4 {
                return Err(js("invalid SVG patch profile"));
            }
            entry.handle.set_scene(frame).map_err(js)
        })
    }
    #[wasm_bindgen]
    pub fn remount_scene_animation(id: &str) -> std::result::Result<(), JsValue> {
        let previous = ENTRIES.with(|entries| entries.borrow_mut().remove(id));
        drop(previous);
        let entry = mount(id, BrowserAnimation::Scene, 1)?;
        ENTRIES.with(|entries| entries.borrow_mut().insert(id.into(), entry));
        Ok(())
    }
    #[wasm_bindgen]
    pub fn remount_layout_profile(id: &str, mode: u8) -> std::result::Result<(), JsValue> {
        if mode > 2 {
            return Err(js("invalid layout mode"));
        }
        let previous = ENTRIES.with(|entries| entries.borrow_mut().remove(id));
        drop(previous);
        let entry = mount(id, BrowserAnimation::default(), mode)?;
        ENTRIES.with(|entries| entries.borrow_mut().insert(id.into(), entry));
        Ok(())
    }
    #[wasm_bindgen]
    pub fn remount_viewport(id: &str) -> std::result::Result<(), JsValue> {
        remount(id)?;
        install_viewport_binding(id, false)
    }
    #[wasm_bindgen]
    pub fn viewport_mode(id: &str, mode: u8) -> std::result::Result<(), JsValue> {
        if mode > 4 {
            return Err(js("invalid viewport mode"));
        }
        with_entry(id, |entry| {
            entry.viewport_mode.set(mode);
            Ok(())
        })
    }
    #[wasm_bindgen]
    pub fn refresh_viewport(id: &str) -> std::result::Result<(), JsValue> {
        with_entry(id, |entry| entry.handle.refresh_viewport().map_err(js))
    }
    #[wasm_bindgen]
    pub fn remount_presentation(id: &str) -> std::result::Result<(), JsValue> {
        let previous = ENTRIES.with(|entries| entries.borrow_mut().remove(id));
        drop(previous);
        let entry = mount(id, BrowserAnimation::default(), 4)?;
        ENTRIES.with(|entries| entries.borrow_mut().insert(id.into(), entry));
        Ok(())
    }
    #[wasm_bindgen]
    pub fn viewport_probe(id: &str) -> std::result::Result<String, JsValue> {
        with_entry(id, |entry| {
            let s = entry.handle.snapshot().map_err(js)?;
            let request = BrowserViewportLayout {
                width: s.width,
                height: s.height,
                plot: viewport_plot(s.width, s.height),
                domains: s.domains.ok_or_else(|| js("no viewport domains"))?,
                piecewise_domains: PiecewiseViewportDomains::default(),
                hidden: s.hidden,
            };
            let scene = viewport_scene(&request).map_err(js)?;
            let points=scene.marks.iter().filter_map(|mark|if let Geometry::Circle{x,y,radius}=&mark.geometry {Some(json!({"id":mark.id,"x":x,"y":y,"radius":radius,"value":mark.value,"label":mark.label}))}else{None}).collect::<Vec<_>>();
            Ok(json!({"plot":{"x":request.plot.x,"y":request.plot.y,"width":request.plot.width,"height":request.plot.height},"points":points,"domains":{"x":request.domains.x,"y":request.domains.left},"calls":entry.viewport_calls.get(),"replacements":entry.viewport_replacements.get()}).to_string())
        })
    }
    #[wasm_bindgen]
    pub fn date_time_samples() -> std::result::Result<String, JsValue> {
        let ny = TimeZone::iana("America/New_York").map_err(js)?;
        let format = DateTimeFormat::new("%Y-%m-%d %H:%M %Z", &TimeLocale::default(), ny.clone())
            .map_err(js)?;
        let zh = DateTimeFormat::new("%c %p %b %Z", &TimeLocale::zh_cn(), ny).map_err(js)?;
        Ok(json!({"beforeDst":format.format(1_710_053_940_000).map_err(js)?,"afterDst":format.format(1_710_054_000_000).map_err(js)?,"futureSummer":format.format(4_149_662_400_000).map_err(js)?,"zh":zh.format(1_710_054_000_000).map_err(js)?,"database":iana_time_database_version().unwrap_or("unknown")}).to_string())
    }
    #[wasm_bindgen]
    pub fn zoned_time_samples() -> std::result::Result<String, JsValue> {
        let calendar = TimeCalendar::new(TimeZone::iana("America/New_York").map_err(js)?);
        let days = ZonedTimeScale::new(
            [1_709_960_400_000, 1_710_216_000_000],
            [0., 100.],
            calendar.clone(),
        )
        .map_err(js)?
        .ticks_every(TimeInterval::new(TimeUnit::Day, 1).map_err(js)?)
        .map_err(js)?;
        let hours =
            ZonedTimeScale::new([1_730_606_400_000, 1_730_620_800_000], [0., 100.], calendar)
                .map_err(js)?
                .ticks_every(TimeInterval::new(TimeUnit::Hour, 1).map_err(js)?)
                .map_err(js)?;
        Ok(json!({"days":days.iter().map(|t|json!({"instant":t.timestamp_ms,"label":t.label})).collect::<Vec<_>>(),"hours":hours.iter().map(|t|json!({"instant":t.timestamp_ms,"label":t.label})).collect::<Vec<_>>()}).to_string())
    }
    #[wasm_bindgen]
    pub fn snapshot(id: &str) -> std::result::Result<String, JsValue> {
        with_entry(id, |entry| {
            let value = match entry.handle.snapshot() {
                Ok(s) => {
                    json!({"active":s.active,"renderer":id,"width":s.width,"height":s.height,"hovered":s.hovered,"selected":s.selected,"hidden":s.hidden,"view":{"scale":s.view.scale,"x":s.view.translate_x,"y":s.view.translate_y},"revision":s.revision,"pending":s.pending,"dropped":s.dropped,"animating":s.animating,"touches":s.touches,"pinching":s.pinching,"capturedTouches":s.captured_touches,"animationKind":s.animation_kind.map(|kind|format!("{kind:?}")),"layouts":entry.layouts.get(),"replacementLayouts":entry.replacement_layouts.get(),"phase":entry.phase.get(),"longCommands":601,"domains":s.domains.map(|d|json!({"x":d.x,"y":d.left,"right":d.right})),"viewportCalls":entry.viewport_calls.get(),"viewportReplacements":entry.viewport_replacements.get()})
                }
                Err(error) => json!({"active":false,"error":error.0,"layouts":entry.layouts.get()}),
            };
            Ok(value.to_string())
        })
    }
    #[wasm_bindgen]
    pub fn events(id: &str) -> std::result::Result<String, JsValue> {
        with_entry(id, |entry| {
            serde_json::to_string(&*entry.events.borrow()).map_err(js)
        })
    }
    #[wasm_bindgen]
    pub fn flush(id: &str) -> std::result::Result<(), JsValue> {
        with_entry(id, |entry| entry.handle.flush().map_err(js))
    }
    #[wasm_bindgen]
    pub fn reset_view(id: &str) -> std::result::Result<(), JsValue> {
        with_entry(id, |entry| entry.handle.reset_view().map_err(js))
    }
    #[wasm_bindgen]
    pub fn set_phase(id: &str, phase: u32, animate: bool) -> std::result::Result<(), JsValue> {
        if phase > 1 {
            return Err(js("phase must be zero or one"));
        }
        with_entry(id, |entry| {
            let s = entry.handle.snapshot().map_err(js)?;
            let next = scene(s.width, s.height, phase);
            if animate {
                entry.handle.animate_to(next).map_err(js)?;
            } else {
                entry.handle.set_scene(next).map_err(js)?;
            }
            entry.phase.set(phase);
            Ok(())
        })
    }
    #[wasm_bindgen]
    pub fn reject_scene(id: &str) -> std::result::Result<bool, JsValue> {
        with_entry(id, |entry| {
            let s = entry.handle.snapshot().map_err(js)?;
            let mut invalid = scene(s.width, s.height, entry.phase.get());
            invalid.width = f64::NAN;
            Ok(entry.handle.set_scene(invalid).is_err())
        })
    }
    #[wasm_bindgen]
    pub fn stop(id: &str) -> std::result::Result<(), JsValue> {
        with_entry(id, |entry| {
            entry.handle.stop();
            Ok(())
        })
    }
    #[wasm_bindgen]
    pub fn drop_chart(id: &str) -> std::result::Result<(), JsValue> {
        ENTRIES.with(|entries| {
            let mut entries = entries.borrow_mut();
            entries
                .get_mut(id)
                .ok_or_else(|| js("unknown fixture"))?
                .chart
                .take();
            Ok(())
        })
    }
    #[wasm_bindgen]
    pub fn arm_stop_on_hover(id: &str) -> std::result::Result<(), JsValue> {
        with_entry(id, |entry| {
            entry.stop_hover.set(true);
            entry.handle.dispatch(SurfaceInput::Leave).map_err(js)
        })
    }
    #[wasm_bindgen]
    pub fn arm_stop_on_resize(id: &str) -> std::result::Result<(), JsValue> {
        with_entry(id, |entry| {
            entry.stop_resize.set(true);
            Ok(())
        })
    }
    #[wasm_bindgen]
    pub fn clear_events(id: &str) -> std::result::Result<(), JsValue> {
        with_entry(id, |entry| {
            entry.events.borrow_mut().clear();
            Ok(())
        })
    }
    #[wasm_bindgen]
    pub fn overflow_actions(id: &str) -> std::result::Result<String, JsValue> {
        with_entry(id, |entry| {
            let mut accepted = 0;
            let mut rejected = 0;
            for _ in 0..512 {
                if entry.handle.dispatch(SurfaceInput::Next).is_ok() {
                    accepted += 1;
                } else {
                    rejected += 1;
                }
            }
            let pending = entry.handle.snapshot().map_err(js)?.pending;
            Ok(json!({"accepted":accepted,"rejected":rejected,"pending":pending}).to_string())
        })
    }
    #[wasm_bindgen]
    pub fn dispatch_control(id: &str, action: &str) -> std::result::Result<(), JsValue> {
        with_entry(id, |entry| {
            let s = entry.handle.snapshot().map_err(js)?;
            let point = (s.width / 2., s.height / 2.);
            let input = match action {
                "down" => SurfaceInput::Down {
                    id: 700,
                    point,
                    brush: false,
                },
                "cancel" => SurfaceInput::Cancel { id: 700 },
                "wheel" => SurfaceInput::Wheel {
                    point,
                    delta: -100.,
                },
                "zoom2" => SurfaceInput::Wheel {
                    point,
                    delta: -std::f64::consts::LN_2 / 0.002,
                },
                "invalid_wheel" => SurfaceInput::Wheel {
                    point,
                    delta: f64::NAN,
                },
                "reset" => SurfaceInput::Reset,
                "clear" => SurfaceInput::Clear,
                _ => return Err(js("unknown control action")),
            };
            entry.handle.dispatch(input).map_err(js)
        })
    }
    #[wasm_bindgen]
    pub fn install_replacing_layout(id: &str) -> std::result::Result<(), JsValue> {
        with_entry(id, |entry| {
            let chart = entry.chart.as_ref().ok_or_else(|| js("chart dropped"))?;
            let key = id.to_string();
            let phase = entry.phase.clone();
            let count = entry.layouts.clone();
            let replacements = entry.replacement_layouts.clone();
            chart.on_resize(move |width, height| {
                count.set(count.get() + 1);
                let new_phase = phase.clone();
                let new_count = count.clone();
                let replacement_count = replacements.clone();
                ENTRIES.with(|entries| {
                    if let Some(chart) = entries
                        .borrow()
                        .get(&key)
                        .and_then(|entry| entry.chart.as_ref())
                    {
                        chart.on_resize(move |width, height| {
                            new_count.set(new_count.get() + 1);
                            replacement_count.set(replacement_count.get() + 1);
                            Ok(scene(width, height, new_phase.get()))
                        });
                    }
                });
                Ok(scene(width, height, phase.get()))
            });
            Ok(())
        })
    }
}
