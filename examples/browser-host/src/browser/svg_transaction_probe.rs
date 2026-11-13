//! UNAPPLIED fixture candidate for the next SVG structure transaction wave.
//! Mount one SVG only; accepted source commits after actual successful flush.
use super::*;
use std::cell::Cell;

#[derive(Clone)]
struct Proposal {
    phase: u8,
    layout: SceneInteraction,
    blends: MarkBlends,
}
struct Probe {
    chart: BrowserChart,
    root: web_sys::Element,
    kind: u8,
    accepted: Rc<RefCell<Proposal>>,
    errors: Rc<RefCell<Vec<String>>>,
    busy: Rc<Cell<bool>>,
}
thread_local! {
    static SVG_TRANSACTION_PROBE: RefCell<Option<Probe>> = const { RefCell::new(None) };
}

fn proposal(kind: u8, phase: u8) -> Result<Proposal> {
    if kind > 1 || phase > 1 {
        return Err(ChartError("unknown SVG transaction probe".into()));
    }
    let mut scene = ChartConfig {
        width: 180.,
        height: 120.,
        ..Default::default()
    }
    .scene()?;
    scene.background = Color(160, 120, 80, 255);
    let mut mark = Mark::new(
        "probe-business",
        Geometry::Rect {
            x: 20.,
            y: 20.,
            width: 90.,
            height: 60.,
        },
        Color(200, 100, 50, 128),
    );
    mark.series = "probe-series".into();
    mark.label = "Authoritative source value 7".into();
    mark.value = 7.;
    scene.marks.push(mark);
    scene.labels.push(Label {
        x: 30.,
        y: 60.,
        text: "Stable masked label".into(),
        color: Color(22, 41, 80, 255),
        size: 18.,
        anchor: TextAnchor::Start,
        rotation: 0.,
    });
    if kind == 0 && phase == 0 {
        scene.label_clips.insert(
            0,
            ClipPath {
                geometry: Geometry::Rect {
                    x: 25.,
                    y: 30.,
                    width: 95.,
                    height: 40.,
                },
                rule: ClipRule::NonZero,
            },
        );
    }
    let blend = if kind == 1 && phase == 1 {
        BlendMode::Multiply
    } else {
        BlendMode::Normal
    };
    let blends = MarkBlends::new(&scene, [("probe-business", blend)])?;
    let layout = SceneInteraction {
        scene,
        interaction: Some(InteractionBinding {
            sites: vec![InteractionSite {
                mark_id: "probe-business".into(),
                series: "probe-series".into(),
                label: "Authoritative source value 7".into(),
                value: Some(7.),
                formatted_value: Some("7".into()),
                color: Color(200, 100, 50, 128),
                position: [65., 50.],
            }],
            ..Default::default()
        }),
        plot: Some(ClipRect {
            x: 0.,
            y: 0.,
            width: 180.,
            height: 120.,
        }),
    };
    layout.validate()?;
    Ok(Proposal {
        phase,
        layout,
        blends,
    })
}

#[wasm_bindgen]
pub fn svg_transaction_probe_mount(kind: u8) -> std::result::Result<(), JsValue> {
    let initial = proposal(kind, 0).map_err(js)?;
    svg_transaction_probe_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div")?;
    root.set_id("svg-transaction-probe");
    root.set_attribute(
        "style",
        "width:180px;max-width:none;position:relative;background:white",
    )?;
    document.body().unwrap().append_child(&root)?;
    let chart = match BrowserChart::mount_with_blends(
        root.clone(),
        initial.layout.scene.clone(),
        BrowserOptions {
            renderer: BrowserRenderer::Svg,
            height: Some(120.),
            motion: browser::MotionPreference::Reduced,
            highlight: browser::BrowserHighlight::Controlled,
            legend: false,
            interaction_data: initial.layout.interaction.clone(),
            tooltip: false,
            interaction: surface::SurfaceOptions {
                plot: initial.layout.plot,
                zoom: false,
                pan: false,
                pinch: false,
                brush: false,
                ..Default::default()
            },
            ..Default::default()
        },
        initial.blends.clone(),
    ) {
        Ok(chart) => chart,
        Err(error) => {
            root.remove();
            return Err(js(error));
        }
    };
    let errors = Rc::new(RefCell::new(Vec::new()));
    let observed = errors.clone();
    chart.on_event(move |event| {
        if let BrowserEvent::Error(error) = event {
            observed.borrow_mut().push(error.0);
        }
    });
    if let Err(error) = chart.flush() {
        chart.stop();
        root.remove();
        return Err(js(error));
    }
    if !errors.borrow().is_empty() {
        chart.stop();
        root.remove();
        return Err(js("initial SVG publication failed"));
    }
    SVG_TRANSACTION_PROBE.with(|slot| {
        *slot.borrow_mut() = Some(Probe {
            chart,
            root,
            kind,
            accepted: Rc::new(RefCell::new(initial)),
            errors,
            busy: Rc::new(Cell::new(false)),
        })
    });
    Ok(())
}

#[wasm_bindgen]
pub fn svg_transaction_probe_update(phase: u8) -> std::result::Result<(), JsValue> {
    // Do not retain the global RefCell borrow while flush synchronously emits callbacks.
    let (chart, kind, accepted, errors, busy) = SVG_TRANSACTION_PROBE.with(|slot| {
        let slot = slot.borrow();
        let probe = slot.as_ref().ok_or_else(|| js("probe is stopped"))?;
        Ok::<_, JsValue>((
            probe.chart.handle(),
            probe.kind,
            probe.accepted.clone(),
            probe.errors.clone(),
            probe.busy.clone(),
        ))
    })?;
    if busy.replace(true) {
        return Err(js("reentrant probe update rejected"));
    }
    let attempt = (|| {
        let next = proposal(kind, phase).map_err(js)?;
        let before = errors.borrow().len();
        chart
            .set_layout_with_blends(next.layout.clone(), next.blends.clone())
            .map_err(js)?;
        chart.flush().map_err(js)?;
        if errors.borrow().len() != before {
            return Err(js("proposal publication rolled back"));
        }
        *accepted.borrow_mut() = next;
        Ok(())
    })();
    busy.set(false);
    attempt
}

#[wasm_bindgen]
pub fn svg_transaction_probe_snapshot() -> std::result::Result<String, JsValue> {
    SVG_TRANSACTION_PROBE.with(|slot| {
        let slot = slot.borrow(); let Some(probe) = slot.as_ref() else { return Ok(json!({"active":false}).to_string()); };
        let source = probe.accepted.borrow(); let actual = probe.chart.snapshot().map_err(js)?;
        let source_svg = render::to_svg_with_namespace_and_blends(&source.layout.scene, "probe-source", &source.blends).map_err(js)?;
        Ok(json!({"active":actual.active,"kind":probe.kind,"acceptedPhase":source.phase,
            "revision":actual.revision,"width":actual.width,"height":actual.height,"pending":actual.pending,
            "errors":probe.errors.borrow().clone(),"authoritativeSourceSVG":source_svg,
            "sourceId":"probe-business","rawValue":7.,"typedId":"probe-business",
            "typedValue":source.layout.interaction.as_ref().unwrap().sites[0].value,
            "selected":actual.selected,"hovered":actual.hovered}).to_string())
    })
}

#[wasm_bindgen]
pub fn svg_transaction_probe_stop() {
    let probe = SVG_TRANSACTION_PROBE.with(|slot| slot.borrow_mut().take());
    if let Some(probe) = probe {
        probe.chart.stop();
        probe.root.remove();
    }
}
