use super::*;
thread_local! {static CLIPS:RefCell<Vec<(String,BrowserChart,web_sys::Element)>>=const{RefCell::new(Vec::new())};}
thread_local! {static CLIP_ERRORS:RefCell<Vec<String>>=const{RefCell::new(Vec::new())};}
fn scene(case: u8) -> Result<Scene> {
    let mut scene = ChartConfig {
        width: 600.,
        height: 280.,
        title: "Retained SVG clip and dash".into(),
        ..Default::default()
    }
    .scene()?;
    let mut main = Mark::new(
        "clip-main",
        Geometry::Rect {
            x: 60.,
            y: 50.,
            width: 120.,
            height: 100.,
        },
        Color(45, 110, 185, 255),
    );
    main.stroke = Some(GraphicStroke {
        color: Color(225, 115, 30, 255),
        width: 20.,
        cap: StrokeCap::Butt,
    });
    main.clip = Some(ClipRect {
        x: 55.,
        y: 45.,
        width: 130.,
        height: 110.,
    });
    main.clip_path = Some(Box::new(ClipPath::circle(
        120.,
        100.,
        if case == 1 { 50. } else { 70. },
    )));
    let mut dashed = Mark::new(
        "clip-dash",
        Geometry::Path(vec![
            PathCommand::Move(GraphicPoint::new(250., 70.)),
            PathCommand::Line(GraphicPoint::new(550., 70.)),
        ]),
        Color(0, 0, 0, 255),
    );
    dashed.fill = None;
    dashed.stroke = Some(GraphicStroke {
        color: Color(125, 45, 160, 255),
        width: 10.,
        cap: StrokeCap::Round,
    });
    dashed.stroke_dash = Some(StrokeDash {
        segments: vec![15., 10.],
        offset: 3.,
    });
    dashed.clip = Some(ClipRect {
        x: 280.,
        y: 20.,
        width: 200.,
        height: 90.,
    });
    dashed.clip_path = Some(Box::new(ClipPath::circle(400., 70., 100.)));
    let mut shape = Geometry::Rect {
        x: 300.,
        y: 120.,
        width: 200.,
        height: 140.,
    }
    .commands();
    shape.extend(
        Geometry::Rect {
            x: 350.,
            y: 160.,
            width: 100.,
            height: 60.,
        }
        .commands(),
    );
    let mut hole = Mark::new(
        "clip-hole",
        Geometry::Rect {
            x: 300.,
            y: 120.,
            width: 200.,
            height: 140.,
        },
        Color(55, 165, 85, 255),
    );
    hole.clip_path = Some(Box::new(ClipPath {
        geometry: Geometry::Path(shape),
        rule: if case == 1 {
            ClipRule::NonZero
        } else {
            ClipRule::EvenOdd
        },
    }));
    scene.marks = vec![main, dashed, hole];
    scene.labels.push(Label {
        x: 40.,
        y: 235.,
        text: "CLIPPED ROTATED TEXT".into(),
        size: 24.,
        color: Color(15, 25, 45, 255),
        anchor: TextAnchor::Start,
        rotation: -15.,
    });
    scene.label_clips.insert(
        0,
        ClipPath {
            geometry: Geometry::Rect {
                x: 40.,
                y: 190.,
                width: if case == 1 { 110. } else { 140. },
                height: 70.,
            },
            rule: ClipRule::NonZero,
        },
    );
    if case == 1 || case == 2 {
        scene.marks[1].stroke_dash = None;
    }
    if case == 2 || case == 4 {
        for mark in &mut scene.marks {
            mark.clip_path = None;
        }
        scene.label_clips.clear();
    }
    if case == 2 || case == 3 {
        for mark in &mut scene.marks {
            mark.clip = None;
        }
    }
    scene.draw_order = if case == 1 {
        vec![
            DrawItem::Label(0),
            DrawItem::Mark(2),
            DrawItem::Mark(0),
            DrawItem::Mark(1),
        ]
    } else {
        vec![
            DrawItem::Mark(0),
            DrawItem::Mark(1),
            DrawItem::Mark(2),
            DrawItem::Label(0),
        ]
    };
    scene.validate()?;
    Ok(scene)
}
#[wasm_bindgen]
pub fn clips_mount() -> std::result::Result<(), JsValue> {
    clips_stop();
    CLIP_ERRORS.with(|errors| errors.borrow_mut().clear());
    let document = web_sys::window().unwrap().document().unwrap();
    for (id, renderer) in [
        ("clip-svg", BrowserRenderer::Svg),
        ("clip-canvas", BrowserRenderer::Canvas),
    ] {
        let root = document.create_element("div")?;
        root.set_id(id);
        root.set_attribute("style", "width:600px;max-width:100%;margin:10px 0")?;
        document.body().unwrap().append_child(&root)?;
        let chart = BrowserChart::mount(
            root.clone(),
            scene(0).map_err(js)?,
            BrowserOptions {
                renderer,
                legend: false,
                data_controls: false,
                tooltip: false,
                height: Some(280.),
                motion: browser::MotionPreference::Reduced,
                ..Default::default()
            },
        )
        .map_err(js)?;
        chart.on_event(|event| {
            if let BrowserEvent::Error(error) = event {
                CLIP_ERRORS.with(|errors| errors.borrow_mut().push(error.to_string()));
            }
        });
        CLIPS.with(|entries| entries.borrow_mut().push((id.into(), chart, root)));
    }
    Ok(())
}
#[wasm_bindgen]
pub fn clips_flush() -> std::result::Result<(), JsValue> {
    CLIPS.with(|entries| {
        for (_, chart, _) in entries.borrow().iter() {
            chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn clips_errors() -> String {
    CLIP_ERRORS.with(|errors| json!(*errors.borrow()).to_string())
}
#[wasm_bindgen]
pub fn clips_phase(case: u8) -> std::result::Result<(), JsValue> {
    if case > 4 {
        return Err(js("invalid clip case"));
    }
    let scene = scene(case).map_err(js)?;
    CLIPS.with(|entries| {
        for (_, chart, _) in entries.borrow().iter() {
            chart.set_scene(scene.clone()).map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn clips_snapshot() -> std::result::Result<String, JsValue> {
    CLIPS.with(|entries| {
        entries
            .borrow()
            .iter()
            .map(|(id, chart, _)| {
                let s = chart.snapshot().map_err(js)?;
                Ok(json!({"id":id,"active":s.active,"pending":s.pending,"revision":s.revision}))
            })
            .collect::<std::result::Result<Vec<Value>, JsValue>>()
            .map(|states| json!(states).to_string())
    })
}
#[wasm_bindgen]
pub fn clips_stop() {
    let entries = CLIPS.with(|entries| std::mem::take(&mut *entries.borrow_mut()));
    for (_, chart, root) in entries {
        drop(chart);
        root.remove();
    }
}
