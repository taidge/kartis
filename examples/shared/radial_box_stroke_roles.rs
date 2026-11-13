//! Shared native/WASM application for RadialBar and BoxPlot stroke roles.
use glory::render::{GraphicStroke, PathCommand, StrokeCap};
use kartis::*;
use serde_json::{Value, json};

pub const CASE_COUNT: u32 = 9;
pub const CASE_NAMES: [&str; 9] = [
    "radial-legacy",
    "radial-collapsed",
    "radial-contrast",
    "box-vertical",
    "box-horizontal",
    "box-fixed-theme",
    "box-borderless",
    "box-contrast",
    "box-zero-width",
];
#[derive(Clone)]
pub struct Frame {
    pub layout: SceneInteraction,
    pub facts: Value,
}
pub fn rgba(c: Color) -> [u8; 4] {
    [c.0, c.1, c.2, c.3]
}
fn stroke(c: Color, width: f64) -> GraphicStroke {
    GraphicStroke {
        color: c,
        width,
        cap: StrokeCap::Square,
    }
}
fn role(dark: f64, opacity: f64) -> InheritedColor {
    InheritedColor::FromContext(vec![
        ColorModifier::Darker(dark),
        ColorModifier::Opacity(opacity),
    ])
}
fn contrast() -> InheritedColor {
    InheritedColor::Contrast {
        dark: Color(0, 0, 0, 255),
        light: Color(255, 255, 255, 255),
    }
}
pub fn palette(phase: u32) -> [Color; 2] {
    if phase == 0 {
        [Color(20, 40, 60, 160), Color(200, 180, 160, 220)]
    } else {
        [Color(60, 20, 100, 192), Color(100, 200, 40, 240)]
    }
}
fn geometry(g: &Geometry) -> Value {
    match g {
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => json!({"kind":"rect","x":x,"y":y,"width":width,"height":height}),
        Geometry::Circle { x, y, radius } => json!({"kind":"circle","x":x,"y":y,"radius":radius}),
        Geometry::Path(commands) => json!({"kind":"path","commands":commands.iter().map(|c|match c{
            PathCommand::Move(p)=>json!(["M",p.x,p.y]),PathCommand::Line(p)=>json!(["L",p.x,p.y]),
            PathCommand::Cubic{first,second,end}=>json!(["C",first.x,first.y,second.x,second.y,end.x,end.y]),
            PathCommand::Close=>json!(["Z"]),}).collect::<Vec<_>>()}),
    }
}
fn scene_facts(s: &Scene) -> Value {
    json!({"width":s.width,"height":s.height,"background":rgba(s.background),"marks":s.marks.iter().map(|m|json!({
        "id":m.id,"series":m.series,"value":m.value,"label":m.label,"geometry":geometry(&m.geometry),
        "fill":m.fill.map(rgba),"stroke":m.stroke.map(|s|json!({"rgba":rgba(s.color),"width":s.width,"cap":format!("{:?}",s.cap)})),
        "opacity":m.opacity,"clip":m.clip.map(|r|[r.x,r.y,r.width,r.height]),"fillPaint":format!("{:?}",m.fill_paint),
    })).collect::<Vec<_>>(),"labels":s.labels.iter().map(|l|json!({"text":l.text,"rgba":rgba(l.color),"position":[l.x,l.y]})).collect::<Vec<_>>()})
}
pub fn build(case: u32, phase: u32) -> Result<Frame> {
    build_impl(case, phase, None)
}
pub fn build_invalid(case: u32, phase: u32, mode: u32) -> Result<Frame> {
    if mode > 1 {
        return Err(ChartError("unknown stroke-role invalid mode".into()));
    }
    build_impl(case, phase, Some(mode))
}
fn build_impl(case: u32, phase: u32, invalid: Option<u32>) -> Result<Frame> {
    if case >= CASE_COUNT || phase > 3 {
        return Err(ChartError("unknown stroke-role case or phase".into()));
    }
    let mut c = ChartConfig {
        width: 420.,
        height: 240.,
        margin: Margin {
            left: 28.,
            right: 28.,
            top: 28.,
            bottom: 28.,
        },
        ..Default::default()
    };
    c.theme.font.family = Some(vec![FontFamily::Named("Lato".into())]);
    let mut pal = if case == 2 || case == 7 {
        [Color(0, 0, 0, 255), Color(255, 255, 255, 255)]
    } else {
        palette(phase)
    };
    if invalid == Some(1) {
        c.theme.background = Color(255, 255, 255, 0);
        for color in &mut pal {
            color.3 = 64;
        }
    }
    let selected = |normal: InheritedColor| match invalid {
        Some(0) => InheritedColor::FromContext(vec![ColorModifier::Opacity(f64::NAN)]),
        Some(1) => contrast(),
        _ => normal,
    };
    if case < 3 {
        let cats = ["late", "first", "zero", "empty"].map(str::to_string);
        let mut rows = if phase == 0 {
            vec![
                BarSeries::new("outer", vec![Some(2.), Some(8.), Some(0.), None]),
                BarSeries::new("inner", vec![Some(4.), Some(6.), Some(0.), None]),
            ]
        } else {
            vec![
                BarSeries::new("outer", vec![Some(4.), Some(6.), Some(0.), None]),
                BarSeries::new("inner", vec![Some(1.), Some(9.), Some(0.), None]),
            ]
        };
        if case == 1 {
            rows[1].values[0] = Some(if phase == 0 { 0.05 } else { 0.1 });
        }
        let mut o = StackedRadialBarOptions {
            inner_radius: 0.4,
            start_angle: 0.,
            end_angle: std::f64::consts::PI * 1.5,
            padding: 0.2,
            tracks: true,
            track_color: Some(Color(232, 236, 240, 255)),
            pad_angle: if case == 1 { 0.3 } else { 0. },
            border: Some(stroke(
                Color(70, 80, 90, if case == 1 { 0 } else { 101 }),
                6.,
            )),
            border_color_profile: Some(selected(if case == 2 {
                contrast()
            } else {
                role(1., 0.5)
            })),
            opacity: 0.8,
            legend: false,
            axes: PolarAxesOptions {
                radial_start: None,
                circular_outer: None,
                grid: PolarGridStyle {
                    radial: false,
                    circular: false,
                    ..Default::default()
                },
                ..Default::default()
            },
            colors: [
                ("late".into(), pal[0]),
                ("first".into(), pal[1]),
                ("zero".into(), Color(90, 100, 110, 64)),
            ]
            .into(),
            ..Default::default()
        };
        if phase == 2 {
            o.hidden.insert("late".into());
        }
        let source = rows
            .iter()
            .map(|r| json!({"id":r.id,"values":r.values}))
            .collect::<Vec<_>>();
        let layout = if case == 1 {
            layout_stacked_radial_bar_with_arc_style(
                &c,
                &cats,
                &rows,
                &o,
                &StackedRadialArcStyle {
                    corner_radius: 8.,
                    ..Default::default()
                },
            )?
        } else {
            layout_stacked_radial_bar(&c, &cats, &rows, &o)?
        };
        let numeric=layout.segments.iter().map(|s|json!({"id":s.mark_id,"ring":s.ring_id,"category":s.category,"value":s.value,"cumulative":s.cumulative,
            "angles":s.angles,"radii":[s.inner_radius,s.outer_radius],"visible":s.visible})).collect::<Vec<_>>();
        let frame_geometry = json!({"center":layout.frame.center,"innerRadius":layout.frame.inner_radius,"outerRadius":layout.frame.outer_radius,
            "domain":layout.domain,"rings":layout.rings.iter().map(|r|json!({"id":r.id,"radii":[r.inner_radius,r.outer_radius],"total":r.total})).collect::<Vec<_>>()});
        let layout = layout.into_scene_interaction()?;
        let facts = json!({"name":CASE_NAMES[case as usize],"case":case,"phase":phase,"source":source,"palette":pal.map(rgba),"numeric":numeric,
            "radial":frame_geometry,"scene":scene_facts(&layout.scene),"binding":layout.interaction.as_ref().map(|b|format!("{b:?}"))});
        Ok(Frame { layout, facts })
    } else {
        let values = if phase == 0 {
            [0., 2., 4., 6., 8., 10.]
        } else {
            [0., 1., 3., 5., 7., 10.]
        };
        let mut data = Vec::new();
        for group in ["B", "A"] {
            for sub in ["late", "first"] {
                for (i, value) in values.iter().enumerate() {
                    data.push(BoxObservation::new(
                        format!("{group}/{sub}/{i}"),
                        group,
                        sub,
                        Some(*value),
                    ));
                }
            }
        }
        data.push(BoxObservation::new("missing", "empty", "first", None));
        let mut o = BoxPlotLayoutOptions {
            groups: Some(vec!["B".into(), "A".into(), "empty".into()]),
            subgroups: Some(vec!["late".into(), "first".into()]),
            orientation: if case == 4 {
                StatisticsOrientation::Horizontal
            } else {
                StatisticsOrientation::Vertical
            },
            domain: Some([0., 10.]),
            border: if case == 6 {
                None
            } else {
                Some(stroke(
                    Color(70, 80, 90, 101),
                    if case == 8 { 0. } else { 6. },
                ))
            },
            median: stroke(Color(200, 20, 180, 190), 4.),
            whisker: stroke(Color(40, 90, 140, 170), 4.),
            border_color_profile: Some(selected(if case == 5 {
                InheritedColor::Theme
            } else if case == 7 {
                contrast()
            } else {
                role(1., 0.5)
            })),
            median_color_profile: Some(selected(if case == 5 {
                InheritedColor::Fixed(Color(190, 30, 110, 160))
            } else if case == 7 {
                contrast()
            } else {
                role(2., 0.75)
            })),
            whisker_color_profile: Some(selected(if case == 5 || case == 7 {
                contrast()
            } else {
                InheritedColor::FromContext(vec![])
            })),
            colors: [("late".into(), pal[0]), ("first".into(), pal[1])].into(),
            fill_opacity: if case == 5 { 0.25 } else { 0.6 },
            border_radius: if case == 5 { 8. } else { 0. },
            outlier_radius: 0.,
            legend: false,
            cap_size: 0.6,
            value_axis: AxisOptions {
                visible: false,
                grid: false,
                ..Default::default()
            },
            group_axis: CategoryAxisOptions {
                visible: false,
                ..Default::default()
            },
            ..Default::default()
        };
        if phase == 2 {
            o.hidden.insert(boxplot_series_id("late"));
        }
        let layout = boxplot_layout(&c, &data, &o)?;
        let numeric=layout.boxes.iter().map(|b|json!({"id":b.id,"group":b.group,"subgroup":b.subgroup,"count":b.count,"extrema":b.extrema,"quantiles":b.quantiles,
            "whiskers":b.whiskers,"sourceIds":b.source_ids,"center":b.center,"breadth":b.breadth,"rect":[b.rect.x,b.rect.y,b.rect.width,b.rect.height]})).collect::<Vec<_>>();
        let mut facts = json!({"name":CASE_NAMES[case as usize],"case":case,"phase":phase,"source":data.iter().map(|d|json!({"id":d.id,"group":d.group,"subgroup":d.subgroup,"value":d.value})).collect::<Vec<_>>(),
            "palette":pal.map(rgba),"numeric":numeric,"domain":layout.domain,"groups":layout.groups,"subgroups":layout.subgroups,"scene":scene_facts(&layout.scene)});
        if case == 8 {
            facts["configuredBorder"] = json!({"width":o.border.unwrap().width,"rgba":rgba(o.border.unwrap().color),"cap":format!("{:?}",o.border.unwrap().cap)});
        }
        Ok(Frame {
            layout: SceneInteraction {
                scene: layout.scene,
                interaction: None,
                plot: None,
            },
            facts,
        })
    }
}
