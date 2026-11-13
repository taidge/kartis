//! Actual borrowed source-color/caption factories; no closure is retained.
use std::cell::RefCell;

use glory::render::{GraphicStroke, PathCommand, StrokeCap};
use kartis::surface::SurfacePhysicalHit;
use kartis::*;
use serde_json::{Value, json};

pub const WIDTH: f64 = 480.;
pub const HEIGHT: f64 = 300.;
pub const CASE_COUNT: u32 = 8;
pub const PHASE_COUNT: u32 = 4;
pub const INVALID_COUNT: u32 = 5;
pub const CASE_NAMES: [&str; 8] = [
    "subgroup-cell-color-pairs",
    "group-cell-color-category",
    "cross-key-first-color-target",
    "keys-reserved-fixed-strokes",
    "typed-alpha-zero-fixed-ink",
    "horizontal-reversed-fixed-outlier",
    "legend-false-required-both",
    "empty-built-required-both",
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
    scale: Option<ContinuousScaleOptions>,
    factory_trace: Vec<Value>,
    trace: Vec<Value>,
}
pub fn rgba(color: Color) -> [u8; 4] {
    [color.0, color.1, color.2, color.3]
}
fn key(key: &BoxPlotKey) -> Value {
    json!({"group":key.group,"subgroup":key.subgroup})
}
pub fn target_json(target: &LegendTarget) -> Value {
    match target {
        LegendTarget::Series(id) => json!({"kind":"Series","id":id}),
        LegendTarget::Category(id) => json!({"kind":"Category","id":id}),
    }
}
fn stroke(color: Color, width: f64) -> GraphicStroke {
    GraphicStroke {
        color,
        width,
        cap: StrokeCap::Square,
    }
}
fn source_json(source: &[BoxObservation]) -> Value {
    json!(source.iter().map(|row|json!({"id":row.id,"group":row.group,"subgroup":row.subgroup,"value":row.value})).collect::<Vec<_>>())
}
fn inputs(
    case: u32,
    phase: u32,
) -> Result<(
    ChartConfig,
    Vec<BoxObservation>,
    BoxPlotLayoutOptions,
    Option<ContinuousScaleOptions>,
)> {
    if case >= CASE_COUNT || phase >= PHASE_COUNT {
        return Err(ChartError("unknown colors-function case or phase".into()));
    }
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
            colors: vec![
                Color(180, 40, 60, 255),
                Color(20, 40, 160, 255),
                Color(30, 130, 70, 255),
                Color(120, 80, 150, 255),
            ],
            font: FontOptions {
                family: Some(vec![FontFamily::Named("Lato".into())]),
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    };
    let mut source = Vec::new();
    if !(case == 7 && phase == 0) {
        for group in ["A", "B"] {
            for subgroup in ["x", "y"] {
                if phase == 2 && group == "B" && subgroup == "y" {
                    continue;
                }
                for (index, value) in [0., 2., 4., 6., 8., 10.].into_iter().enumerate() {
                    source.push(BoxObservation::new(
                        format!("{group}/{subgroup}/{index}"),
                        group,
                        subgroup,
                        Some(value),
                    ));
                }
            }
        }
        source.push(BoxObservation::new("A/x/null", "A", "x", None));
        source.push(BoxObservation::new("null/x/only", "null", "x", None));
    }
    let options = BoxPlotLayoutOptions {
        groups: Some(["reserved", "B", "A", "null"].map(str::to_string).into()),
        subgroups: Some(["missing", "y", "x"].map(str::to_string).into()),
        orientation: if case == 5 {
            StatisticsOrientation::Horizontal
        } else {
            StatisticsOrientation::Vertical
        },
        color_by: if case == 1 {
            BoxPlotColorBy::Group
        } else {
            BoxPlotColorBy::SubGroup
        },
        legend_mode: if case == 3 {
            BoxPlotLegendMode::Keys
        } else {
            BoxPlotLegendMode::ByLabel
        },
        legend_labels: if case == 3 {
            Some(
                ["missing", "y", "x"]
                    .map(|raw| {
                        (
                            raw.to_string(),
                            if phase == 0 {
                                format!("Key {raw}")
                            } else if phase == 1 {
                                format!("Key {raw} <&> Å café")
                            } else {
                                "duplicate key caption".into()
                            },
                        )
                    })
                    .into(),
            )
        } else {
            None
        },
        legend: case != 6,
        hidden: if phase == 3 {
            [boxplot_series_id("y")].into()
        } else {
            Default::default()
        },
        domain: Some([-2., 12.]),
        padding: 0.1,
        inner_padding: 4.,
        fill_opacity: 0.65,
        border: Some(stroke(Color(40, 50, 60, 220), 2.)),
        median: stroke(Color(20, 30, 40, 230), 3.),
        whisker: stroke(Color(60, 70, 80, 240), 2.),
        border_color_profile: if matches!(case, 3 | 4) {
            None
        } else {
            Some(InheritedColor::FromContext(vec![ColorModifier::Darker(
                0.5,
            )]))
        },
        median_color_profile: if matches!(case, 3 | 4) {
            None
        } else {
            Some(InheritedColor::FromContext(vec![ColorModifier::Opacity(
                0.75,
            )]))
        },
        whisker_color_profile: if matches!(case, 3 | 4) {
            None
        } else {
            Some(InheritedColor::FromContext(vec![]))
        },
        outlier_fill: if case == 5 {
            Some(Color(210, 80, 30, 255))
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
    let scale = (case == 5).then_some(ContinuousScaleOptions {
        reverse: true,
        ..Default::default()
    });
    Ok((config, source, options, scale))
}
fn source_color(case: u32, phase: u32, ctx: &BoxPlotColorContext<'_>) -> Color {
    let g = ctx.group_index as u8;
    let s = ctx.subgroup_index as u8;
    let mean = ctx.computed_box.mean.round() as u8;
    let d = [0, 7, 13, 19][phase as usize];
    Color(
        30 + 40 * g + 10 * s + mean + d,
        60 + 10 * g + 20 * s + d,
        100 + 20 * g + 5 * s + d,
        if case == 4 { 0 } else { 128 },
    )
}
fn label(case: u32, phase: u32, ctx: &BoxPlotLegendContext<'_>) -> String {
    if case == 2 {
        return if phase == 1 {
            "shared <&> Å café".into()
        } else {
            "shared caption".into()
        };
    }
    if phase >= 2 {
        return "same caption".into();
    }
    if phase == 1 || case == 5 {
        format!(
            "{}/{} <&> Å café",
            ctx.computed_box.group, ctx.computed_box.subgroup
        )
    } else {
        format!("{}/{}", ctx.computed_box.group, ctx.computed_box.subgroup)
    }
}
fn color_context(ctx: &BoxPlotColorContext<'_>, color: Color, operation: &str) -> Value {
    let b = ctx.computed_box;
    json!({"brand":"kartis","factory":"color","operation":operation,"summaryIndex":ctx.summary_index,"groupIndex":ctx.group_index,"subgroupIndex":ctx.subgroup_index,"cellIndex":ctx.cell_index,"colorIndex":ctx.color_index,"rawColorKey":ctx.raw_color_key,"colorBy":format!("{:?}",ctx.color_by),"defaultRGBA":rgba(ctx.default_color),"returnedRGBA":rgba(color),"quantileProbabilities":ctx.quantile_probabilities,"observationIndices":ctx.observation_indices,"data":source_json(ctx.data),"computedBox":{"id":b.id,"group":b.group,"subgroup":b.subgroup,"count":b.count,"quantiles":b.quantiles,"extrema":b.extrema,"mean":b.mean,"sourceIds":b.source_ids,"rect":[b.rect.x,b.rect.y,b.rect.width,b.rect.height]}})
}
fn legend_context(ctx: &BoxPlotLegendContext<'_>, text: &str, operation: &str) -> Value {
    json!({"brand":"kartis","factory":"legend","operation":operation,"summaryIndex":ctx.summary_index,"groupIndex":ctx.group_index,"subgroupIndex":ctx.subgroup_index,"cellIndex":ctx.cell_index,"colorIndex":ctx.color_index,"rawColorKey":ctx.raw_color_key,"defaultText":ctx.default_text,"resolvedSourceRGBA":rgba(ctx.color),"returnedLabel":text,"computedBox":{"id":ctx.computed_box.id,"group":ctx.computed_box.group,"subgroup":ctx.computed_box.subgroup,"quantiles":ctx.computed_box.quantiles},"observationIndices":ctx.observation_indices})
}
fn refresh(frame: &mut Frame) -> Result<()> {
    frame.layout = frame.controller.whole_parts_scene_interaction()?;
    frame.facts = observe(frame);
    Ok(())
}
pub fn build(case: u32, phase: u32) -> Result<Frame> {
    let (config, source, options, scale) = inputs(case, 0)?;
    let calls = RefCell::new(Vec::new());
    let controller = {
        let mut color = |ctx: &BoxPlotColorContext<'_>| {
            let color = source_color(case, 0, ctx);
            calls.borrow_mut().push(color_context(ctx, color, "new"));
            Ok(color)
        };
        let mut legend = |ctx: &BoxPlotLegendContext<'_>| {
            let text = label(case, 0, ctx);
            calls.borrow_mut().push(legend_context(ctx, &text, "new"));
            Ok(text)
        };
        let factories = BoxPlotFactories {
            color: Some(&mut color),
            legend_label: if case == 3 { None } else { Some(&mut legend) },
        };
        if let Some(scale) = &scale {
            BoxPlotInteraction::with_scale_options_and_factories(
                &config,
                &source,
                &options,
                scale,
                &Default::default(),
                factories,
            )?
        } else {
            BoxPlotInteraction::with_factories(
                &config,
                &source,
                &options,
                &Default::default(),
                factories,
            )?
        }
    };
    let mut frame = Frame {
        layout: controller.whole_parts_scene_interaction()?,
        facts: Value::Null,
        controller,
        case,
        phase: 0,
        config,
        source,
        options,
        scale,
        factory_trace: calls.into_inner(),
        trace: vec![
            json!({"operation":"new","phase":0,"colorFactoryBuilt":true,"legendFactoryBuilt":case!=3}),
        ],
    };
    refresh(&mut frame)?;
    for next in 1..=phase {
        frame = advance(&frame, next)?;
    }
    Ok(frame)
}
pub fn advance(frame: &Frame, phase: u32) -> Result<Frame> {
    let (config, source, options, scale) = inputs(frame.case, phase)?;
    let mut next = frame.clone();
    let case = next.case;
    let calls = RefCell::new(Vec::new());
    {
        let mut color = |ctx: &BoxPlotColorContext<'_>| {
            let color = source_color(case, phase, ctx);
            calls
                .borrow_mut()
                .push(color_context(ctx, color, "replace-source"));
            Ok(color)
        };
        let mut legend = |ctx: &BoxPlotLegendContext<'_>| {
            let text = label(case, phase, ctx);
            calls
                .borrow_mut()
                .push(legend_context(ctx, &text, "replace-source"));
            Ok(text)
        };
        let factories = BoxPlotFactories {
            color: Some(&mut color),
            legend_label: if case == 3 { None } else { Some(&mut legend) },
        };
        if let Some(scale) = &scale {
            next.controller
                .replace_source_with_scale_options_and_factories(
                    &config, &source, &options, scale, factories,
                )?;
        } else {
            next.controller
                .replace_source_with_factories(&config, &source, &options, factories)?;
        }
    }
    let added = calls.into_inner();
    let color_added = added.iter().filter(|r| r["factory"] == "color").count();
    let legend_added = added.len() - color_added;
    next.factory_trace.extend(added);
    next.phase = phase;
    next.config = config;
    next.source = source;
    next.options = options;
    next.scale = scale;
    next.trace.push(json!({"operation":"replace-source-with-explicit-factories","phase":phase,"colorCallsAdded":color_added,"legendCallsAdded":legend_added,"activeRetained":next.controller.active().map(key)}));
    refresh(&mut next)?;
    Ok(next)
}
/// Actual attempted controller operations, without flush/publication.
pub fn try_invalid(frame: &mut Frame, mode: u32) -> Result<Value> {
    if mode >= INVALID_COUNT {
        return Err(ChartError(
            "unknown colors-function invalid operation".into(),
        ));
    }
    let case = frame.case;
    let phase = frame.phase;
    let before = frame.controller.layout.clone();
    let active = frame.controller.active().cloned();
    let hover = *frame.controller.options();
    let failed = RefCell::new(Vec::new());
    let (_, mut source, mut options, _) = inputs(
        frame.case,
        if frame.source.is_empty() {
            1
        } else {
            frame.phase
        },
    )?;
    if mode == 3 {
        source.push(BoxObservation::new(
            "invalid-source-group",
            "",
            "x",
            Some(3.),
        ));
    }
    if mode == 2 && frame.case == 3 {
        options.legend_mode = BoxPlotLegendMode::Keys;
    }
    let result = if mode == 4 {
        frame.controller.set_options(&BoxPlotInteractionOptions {
            opacity: f64::NAN,
            ..hover
        })
    } else {
        let mut color = |ctx: &BoxPlotColorContext<'_>| {
            let value = source_color(case, phase, ctx);
            failed.borrow_mut().push(color_context(
                ctx,
                value,
                if mode == 0 {
                    "returned-color-Err"
                } else {
                    "unexpected-before-validation"
                },
            ));
            if mode == 0 {
                Err(ChartError("deliberate source-color factory error".into()))
            } else {
                Ok(value)
            }
        };
        let mut legend = |ctx: &BoxPlotLegendContext<'_>| {
            let text = label(case, phase, ctx);
            failed
                .borrow_mut()
                .push(legend_context(ctx, &text, "unexpected-before-validation"));
            Ok(text)
        };
        let factories = BoxPlotFactories {
            color: if mode == 1 { None } else { Some(&mut color) },
            legend_label: if (mode == 2) != (frame.case == 3) {
                None
            } else {
                Some(&mut legend)
            },
        };
        frame
            .controller
            .replace_source_with_factories(&frame.config, &source, &options, factories)
    };
    Ok(
        json!({"mode":mode,"operation":match mode{0=>"color-factory-returned-Err",1=>"missing-required-color",2=>if frame.case==3{"Keys-plus-legend-factory"}else{"missing-required-legend"},3=>"bad-source-empty-group",4=>"controller-factor-NaN",_=>unreachable!()},"error":result.err().map(|e|e.0),"layoutEqualsBefore":frame.controller.layout==before,"activeEqualsBefore":frame.controller.active()==active.as_ref(),"optionsEqualBefore":frame.controller.options()==&hover,"externalFailedFactoryCalls":failed.into_inner(),"acceptedFactoryCalls":frame.factory_trace.len(),"bindingErrorAfter":frame.controller.whole_parts_interaction_binding().err().map(|e|e.0)}),
    )
}

