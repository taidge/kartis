//! Retained Radar with caller-authored offset point shapes.
use kartis::surface::{Picking, SurfaceOptions};
use web_sys::Element;

use super::*;

const WIDTH: f64 = 320.;
const HEIGHT: f64 = 240.;
const INK: Color = Color(190, 24, 76, 255);

struct Fixture {
    chart: BrowserChart,
    root: Element,
    renderer: &'static str,
    mode: u8,
    phase: u8,
}

thread_local! {
    static FIXTURES: RefCell<Vec<Fixture>> = const { RefCell::new(Vec::new()) };
}

fn config() -> ChartConfig {
    ChartConfig {
        width: WIDTH,
        height: HEIGHT,
        margin: Margin {
            left: 60.,
            right: 60.,
            top: 20.,
            bottom: 20.,
        },
        title: "Custom Radar dots".into(),
        ..Default::default()
    }
}

fn source(mode: u8, phase: u8) -> Result<RadarLayout> {
    let mut options = RadarLayoutOptions {
        domain: Some([0., 10.]),
        radius_ratio: 1.,
        axis_labels: false,
        ..Default::default()
    };
    options.grid.line.enabled = false;
    options.grid.spokes.enabled = false;
    options.series.fill_enabled = false;
    options.series.border.enabled = false;
    options.points.fill = Some(INK);
    let mut data = vec![RadarSeries::new(
        "main",
        vec![5. + 2. * f64::from(phase), 10., 0., 5.],
    )];
    if mode == 2 {
        data.push(RadarSeries::new("quiet", vec![2.; 4]));
        options.series_styles.insert(
            "quiet".into(),
            RadarSeriesStyle {
                points: Some(RadarPointOptions {
                    enabled: false,
                    ..Default::default()
                }),
                ..options.series.clone()
            },
        );
    }
    radar_layout_with_point_factory(
        &config(),
        &["north", "east", "south", "west"].map(str::to_owned),
        &data,
        &options,
        &RadarInteractionOptions {
            slices: mode == 2,
            ..Default::default()
        },
        |ctx| {
            let [x, y] = ctx.position;
            if ctx.axis_id == "north" {
                let mut node = ctx.inherit(Geometry::Path(vec![
                    PathCommand::Move(GraphicPoint::new(x + 22., y - 10.)),
                    PathCommand::Line(GraphicPoint::new(x + 46., y - 10.)),
                    PathCommand::Line(GraphicPoint::new(x + 46., y + 10.)),
                    PathCommand::Line(GraphicPoint::new(x + 22., y + 10.)),
                    PathCommand::Close,
                ]));
                node.fill = None;
                node.stroke = Some(GraphicStroke {
                    color: INK,
                    width: 4.,
                    cap: StrokeCap::Butt,
                });
                return Ok(node);
            }
            if ctx.axis_id == "east" {
                return Ok(ctx.inherit(Geometry::Path(vec![
                    PathCommand::Move(GraphicPoint::new(x - 7., y - 6.)),
                    PathCommand::Line(GraphicPoint::new(x + 12., y - 6.)),
                    PathCommand::Line(GraphicPoint::new(x + 12., y + 6.)),
                    PathCommand::Line(GraphicPoint::new(x - 7., y + 6.)),
                    PathCommand::Close,
                ])));
            }
            Ok(ctx.inherit(Geometry::Circle { x, y, radius: 0. }))
        },
    )
}

#[wasm_bindgen]
pub fn radar_custom_mount(mode: u8) -> std::result::Result<(), JsValue> {
    if mode > 2 {
        return Err(js("unknown custom Radar interaction mode"));
    }
    radar_custom_stop();
    let layout = source(mode, 0).map_err(js)?;
    let document = web_sys::window().unwrap().document().unwrap();
    for (renderer, backend) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = document.create_element("div")?;
        root.set_id(&format!("radar-custom-{renderer}"));
        root.set_attribute("style", "width:320px;margin:12px 0")?;
        document.body().unwrap().append_child(&root)?;
        let chart = BrowserChart::mount(
            root.clone(),
            layout.scene.clone(),
            BrowserOptions {
                renderer: backend,
                height: Some(HEIGHT),
                highlight: browser::BrowserHighlight::Controlled,
                interaction_data: Some(layout.interaction_binding()),
                interaction: SurfaceOptions {
                    plot: Some(ClipRect::plot(&config())),
                    picking: if mode == 1 {
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
            radar_custom_stop();
            js(error)
        })?;
        FIXTURES.with(|entries| {
            entries.borrow_mut().push(Fixture {
                chart,
                root,
                renderer,
                mode,
                phase: 0,
            })
        });
    }
    Ok(())
}

#[wasm_bindgen]
pub fn radar_custom_phase(phase: u8) -> std::result::Result<(), JsValue> {
    if phase > 1 {
        return Err(js("unknown custom Radar phase"));
    }
    FIXTURES.with(|entries| {
        for fixture in entries.borrow_mut().iter_mut() {
            let layout = source(fixture.mode, phase).map_err(js)?;
            let binding = layout.interaction_binding();
            fixture
                .chart
                .set_scene_with_interaction(layout.scene, binding)
                .map_err(js)?;
            fixture.phase = phase;
        }
        Ok(())
    })
}

#[wasm_bindgen]
pub fn radar_custom_flush() -> std::result::Result<(), JsValue> {
    FIXTURES.with(|entries| {
        for fixture in entries.borrow().iter() {
            fixture.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}

#[wasm_bindgen]
pub fn radar_custom_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with(|entries| {
        let entries = entries.borrow();
        let Some(fixture) = entries.iter().find(|fixture| fixture.renderer == renderer) else {
            return Ok(json!({"active":false}).to_string());
        };
        let state = fixture.chart.snapshot().map_err(js)?;
        let layout = source(fixture.mode, fixture.phase).map_err(js)?;
        let north = &layout.slices[0];
        let target = &north.rows[0];
        let east = &layout.slices[1].rows[0];
        let active = state.interaction.as_ref().map(|active| {
            json!({
                "slice":active.slice_id,
                "anchor":active.anchor,
                "rows":active.rows.iter().map(|row| json!({"id":row.mark_id,"raw":row.value})).collect::<Vec<_>>()
            })
        });
        Ok(json!({
            "active":state.active,"revision":state.revision,"pending":state.pending,
            "width":state.width,"height":state.height,"center":layout.center,
            "hovered":state.hovered,"selected":state.selected,"current":active,
            "view":[state.view.scale,state.view.translate_x,state.view.translate_y],
            "target":{"id":target.point_mark_id,"position":target.position,"raw":target.raw_value},
            "east":{"id":east.point_mark_id,"position":east.position},
            "northSlice":north.id
        })
        .to_string())
    })
}

#[wasm_bindgen]
pub fn radar_custom_rejected() -> bool {
    FIXTURES.with(|entries| {
        entries.borrow().iter().all(|fixture| {
            let Ok(mut layout) = source(fixture.mode, fixture.phase) else {
                return false;
            };
            let binding = layout.interaction_binding();
            let target = &layout.slices[0].rows[0].point_mark_id;
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
pub fn radar_custom_stop() {
    let fixtures = FIXTURES.with(|entries| std::mem::take(&mut *entries.borrow_mut()));
    for fixture in fixtures {
        fixture.chart.stop();
        fixture.root.remove();
    }
}
