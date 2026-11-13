//! Key-stable BoxPlot legend captions shared by native exports and real web hosts.
//! Captions are plain text from a bounded raw-key map, never callbacks or IDs.
use glory::render::{GraphicStroke, PathCommand, StrokeCap};
use kartis::*;
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub const WIDTH: f64 = 480.;
pub const HEIGHT: f64 = 300.;
pub const SIZE: [f64; 2] = [WIDTH, HEIGHT];
pub const CASE_COUNT: u32 = 4;
pub const PHASE_COUNT: u32 = 4;
pub const INVALID_COUNT: u32 = 3;
pub const CASE_NAMES: [&str; 4] = [
    "subgroup-normal",
    "subgroup-hidden-reserved",
    "group-normal",
    "group-hidden-reserved",
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
fn stroke(color: Color, width: f64) -> GraphicStroke {
    GraphicStroke {
        color,
        width,
        cap: StrokeCap::Square,
    }
}
fn check(case: u32, phase: u32) -> Result<()> {
    if case >= CASE_COUNT || phase >= PHASE_COUNT {
        Err(ChartError(
            "unknown BoxPlot legend-label case or phase".into(),
        ))
    } else {
        Ok(())
    }
}
fn captions(case: u32, phase: u32, domain: &[String]) -> Option<BTreeMap<String, String>> {
    if phase == 0 {
        return None;
    }
    Some(
        domain
            .iter()
            .map(|key| {
                let text = match phase {
                    1 => format!("{} <&> Å", key),
                    2 if case == 2 => String::new(),
                    2 if matches!(
                        key.as_str(),
                        "empty" | "hidden" | "C" | "reserve/0" | "null"
                    ) =>
                    {
                        String::new()
                    }
                    2 => "Repeat".into(),
                    3 => format!("{} · café", key),
                    _ => unreachable!(),
                };
                (key.clone(), text)
            })
            .collect(),
    )
}
fn inputs(case: u32, phase: u32) -> (ChartConfig, Vec<BoxObservation>, BoxPlotLayoutOptions) {
    let updated = phase == 3;
    let special = case % 2 == 1;
    let group_mode = case >= 2;
    let colors = [
        Color(180, 40, 60, 255),
        Color(20, 40, 160, 255),
        Color(30, 130, 70, 255),
        Color(120, 80, 150, 255),
        Color(90, 100, 110, 255),
    ];
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
    config.theme.colors = colors.to_vec();
    config.theme.font.family = Some(vec![FontFamily::Named("Lato".into())]);
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
    if special {
        for (i, value) in values.into_iter().enumerate() {
            source.push(BoxObservation::new(
                format!("C/x/{i}"),
                "C",
                "x",
                Some(value),
            ));
            source.push(BoxObservation::new(
                format!("A/hidden/{i}"),
                "A",
                "hidden",
                Some(value),
            ));
        }
    }
    source.push(BoxObservation::new("explicit-null", "null", "x", None));
    let mut groups = vec!["reserve/0".into(), "B".into(), "A".into(), "null".into()];
    if special {
        groups.push("C".into());
    }
    let subgroups: Vec<String> = if special {
        ["empty", "y", "x", "hidden"].map(str::to_string).to_vec()
    } else {
        ["y", "x", "empty"].map(str::to_string).to_vec()
    };
    let domain = if group_mode { &groups } else { &subgroups };
    let mut hidden = std::collections::BTreeSet::new();
    if special {
        hidden.insert(boxplot_series_id("hidden"));
    }
    if updated {
        hidden.insert(boxplot_series_id(if special { "x" } else { "y" }));
    }
    let palette = if group_mode {
        [
            ("A".into(), colors[0]),
            ("B".into(), colors[1]),
            ("C".into(), colors[2]),
        ]
        .into_iter()
        .filter(|(key, _)| groups.contains(key))
        .collect()
    } else {
        [
            ("x".into(), colors[0]),
            ("y".into(), colors[1]),
            ("empty".into(), colors[2]),
            ("hidden".into(), colors[3]),
        ]
        .into_iter()
        .filter(|(key, _)| subgroups.contains(key))
        .collect()
    };
    let options = BoxPlotLayoutOptions {
        color_by: if group_mode {
            BoxPlotColorBy::Group
        } else {
            BoxPlotColorBy::SubGroup
        },
        legend_labels: captions(case, phase, domain),
        groups: Some(groups),
        subgroups: Some(subgroups),
        hidden,
        colors: palette,
        domain: Some([-2., 12.]),
        fill_opacity: 0.65,
        border: Some(stroke(Color(40, 50, 60, 255), 2.)),
        median: stroke(Color(20, 20, 20, 255), 3.),
        whisker: stroke(Color(60, 70, 80, 255), 2.),
        outlier_radius: 3.,
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
        legend: true,
        ..Default::default()
    };
    (config, source, options)
}
pub fn build(case: u32, phase: u32) -> Result<Frame> {
    check(case, phase)?;
    let (config, source, options) = inputs(case, 0);
    let controller = BoxPlotInteraction::new(
        &config,
        &source,
        &options,
        &BoxPlotInteractionOptions::default(),
    )?;
    let mut frame = Frame {
        layout: controller.scene_interaction()?,
        facts: Value::Null,
        controller,
        case,
        phase: 0,
        config,
        source,
        options,
        trace: vec![json!({"operation":"new","legendLabels":null,"active":null})],
    };
    refresh(&mut frame)?;
    for next in 1..=phase {
        frame = advance(&frame, next)?;
    }
    Ok(frame)
}
pub fn advance(frame: &Frame, phase: u32) -> Result<Frame> {
    check(frame.case, phase)?;
    let mut next = frame.clone();
    if phase == 0 {
        next.controller.set_active(None)?;
    }
    if phase >= 2 {
        // If a prior updated frame hid x, restore the actual baseline source
        // before selecting this existing typed key again.
        if next.phase == 3 {
            let (c, d, o) = inputs(next.case, 0);
            next.controller.replace_source(&c, &d, &o)?;
            next.trace.push(json!({"operation":"restore_source_for_selection","activeAfter":next.controller.active().map(key)}));
        }
        next.controller
            .set_active(Some(BoxPlotKey::new("A", "x")))?;
    }
    let active_before = next.controller.active().map(key);
    let (config, source, options) = inputs(next.case, phase);
    next.controller.replace_source(&config, &source, &options)?;
    next.config = config;
    next.source = source;
    next.options = options;
    next.phase = phase;
    next.trace.push(json!({"operation":"replace_source","phase":phase,"legendLabels":next.options.legend_labels,"activeBefore":active_before,"activeAfter":next.controller.active().map(key)}));
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
    let binding = match ctl.interaction_binding() {
        Ok(b) => {
            json!({"sites":b.sites.iter().map(|s|json!({"id":s.mark_id,"series":s.series,"label":s.label,"value":s.value,"formatted":s.formatted_value,"rgba":rgba(s.color),"position":s.position})).collect::<Vec<_>>(),"anchors":format!("{:?}",b.anchors)})
        }
        Err(e) => json!({"error":e.0}),
    };
    json!({"case":frame.case,"name":CASE_NAMES[frame.case as usize],"phase":frame.phase,"source":frame.source.iter().map(|d|json!({"id":d.id,"group":d.group,"subgroup":d.subgroup,"value":d.value})).collect::<Vec<_>>(),
        "colorBy":format!("{:?}",frame.options.color_by),"groups":ctl.layout.groups,"subgroups":ctl.layout.subgroups,"colorDomain":if frame.options.color_by==BoxPlotColorBy::Group{&ctl.layout.groups}else{&ctl.layout.subgroups},"configuredLegendLabels":frame.options.legend_labels,"configuredHidden":frame.options.hidden,"palette":frame.config.theme.colors.iter().copied().map(rgba).collect::<Vec<_>>(),"colorOverrides":frame.options.colors.iter().map(|(k,c)|json!({"key":k,"rgba":rgba(*c)})).collect::<Vec<_>>(),"domain":ctl.layout.domain,
        "configured":{"legend":frame.options.legend,"fillOpacity":frame.options.fill_opacity,"border":frame.options.border.map(|s|json!({"rgba":rgba(s.color),"width":s.width,"cap":format!("{:?}",s.cap)})),"medianWidth":frame.options.median.width,"whiskerWidth":frame.options.whisker.width,"interaction":{"opacity":ctl.options().opacity,"activeOpacity":ctl.options().active_opacity,"inactiveOpacity":ctl.options().inactive_opacity}},
        "active":ctl.active().map(key),"activeKeys":ctl.active_keys().iter().map(key).collect::<Vec<_>>(),"boxes":ctl.layout.boxes.iter().map(|b|json!({"id":b.id,"group":b.group,"subgroup":b.subgroup,"rect":[b.rect.x,b.rect.y,b.rect.width,b.rect.height],"count":b.count,"extrema":b.extrema,"quantiles":b.quantiles,"mean":b.mean,"whiskers":b.whiskers,"sourceIds":b.source_ids,"outliers":b.outliers.iter().map(|o|json!({"id":o.source_id,"value":o.value})).collect::<Vec<_>>()})).collect::<Vec<_>>(),"binding":binding,"scene":raw_scene(scene),"trace":frame.trace})
}
/// Real replace_source attempts. Intentional public tampering is restored only
/// after the original accepted-whole-layout guard has rejected the operation.
pub fn try_invalid(frame: &mut Frame, mode: u32) -> Result<Value> {
    if mode >= INVALID_COUNT {
        return Err(ChartError(
            "unknown BoxPlot legend-label invalid mode".into(),
        ));
    }
    let before = frame.controller.layout.clone();
    let active_before = frame.controller.active().cloned();
    let mut options = frame.options.clone();
    let mut labels = options.legend_labels.clone().unwrap_or_default();
    match mode {
        0 => {
            labels.insert("unknown-caption-domain-key".into(), "Unknown".into());
            options.legend_labels = Some(labels);
        }
        1 => {
            let raw_key = if options.color_by == BoxPlotColorBy::Group {
                before.groups[0].clone()
            } else {
                before.subgroups[0].clone()
            };
            labels.insert(raw_key, "é".repeat(513));
            options.legend_labels = Some(labels);
        }
        2 => {
            if let Some(item) = frame.controller.layout.scene.legend.first_mut() {
                item.label.push_str(" deliberate public caption mutation");
            }
        }
        _ => unreachable!(),
    }
    let requested = (mode == 2).then(|| raw_scene(&frame.controller.layout.scene));
    let error = frame
        .controller
        .replace_source(&frame.config, &frame.source, &options)
        .err()
        .map(|e| e.0);
    let after_attempt = raw_scene(&frame.controller.layout.scene);
    if mode == 2 {
        frame.controller.layout = before.clone();
    }
    Ok(
        json!({"mode":mode,"operation":match mode{0=>"replace_source_unknown_caption_key",1=>"replace_source_utf8_caption1026",2=>"replace_source_public_whole_layout_guard",_=>unreachable!()},"attemptedLegendLabels":options.legend_labels,"error":error,"activeBefore":active_before.as_ref().map(key),"activeAfter":frame.controller.active().map(key),"beforeScene":raw_scene(&before.scene),"requestedScene":requested,"afterAttemptScene":after_attempt,"currentScene":raw_scene(&frame.controller.layout.scene),"bindingErrorAfter":frame.controller.interaction_binding().err().map(|e|e.0)}),
    )
}
