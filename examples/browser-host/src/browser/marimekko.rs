//! Independent category thickness and dimension stacks on both real backends.
use super::*;
use kartis::stack::{StackOffset, StackOptions};
use std::collections::BTreeSet;
struct Fixture {
    chart: BrowserChart,
    root: web_sys::Element,
    renderer: String,
    case: u8,
    phase: Rc<Cell<u8>>,
    hide_category: Rc<Cell<bool>>,
    accepted: Rc<RefCell<MarimekkoLayout>>,
    events: Rc<RefCell<Vec<Value>>>,
}
thread_local! {static MEKKOS:RefCell<Vec<Fixture>>=const{RefCell::new(Vec::new())};}
fn config(width: f64, height: f64) -> ChartConfig {
    ChartConfig {
        width,
        height,
        title: "Independent Marimekko".into(),
        margin: Margin {
            left: 66.,
            right: 65.,
            top: 42.,
            bottom: 55.,
        },
        ..Default::default()
    }
}
fn proposal(
    case: u8,
    phase: u8,
    hide_category: bool,
    width: f64,
    height: f64,
    hidden: BTreeSet<String>,
) -> Result<MarimekkoLayout> {
    if case > 3 || phase > 1 {
        return Err(ChartError("invalid Marimekko case/phase".into()));
    }
    let c = config(width, height);
    let weights = if case == 3 {
        [1., 2., 3.]
    } else if case == 1 && phase == 1 {
        [5., 2., 0.]
    } else {
        [2., 5., 0.]
    };
    let categories = ["A", "B", if case == 3 { "C" } else { "zero" }]
        .into_iter()
        .zip(weights)
        .map(|(id, w)| MarimekkoCategory::new(id, w))
        .collect::<Vec<_>>();
    let values = if case == 2 {
        [
            vec![Some(-4. - f64::from(phase) * 2.), Some(3.), Some(-20.)],
            vec![Some(6.), None, Some(30.)],
            vec![Some(-2.), Some(4.), Some(-10.)],
        ]
    } else if case == 3 {
        [
            vec![Some(1.), Some(2.), Some(1.)],
            vec![Some(3.), Some(1.), Some(4.)],
            vec![Some(2.), Some(3.), Some(1.)],
        ]
    } else {
        let factor = if case == 0 && phase == 1 { 1.5 } else { 1. };
        [
            vec![Some(4. * factor), Some(2. * factor), Some(1.)],
            vec![Some(6. * factor), Some(3. * factor), Some(2.)],
            vec![Some(2. * factor), Some(5. * factor), Some(3.)],
        ]
    };
    let dimensions = ["alpha", "beta", "gamma"]
        .into_iter()
        .zip(values)
        .map(|(id, v)| BarSeries::new(id, v))
        .collect::<Vec<_>>();
    let horizontal = case == 1 || case == 3;
    let value_axis = AxisOptions {
        format: if case == 1 {
            ValueFormat::Percent(0)
        } else {
            ValueFormat::Fixed(0)
        },
        tick_count: 5,
        ..Default::default()
    };
    let axes = if horizontal {
        MarimekkoAxes {
            bottom: Some(value_axis),
            ..Default::default()
        }
    } else {
        MarimekkoAxes {
            right: Some(value_axis),
            ..Default::default()
        }
    };
    let border = Some(GraphicStroke {
        color: Color(255, 255, 255, 255),
        width: 2.,
        cap: StrokeCap::Butt,
    });
    let style = MarimekkoDimensionStyle {
        fill_opacity: 0.8,
        border,
        ..Default::default()
    };
    let mut styles = BTreeMap::new();
    if case == 0 && phase == 1 {
        styles.insert(
            "alpha".into(),
            MarimekkoDimensionStyle {
                paint: Some(FillPaint::Linear {
                    start: [66., 42.],
                    end: [575., 345.],
                    stops: vec![
                        GradientStop::new(0., Color(30, 110, 190, 255)),
                        GradientStop::new(1., Color(130, 190, 220, 255)),
                    ],
                }),
                ..style.clone()
            },
        );
    }
    marimekko_layout(
        &c,
        &categories,
        &dimensions,
        &MarimekkoLayoutOptions {
            horizontal,
            stack: StackOptions {
                offset: match case {
                    1 => StackOffset::Expand,
                    2 => StackOffset::Diverging,
                    3 => {
                        if phase == 0 {
                            StackOffset::Wiggle
                        } else {
                            StackOffset::Silhouette
                        }
                    }
                    _ => StackOffset::Zero,
                },
                ..Default::default()
            },
            outer_padding: 8.,
            inner_padding: 7.,
            hidden_dimensions: hidden,
            hidden_categories: if hide_category {
                BTreeSet::from(["zero".into()])
            } else {
                Default::default()
            },
            style,
            styles,
            axes,
            grid_x: MarimekkoGrid {
                enabled: horizontal,
                ..Default::default()
            },
            grid_y: MarimekkoGrid {
                enabled: !horizontal,
                ..Default::default()
            },
            category_axis: Some(MarimekkoCategoryAxis {
                side: if horizontal {
                    AxisSide::Left
                } else {
                    AxisSide::Bottom
                },
                options: CategoryAxisOptions::default(),
            }),
            labels: BarLabels {
                enabled: true,
                value: if case == 1 {
                    BarLabelValue::Displayed
                } else {
                    BarLabelValue::Raw
                },
                format: if case == 1 {
                    ValueFormat::Percent(0)
                } else {
                    ValueFormat::Fixed(0)
                },
                skip_width: 15.,
                skip_height: 15.,
                style: BarTextStyle {
                    color: Some(Color(30, 40, 55, 255)),
                    size: Some(13.),
                    ..Default::default()
                },
                ..Default::default()
            },
            tooltip_format: ValueFormat::Affix {
                prefix: "$".into(),
                suffix: String::new(),
                precision: 0,
            },
            ..Default::default()
        },
    )
}
#[wasm_bindgen]
pub fn marimekko_mount(case: u8) -> std::result::Result<(), JsValue> {
    marimekko_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for renderer in ["svg", "canvas"] {
        let root = document.create_element("div")?;
        root.set_id(&format!("marimekko-{renderer}"));
        root.set_attribute("style", "width:640px;max-width:100%;margin:12px 0")?;
        document.body().unwrap().append_child(&root)?;
        let initial = proposal(case, 0, false, 640., 400., Default::default()).map_err(js)?;
        let accepted = Rc::new(RefCell::new(initial.clone()));
        let phase = Rc::new(Cell::new(0));
        let hide_category = Rc::new(Cell::new(false));
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
        let controller = CategoryViewport::new(
            vec!["fixed".into()],
            ClipRect::plot(&config(640., 400.)),
            ViewportLimits::default(),
        )
        .map_err(js)?
        .into_controller();
        let p = phase.clone();
        let h = hide_category.clone();
        let cache = accepted.clone();
        chart
            .on_viewport(controller, ViewportSelection::X, move |request| {
                let next = proposal(
                    case,
                    p.get(),
                    h.get(),
                    request.width,
                    request.height,
                    request.hidden,
                )?;
                let scene = next.scene.clone();
                *cache.borrow_mut() = next;
                Ok(scene)
            })
            .map_err(js)?;
        let events = Rc::new(RefCell::new(Vec::new()));
        let log = events.clone();
        chart.on_event(move |event| log.borrow_mut().push(json!({"event":format!("{event:?}")})));
        MEKKOS.with(|slot| {
            slot.borrow_mut().push(Fixture {
                chart,
                root,
                renderer: renderer.into(),
                case,
                phase,
                hide_category,
                accepted,
                events,
            })
        });
    }
    Ok(())
}
#[wasm_bindgen]
pub fn marimekko_phase(phase: u8) -> std::result::Result<(), JsValue> {
    MEKKOS.with(|slot| {
        for f in slot.borrow().iter() {
            proposal(
                f.case,
                phase,
                f.hide_category.get(),
                640.,
                400.,
                Default::default(),
            )
            .map_err(js)?;
            f.phase.set(phase);
            f.chart.handle().refresh_viewport().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn marimekko_hide_category(hidden: bool) -> std::result::Result<(), JsValue> {
    MEKKOS.with(|slot| {
        for f in slot.borrow().iter() {
            proposal(
                f.case,
                f.phase.get(),
                hidden,
                640.,
                400.,
                Default::default(),
            )
            .map_err(js)?;
            f.hide_category.set(hidden);
            f.chart.handle().refresh_viewport().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn marimekko_flush() -> std::result::Result<(), JsValue> {
    MEKKOS.with(|slot| {
        for f in slot.borrow().iter() {
            f.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn marimekko_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    MEKKOS.with(|slot|{let entries=slot.borrow();let Some(f)=entries.iter().find(|f|f.renderer==renderer)else{return Ok(json!({"active":false}).to_string())};let s=f.chart.snapshot().map_err(js)?;let l=f.accepted.borrow();let target=l.cells.iter().find(|c|c.dimension=="alpha"&&c.category=="A"&&c.mark_index.is_some()).or_else(||l.cells.iter().find(|c|c.mark_index.is_some())).map(|cell|{let r=cell.visible_rect.unwrap();json!({"id":cell.id,"x":r.x+r.width/2.,"y":r.y+r.height/2.,"value":cell.raw_value,"displayed":cell.displayed_value,"series":cell.dimension})});Ok(json!({"active":s.active,"revision":s.revision,"width":s.width,"height":s.height,"phase":f.phase.get(),"case":f.case,"hidden":s.hidden,"hovered":s.hovered,"selected":s.selected,"pending":s.pending,"target":target,"thicknessDomain":l.thickness_domain,"dimensionsDomain":l.dimensions_domain,"baseline":l.stack.baseline,"categories":l.categories.iter().map(|d|json!({"id":d.id,"band":d.band,"thickness":d.thickness,"hidden":d.hidden})).collect::<Vec<_>>(),"cells":l.cells.iter().map(|d|json!({"id":d.id,"category":d.category,"dimension":d.dimension,"raw":d.raw_value,"displayed":d.displayed_value,"interval":d.interval,"mark":d.mark_index,"sourceCategory":d.source_category,"sourceDimension":d.source_dimension})).collect::<Vec<_>>(),"owned":l.scene.label_series.values().cloned().collect::<Vec<_>>(),"events":f.events.borrow().clone()}).to_string())})
}
#[wasm_bindgen]
pub fn marimekko_rejected() -> bool {
    MEKKOS.with(|slot| {
        slot.borrow().iter().all(|f| {
            let bad = marimekko_with_options(
                &config(640., 400.),
                &[MarimekkoCategory::new("bad", f64::NAN)],
                &[BarSeries::new("source", vec![Some(1.)])],
                &Default::default(),
            )
            .is_err();
            let mut scene = f.accepted.borrow().scene.clone();
            scene.marks[0].opacity = f64::NAN;
            bad && f.chart.set_scene(scene).is_err()
        })
    })
}
#[wasm_bindgen]
pub fn marimekko_stop() {
    let entries = MEKKOS.with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    for f in entries {
        f.chart.stop();
        f.root.remove();
    }
}
