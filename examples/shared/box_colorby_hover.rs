//! One controlled BoxPlot application shared by native exports and web hosts.
use glory::render::{GraphicStroke, PathCommand, StrokeCap};
use kartis::*;
use serde_json::{Value, json};

pub const WIDTH: f64 = 480.;
pub const HEIGHT: f64 = 300.;
pub const CASE_COUNT: u32 = 6;
pub const PHASE_COUNT: u32 = 4;
pub const INVALID_COUNT: u32 = 3;
pub const CASE_NAMES: [&str; 6] = [
    "subgroup-vertical",
    "group-vertical",
    "group-horizontal-theme-slots",
    "subgroup-horizontal-rounded",
    "group-zero-active",
    "group-borderless-default-path",
];
#[derive(Clone)]
pub struct Frame {
    pub layout: SceneInteraction,
    pub facts: Value,
    pub controller: BoxPlotInteraction,
    case: u32,
    phase: u32,
    config: ChartConfig,
    source: Vec<BoxObservation>,
    options: BoxPlotLayoutOptions,
    trace: Vec<Value>,
}
pub fn rgba(c: Color) -> [u8; 4] {
    [c.0, c.1, c.2, c.3]
}
pub fn palette(updated: bool) -> [Color; 2] {
    if updated {
        [Color(60, 20, 100, 192), Color(100, 200, 40, 240)]
    } else {
        [Color(20, 40, 60, 160), Color(200, 180, 160, 220)]
    }
}
fn stroke(color: Color, width: f64) -> GraphicStroke {
    GraphicStroke {
        color,
        width,
        cap: StrokeCap::Square,
    }
}
fn inputs(
    case: u32,
    updated: bool,
) -> (
    ChartConfig,
    Vec<BoxObservation>,
    BoxPlotLayoutOptions,
    BoxPlotInteractionOptions,
) {
    let pal = palette(updated);
    let mut config = ChartConfig {
        width: WIDTH,
        height: HEIGHT,
        margin: Margin {
            left: 40.,
            right: 40.,
            top: 30.,
            bottom: 30.,
        },
        ..Default::default()
    };
    config.theme.background = Color(255, 255, 255, 255);
    config.theme.font.family = Some(vec![FontFamily::Named("Lato".into())]);
    config.theme.colors = vec![
        Color(90, 100, 110, 255),
        pal[0],
        pal[1],
        Color(120, 130, 140, 255),
    ];
    let values = if updated {
        [0., 1., 3., 5., 7., 10.]
    } else {
        [0., 2., 4., 6., 8., 10.]
    };
    let mut source = Vec::new();
    for group in ["A", "B"] {
        for subgroup in ["x", "y"] {
            for (i, value) in values.into_iter().enumerate() {
                source.push(BoxObservation::new(
                    format!("{group}/{subgroup}/{i}"),
                    group,
                    subgroup,
                    Some(value),
                ));
            }
        }
    }
    source.push(BoxObservation::new("explicit-null", "null", "x", None));
    let group_mode = matches!(case, 1 | 2 | 4 | 5);
    let colors = if case == 2 {
        Default::default()
    } else if group_mode {
        [("A".into(), pal[0]), ("B".into(), pal[1])].into()
    } else {
        [("x".into(), pal[0]), ("y".into(), pal[1])].into()
    };
    let options = BoxPlotLayoutOptions {
        color_by: if group_mode {
            BoxPlotColorBy::Group
        } else {
            BoxPlotColorBy::SubGroup
        },
        orientation: if matches!(case, 2 | 3) {
            StatisticsOrientation::Horizontal
        } else {
            StatisticsOrientation::Vertical
        },
        groups: Some(["reserve/0", "B", "A", "null"].map(str::to_string).to_vec()),
        subgroups: Some(["y", "x", "empty"].map(str::to_string).to_vec()),
        hidden: if updated {
            [boxplot_series_id("x")].into()
        } else {
            Default::default()
        },
        colors,
        domain: Some([-2., 12.]),
        fill_opacity: 0.5,
        border: if case == 5 {
            None
        } else {
            Some(stroke(Color(70, 80, 90, 101), 6.))
        },
        border_radius: if case == 3 { 8. } else { 0. },
        border_color_profile: Some(InheritedColor::FromContext(vec![
            ColorModifier::Darker(1.),
            ColorModifier::Opacity(0.5),
        ])),
        median: stroke(Color(40, 90, 140, 151), 8.),
        median_color_profile: Some(if case == 5 {
            InheritedColor::Theme
        } else {
            InheritedColor::Fixed(Color(220, 40, 60, 192))
        }),
        whisker: stroke(Color(30, 60, 90, 201), 4.),
        whisker_color_profile: Some(if case == 5 {
            InheritedColor::Fixed(Color(90, 30, 150, 201))
        } else {
            InheritedColor::FromContext(vec![])
        }),
        outlier_radius: 5.,
        outlier_shape: if case == 3 {
            BoxOutlierShape::Diamond
        } else {
            BoxOutlierShape::Circle
        },
        outlier_fill: if case == 5 {
            Some(Color(10, 130, 90, 200))
        } else {
            None
        },
        outlier_border: if case == 5 {
            Some(stroke(Color(30, 60, 90, 255), 2.))
        } else {
            None
        },
        value_axis: AxisOptions {
            visible: false,
            grid: false,
            ..Default::default()
        },
        group_axis: CategoryAxisOptions {
            visible: false,
            grid: false,
            ..Default::default()
        },
        ..Default::default()
    };
    let active = BoxPlotInteractionOptions {
        opacity: if case == 5 { 1. } else { 0.6 },
        active_opacity: if case == 4 { 0. } else { 0.6 },
        inactive_opacity: 0.25,
    };
    (config, source, options, active)
}
fn checked(case: u32, phase: u32) -> Result<()> {
    if case >= CASE_COUNT || phase >= PHASE_COUNT {
        Err(ChartError(
            "unknown BoxPlot colorBy/hover case or phase".into(),
        ))
    } else {
        Ok(())
    }
}
pub fn build(case: u32, phase: u32) -> Result<Frame> {
    checked(case, phase)?;
    let (config, source, options, interaction) = inputs(case, false);
    let controller = BoxPlotInteraction::new(&config, &source, &options, &interaction)?;
    let mut frame = Frame {
        layout: controller.scene_interaction()?,
        facts: Value::Null,
        controller,
        case,
        phase: 0,
        config,
        source,
        options,
        trace: vec![json!({"operation":"new","active":null})],
    };
    refresh(&mut frame)?;
    for next in 1..=phase {
        frame = advance(&frame, next)?;
    }
    Ok(frame)
}
fn replace(frame: &mut Frame, updated: bool) -> Result<()> {
    let (config, source, options, _) = inputs(frame.case, updated);
    frame
        .controller
        .replace_source(&config, &source, &options)?;
    frame.config = config;
    frame.source = source;
    frame.options = options;
    frame.trace.push(json!({"operation":"replace_source","updated":updated,"activeAfter":frame.controller.active().map(key)}));
    Ok(())
}
pub fn advance(frame: &Frame, phase: u32) -> Result<Frame> {
    checked(frame.case, phase)?;
    let mut next = frame.clone();
    if next.phase == 3 {
        replace(&mut next, false)?;
    }
    match phase {
        0 => {
            next.controller.set_active(None)?;
            next.trace
                .push(json!({"operation":"set_active","key":null}));
        }
        1 => {
            next.controller
                .set_active(Some(BoxPlotKey::new("A", "x")))?;
            next.trace
                .push(json!({"operation":"set_active","key":{"group":"A","subgroup":"x"}}));
        }
        2 => {
            next.controller
                .set_active(Some(BoxPlotKey::new("A", "x")))?;
            next.controller.set_active(None)?;
            next.trace
                .push(json!({"operation":"active_then_none","key":null}));
        }
        3 => {
            next.controller
                .set_active(Some(BoxPlotKey::new("A", "x")))?;
            next.trace
                .push(json!({"operation":"set_active","key":{"group":"A","subgroup":"x"}}));
            replace(&mut next, true)?;
        }
        _ => unreachable!(),
    }
    next.phase = phase;
    refresh(&mut next)?;
    Ok(next)
}
fn refresh(frame: &mut Frame) -> Result<()> {
    frame.layout = frame.controller.scene_interaction()?;
    frame.facts = observe(frame);
    Ok(())
}
fn key(k: &BoxPlotKey) -> Value {
    json!({"group":k.group,"subgroup":k.subgroup})
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
        Geometry::Path(cs) => {
            json!({"kind":"path","commands":cs.iter().map(|c|match c{PathCommand::Move(p)=>json!(["M",p.x,p.y]),PathCommand::Line(p)=>json!(["L",p.x,p.y]),PathCommand::Cubic{first,second,end}=>json!(["C",first.x,first.y,second.x,second.y,end.x,end.y]),PathCommand::Close=>json!(["Z"])}).collect::<Vec<_>>()})
        }
    }
}
fn raw_scene(s: &Scene) -> Value {
    json!({"width":s.width,"height":s.height,"background":rgba(s.background),"marks":s.marks.iter().map(|m|json!({"id":m.id,"series":m.series,"value":m.value,"label":m.label,"geometry":geometry(&m.geometry),"fill":m.fill.map(rgba),"stroke":m.stroke.map(|s|json!({"rgba":rgba(s.color),"width":s.width,"cap":format!("{:?}",s.cap)})),"opacity":m.opacity,"effectiveOpacity":s.effective_mark_opacity(m),"clip":m.clip.map(|r|[r.x,r.y,r.width,r.height]),"fillPaint":format!("{:?}",m.fill_paint)})).collect::<Vec<_>>(),
        "opacityGroups":s.opacity_groups.iter().map(|g|json!({"id":g.id,"members":g.members,"opacity":g.opacity})).collect::<Vec<_>>(),
        "drawOrder":s.draw_items().iter().map(|d|format!("{d:?}")).collect::<Vec<_>>(),
        "labels":s.labels.iter().map(|l|json!({"text":l.text,"position":[l.x,l.y],"rgba":rgba(l.color)})).collect::<Vec<_>>(),
        "legend":s.legend.iter().map(|l|json!({"id":l.id,"label":l.label,"rgba":rgba(l.color),"role":format!("{:?}",s.legend_role(&l.id))})).collect::<Vec<_>>()})
}
pub fn observe(frame: &Frame) -> Value {
    let ctl = &frame.controller;
    let scene = &ctl.layout.scene;
    let boxes=ctl.layout.boxes.iter().map(|b|{
        let k=BoxPlotKey::new(&b.group,&b.subgroup);
        let members=scene.marks.iter().filter(|m|ctl.key_for_mark(&m.id).ok().flatten()==Some(&k)).map(|m|json!({"id":m.id,"fill":m.fill.map(rgba),"stroke":m.stroke.map(|s|rgba(s.color)),"opacity":m.opacity,"effectiveOpacity":scene.effective_mark_opacity(m),"outlier":ctl.outlier_for_mark(&m.id).ok().flatten().map(|o|json!({"id":o.source_id,"value":o.value}))})).collect::<Vec<_>>();
        let median=scene.marks.iter().find(|m|m.id==format!("{}:median",b.id)).map(|m|match m.geometry.commands().as_slice(){[PathCommand::Move(a),PathCommand::Line(z)]=>json!([(a.x+z.x)/2.,(a.y+z.y)/2.]),_=>Value::Null}).unwrap_or(Value::Null);
        let numeric = scene.marks.iter().find(|m|m.id==b.id).and_then(|m|m.fill).map(rgba);
        json!({"group":b.group,"subgroup":b.subgroup,"bodyId":b.id,"numericRGBA":numeric,"rect":[b.rect.x,b.rect.y,b.rect.width,b.rect.height],"bodyCenter":[b.rect.x+b.rect.width/2.,b.rect.y+b.rect.height/2.],"medianPosition":median,"count":b.count,"extrema":b.extrema,"quantiles":b.quantiles,"mean":b.mean,"whiskers":b.whiskers,"sourceIds":b.source_ids,"outliers":b.outliers.iter().map(|o|json!({"id":o.source_id,"value":o.value})).collect::<Vec<_>>(),"members":members})
    }).collect::<Vec<_>>();
    let binding = ctl.interaction_binding();
    let binding_json = match &binding {
        Ok(b) => {
            json!({"sites":b.sites.iter().map(|s|json!({"id":s.mark_id,"series":s.series,"label":s.label,"value":s.value,"formatted":s.formatted_value,"rgba":rgba(s.color),"position":s.position})).collect::<Vec<_>>(),"anchors":format!("{:?}",b.anchors)})
        }
        Err(e) => json!({"error":e.0}),
    };
    json!({"case":frame.case,"name":CASE_NAMES[frame.case as usize],"phase":frame.phase,"source":frame.source.iter().map(|d|json!({"id":d.id,"group":d.group,"subgroup":d.subgroup,"value":d.value})).collect::<Vec<_>>(),
        "colorBy":format!("{:?}",frame.options.color_by),"colorDomain":if frame.options.color_by==BoxPlotColorBy::Group{&ctl.layout.groups}else{&ctl.layout.subgroups},"groups":ctl.layout.groups,"subgroups":ctl.layout.subgroups,"palette":frame.config.theme.colors.iter().copied().map(rgba).collect::<Vec<_>>(),"overrides":frame.options.colors.iter().map(|(k,v)|json!({"key":k,"rgba":rgba(*v)})).collect::<Vec<_>>(),"hidden":frame.options.hidden,"domain":ctl.layout.domain,
        "configured":{"orientation":format!("{:?}",frame.options.orientation),"fillOpacity":frame.options.fill_opacity,"border":frame.options.border.map(|s|json!({"rgba":rgba(s.color),"width":s.width,"cap":format!("{:?}",s.cap)})),"medianWidth":frame.options.median.width,"whiskerWidth":frame.options.whisker.width,"borderRadius":frame.options.border_radius,"outlierRadius":frame.options.outlier_radius,"interaction":{"opacity":ctl.options().opacity,"activeOpacity":ctl.options().active_opacity,"inactiveOpacity":ctl.options().inactive_opacity}},
        "active":ctl.active().map(key),"activeKeys":ctl.active_keys().iter().map(key).collect::<Vec<_>>(),"boxes":boxes,"binding":binding_json,"scene":raw_scene(scene),"trace":frame.trace})
}
/// Actual operations on the currently retained controller. Intentional public
/// tampering is restored to the exact original frame only after guard rejection.
pub fn try_invalid(frame: &mut Frame, mode: u32) -> Result<Value> {
    if mode >= INVALID_COUNT {
        return Err(ChartError("unknown BoxPlot invalid mode".into()));
    }
    let before = frame.controller.layout.clone();
    let before_active = frame.controller.active().cloned();
    let requested = if mode == 2 {
        if let Some(mark) = frame.controller.layout.scene.marks.first_mut() {
            mark.label.push_str(" deliberate public mutation");
        }
        Some(raw_scene(&frame.controller.layout.scene))
    } else {
        None
    };
    let result = match mode {
        0 => frame
            .controller
            .set_active(Some(BoxPlotKey::new("unknown-source-group", "x"))),
        1 => frame.controller.set_options(&BoxPlotInteractionOptions {
            opacity: f64::NAN,
            ..*frame.controller.options()
        }),
        2 => frame.controller.set_active(None),
        _ => unreachable!(),
    };
    let error = result.err().map(|e| e.0);
    let after_attempt = raw_scene(&frame.controller.layout.scene);
    if mode == 2 {
        frame.controller.layout = before.clone();
    }
    Ok(
        json!({"mode":mode,"operation":match mode{0=>"set_active_unknown_key",1=>"set_options_nan_opacity",2=>"public_layout_guard",_=>unreachable!()},"error":error,"activeBefore":before_active.as_ref().map(key),"activeAfter":frame.controller.active().map(key),"beforeScene":raw_scene(&before.scene),"requestedScene":requested,"afterAttemptScene":after_attempt,"currentScene":raw_scene(&frame.controller.layout.scene),"bindingErrorAfter":frame.controller.interaction_binding().err().map(|e|e.0)}),
    )
}
