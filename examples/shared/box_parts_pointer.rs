//! Actual controlled BoxPlot sources, shared by native and four retained hosts.
use glory::render::{GraphicStroke, PathCommand, StrokeCap};
use kartis::surface::SurfacePhysicalHit;
use kartis::*;
use serde_json::{Value, json};

pub const WIDTH: f64 = 480.;
pub const HEIGHT: f64 = 300.;
pub const CASE_COUNT: u32 = 4;
pub const PHASE_COUNT: u32 = 4;
pub const CASE_NAMES: [&str; 4] = [
    "vertical-all-parts",
    "horizontal-rounded-clipped-outliers",
    "body-zero-visible-parts",
    "body-only-alpha-empty-map",
];
#[derive(Clone)]
pub struct Frame {
    pub layout: SceneInteraction,
    pub facts: Value,
    pub controller: BoxPlotInteraction,
    pub case: u32,
    pub phase: u32,
    config: ChartConfig,
    source: Vec<BoxObservation>,
    options: BoxPlotLayoutOptions,
    trace: Vec<Value>,
}
pub fn rgba(c: Color) -> [u8; 4] {
    [c.0, c.1, c.2, c.3]
}
fn key(k: &BoxPlotKey) -> Value {
    json!({"group":k.group,"subgroup":k.subgroup})
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
    phase: u32,
) -> Result<(
    ChartConfig,
    Vec<BoxObservation>,
    BoxPlotLayoutOptions,
    BoxPlotInteractionOptions,
)> {
    if case >= CASE_COUNT || phase >= PHASE_COUNT {
        return Err(ChartError("unknown whole-parts fixture case/phase".into()));
    }
    let alpha = if case == 3 && phase < 2 { 0 } else { 255 };
    let palette = if phase >= 2 {
        vec![Color(60, 120, 80, alpha), Color(170, 80, 120, alpha)]
    } else {
        vec![Color(40, 100, 160, alpha), Color(180, 120, 60, alpha)]
    };
    let config = ChartConfig {
        width: WIDTH,
        height: HEIGHT,
        margin: Margin {
            left: 40.,
            right: 40.,
            top: 30.,
            bottom: 30.,
        },
        theme: Theme {
            background: Color(255, 255, 255, 255),
            colors: palette,
            font: FontOptions {
                family: Some(vec![FontFamily::Named("Lato".into())]),
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    };
    let mut source = Vec::new();
    for group in ["A", "B"] {
        for subgroup in ["x", "y"] {
            if phase == 3 && group == "A" && subgroup == "y" {
                continue;
            }
            for (i, mut value) in [0., 2., 4., 6., 8., 10.].into_iter().enumerate() {
                if phase >= 2 && i == 0 {
                    value = -1.;
                }
                source.push(BoxObservation::new(
                    format!("{group}/{subgroup}/{i}"),
                    group,
                    subgroup,
                    Some(value),
                ));
            }
        }
    }
    source.push(BoxObservation::new("A/x/null", "A", "x", None));
    let group_mode = matches!(case, 1 | 3);
    let labels = if phase == 0 {
        None
    } else {
        Some(if group_mode {
            [("A".into(), "North".into()), ("B".into(), "South".into())].into()
        } else {
            [("x".into(), "Observed".into()), ("y".into(), "Plan".into())].into()
        })
    };
    let options = BoxPlotLayoutOptions {
        orientation: if case == 1 {
            StatisticsOrientation::Horizontal
        } else {
            StatisticsOrientation::Vertical
        },
        groups: Some(vec!["A".into(), "B".into()]),
        subgroups: Some(vec!["x".into(), "y".into()]),
        color_by: if group_mode {
            BoxPlotColorBy::Group
        } else {
            BoxPlotColorBy::SubGroup
        },
        legend_labels: labels,
        legend: true,
        hidden: if phase == 3 {
            [boxplot_series_id("x")].into()
        } else {
            Default::default()
        },
        domain: Some(if case == 1 { [0.5, 9.5] } else { [-2., 12.] }),
        border_radius: if case == 1 { 8. } else { 0. },
        fill_opacity: if case == 2 { 0. } else { 0.6 },
        border: if case >= 2 {
            None
        } else {
            Some(stroke(Color(10, 30, 50, 255), 6.))
        },
        median: stroke(Color(220, 40, 60, 255), if case == 3 { 0. } else { 8. }),
        whisker: stroke(Color(20, 40, 60, 255), if case == 3 { 0. } else { 6. }),
        cap_size: 0.5,
        outlier_radius: if case == 3 { 0. } else { 5. },
        outlier_border: None,
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
    let zero = case == 2 && phase == 1;
    let interaction = BoxPlotInteractionOptions {
        opacity: if zero { 0. } else { 0.6 },
        active_opacity: if zero { 0. } else { 0.9 },
        inactive_opacity: if zero { 0. } else { 0.25 },
    };
    Ok((config, source, options, interaction))
}
fn refresh(frame: &mut Frame) -> Result<()> {
    frame.layout = frame.controller.whole_parts_scene_interaction()?;
    frame.facts = observe(frame);
    Ok(())
}
/// Fresh native/browser reference frames never seed an active key.
pub fn build(case: u32, phase: u32) -> Result<Frame> {
    let (config, source, options, interaction) = inputs(case, phase)?;
    let controller = BoxPlotInteraction::new(&config, &source, &options, &interaction)?;
    let layout = controller.whole_parts_scene_interaction()?;
    let mut frame = Frame {
        layout,
        facts: Value::Null,
        controller,
        case,
        phase,
        config,
        source,
        options,
        trace: vec![json!({"operation":"new","active":null})],
    };
    refresh(&mut frame)?;
    Ok(frame)
}
/// Retain only activity actually established by a preceding accepted pointer.
/// The host republishes this source, then its real current-point hit decides
/// whether the previous activity still has any ink at that point.
pub fn advance(frame: &Frame, phase: u32) -> Result<Frame> {
    let (config, source, options, interaction) = inputs(frame.case, phase)?;
    let mut next = frame.clone();
    next.controller.replace_source(&config, &source, &options)?;
    next.controller.set_options(&interaction)?;
    next.phase = phase;
    next.config = config;
    next.source = source;
    next.options = options;
    next.trace.push(json!({"operation":"replace-source","phase":phase,"activeRetained":next.controller.active().map(key)}));
    refresh(&mut next)?;
    Ok(next)
}
/// Call only with a host-validated current-frame PhysicalHover event. Every
/// part observation remains in the trace; only owner changes repaint.
pub fn accept_physical_hover(frame: &mut Frame, hit: Option<SurfacePhysicalHit>) -> Result<bool> {
    let binding = frame
        .layout
        .interaction
        .as_ref()
        .ok_or_else(|| ChartError("missing strict fixture binding".into()))?;
    let physical = hit.as_ref().map(|h| h.physical_mark_id.clone());
    let next_key = if let Some(hit) = &hit {
        let logical = if binding
            .sites
            .iter()
            .any(|s| s.mark_id == hit.physical_mark_id)
        {
            Some(hit.physical_mark_id.as_str())
        } else {
            binding
                .ink_owners
                .get(&hit.physical_mark_id)
                .map(String::as_str)
        };
        if logical != Some(hit.logical_site_id.as_str())
            || binding.part_data.get(&hit.physical_mark_id) != hit.source_data.as_ref()
        {
            return Err(ChartError(
                "observed physical snapshot does not belong to current fixture source".into(),
            ));
        }
        Some(
            frame
                .controller
                .key_for_mark(&hit.physical_mark_id)?
                .cloned()
                .ok_or_else(|| ChartError("unowned physical fixture observation".into()))?,
        )
    } else {
        None
    };
    let changed = frame.controller.active() != next_key.as_ref();
    frame.trace.push(json!({"operation":"physical-hover","hit":hit.as_ref().map(hit_json),"ownerChanged":changed}));
    if changed {
        frame.controller.set_hovered_mark(physical.as_deref())?;
    }
    refresh(frame)?;
    Ok(changed)
}
pub fn hit_json(hit: &SurfacePhysicalHit) -> Value {
    json!({"physicalId":hit.physical_mark_id,"logicalId":hit.logical_site_id,"scenePoint":hit.scene_point,
        "sourceData":hit.source_data.as_ref().map(|d|json!({"sourceId":d.source_id,"value":d.value})),"frameTokenDebug":format!("{:?}",hit.frame_token)})
}
pub fn geometry(g: &Geometry) -> Value {
    match g {
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => json!({"kind":"rect","x":x,"y":y,"width":width,"height":height}),
        Geometry::Circle { x, y, radius } => json!({"kind":"circle","x":x,"y":y,"radius":radius}),
        Geometry::Path(commands) => {
            json!({"kind":"path","commands":commands.iter().map(|c|match c {PathCommand::Move(p)=>json!(["M",p.x,p.y]),PathCommand::Line(p)=>json!(["L",p.x,p.y]),PathCommand::Cubic{first,second,end}=>json!(["C",first.x,first.y,second.x,second.y,end.x,end.y]),PathCommand::Close=>json!(["Z"])}).collect::<Vec<_>>()})
        }
    }
}
fn center(mark: &Mark) -> Option<[f64; 2]> {
    match mark.geometry {
        Geometry::Circle { x, y, .. } => Some([x, y]),
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => Some([x + width / 2., y + height / 2.]),
        _ => None,
    }
}
fn segment(mark: &Mark, t: f64) -> Option<[f64; 2]> {
    let Geometry::Path(commands) = &mark.geometry else {
        return None;
    };
    match commands.get(..2)? {
        [PathCommand::Move(a), PathCommand::Line(b)] => {
            Some([a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t])
        }
        _ => None,
    }
}
fn probes(frame: &Frame) -> Value {
    let scene = &frame.layout.scene;
    let Some(summary) = frame.controller.layout.boxes.first() else {
        return json!({"summary":null,"points":[],"absentParts":[],"paintSamples":[]});
    };
    let mut points = vec![
        json!({"name":"blank","point":[10.,10.],"physicalId":null,"logicalId":null,"basis":"actual viewport point outside plot; no invented source"}),
    ];
    let mut absent = Vec::new();
    let mut samples = Vec::new();
    for part in ["body", "border", "median", "whiskers", "caps", "outlier"] {
        let mark = if part == "body" {
            scene.marks.iter().find(|m| m.id == summary.id)
        } else if part == "outlier" {
            scene.marks.iter().find(|m| {
                frame
                    .controller
                    .outlier_for_mark(&m.id)
                    .ok()
                    .flatten()
                    .is_some()
                    && frame.controller.key_for_mark(&m.id).ok().flatten()
                        == Some(&BoxPlotKey::new(&summary.group, &summary.subgroup))
            })
        } else {
            scene
                .marks
                .iter()
                .find(|m| m.id == format!("{}:{part}", summary.id))
        };
        let Some(mark) = mark else {
            absent.push(part);
            continue;
        };
        let r = summary.rect;
        let point = match part {
            "body" => Some([r.x + r.width / 4., r.y + r.height / 4.]),
            "border" => Some(
                if frame.options.orientation == StatisticsOrientation::Vertical {
                    [r.x - mark.stroke.unwrap().width / 4., r.y + r.height / 4.]
                } else {
                    [r.x + r.width / 4., r.y - mark.stroke.unwrap().width / 4.]
                },
            ),
            "whiskers" => segment(mark, 0.5),
            "median" | "caps" => segment(mark, 0.25),
            "outlier" => center(mark),
            _ => None,
        };
        let Some(point) = point else {
            absent.push(part);
            continue;
        };
        let alpha = if part == "body" || part == "outlier" {
            mark.fill.map_or(0, |c| c.3)
        } else {
            mark.stroke.map_or(0, |s| s.color.3)
        };
        let inside_clip = mark
            .clip
            .is_none_or(|c| c.width > 0. && c.height > 0. && c.contains(point[0], point[1]));
        let raw = json!({"name":part,"point":point,"physicalId":mark.id,"logicalId":summary.id,"geometry":geometry(&mark.geometry),
            "basis":if part=="body" {"quarter of actual summary rect, away from median/corners"} else if part=="border" {"quarter actual stroke width outside straight box side"} else if part=="outlier" {"actual emitted circle center"} else {"actual authored straight segment interior"},
            "rawPaintAlpha":alpha,"markOpacity":mark.opacity,"groupOpacity":scene.opacity_groups.iter().find(|g|g.members.contains(&mark.id)).map_or(1.,|g|g.opacity),"insideActualClip":inside_clip,"sourceData":frame.layout.interaction.as_ref().and_then(|b|b.part_data.get(&mark.id)).map(|d|json!({"sourceId":d.source_id,"value":d.value}))});
        if alpha > 0 && scene.effective_mark_opacity(mark) > 0. && inside_clip {
            samples.push(raw.clone());
        }
        points.push(raw);
        if part == "whiskers" {
            let half = mark.stroke.unwrap().width / 2.;
            let outside = if frame.options.orientation == StatisticsOrientation::Vertical {
                [point[0] + half + 0.75, point[1]]
            } else {
                [point[0], point[1] + half + 0.75]
            };
            points.push(json!({"name":"outside-halfwidth","point":outside,"physicalId":null,"logicalId":null,"basis":"actual stem midpoint displaced perpendicular by half width + .75; outside body"}));
        }
    }
    json!({"summary":{"group":summary.group,"subgroup":summary.subgroup,"bodyId":summary.id},"points":points,"absentParts":absent,"paintSamples":samples})
}
pub fn observe(frame: &Frame) -> Value {
    let scene = &frame.layout.scene;
    let binding = frame.layout.interaction.as_ref();
    json!({"case":frame.case,"name":CASE_NAMES[frame.case as usize],"phase":frame.phase,"width":WIDTH,"height":HEIGHT,
        "source":frame.source.iter().map(|d|json!({"id":d.id,"group":d.group,"subgroup":d.subgroup,"value":d.value})).collect::<Vec<_>>(),
        "active":frame.controller.active().map(key),"activeKeys":frame.controller.active_keys().iter().map(key).collect::<Vec<_>>(),"trace":frame.trace,
        "options":{"orientation":format!("{:?}",frame.options.orientation),"domain":frame.options.domain,"hidden":frame.options.hidden,"fillOpacity":frame.options.fill_opacity,"palette":frame.config.theme.colors.iter().copied().map(rgba).collect::<Vec<_>>(),"colorBy":format!("{:?}",frame.options.color_by),"captions":frame.options.legend_labels,"groupOpacity":frame.controller.options().opacity,"activeOpacity":frame.controller.options().active_opacity,"inactiveOpacity":frame.controller.options().inactive_opacity},
        "boxes":frame.controller.layout.boxes.iter().map(|b|json!({"bodyId":b.id,"group":b.group,"subgroup":b.subgroup,"rect":[b.rect.x,b.rect.y,b.rect.width,b.rect.height],"quantiles":b.quantiles,"whiskers":b.whiskers,"extrema":b.extrema,"count":b.count,"sourceIds":b.source_ids,"outliers":b.outliers.iter().map(|o|json!({"sourceId":o.source_id,"value":o.value})).collect::<Vec<_>>()})).collect::<Vec<_>>(),
        "binding":binding.map(|b|json!({"profile":format!("{:?}",b.picking_profile),"sites":b.sites.iter().map(|s|json!({"id":s.mark_id,"series":s.series,"value":s.value,"label":s.label,"rgba":rgba(s.color),"position":s.position})).collect::<Vec<_>>(),"inkOwners":b.ink_owners,"partData":b.part_data.iter().map(|(id,d)|json!({"physicalId":id,"sourceId":d.source_id,"value":d.value})).collect::<Vec<_>>()})),
        "scene":{"width":scene.width,"height":scene.height,"background":rgba(scene.background),"marks":scene.marks.iter().map(|m|json!({"id":m.id,"series":m.series,"label":m.label,"value":m.value,"geometry":geometry(&m.geometry),"fill":m.fill.map(rgba),"stroke":m.stroke.map(|s|json!({"rgba":rgba(s.color),"width":s.width,"cap":format!("{:?}",s.cap)})),"opacity":m.opacity,"effectiveOpacity":scene.effective_mark_opacity(m),"clip":m.clip.map(|c|[c.x,c.y,c.width,c.height])})).collect::<Vec<_>>(),"opacityGroups":scene.opacity_groups.iter().map(|g|json!({"id":g.id,"members":g.members,"opacity":g.opacity})).collect::<Vec<_>>(),"drawOrder":scene.draw_items().iter().map(|d|format!("{d:?}")).collect::<Vec<_>>(),"legend":scene.legend.iter().map(|l|json!({"id":l.id,"label":l.label,"rgba":rgba(l.color),"role":format!("{:?}",scene.legend_role(&l.id))})).collect::<Vec<_>>()},
        "probes":probes(frame)})
}
