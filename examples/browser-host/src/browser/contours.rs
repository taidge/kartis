use kartis::browser::MotionPreference;
use kartis::motion::{
    ClosedAlignment, ContourBindings, ContourMorphOptions, MorphContour, PathMorphOptions,
};

use super::*;

struct ContourEntry {
    chart: Option<BrowserChart>,
    handle: BrowserHandle,
    errors: Rc<Cell<usize>>,
}
thread_local! { static CONTOURS: RefCell<BTreeMap<String,ContourEntry>> = const { RefCell::new(BTreeMap::new()) }; }

fn circle(width: f64, height: f64, radius: f64, reverse: bool) -> Vec<PathCommand> {
    let commands = Geometry::Circle {
        x: width * 0.5,
        y: height * 0.5,
        radius,
    }
    .commands();
    if !reverse {
        return commands;
    }
    let PathCommand::Move(start) = commands[0] else {
        unreachable!()
    };
    let mut cursor = start;
    let mut segments = Vec::new();
    for command in &commands[1..] {
        if let PathCommand::Cubic { first, second, end } = command {
            segments.push((cursor, *first, *second));
            cursor = *end;
        }
    }
    let mut output = vec![PathCommand::Move(start)];
    for (end, second, first) in segments.into_iter().rev() {
        output.push(PathCommand::Cubic { first, second, end });
    }
    output.push(PathCommand::Close);
    output
}
fn contour_scene(width: f64, height: f64, phase: u32) -> Scene {
    let mut output = ChartConfig {
        width,
        height,
        title: "Explicit keyed contour morph".into(),
        ..Default::default()
    }
    .scene()
    .unwrap();
    let outer = circle(width, height, if phase == 1 { 90. } else { 75. }, false);
    let hole = if phase == 0 || phase == 3 {
        vec![]
    } else {
        circle(width, height, if phase == 1 { 30. } else { 20. }, true)
    };
    let geometry = Geometry::Path(if phase == 1 {
        hole.into_iter().chain(outer).collect()
    } else {
        outer.into_iter().chain(hole).collect()
    });
    let mut mark = Mark::new("donut", geometry, Color(30, 90, 160, 255));
    mark.series = "series".into();
    mark.label = "Donut".into();
    mark.value = phase as f64;
    output.marks.push(mark);
    output
}
fn bindings(keys: &[&str]) -> ContourBindings {
    ContourBindings::from([(
        "donut".into(),
        keys.iter().map(|key| (*key).into()).collect(),
    )])
}

#[wasm_bindgen]
pub fn contours_mount(id: &str, scene_animation: bool) -> std::result::Result<(), JsValue> {
    let previous = ENTRIES.with(|entries| entries.borrow_mut().remove(id));
    drop(previous);
    let previous = CONTOURS.with(|entries| entries.borrow_mut().remove(id));
    drop(previous);
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document
        .get_element_by_id(&format!("{id}-root"))
        .ok_or_else(|| js("unknown contour surface"))?;
    let width = root.get_bounding_client_rect().width().max(240.);
    let height = 260.;
    let mut options = motion::PropertyMotionOptions::default();
    options.geometry.contour_morphs.insert(
        "donut".into(),
        ContourMorphOptions {
            sampling: PathMorphOptions {
                samples: 96,
                tolerance: 0.1,
                alignment: ClosedAlignment::Rotate,
            },
            contours: vec![
                MorphContour {
                    key: "outer".into(),
                    closed: true,
                    collapse: [width * 0.5, height * 0.5],
                },
                MorphContour {
                    key: "hole".into(),
                    closed: true,
                    collapse: [width * 0.5, height * 0.5],
                },
            ],
        },
    );
    let animation = if scene_animation {
        BrowserAnimation::Scene
    } else {
        BrowserAnimation::PropertiesWithContours {
            options,
            bindings: bindings(&["outer"]),
        }
    };
    let chart = BrowserChart::mount(
        root,
        contour_scene(width, height, 0),
        BrowserOptions {
            animation,
            renderer: if id == "svg" {
                BrowserRenderer::Svg
            } else {
                BrowserRenderer::Canvas
            },
            height: Some(height),
            legend: false,
            data_controls: false,
            tooltip: false,
            motion: MotionPreference::System,
            ..Default::default()
        },
    )
    .map_err(js)?;
    let errors = Rc::new(Cell::new(0));
    let observed = errors.clone();
    chart.on_event(move |event| {
        if matches!(event, BrowserEvent::Error(_)) {
            observed.set(observed.get() + 1);
        }
    });
    let handle = chart.handle();
    CONTOURS.with(|entries| {
        entries.borrow_mut().insert(
            id.into(),
            ContourEntry {
                chart: Some(chart),
                handle,
                errors,
            },
        )
    });
    Ok(())
}
#[wasm_bindgen]
pub fn contours_phase(id: &str, phase: u32) -> std::result::Result<(), JsValue> {
    CONTOURS.with(|entries| {
        let entries = entries.borrow();
        let entry = entries
            .get(id)
            .ok_or_else(|| js("missing contour fixture"))?;
        let size = entry.handle.snapshot().map_err(js)?;
        let scene = contour_scene(size.width, size.height, phase);
        if phase == 2 {
            entry.handle.animate_to(scene).map_err(js)
        } else {
            entry
                .handle
                .animate_to_with_contours(
                    scene,
                    bindings(if phase == 1 {
                        &["hole", "outer"]
                    } else {
                        &["outer"]
                    }),
                )
                .map_err(js)
        }
    })
}
#[wasm_bindgen]
pub fn contours_bad_binding_rejected(id: &str) -> std::result::Result<bool, JsValue> {
    CONTOURS.with(|entries| {
        let entries = entries.borrow();
        let entry = entries
            .get(id)
            .ok_or_else(|| js("missing contour fixture"))?;
        let size = entry.handle.snapshot().map_err(js)?;
        Ok(entry
            .handle
            .animate_to_with_contours(
                contour_scene(size.width, size.height, 1),
                bindings(&["outer", "outer"]),
            )
            .is_err())
    })
}
#[wasm_bindgen]
pub fn contours_snapshot(id: &str) -> std::result::Result<String, JsValue> {
    CONTOURS.with(|entries| {
        let entries = entries.borrow();
        let entry = entries.get(id).ok_or_else(||js("missing contour fixture"))?;
        if !entry.handle.is_active() { return Ok(json!({"active":false,"errors":entry.errors.get()}).to_string()); }
        let state = entry.handle.snapshot().map_err(js)?;
        Ok(json!({"active":state.active,"animating":state.animating,"pending":state.pending,"revision":state.revision,
            "width":state.width,"height":state.height,"kind":format!("{:?}",state.animation_kind),"errors":entry.errors.get()}).to_string())
    })
}
#[wasm_bindgen]
pub fn contours_drop(id: &str) -> std::result::Result<(), JsValue> {
    CONTOURS.with(|entries| {
        let mut entries = entries.borrow_mut();
        let entry = entries
            .get_mut(id)
            .ok_or_else(|| js("missing contour fixture"))?;
        drop(entry.chart.take());
        Ok(())
    })
}