pub fn raw_scene(scene: &Scene) -> Value {
    json!({"width":scene.width,"height":scene.height,"background":rgba(scene.background),"marks":scene.marks.iter().map(|m|json!({"id":m.id,"series":m.series,"label":m.label,"value":m.value,"geometry":geometry(&m.geometry),"fill":m.fill.map(rgba),"stroke":m.stroke.map(|s|json!({"rgba":rgba(s.color),"width":s.width,"cap":format!("{:?}",s.cap)})),"opacity":m.opacity,"effectiveOpacity":scene.effective_mark_opacity(m),"clip":m.clip.map(|c|[c.x,c.y,c.width,c.height]),"fillPaint":format!("{:?}",m.fill_paint)})).collect::<Vec<_>>(),"opacityGroups":scene.opacity_groups.iter().map(|g|json!({"id":g.id,"members":g.members,"opacity":g.opacity})).collect::<Vec<_>>(),"drawOrder":scene.draw_items().iter().map(|d|format!("{d:?}")).collect::<Vec<_>>(),"labels":scene.labels.iter().map(|l|json!({"text":l.text,"position":[l.x,l.y],"rgba":rgba(l.color)})).collect::<Vec<_>>(),"legend":scene.legend.iter().map(|l|json!({"id":l.id,"label":l.label,"rgba":rgba(l.color),"role":scene.legend_role(&l.id).map(|r|format!("{r:?}")),"target":scene.legend_target(&l.id).as_ref().map(target_json),"explicitTarget":scene.legend_targets.get(&l.id).map(target_json)})).collect::<Vec<_>>(),"legendTargets":scene.legend_targets.iter().map(|(id,t)|json!({"rowId":id,"target":target_json(t)})).collect::<Vec<_>>()})
}

