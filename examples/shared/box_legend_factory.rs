//! Borrowed per-summary legend factories, shared by native and retained hosts.
//! No factory is retained. Generic target overrides only touch a rendered clone.
use glory::render::{GraphicStroke, PathCommand, StrokeCap};
use kartis::{surface::SurfacePhysicalHit, *};
use serde_json::{Value, json};
use std::collections::BTreeMap;

pub const WIDTH: f64 = 480.;
pub const HEIGHT: f64 = 300.;
pub const CASE_COUNT: u32 = 7;
pub const PHASE_COUNT: u32 = 4;
pub const INVALID_COUNT: u32 = 3;
pub const CASE_NAMES: [&str; 7] = [
    "subgroup-pair-shared-target",
    "group-pair-category",
    "bylabel-raw-key-and-map-no-factory",
    "raw-empty-subgroup-empty-text",
    "legend-false-factory-policy",
    "horizontal-escaped-duplicate",
    "generic-rendered-target-only-swap",
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
) -> Result<(ChartConfig, Vec<BoxObservation>, BoxPlotLayoutOptions)> {
    if case >= CASE_COUNT || phase >= PHASE_COUNT {
        return Err(ChartError("unknown legend-factory case or phase".into()));
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
    let secondary = if case == 3 { "" } else { "y" };
    let mut source = Vec::new();
    for group in ["A", "B"] {
        for subgroup in ["x", secondary] {
            if phase == 3
                && ((case == 1 && group == "B")
                    || (!matches!(case, 2 | 4) && group == "B" && subgroup == secondary))
            {
                continue;
            }
            for (i, value) in [0., 2., 4., 6., 8., 10.].into_iter().enumerate() {
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
    source.push(BoxObservation::new("null/x/only", "null", "x", None));
    let subgroups: Vec<String> = ["missing", secondary, "x"].map(str::to_string).into();
    let captions = if case == 2 && phase > 0 {
        Some(
            subgroups
                .iter()
                .map(|raw| {
                    (
                        raw.clone(),
                        if phase == 1 {
                            format!("{raw} <&> Å café")
                        } else {
                            "same caption".into()
                        },
                    )
                })
                .collect::<BTreeMap<_, _>>(),
        )
    } else {
        None
    };
    let options = BoxPlotLayoutOptions {
        groups: Some(["reserved", "B", "A", "null"].map(str::to_string).into()),
        subgroups: Some(subgroups),
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
        legend_mode: BoxPlotLegendMode::ByLabel,
        legend_labels: captions,
        legend: case != 4,
        hidden: if phase == 3 && matches!(case, 2 | 4) {
            [boxplot_series_id("y")].into()
        } else {
            Default::default()
        },
        domain: Some([-2., 12.]),
        padding: 0.1,
        inner_padding: 4.,
        fill_opacity: 0.65,
        border: Some(stroke(Color(40, 50, 60, 255), 2.)),
        median: stroke(Color(20, 20, 20, 255), 3.),
        whisker: stroke(Color(60, 70, 80, 255), 2.),
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
    Ok((config, source, options))
}
fn label(case: u32, phase: u32, context: &BoxPlotLegendContext<'_>) -> String {
    if case == 6 {
        return "Target swap".into();
    }
    if case == 3 && phase != 1 {
        return String::new();
    }
    if phase >= 2 {
        return "same caption".into();
    }
    let summary = &context.computed_box;
    if phase == 1 {
        format!("{}/{} <&> Å café", summary.group, summary.subgroup)
    } else {
        format!("{}/{}", summary.group, summary.subgroup)
    }
}
fn context_json(
    context: &BoxPlotLegendContext<'_>,
    text: &str,
    operation: &str,
    call_index: usize,
) -> Value {
    let b = context.computed_box;
    json!({"operation":operation,"callIndex":call_index,"summaryIndex":context.summary_index,"groupIndex":context.group_index,"subgroupIndex":context.subgroup_index,"cellIndex":context.cell_index,"colorIndex":context.color_index,"colorBy":format!("{:?}",context.color_by),"rawColorKey":context.raw_color_key,"defaultText":context.default_text,"numericRGBA":rgba(context.color),"quantileProbabilities":context.quantile_probabilities,"observationIndices":context.observation_indices,"data":source_json(context.data),"computedBox":{"id":b.id,"group":b.group,"subgroup":b.subgroup,"count":b.count,"quantiles":b.quantiles,"extrema":b.extrema,"mean":b.mean,"sourceIds":b.source_ids,"rect":[b.rect.x,b.rect.y,b.rect.width,b.rect.height]},"returnedLabel":text})
}
fn refresh(frame: &mut Frame) -> Result<()> {
    frame.layout = frame.controller.whole_parts_scene_interaction()?;
    if frame.case == 6 {
        if frame.phase == 1 {
            let row = frame
                .layout
                .scene
                .legend
                .first()
                .ok_or_else(|| {
                    ChartError("generic target swap requires an actual legend row".into())
                })?
                .id
                .clone();
            frame
                .layout
                .scene
                .legend_targets
                .insert(row, LegendTarget::Series(boxplot_series_id("x")));
        }
        let row_id = "generic-empty-category".to_string();
        frame.layout.scene.legend.push(LegendItem {
            id: row_id.clone(),
            label: "Empty category <&> Å".into(),
            color: Color(120, 80, 150, 255),
        });
        frame
            .layout
            .scene
            .legend_targets
            .insert(row_id, LegendTarget::Category(String::new()));
        frame.layout.validate()?;
    }
    frame.facts = observe(frame);
    Ok(())
}
pub fn build(case: u32, phase: u32) -> Result<Frame> {
    let (config, source, options) = inputs(case, 0)?;
    let mut factory_trace = Vec::new();
    let controller = if case == 2 {
        BoxPlotInteraction::new(
            &config,
            &source,
            &options,
            &BoxPlotInteractionOptions::default(),
        )?
    } else {
        let mut factory = |context: &BoxPlotLegendContext<'_>| {
            let text = label(case, 0, context);
            factory_trace.push(context_json(context, &text, "new", factory_trace.len() + 1));
            Ok(text)
        };
        BoxPlotInteraction::with_legend_label_factory(
            &config,
            &source,
            &options,
            &BoxPlotInteractionOptions::default(),
            &mut factory,
        )?
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
        factory_trace,
        trace: vec![json!({"operation":"new","case":case,"phase":0,"factoryBuilt":case!=2})],
    };
    refresh(&mut frame)?;
    for next in 1..=phase {
        frame = advance(&frame, next)?;
    }
    Ok(frame)
}
/// The new factory is borrowed only for this genuine replacement call.
pub fn advance(frame: &Frame, phase: u32) -> Result<Frame> {
    let (config, source, options) = inputs(frame.case, phase)?;
    let mut next = frame.clone();
    if frame.case == 6 && phase < 3 && frame.phase < 3 {
        next.phase = phase;
        next.trace.push(json!({"operation":"generic-rendered-target-only-update","phase":phase,"controllerSourceRebuilt":false,"factoryCallsAdded":0}));
        refresh(&mut next)?;
        return Ok(next);
    }
    let count_before = next.factory_trace.len();
    if next.case == 2 {
        next.controller.replace_source(&config, &source, &options)?;
    } else {
        let case = next.case;
        let calls = &mut next.factory_trace;
        let mut factory = |context: &BoxPlotLegendContext<'_>| {
            let text = label(case, phase, context);
            calls.push(context_json(
                context,
                &text,
                "replace-source",
                calls.len() + 1,
            ));
            Ok(text)
        };
        next.controller.replace_source_with_legend_label_factory(
            &config,
            &source,
            &options,
            &mut factory,
        )?;
    }
    next.phase = phase;
    next.config = config;
    next.source = source;
    next.options = options;
    next.trace.push(json!({"operation":"replace-source-with-explicit-factory-policy","phase":phase,"factoryCallsAdded":next.factory_trace.len()-count_before,"activeRetained":next.controller.active().map(key)}));
    refresh(&mut next)?;
    Ok(next)
}

/// Actual invalid operations do not flush, publish, or mutate accepted traces.
/// Failed callback side effects are returned separately from accepted state.
pub fn try_invalid(frame: &mut Frame, mode: u32) -> Result<Value> {
    if mode >= INVALID_COUNT {
        return Err(ChartError(
            "unknown legend-factory invalid operation".into(),
        ));
    }
    let before = frame.controller.layout.clone();
    let active_before = frame.controller.active().cloned();
    let options_before = *frame.controller.options();
    let mut failed_calls = Vec::new();
    let plain_policy_error = if mode == 2 && frame.case == 4 {
        frame
            .controller
            .replace_source(&frame.config, &frame.source, &frame.options)
            .err()
            .map(|e| e.0)
    } else {
        None
    };
    let outcome = match mode {
        0 => frame.controller.set_options(&BoxPlotInteractionOptions {
            opacity: f64::NAN,
            ..options_before
        }),
        1 => {
            let mut source = frame.source.clone();
            source[0].group = String::new();
            if frame.case == 2 {
                frame
                    .controller
                    .replace_source(&frame.config, &source, &frame.options)
            } else {
                let mut factory = |context: &BoxPlotLegendContext<'_>| {
                    failed_calls.push(context_json(
                        context,
                        "unreachable-invalid-source",
                        "failed-invalid-source",
                        failed_calls.len() + 1,
                    ));
                    Ok("unreachable-invalid-source".into())
                };
                frame.controller.replace_source_with_legend_label_factory(
                    &frame.config,
                    &source,
                    &frame.options,
                    &mut factory,
                )
            }
        }
        2 => {
            let mut options = frame.options.clone();
            if frame.case == 4 {
                options.legend_mode = BoxPlotLegendMode::Keys;
            }
            let mut factory = |context: &BoxPlotLegendContext<'_>| {
                failed_calls.push(context_json(
                    context,
                    "returned Err",
                    "failed-callback",
                    failed_calls.len() + 1,
                ));
                Err(ChartError(
                    "deliberate legend factory callback error".into(),
                ))
            };
            frame.controller.replace_source_with_legend_label_factory(
                &frame.config,
                &frame.source,
                &options,
                &mut factory,
            )
        }
        _ => unreachable!(),
    };
    Ok(
        json!({"mode":mode,"operation":match mode {0=>"controller-factor-NaN",1=>"bad-source-empty-group",2=>if frame.case==4 {"Keys-plus-factory-before-callback"} else {"borrowed-factory-returned-Err"},_=>unreachable!()},"error":outcome.err().map(|e|e.0),"plainReplaceWithoutFactoryError":plain_policy_error,"layoutEqualsBefore":frame.controller.layout==before,"activeEqualsBefore":frame.controller.active()==active_before.as_ref(),"optionsEqualBefore":frame.controller.options()==&options_before,"externalFailedFactoryCalls":failed_calls,"acceptedFactoryCalls":frame.factory_trace.len(),"bindingErrorAfter":frame.controller.whole_parts_interaction_binding().err().map(|e|e.0)}),
    )
}

pub fn raw_scene(scene: &Scene) -> Value {
    json!({"width":scene.width,"height":scene.height,"background":rgba(scene.background),"marks":scene.marks.iter().map(|m|json!({"id":m.id,"series":m.series,"label":m.label,"value":m.value,"geometry":geometry(&m.geometry),"fill":m.fill.map(rgba),"stroke":m.stroke.map(|s|json!({"rgba":rgba(s.color),"width":s.width,"cap":format!("{:?}",s.cap)})),"opacity":m.opacity,"effectiveOpacity":scene.effective_mark_opacity(m),"clip":m.clip.map(|c|[c.x,c.y,c.width,c.height]),"fillPaint":format!("{:?}",m.fill_paint)})).collect::<Vec<_>>(),"opacityGroups":scene.opacity_groups.iter().map(|g|json!({"id":g.id,"members":g.members,"opacity":g.opacity})).collect::<Vec<_>>(),"drawOrder":scene.draw_items().iter().map(|d|format!("{d:?}")).collect::<Vec<_>>(),"labels":scene.labels.iter().map(|l|json!({"text":l.text,"position":[l.x,l.y],"rgba":rgba(l.color)})).collect::<Vec<_>>(),"legend":scene.legend.iter().map(|l|json!({"id":l.id,"label":l.label,"rgba":rgba(l.color),"role":scene.legend_role(&l.id).map(|r|format!("{r:?}")),"target":scene.legend_target(&l.id).as_ref().map(target_json),"explicitTarget":scene.legend_targets.get(&l.id).map(target_json)})).collect::<Vec<_>>(),"legendTargets":scene.legend_targets.iter().map(|(id,t)|json!({"rowId":id,"target":target_json(t)})).collect::<Vec<_>>()})
}
pub fn observe(frame: &Frame) -> Value {
    let ctl = &frame.controller;
    let binding = frame.layout.interaction.as_ref();
    json!({"case":frame.case,"name":CASE_NAMES[frame.case as usize],"phase":frame.phase,"width":WIDTH,"height":HEIGHT,"source":source_json(&frame.source),"groups":ctl.layout.groups,"subgroups":ctl.layout.subgroups,"domain":ctl.layout.domain,"colorBy":format!("{:?}",frame.options.color_by),"configuredLegendLabels":frame.options.legend_labels,"configuredHidden":frame.options.hidden,"palette":frame.config.theme.colors.iter().copied().map(rgba).collect::<Vec<_>>(),"orientation":format!("{:?}",frame.options.orientation),"configured":{"legend":frame.options.legend,"mode":format!("{:?}",frame.options.legend_mode),"factoryBuilt":frame.case!=2,"domain":frame.options.domain,"padding":frame.options.padding,"innerPadding":frame.options.inner_padding,"quantileProbabilities":frame.options.quantiles,"fillOpacity":frame.options.fill_opacity,"borderWidth":frame.options.border.map(|s|s.width),"medianWidth":frame.options.median.width,"whiskerWidth":frame.options.whisker.width,"controllerOpacity":ctl.options().opacity,"activeOpacity":ctl.options().active_opacity,"inactiveOpacity":ctl.options().inactive_opacity},"active":ctl.active().map(key),"activeKeys":ctl.active_keys().iter().map(key).collect::<Vec<_>>(),"factoryCallCount":frame.factory_trace.len(),"factoryTrace":frame.factory_trace,"trace":frame.trace,
      "boxes":ctl.layout.boxes.iter().map(|b|json!({"id":b.id,"group":b.group,"subgroup":b.subgroup,"count":b.count,"rect":[b.rect.x,b.rect.y,b.rect.width,b.rect.height],"quantiles":b.quantiles,"extrema":b.extrema,"whiskers":b.whiskers,"mean":b.mean,"sourceIds":b.source_ids,"outliers":b.outliers.iter().map(|o|json!({"id":o.source_id,"value":o.value})).collect::<Vec<_>>()})).collect::<Vec<_>>(),
      "binding":binding.map(|b|json!({"profile":format!("{:?}",b.picking_profile),"sites":b.sites.iter().map(|s|json!({"id":s.mark_id,"series":s.series,"value":s.value,"label":s.label,"rgba":rgba(s.color),"position":s.position})).collect::<Vec<_>>(),"inkOwners":b.ink_owners,"partData":b.part_data.iter().map(|(id,d)|json!({"physicalId":id,"sourceId":d.source_id,"value":d.value})).collect::<Vec<_>>()})),
      "controllerLayoutDebug":format!("{:?}",ctl.layout),"controllerSceneDebug":format!("{:?}",ctl.layout.scene),"renderedLayoutDebug":format!("{:?}",frame.layout),"bindingDebug":format!("{:?}",frame.layout.interaction),"controllerScene":raw_scene(&ctl.layout.scene),"scene":raw_scene(&frame.layout.scene),"renderedLayoutOverride":{"enabled":frame.case==6,"targetOnlySwap":frame.case==6&&frame.phase==1,"scope":"Generic caller adds one Category legend row and optionally changes a rendered target; genuine controller base/guard remains intact","controllerBindingValid":ctl.whole_parts_interaction_binding().is_ok()},"probes":probes(frame)})
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
