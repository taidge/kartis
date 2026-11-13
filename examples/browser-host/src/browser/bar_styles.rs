//! Real styled Bar scenes: hidden legend input rebuilds slots and aggregates.
use super::*;
struct BarFixture {
    chart: BrowserChart,
    root: web_sys::Element,
    renderer: String,
    case: u8,
    phase: Rc<Cell<u8>>,
    accepted: Rc<RefCell<StyledBarLayout>>,
    events: Rc<RefCell<Vec<Value>>>,
}
thread_local! {static STYLED_BARS:RefCell<Vec<BarFixture>>=const{RefCell::new(Vec::new())};}
fn config(width: f64, height: f64) -> ChartConfig {
    ChartConfig {
        width,
        height,
        margin: Margin {
            left: 64.,
            right: 60.,
            top: 42.,
            bottom: 52.,
        },
        title: "Configured bars".into(),
        ..Default::default()
    }
}
fn source(
    case: u8,
    phase: u8,
    width: f64,
    height: f64,
    hidden: std::collections::BTreeSet<String>,
) -> Result<StyledBarLayout> {
    if case > 2 || phase > 2 {
        return Err(ChartError("invalid styled bar case/phase".into()));
    }
    let c = config(width, height);
    let categories = ["A", "B", "C"].map(str::to_string);
    let shift = f64::from(phase) * 2.;
    let data = [
        BarSeries::new("alpha", vec![Some(7. + shift), Some(-4.), Some(5.)]),
        BarSeries::new("beta", vec![Some(-2.), Some(-6.), Some(3.)]),
        BarSeries::new("gamma", vec![Some(4.), Some(2.), Some(1.)]),
    ];
    let options = BarOptions {
        layout: match case {
            0 => BarLayout::Grouped,
            1 => BarLayout::Stacked,
            _ => BarLayout::Percent,
        },
        horizontal: case == 1,
        value_scale: if case == 1 {
            ScaleKind::SymLog { constant: 2. }
        } else {
            ScaleKind::Linear
        },
        value_domain: Some(if case == 2 { [-1., 1.] } else { [-15., 20.] }),
        corner_radius: 9.,
        border: Some(GraphicStroke {
            color: Color(255, 255, 255, 255),
            width: 1.,
            cap: StrokeCap::Butt,
        }),
        category_window: if case == 2 { Some([0.12, 0.88]) } else { None },
        hidden,
        value_axis: AxisOptions {
            format: if case == 2 {
                ValueFormat::Percent(0)
            } else {
                ValueFormat::Affix {
                    prefix: "$".into(),
                    suffix: String::new(),
                    precision: 0,
                }
            },
            grid: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let style = BarStyleOptions {
        category_inner_padding: Some(if phase == 2 { f64::NAN } else { 0.15 }),
        category_outer_padding: Some(0.08),
        inner_padding: if case == 0 { 8. } else { 0. },
        tooltip_format: Some(ValueFormat::Affix {
            prefix: "$".into(),
            suffix: String::new(),
            precision: 0,
        }),
        color_by: if phase == 0 {
            BarColorBy::Series
        } else {
            BarColorBy::Category
        },
        colors: if phase == 0 {
            BTreeMap::from([
                ("alpha".into(), Color(55, 125, 185, 255)),
                ("beta".into(), Color(230, 135, 45, 255)),
                ("gamma".into(), Color(65, 155, 115, 255)),
            ])
        } else {
            BTreeMap::from([
                ("A".into(), Color(35, 155, 155, 255)),
                ("B".into(), Color(145, 95, 175, 255)),
                ("C".into(), Color(65, 125, 190, 255)),
            ])
        },
        labels: BarLabels {
            enabled: true,
            position: if case == 1 {
                BarLabelPosition::End
            } else {
                BarLabelPosition::Middle
            },
            offset: if case == 1 { -5. } else { 0. },
            skip_width: 14.,
            skip_height: 12.,
            value: if case == 2 {
                BarLabelValue::Displayed
            } else {
                BarLabelValue::Raw
            },
            format: if case == 2 {
                ValueFormat::Percent(0)
            } else {
                ValueFormat::Fixed(0)
            },
            style: BarTextStyle {
                color: Some(Color(35, 45, 60, 255)),
                size: Some(13.),
                rotation: 0.,
            },
            ..Default::default()
        },
        totals: BarTotals {
            enabled: true,
            mode: if case == 1 {
                BarTotalMode::PositiveNegative
            } else {
                BarTotalMode::Net
            },
            format: ValueFormat::Affix {
                prefix: "Σ".into(),
                suffix: String::new(),
                precision: 0,
            },
            style: BarTextStyle {
                color: Some(Color(115, 50, 145, 255)),
                size: Some(14.),
                rotation: 0.,
            },
            offset: 13.,
            ..Default::default()
        },
        ..Default::default()
    };
    bar_layout_with_style(&c, &categories, &data, &options, &style)
}
#[wasm_bindgen]
pub fn bar_style_mount(case: u8) -> std::result::Result<(), JsValue> {
    bar_style_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for renderer in ["svg", "canvas"] {
        let root = document.create_element("div")?;
        root.set_id(&format!("bar-style-{renderer}"));
        root.set_attribute("style", "width:640px;max-width:100%;margin:12px 0")?;
        document.body().unwrap().append_child(&root)?;
        let initial = source(case, 0, 640., 400., Default::default()).map_err(js)?;
        let accepted = Rc::new(RefCell::new(initial.clone()));
        let chart = BrowserChart::mount(
            root.clone(),
            initial.scene,
            BrowserOptions {
                renderer: if renderer == "svg" {
                    BrowserRenderer::Svg
                } else {
                    BrowserRenderer::Canvas
                },
                height: Some(400.),
                motion: browser::MotionPreference::Reduced,
                interaction: surface::SurfaceOptions {
                    zoom: false,
                    pan: false,
                    brush: false,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .map_err(js)?;
        let phase = Rc::new(Cell::new(0));
        let p = phase.clone();
        let cache = accepted.clone();
        let controller = CategoryViewport::new(
            ["A", "B", "C"].map(str::to_string).to_vec(),
            ClipRect::plot(&config(640., 400.)),
            ViewportLimits::default(),
        )
        .map_err(js)?
        .into_controller();
        chart
            .on_viewport(controller, ViewportSelection::X, move |request| {
                let layout = source(case, p.get(), request.width, request.height, request.hidden)?;
                let scene = layout.scene.clone();
                *cache.borrow_mut() = layout;
                Ok(scene)
            })
            .map_err(js)?;
        let events = Rc::new(RefCell::new(vec![]));
        let log = events.clone();
        chart.on_event(move |event| log.borrow_mut().push(json!({"event":format!("{event:?}")})));
        STYLED_BARS.with(|slot| {
            slot.borrow_mut().push(BarFixture {
                chart,
                root,
                renderer: renderer.into(),
                case,
                phase,
                accepted,
                events,
            })
        });
    }
    Ok(())
}
#[wasm_bindgen]
pub fn bar_style_phase(phase: u8) -> std::result::Result<(), JsValue> {
    STYLED_BARS.with(|slot| {
        for f in slot.borrow().iter() {
            source(f.case, phase, 640., 400., Default::default()).map_err(js)?;
            f.phase.set(phase);
            f.chart.handle().refresh_viewport().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn bar_style_flush() -> std::result::Result<(), JsValue> {
    STYLED_BARS.with(|slot| {
        for f in slot.borrow().iter() {
            f.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn bar_style_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    STYLED_BARS.with(|slot|{let entries=slot.borrow();let Some(f)=entries.iter().find(|f|f.renderer==renderer)else{return Ok(json!({"active":false}).to_string())};let s=f.chart.snapshot().map_err(js)?;let layout=f.accepted.borrow();let target=layout.bars.iter().find(|b|b.series=="alpha"&&b.category=="A").unwrap();let r=target.visible_rect;Ok(json!({"active":s.active,"width":s.width,"height":s.height,"revision":s.revision,"hidden":s.hidden,"hovered":s.hovered,"selected":s.selected,"pending":s.pending,"phase":f.phase.get(),"case":f.case,"target":{"id":target.id,"x":r.x+r.width/2.,"y":r.y+r.height/2.,"value":target.raw_value,"band":target.band,"label":layout.scene.labels.get(target.label_index.unwrap_or(usize::MAX)).map(|l|l.text.clone())},"totals":layout.totals.iter().map(|t|json!({"category":t.category,"kind":format!("{:?}",t.kind),"value":t.value,"anchor":t.anchor_value,"position":t.position})).collect::<Vec<_>>(),"labels":layout.scene.labels.iter().map(|l|l.text.clone()).collect::<Vec<_>>(),"owned":layout.scene.label_series.values().cloned().collect::<Vec<_>>(),"events":f.events.borrow().clone()}).to_string())})
}
#[wasm_bindgen]
pub fn bar_style_rejected() -> bool {
    STYLED_BARS.with(|slot| {
        slot.borrow().iter().all(|f| {
            let invalid = source(f.case, 2, 640., 400., Default::default()).is_err();
            let mut scene = f.accepted.borrow().scene.clone();
            scene.marks[0].opacity = f64::NAN;
            invalid && f.chart.set_scene(scene).is_err()
        })
    })
}
#[wasm_bindgen]
pub fn bar_style_stop() {
    let entries = STYLED_BARS.with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    for f in entries {
        f.chart.stop();
        f.root.remove();
    }
}