pub fn observe(frame: &Frame) -> Value {
    let ctl = &frame.controller;
    let binding = frame.layout.interaction.as_ref();
    let colors = frame
        .factory_trace
        .iter()
        .filter(|row| row["factory"] == "color")
        .count();
    let labels = frame
        .factory_trace
        .iter()
        .filter(|row| row["factory"] == "legend")
        .count();
    let o = &frame.options;
    let stroke_json = |s: GraphicStroke| json!({"rgba":rgba(s.color),"width":s.width,"cap":format!("{:?}",s.cap)});
    json!({
        "brand":"kartis","case":frame.case,"name":CASE_NAMES[frame.case as usize],"phase":frame.phase,
        "width":WIDTH,"height":HEIGHT,"source":source_json(&frame.source),
        "groups":ctl.layout.groups,"subgroups":ctl.layout.subgroups,"domain":ctl.layout.domain,
        "colorBy":format!("{:?}",o.color_by),"configuredLegendLabels":o.legend_labels,
        "configuredHidden":o.hidden,"palette":frame.config.theme.colors.iter().copied().map(rgba).collect::<Vec<_>>(),
        "orientation":format!("{:?}",o.orientation),
        "configured":{"legend":o.legend,"mode":format!("{:?}",o.legend_mode),
            "factoryPolicy":{"color":true,"legend":frame.case!=3,"origin":"declared construction configuration; actual replacement rejection returned separately"},
            "domain":o.domain,"padding":o.padding,"innerPadding":o.inner_padding,
            "quantileProbabilities":o.quantiles,"fillOpacity":o.fill_opacity,
            "border":o.border.map(stroke_json),"median":stroke_json(o.median),"whisker":stroke_json(o.whisker),
            "borderProfile":format!("{:?}",o.border_color_profile),"medianProfile":format!("{:?}",o.median_color_profile),"whiskerProfile":format!("{:?}",o.whisker_color_profile),
            "outlierFill":o.outlier_fill.map(rgba),"outlierRadius":o.outlier_radius,
            "valueScale":frame.scale.map(|s|json!({"kind":format!("{:?}",s.kind),"reverse":s.reverse,"round":s.round,"clamp":s.clamp})),
            "controllerOpacity":ctl.options().opacity,"activeOpacity":ctl.options().active_opacity,"inactiveOpacity":ctl.options().inactive_opacity},
        "active":ctl.active().map(key),"activeKeys":ctl.active_keys().iter().map(key).collect::<Vec<_>>(),
        "factoryCallCount":frame.factory_trace.len(),"colorCallCount":colors,"legendCallCount":labels,
        "factoryTrace":frame.factory_trace,"trace":frame.trace,
        "boxes":ctl.layout.boxes.iter().map(|b|json!({"id":b.id,"group":b.group,"subgroup":b.subgroup,"count":b.count,"rect":[b.rect.x,b.rect.y,b.rect.width,b.rect.height],"quantiles":b.quantiles,"extrema":b.extrema,"whiskers":b.whiskers,"mean":b.mean,"sourceIds":b.source_ids,"outliers":b.outliers.iter().map(|o|json!({"id":o.source_id,"value":o.value})).collect::<Vec<_>>()})).collect::<Vec<_>>(),
        "binding":binding.map(|b|json!({"profile":format!("{:?}",b.picking_profile),"sites":b.sites.iter().map(|s|json!({"id":s.mark_id,"series":s.series,"value":s.value,"label":s.label,"rgba":rgba(s.color),"position":s.position})).collect::<Vec<_>>(),"inkOwners":b.ink_owners,"partData":b.part_data.iter().map(|(id,d)|json!({"physicalId":id,"sourceId":d.source_id,"value":d.value})).collect::<Vec<_>>()})),
        "controllerLayoutDebug":format!("{:?}",ctl.layout),"controllerSceneDebug":format!("{:?}",ctl.layout.scene),
        "renderedLayoutDebug":format!("{:?}",frame.layout),"bindingDebug":format!("{:?}",frame.layout.interaction),
        "controllerScene":raw_scene(&ctl.layout.scene),"scene":raw_scene(&frame.layout.scene),
        "probes":probes(frame)
    })
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
