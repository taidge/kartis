//! Compact and full-year calendars with real dates, cells and month outlines.
use super::*;
struct Fixture {
    chart: BrowserChart,
    root: web_sys::Element,
    renderer: String,
    case: u8,
    phase: u8,
    events: Rc<RefCell<Vec<Value>>>,
}
thread_local! {static CALENDARS:RefCell<Vec<Fixture>>=const{RefCell::new(Vec::new())};}
fn source(case: u8, phase: u8) -> Result<CalendarLayout> {
    if case > 3 || phase > 1 {
        return Err(ChartError("invalid calendar layout case/phase".into()));
    }
    let c = ChartConfig {
        width: 640.,
        height: 420.,
        margin: Margin {
            left: 70.,
            right: 35.,
            top: 40.,
            bottom: 35.,
        },
        title: "Civil calendar styles <safe>".into(),
        ..Default::default()
    };
    let (start, end, target, zero, null) = if case == 0 {
        (
            (2023, 12, 24),
            (2024, 1, 10),
            (2024, 1, 5),
            (2024, 1, 3),
            (2024, 1, 4),
        )
    } else if case == 3 {
        (
            (2023, 12, 24),
            (2024, 3, 8),
            (2024, 3, 1),
            (2024, 2, 28),
            (2024, 2, 29),
        )
    } else {
        (
            (2024, 2, 24),
            (2024, 3, 8),
            (2024, 3, 1),
            (2024, 2, 28),
            (2024, 2, 29),
        )
    };
    let data = [
        CalendarCell::new(target, Some(6. + f64::from(phase) * 3.)),
        CalendarCell::new(zero, Some(0.)),
        CalendarCell::new(null, None),
        CalendarCell::new(start, Some(2.)),
    ];
    calendar_layout(
        &c,
        &data,
        &CalendarLayoutOptions {
            mode: if case >= 2 {
                CalendarLayoutMode::FullYears
            } else {
                CalendarLayoutMode::Compact
            },
            start: Some(start),
            end: Some(end),
            orientation: if case % 2 == 1 {
                CalendarOrientation::Vertical
            } else {
                CalendarOrientation::Horizontal
            },
            first_weekday: if phase == 0 {
                CalendarWeekday::Monday
            } else {
                CalendarWeekday::Sunday
            },
            weekday_ticks: vec![0, 2, 4, 6],
            weekday_space: 42.,
            weekday_style: CalendarTextStyle {
                color: Some(Color(20, 80, 130, 255)),
                size: Some(13.),
                rotation: if case % 2 == 1 { -90. } else { 0. },
            },
            square: case != 0,
            day_spacing: if case >= 2 { 1. } else { 3. + f64::from(phase) },
            day_radius: if case == 1 {
                if phase == 0 { 8. } else { 10. }
            } else if phase == 0 {
                2.
            } else {
                3.
            },
            day_border: if case >= 2 {
                None
            } else {
                Some(GraphicStroke {
                    color: Color(255, 255, 255, 255),
                    width: 1.,
                    cap: StrokeCap::Butt,
                })
            },
            day_labels: if case < 2 {
                CalendarDayLabel::Value
            } else {
                CalendarDayLabel::None
            },
            day_label_style: CalendarTextStyle {
                color: Some(Color(20, 30, 60, 255)),
                size: Some(12.),
                ..Default::default()
            },
            colors: ColorScaleOptions {
                domain: Some([0., 12.]),
                colors: vec![Color(35, 80, 190, 255), Color(220, 45, 50, 255)],
                unknown: Color(235, 235, 238, 255),
                ..Default::default()
            },
            value_format: ValueFormat::Fixed(1),
            empty_label: "NULL".into(),
            month_border: Some(GraphicStroke {
                color: Color(90, 30, 140, 255),
                width: 1.5,
                cap: StrokeCap::Butt,
            }),
            month_outline_padding: 0.2,
            month_label_position: if phase == 0 {
                CalendarLegendPosition::Before
            } else {
                CalendarLegendPosition::After
            },
            month_style: CalendarTextStyle {
                color: Some(Color(80, 25, 130, 255)),
                size: Some(if case < 2 { 14. } else { 11. }),
                rotation: if phase == 0 { 0. } else { -10. },
            },
            month_names: [((2024, 2), "Feb <safe>".into())].into(),
            year_labels: Some(true),
            year_style: CalendarTextStyle {
                color: Some(Color(40, 60, 80, 255)),
                size: Some(14.),
                ..Default::default()
            },
            year_gap: 18.,
            year_legend: true,
            ..Default::default()
        },
    )
}
fn target(case: u8) -> CalendarCivilDate {
    if case == 0 {
        (2024, 1, 5)
    } else {
        (2024, 3, 1)
    }
}
#[wasm_bindgen]
pub fn calendar_layout_mount(case: u8) -> std::result::Result<(), JsValue> {
    let layout = source(case, 0).map_err(js)?;
    calendar_layout_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for (name, renderer) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = document.create_element("div")?;
        root.set_id(&format!("calendar-layout-{name}"));
        root.set_attribute(
            "style",
            "width:640px;max-width:none;position:relative;margin:12px 0",
        )?;
        document.body().unwrap().append_child(&root)?;
        let chart = match BrowserChart::mount(
            root.clone(),
            layout.scene.clone(),
            BrowserOptions {
                renderer,
                height: Some(420.),
                data_controls: case < 2,
                highlight: browser::BrowserHighlight::Controlled,
                motion: browser::MotionPreference::Reduced,
                tooltip_options: TooltipOptions {
                    lines: vec![vec![TooltipPart::Label]],
                    ..Default::default()
                },
                interaction: surface::SurfaceOptions {
                    zoom: false,
                    pan: false,
                    pinch: false,
                    brush: false,
                    ..Default::default()
                },
                ..Default::default()
            },
        ) {
            Ok(c) => c,
            Err(error) => {
                root.remove();
                calendar_layout_stop();
                return Err(js(error));
            }
        };
        let events = Rc::new(RefCell::new(Vec::new()));
        let captured = events.clone();
        chart.on_event(move |event| {
            let mut e = captured.borrow_mut();
            if e.len() == 32 {
                e.remove(0);
            }
            e.push(event_value(event));
        });
        chart.flush().map_err(js)?;
        CALENDARS.with(|slot| {
            slot.borrow_mut().push(Fixture {
                chart,
                root,
                renderer: name.into(),
                case,
                phase: 0,
                events,
            })
        });
    }
    Ok(())
}
#[wasm_bindgen]
pub fn calendar_layout_phase(phase: u8) -> std::result::Result<(), JsValue> {
    CALENDARS.with(|slot| {
        for f in slot.borrow_mut().iter_mut() {
            f.chart
                .set_scene(source(f.case, phase).map_err(js)?.scene)
                .map_err(js)?;
            f.phase = phase;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn calendar_layout_flush() -> std::result::Result<(), JsValue> {
    CALENDARS.with(|slot| {
        for f in slot.borrow().iter() {
            f.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn calendar_layout_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    CALENDARS.with(|slot|{
 let entries=slot.borrow();let Some(f)=entries.iter().find(|f|f.renderer==renderer)else{return Ok(json!({"active":false}).to_string());};let state=f.chart.snapshot().map_err(js)?;let l=source(f.case,f.phase).map_err(js)?;
 let value=|d:&ComputedCalendarDay|json!({"id":d.id,"date":[d.date.0,d.date.1,d.date.2],"value":d.value,"provided":d.provided,"formatted":d.formatted_value,"label":d.label,"rect":[d.rect.x,d.rect.y,d.rect.width,d.rect.height],"week":d.week,"weekday":d.weekday,"visible":d.visible});
 let t=l.day(target(f.case)).unwrap();let zero=l.days.iter().find(|d|d.value==Some(0.)).unwrap();let null=l.days.iter().find(|d|d.value.is_none()&&d.provided).unwrap();let actual=l.hit_test(t.rect.x+t.rect.width/2.,t.rect.y+t.rect.height/2.).map(|d|d.id.clone());
 Ok(json!({"active":state.active,"case":f.case,"phase":f.phase,"width":state.width,"height":state.height,"revision":state.revision,"pending":state.pending,"hovered":state.hovered,"selected":state.selected,"hidden":state.hidden,"start":[l.start.0,l.start.1,l.start.2],"end":[l.end.0,l.end.1,l.end.2],"dayCount":l.days.len(),"target":value(t),"zero":value(zero),"null":value(null),"sceneHit":actual,"panels":l.panels.iter().map(|p|json!({"year":p.year,"weeks":p.weeks,"grid":[p.grid.x,p.grid.y,p.grid.width,p.grid.height],"cell":[p.cell_width,p.cell_height]})).collect::<Vec<_>>(),"months":l.months.iter().map(|m|json!({"year":m.year,"month":m.month,"id":m.mark_id,"days":m.day_ids,"commands":m.outline.commands().len(),"bounds":[m.bounds.x,m.bounds.y,m.bounds.width,m.bounds.height]})).collect::<Vec<_>>(),"sharedLabels":l.scene.labels.iter().enumerate().filter(|(i,_)|!l.scene.label_series.contains_key(i)).map(|(_,l)|l.text.clone()).collect::<Vec<_>>(),"ownedLabels":l.scene.label_series.values().cloned().collect::<Vec<_>>(),"events":f.events.borrow().clone()}).to_string())
})
}
#[wasm_bindgen]
pub fn calendar_layout_rejected() -> bool {
    CALENDARS.with(|slot| {
        slot.borrow().iter().all(|f| {
            let Ok(mut scene) = source(f.case, f.phase).map(|l| l.scene) else {
                return false;
            };
            scene.marks[0].opacity = f64::NAN;
            f.chart.set_scene(scene).is_err()
        })
    })
}
#[wasm_bindgen]
pub fn calendar_layout_stop() {
    let entries = CALENDARS.with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    for f in entries {
        f.chart.stop();
        f.root.remove();
    }
}
