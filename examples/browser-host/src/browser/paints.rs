use super::*;
thread_local! {static PAINTS:RefCell<Vec<(String,BrowserChart,web_sys::Element)>>=const{RefCell::new(Vec::new())};}
fn scene(case: u8) -> Result<Scene> {
    let mut s = ChartConfig {
        width: 600.,
        height: 260.,
        ..Default::default()
    }
    .scene()?;
    let stops = if case == 1 {
        vec![
            GradientStop::new(0., Color(20, 180, 80, 255)),
            GradientStop::new(1., Color(180, 40, 200, 255)),
        ]
    } else {
        vec![
            GradientStop::new(0., Color(240, 40, 60, 255)),
            GradientStop::new(1., Color(30, 90, 230, 255)),
        ]
    };
    let mut a = Mark::new(
        "linear",
        Geometry::Rect {
            x: 20.,
            y: 30.,
            width: 240.,
            height: 180.,
        },
        Color(40, 150, 90, 255),
    );
    a.fill_paint = Some(FillPaint::Linear {
        start: [20., 30.],
        end: [260., 30.],
        stops,
    });
    a.series = "data".into();
    a.clip = Some(ClipRect {
        x: 35.,
        y: 35.,
        width: 220.,
        height: 170.,
    });
    let mut b = Mark::new(
        "radial",
        Geometry::Circle {
            x: 420.,
            y: 120.,
            radius: 85.,
        },
        Color(230, 140, 30, 255),
    );
    b.fill = None;
    b.fill_paint = Some(FillPaint::Radial {
        center: [420., 120.],
        radius: 85.,
        stops: vec![
            GradientStop::new(0., Color(255, 230, 110, 255)),
            GradientStop::new(0.5, Color(240, 120, 30, 255)),
            GradientStop::new(1., Color(110, 35, 100, 255)),
        ],
    });
    b.series = "data".into();
    s.marks = vec![a, b];
    if case == 2 {
        for m in &mut s.marks {
            m.fill_paint = None;
            m.fill = Some(Color(40, 150, 90, 255));
        }
    }
    if case == 3 {
        s.marks.remove(0);
    }
    if (4..=6).contains(&case) {
        let pattern = PatternFill {
            shape: match case {
                4 => PatternShape::Dots {
                    diameter: 7.,
                    stagger: true,
                },
                5 => PatternShape::Squares {
                    size: 6.,
                    stagger: true,
                },
                _ => PatternShape::Lines { width: 3. },
            },
            spacing: 13.25,
            rotation: if case == 6 { 37. } else { -17. },
            offset: [3.5, -2.25],
            foreground: Color(25, 85, 190, 220),
            background: Color(245, 230, 190, 255),
        };
        for m in &mut s.marks {
            m.fill_paint = Some(FillPaint::Pattern(pattern.clone()));
            m.opacity = 0.85;
        }
    }
    s.validate()?;
    Ok(s)
}
#[wasm_bindgen]
pub fn paints_mount() -> std::result::Result<(), JsValue> {
    paints_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for (id, renderer) in [
        ("paint-svg", BrowserRenderer::Svg),
        ("paint-canvas", BrowserRenderer::Canvas),
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
                height: Some(260.),
                motion: browser::MotionPreference::Reduced,
                ..Default::default()
            },
        )
        .map_err(js)?;
        PAINTS.with(|p| p.borrow_mut().push((id.into(), chart, root)));
    }
    Ok(())
}
#[wasm_bindgen]
pub fn paints_case(case: u8) -> std::result::Result<(), JsValue> {
    if case > 6 {
        return Err(js("invalid paint fixture"));
    }
    PAINTS.with(|p| {
        for (_, chart, _) in p.borrow().iter() {
            chart
                .handle()
                .set_scene(scene(case).map_err(js)?)
                .map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn paints_ready() -> bool {
    PAINTS.with(|p| {
        p.borrow().len() == 2
            && p.borrow().iter().all(|(_, c, _)| {
                c.handle()
                    .snapshot()
                    .is_ok_and(|s| s.pending == 0 && s.revision > 0)
            })
    })
}
#[wasm_bindgen]
pub fn paints_flush() -> std::result::Result<(), JsValue> {
    PAINTS.with(|p| {
        for (_, c, _) in p.borrow().iter() {
            c.handle().flush().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn paints_stop() {
    PAINTS.with(|p| {
        for (_, chart, root) in p.borrow_mut().drain(..) {
            chart.stop();
            root.remove();
        }
    });
}
