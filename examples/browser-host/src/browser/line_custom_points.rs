//! Retained PointLine with caller-authored geometry and original typed keys.
use super::*;
use kartis::surface::{Picking, SurfaceOptions};
use web_sys::Element;

const INK: Color = Color(174, 34, 97, 255);

struct Fixture {
    chart: BrowserChart,
    root: Element,
    renderer: &'static str,
    layout: PointLineLayout,
}

thread_local! {
    static FIXTURES: RefCell<Vec<Fixture>> = const { RefCell::new(Vec::new()) };
}

fn config() -> ChartConfig {
    ChartConfig {
        width: 320.,
        height: 240.,
        margin: Margin {
            left: 60.,
            right: 60.,
            top: 20.,
            bottom: 20.,
        },
        title: "Custom PointLine dots".into(),
        ..Default::default()
    }
}

fn rect(x: f64, y: f64, width: f64, height: f64) -> Geometry {
    Geometry::Path(vec![
        PathCommand::Move(GraphicPoint::new(x, y)),
        PathCommand::Line(GraphicPoint::new(x + width, y)),
        PathCommand::Line(GraphicPoint::new(x + width, y + height)),
        PathCommand::Line(GraphicPoint::new(x, y + height)),
        PathCommand::Close,
    ])
}

fn source(phase: u8) -> Result<PointLineLayout> {
    let domain = [
        CategoryKey::from("1"),
        CategoryKey::from(1.),
        CategoryKey::from("edge"),
    ];
    let data = [PointLineSeries::new(
        "main",
        vec![
            PointLineDatum::new(domain[0].clone(), 2.),
            PointLineDatum::new(domain[1].clone(), 5. + 2. * f64::from(phase)),
            PointLineDatum::new(domain[2].clone(), 4.),
        ],
    )];
    let mut options = PointLineOptions {
        domain: Some(domain.to_vec()),
        ..Default::default()
    };
    options.line.y_domain = Some([0., 10.]);
    options.line.axes.x.visible = false;
    options.line.axes.x.grid = false;
    options.line.axes.y.visible = false;
    options.line.axes.y.grid = false;
    let style = LineStyleOptions {
        series: LineSeriesStyle {
            width: 0.,
            point_color: Some(INK),
            ..Default::default()
        },
        ..Default::default()
    };
    point_line_layout_with_point_factory(&config(), &data, &options, &style, None, |point| {
        if point.raw_x != LinePointX::Category(&domain[point.source_index]) {
            return Err(ChartError(
                "custom PointLine fixture received an incorrect raw key".into(),
            ));
        }
        let [x, y] = point.position;
        match point.source_index {
            0 => Ok(point.inherit(Geometry::Circle { x, y, radius: 6. })),
            1 => {
                let mut node = point.inherit(rect(x + 22., y - 10., 24., 20.));
                node.fill = None;
                node.stroke = Some(GraphicStroke {
                    color: INK,
                    width: 4.,
                    cap: StrokeCap::Butt,
                });
                Ok(node)
            }
            _ => Ok(point.inherit(rect(x - 7., y - 6., 19., 12.))),
        }
    })
}

#[wasm_bindgen]
pub fn line_custom_mount(nearest: bool) -> std::result::Result<(), JsValue> {
    line_custom_stop();
    let layout = source(0).map_err(js)?;
    let document = web_sys::window().unwrap().document().unwrap();
    for (renderer, backend) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = document.create_element("div")?;
        root.set_id(&format!("line-custom-{renderer}"));
        root.set_attribute("style", "width:320px;margin:12px 0")?;
        document.body().unwrap().append_child(&root)?;
        let chart = BrowserChart::mount(
            root.clone(),
            layout.scene.clone(),
            BrowserOptions {
                renderer: backend,
                height: Some(240.),
                highlight: browser::BrowserHighlight::Controlled,
                interaction_data: Some(layout.interaction_binding()),
                interaction: SurfaceOptions {
                    plot: Some(layout.plot),
                    picking: if nearest {
                        Picking::Nearest { distance: 6. }
                    } else {
                        Picking::Geometry
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .map_err(|error| {
            root.remove();
            line_custom_stop();
            js(error)
        })?;
        FIXTURES.with(|entries| {
            entries.borrow_mut().push(Fixture {
                chart,
                root,
                renderer,
                layout: layout.clone(),
            })
        });
    }
    Ok(())
}

#[wasm_bindgen]
pub fn line_custom_phase(phase: u8) -> std::result::Result<(), JsValue> {
    if phase > 1 {
        return Err(js("unknown custom PointLine phase"));
    }
    let layout = source(phase).map_err(js)?;
    FIXTURES.with(|entries| {
        for fixture in entries.borrow_mut().iter_mut() {
            fixture
                .chart
                .set_layout(layout.clone().into())
                .map_err(js)?;
            fixture.layout = layout.clone();
        }
        Ok(())
    })
}

#[wasm_bindgen]
pub fn line_custom_flush() -> std::result::Result<(), JsValue> {
    FIXTURES.with(|entries| {
        for fixture in entries.borrow().iter() {
            fixture.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}

#[wasm_bindgen]
pub fn line_custom_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with(|entries| {
        let entries = entries.borrow();
        let Some(fixture) = entries.iter().find(|fixture| fixture.renderer == renderer) else {
            return Ok(json!({"active":false}).to_string());
        };
        let state = fixture.chart.snapshot().map_err(js)?;
        let sites: Vec<_> = fixture.layout.sites.iter().map(|site| {
            let x = match &site.raw_x {
                CategoryKey::Number(value) => json!({"number":value}),
                CategoryKey::String(value) => json!({"text":value}),
            };
            json!({"id":site.mark_id,"position":site.position,"x":x,"y":site.raw_y,"formattedX":site.formatted_x})
        }).collect();
        Ok(json!({
            "active":state.active,"revision":state.revision,"pending":state.pending,
            "width":state.width,"height":state.height,"hovered":state.hovered,"selected":state.selected,
            "sites":sites,
            "current":state.interaction.as_ref().map(|active| json!({
                "anchor":active.anchor,
                "id":active.rows[0].mark_id,
                "raw":active.rows[0].value,
            }))
        }).to_string())
    })
}

#[wasm_bindgen]
pub fn line_custom_rejected() -> bool {
    FIXTURES.with(|entries| {
        entries.borrow().iter().all(|fixture| {
            let mut layout = fixture.layout.clone();
            let binding = layout.interaction_binding();
            let target = &layout.sites[1].mark_id;
            let Some(mark) = layout
                .scene
                .marks
                .iter_mut()
                .find(|mark| &mark.id == target)
            else {
                return false;
            };
            let Geometry::Path(path) = &mut mark.geometry else {
                return false;
            };
            path.pop();
            fixture
                .chart
                .set_scene_with_interaction(layout.scene, binding)
                .is_err()
        })
    })
}

#[wasm_bindgen]
pub fn line_custom_stop() {
    let fixtures = FIXTURES.with(|entries| std::mem::take(&mut *entries.borrow_mut()));
    for fixture in fixtures {
        fixture.chart.stop();
        fixture.root.remove();
    }
}
