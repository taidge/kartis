use super::*;
#[wasm_bindgen]
pub fn remount_morph(id: &str) -> std::result::Result<(), JsValue> {
    let previous = ENTRIES.with(|entries| entries.borrow_mut().remove(id));
    drop(previous);
    let mut options = motion::PropertyMotionOptions::default();
    options.geometry.morphs.insert(
        "datum-alpha".into(),
        motion::PathMorphOptions {
            samples: 48,
            alignment: motion::ClosedAlignment::Rotate,
            ..Default::default()
        },
    );
    let entry = mount(id, BrowserAnimation::Properties(options), 1)?;
    ENTRIES.with(|entries| entries.borrow_mut().insert(id.into(), entry));
    Ok(())
}
#[wasm_bindgen]
pub fn morph_phase(id: &str, phase: u32) -> std::result::Result<(), JsValue> {
    with_entry(id, |entry| {
        let size = entry.handle.snapshot().map_err(js)?;
        let mut target = scene(size.width, size.height, phase % 2);
        if phase % 2 == 1 {
            target.marks[0].geometry = Geometry::Circle {
                x: size.width * 0.44,
                y: size.height * 0.425,
                radius: size.height * 0.13,
            };
        }
        entry.handle.animate_to(target).map_err(js)
    })
}
struct NavigationState {
    category: bool,
    category_count: usize,
    inset: Rc<Cell<f64>>,
    reject: Rc<Cell<bool>>,
}
thread_local! { static NAVIGATION:RefCell<BTreeMap<String,NavigationState>>=const{RefCell::new(BTreeMap::new())}; }
fn categories(count: usize) -> Vec<String> {
    (0..count).map(|i| format!("Month {i:02}")).collect()
}
fn layout(request: &BrowserViewportLayout, category: bool, count: usize) -> Result<Scene> {
    if !category {
        return viewport_scene(request);
    }
    let c = ChartConfig {
        width: request.width,
        height: request.height,
        margin: Margin {
            left: request.plot.x,
            top: request.plot.y,
            right: request.width - request.plot.x - request.plot.width,
            bottom: request.height - request.plot.y - request.plot.height,
        },
        ..Default::default()
    };
    let window = CategoryViewport::resolve(&categories(count), request.domains.x)?;
    bar(
        &c,
        &categories(count),
        &[
            BarSeries::new("alpha", (0..count).map(|i| Some(20. + i as f64)).collect()),
            BarSeries::new("beta", vec![Some(10.); count]),
        ],
        &BarOptions {
            category_window: Some(window.normalized),
            value_domain: request.domains.left,
            layout: BarLayout::Stacked,
            hidden: request.hidden.clone(),
            category_axis: CategoryAxisOptions {
                tick_count: 12,
                ..Default::default()
            },
            ..Default::default()
        },
    )
}
#[wasm_bindgen]
pub fn remount_navigation(id: &str, category: bool) -> std::result::Result<(), JsValue> {
    remount_navigation_count(id, category, 12)
}
#[wasm_bindgen]
pub fn remount_one_category(id: &str) -> std::result::Result<(), JsValue> {
    remount_navigation_count(id, true, 1)
}
fn remount_navigation_count(
    id: &str,
    category: bool,
    count: usize,
) -> std::result::Result<(), JsValue> {
    remount(id)?;
    let inset = Rc::new(Cell::new(56.));
    let reject = Rc::new(Cell::new(false));
    NAVIGATION.with(|s| {
        s.borrow_mut().insert(
            id.into(),
            NavigationState {
                category,
                category_count: count,
                inset: inset.clone(),
                reject: reject.clone(),
            },
        )
    });
    with_entry(id, |entry| {
        let s = entry.handle.snapshot().map_err(js)?;
        let controller = if category {
            CategoryViewport::with_value_axes(
                categories(count),
                viewport_plot(s.width, s.height),
                ViewportLimits::default(),
                Some(ViewportAxis::new([0., 100.], ScaleKind::Linear)),
                None,
            )
            .map_err(js)?
            .into_controller()
        } else {
            viewport_controller(s.width, s.height).map_err(js)?
        };
        entry
            .chart
            .as_ref()
            .ok_or_else(|| js("chart dropped"))?
            .on_viewport_with_plot(
                controller,
                ViewportSelection::Both,
                move |width, height| {
                    Ok(ClipRect {
                        x: inset.get(),
                        y: 24.,
                        width: width - inset.get() - 24.,
                        height: height - 66.,
                    })
                },
                move |request| {
                    if reject.get() {
                        return Err(ChartError("navigation fixture rejection".into()));
                    }
                    layout(&request, category, count)
                },
            )
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn navigation_zoom(id: &str, axis: u8, factor: f64) -> std::result::Result<(), JsValue> {
    let axes = match axis {
        0 => ViewportSelection::X,
        1 => ViewportSelection::Y,
        2 => ViewportSelection::Both,
        _ => return Err(js("invalid axis")),
    };
    with_entry(id, |entry| {
        let c = entry
            .handle
            .viewport_controller()
            .map_err(js)?
            .ok_or_else(|| js("no viewport"))?;
        let p = c.plot();
        entry
            .handle
            .request_viewport(ViewportRequest::Zoom {
                factor,
                anchor: (p.width / 2., p.height / 2.),
                axes,
            })
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn navigation_inset(id: &str, inset: f64) -> std::result::Result<(), JsValue> {
    navigation_stage_inset(id, inset)?;
    with_entry(id, |e| e.handle.refresh_viewport().map_err(js))
}
/// Change application layout state without queueing a refresh. A DOM wheel
/// emitted in the same JS task will relayout using this new margin.
#[wasm_bindgen]
pub fn navigation_stage_inset(id: &str, inset: f64) -> std::result::Result<(), JsValue> {
    NAVIGATION.with(|s| {
        s.borrow()
            .get(id)
            .ok_or_else(|| js("missing navigation fixture"))
            .map(|s| s.inset.set(inset))
    })
}
#[wasm_bindgen]
pub fn navigation_replace_plot_then_fail(id: &str) -> std::result::Result<(), JsValue> {
    with_entry(id, |entry| {
        let controller = entry
            .handle
            .viewport_controller()
            .map_err(js)?
            .ok_or_else(|| js("no viewport"))?;
        let replacement = controller.clone();
        let key = id.to_owned();
        let events = entry.events.clone();
        let stale_layout = entry.events.clone();
        entry
            .chart
            .as_ref()
            .ok_or_else(|| js("chart dropped"))?
            .on_viewport_with_plot(
                controller,
                ViewportSelection::Both,
                move |_, _| {
                    with_entry(&key, |entry| {
                        let accepted = events.clone();
                        entry
                            .chart
                            .as_ref()
                            .ok_or_else(|| js("chart dropped"))?
                            .on_viewport(
                                replacement.clone(),
                                ViewportSelection::Both,
                                move |request| {
                                    accepted
                                        .borrow_mut()
                                        .push(json!({"type":"navigation-replacement-layout"}));
                                    viewport_scene(&request)
                                },
                            )
                            .map_err(js)
                    })
                    .map_err(|e| ChartError(format!("navigation replacement failed: {e:?}")))?;
                    Err(ChartError("stale dynamic plot failure".into()))
                },
                move |request| {
                    stale_layout
                        .borrow_mut()
                        .push(json!({"type":"navigation-stale-layout"}));
                    viewport_scene(&request)
                },
            )
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn navigation_reject(id: &str, reject: bool) -> std::result::Result<(), JsValue> {
    NAVIGATION.with(|s| {
        s.borrow()
            .get(id)
            .ok_or_else(|| js("missing navigation fixture"))
            .map(|s| s.reject.set(reject))
    })
}
#[wasm_bindgen]
pub fn navigation_probe(id: &str) -> std::result::Result<String, JsValue> {
    with_entry(id, |entry| {
        let s = entry.handle.snapshot().map_err(js)?;
        let c = entry
            .handle
            .viewport_controller()
            .map_err(js)?
            .ok_or_else(|| js("no viewport"))?;
        let (category, count) = NAVIGATION.with(|state| {
            state
                .borrow()
                .get(id)
                .map_or((false, 12), |s| (s.category, s.category_count))
        });
        let request = BrowserViewportLayout {
            width: s.width,
            height: s.height,
            plot: c.plot(),
            domains: c.domains(),
            piecewise_domains: c.piecewise_domains().map_err(js)?,
            hidden: s.hidden,
        };
        let scene = layout(&request, category, count).map_err(js)?;
        let p = c.plot();
        let marks = scene
            .marks
            .iter()
            .filter(|m| !m.series.is_empty())
            .map(|m| match m.geometry {
                Geometry::Rect {
                    x,
                    y,
                    width,
                    height,
                } => {
                    json!({"id":m.id,"x":x+width/2.,"y":y+height/2.,"width":width,"height":height})
                }
                Geometry::Circle { x, y, radius } => json!({"id":m.id,"x":x,"y":y,"radius":radius}),
                _ => json!({"id":m.id}),
            })
            .collect::<Vec<_>>();
        Ok(json!({"plot":{"x":p.x,"y":p.y,"width":p.width,"height":p.height},"zoom":c.zoom(),"marks":marks,"labels":scene.labels.iter().map(|l|l.text.clone()).collect::<Vec<_>>()}).to_string())
    })
}
