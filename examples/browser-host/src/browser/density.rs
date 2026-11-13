//! Independent bounded BrowserChart density fixture; does not share Entries.
use super::*;
const WIDTH: f64 = 640.;
const HEIGHT: f64 = 360.;
struct Density {
    chart: BrowserChart,
    root: web_sys::Element,
    count: usize,
    phase: u32,
    events: Rc<RefCell<Vec<Value>>>,
}
thread_local! {static DENSITY:RefCell<Option<Density>>=const{RefCell::new(None)};}
fn geometry(count: usize, index: usize, phase: u32) -> Geometry {
    let columns = ((count as f64 * WIDTH / HEIGHT).sqrt().ceil() as usize).max(1);
    let rows = count.div_ceil(columns);
    let sx = (WIDTH - 16.) / columns as f64;
    let sy = (HEIGHT - 16.) / rows as f64;
    let shift = f64::from(phase % 11) / 11.;
    Geometry::Rect {
        x: 8. + (index % columns) as f64 * sx + sx * (0.10 + shift * 0.16),
        y: 8. + (index / columns) as f64 * sy + sy * (0.10 + shift * 0.12),
        width: sx * 0.58,
        height: sy * 0.58,
    }
}
fn density_scene(count: usize, phase: u32) -> Result<Scene> {
    if ![1000, 5000, 10000, 20000].contains(&count) {
        return Err(ChartError(
            "density fixture accepts 1k/5k/10k/20k only".into(),
        ));
    }
    let mut s = ChartConfig {
        width: WIDTH,
        height: HEIGHT,
        title: format!("Density {count} changed frame {phase}"),
        ..Default::default()
    }
    .scene()?;
    for i in 0..count {
        let mut mark = Mark::new(
            format!("density-{i}"),
            geometry(count, i, phase),
            if i % 2 == 0 {
                Color(35, 105, 180, 255)
            } else {
                Color(35, 155, 85, 255)
            },
        );
        mark.label = format!("Datum {i}");
        mark.series = "density".into();
        mark.value = i as f64;
        s.marks.push(mark);
    }
    s.validate()?;
    Ok(s)
}
#[wasm_bindgen]
pub fn density_mount(renderer: &str, count: usize) -> std::result::Result<(), JsValue> {
    density_stop();
    let renderer = match renderer {
        "svg" => BrowserRenderer::Svg,
        "canvas" => BrowserRenderer::Canvas,
        _ => return Err(js("unknown density renderer")),
    };
    let scene = density_scene(count, 0).map_err(js)?;
    let document = web_sys::window()
        .and_then(|w| w.document())
        .ok_or_else(|| js("document absent"))?;
    let root = document.create_element("div")?;
    root.set_id("density-root");
    root.set_attribute(
        "style",
        "width:640px;max-width:none;position:relative;margin:16px 0;",
    )?;
    document
        .body()
        .ok_or_else(|| js("document body absent"))?
        .append_child(&root)?;
    let chart = match BrowserChart::mount(
        root.clone(),
        scene,
        BrowserOptions {
            renderer,
            height: Some(HEIGHT),
            legend: false,
            data_controls: false,
            tooltip: false,
            interaction: surface::SurfaceOptions {
                zoom: false,
                pan: false,
                brush: false,
                pinch: false,
                ..Default::default()
            },
            ..Default::default()
        },
    ) {
        Ok(chart) => chart,
        Err(error) => {
            root.remove();
            return Err(js(error));
        }
    };
    let events = Rc::new(RefCell::new(Vec::new()));
    let captured = events.clone();
    chart.on_event(move |event| {
        let mut log = captured.borrow_mut();
        if log.len() == 16 {
            log.remove(0);
        }
        log.push(event_value(event));
    });
    if let Err(error) = chart.flush() {
        chart.stop();
        root.remove();
        return Err(js(error));
    }
    DENSITY.with(|slot| {
        *slot.borrow_mut() = Some(Density {
            chart,
            root,
            count,
            phase: 0,
            events,
        })
    });
    Ok(())
}
#[wasm_bindgen]
pub fn density_update(phase: u32) -> std::result::Result<(), JsValue> {
    DENSITY.with(|slot| {
        let mut stored = slot.borrow_mut();
        let s = stored
            .as_mut()
            .ok_or_else(|| js("density fixture absent"))?;
        if phase % 11 == s.phase % 11 {
            return Err(js("density benchmark requires a changed geometry phase"));
        }
        s.chart
            .set_scene(density_scene(s.count, phase).map_err(js)?)
            .map_err(js)?;
        s.phase = phase;
        Ok(())
    })
}
#[wasm_bindgen]
pub fn density_flush() -> std::result::Result<(), JsValue> {
    DENSITY.with(|slot| {
        slot.borrow()
            .as_ref()
            .ok_or_else(|| js("density fixture absent"))?
            .chart
            .flush()
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn density_snapshot() -> std::result::Result<String, JsValue> {
    DENSITY.with(|slot|{
    let binding=slot.borrow();let Some(d)=binding.as_ref()else{return Ok(json!({"active":false}).to_string())};let s=d.chart.snapshot().map_err(js)?;
    let probe=|index|match geometry(d.count,index,d.phase){Geometry::Rect{x,y,width,height}=>json!({"id":format!("density-{index}"),"x":x+width/2.,"y":y+height/2.,"width":width,"height":height}),_=>unreachable!()};
    Ok(json!({"active":s.active,"count":d.count,"phase":d.phase,"width":s.width,"height":s.height,"revision":s.revision,"pending":s.pending,"hovered":s.hovered,"selected":s.selected,"first":probe(0),"last":probe(d.count-1),"events":d.events.borrow().clone()}).to_string())
})
}
#[wasm_bindgen]
pub fn density_stop() {
    let previous = DENSITY.with(|slot| slot.borrow_mut().take());
    if let Some(density) = previous {
        density.chart.stop();
        density.root.remove();
    }
}
