//! Shared native/WASM application model for ordinary inherited border roles.
use std::cell::RefCell;

use glory::render::{GraphicStroke, PathCommand, StrokeCap};
use kartis::*;
use serde_json::{Value, json};

pub const CASE_COUNT: u32 = 10;
pub const CASE_NAMES: [&str; 10] = [
    "classic-bar",
    "category-bar",
    "custom-inherited-bar",
    "typed-explicit-bar",
    "contrast-bar",
    "rect-heatmap",
    "circle-heatmap",
    "typed-custom-heatmap",
    "zero-size-heatmap",
    "borderless-heatmap",
];

#[derive(Clone)]
pub struct Frame {
    pub layout: SceneInteraction,
    pub facts: Value,
}

pub fn rgba(color: Color) -> [u8; 4] {
    [color.0, color.1, color.2, color.3]
}
pub fn stroke_facts(stroke: Option<GraphicStroke>) -> Value {
    stroke.map_or(
        Value::Null,
        |s| json!({"rgba":rgba(s.color),"width":s.width,"cap":format!("{:?}",s.cap)}),
    )
}
pub fn geometry_facts(geometry: &Geometry) -> Value {
    match geometry {
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => {
            json!({"kind":"rect","x":x,"y":y,"width":width,"height":height})
        }
        Geometry::Circle { x, y, radius } => {
            json!({"kind":"circle","x":x,"y":y,"radius":radius})
        }
        Geometry::Path(commands) => json!({"kind":"path","commands":commands.iter().map(|c| {
            match c {
                PathCommand::Move(p) => json!(["M",p.x,p.y]),
                PathCommand::Line(p) => json!(["L",p.x,p.y]),
                PathCommand::Cubic{first,second,end} => json!(["C",first.x,first.y,second.x,second.y,end.x,end.y]),
                PathCommand::Close => json!(["Z"]),
            }
        }).collect::<Vec<_>>()}),
    }
}
pub fn scene_facts(scene: &Scene) -> Value {
    json!({
        "width":scene.width,"height":scene.height,"background":rgba(scene.background),
        "marks":scene.marks.iter().map(|m|json!({
            "id":m.id,"series":m.series,"value":m.value,"label":m.label,
            "geometry":geometry_facts(&m.geometry),"fill":m.fill.map(rgba),
            "fillPaint":format!("{:?}",m.fill_paint),"stroke":stroke_facts(m.stroke),
            "opacity":m.opacity,"clip":m.clip.map(|r|[r.x,r.y,r.width,r.height]),
            "strokeDash":format!("{:?}",m.stroke_dash),
        })).collect::<Vec<_>>(),
        "labels":scene.labels.iter().enumerate().map(|(i,l)|json!({
            "index":i,"owner":scene.label_series.get(&i),"text":l.text,
            "position":[l.x,l.y],"rgba":rgba(l.color),"size":l.size,
            "anchor":format!("{:?}",l.anchor),"rotation":l.rotation,
        })).collect::<Vec<_>>(),
        "legend":scene.legend.iter().map(|l|json!({"id":l.id,"label":l.label,"rgba":rgba(l.color)})).collect::<Vec<_>>(),
        "drawOrder":format!("{:?}",scene.draw_order),"typography":format!("{:?}",scene.typography),
    })
}
pub fn binding_facts(binding: Option<&InteractionBinding>) -> Value {
    binding.map_or(Value::Null, |b| {
        json!({
            "sites":b.sites.iter().map(|s|json!({"id":s.mark_id,"series":s.series,
                "label":s.label,"value":s.value,"formattedValue":s.formatted_value,
                "representativeRGBA":rgba(s.color),"position":s.position})).collect::<Vec<_>>(),
            "anchors":format!("{:?}",b.anchors),
        })
    })
}

fn palette(phase: u32) -> [Color; 3] {
    if phase == 0 {
        [
            Color(20, 40, 60, 160),
            Color(200, 180, 160, 220),
            Color(90, 100, 110, 64),
        ]
    } else {
        [
            Color(60, 20, 100, 192),
            Color(100, 200, 40, 240),
            Color(180, 60, 20, 128),
        ]
    }
}
fn profile() -> InheritedColor {
    InheritedColor::FromContext(vec![ColorModifier::Darker(1.), ColorModifier::Opacity(0.5)])
}
fn contrast() -> InheritedColor {
    InheritedColor::Contrast {
        dark: Color(0, 0, 0, 255),
        light: Color(255, 255, 255, 255),
    }
}
fn fixed(cap: StrokeCap) -> GraphicStroke {
    GraphicStroke {
        color: Color(70, 80, 90, 101),
        width: 6.,
        cap,
    }
}
fn config(phase: u32) -> ChartConfig {
    let mut c = ChartConfig {
        width: 420.,
        height: 240.,
        margin: Margin {
            left: 28.,
            right: 28.,
            top: 28.,
            bottom: 28.,
        },
        title: "Border color roles".into(),
        ..Default::default()
    };
    c.theme.colors = palette(phase).to_vec();
    // The application supplies the same explicit face to native raster and web.
    c.theme.font.family = Some(vec![FontFamily::Named("Lato".into())]);
    c
}
pub fn build(case: u32, phase: u32) -> Result<Frame> {
    build_impl(case, phase, None)
}
/// Exercise the same real public layout builders, before a host can commit.
/// Mode 0 is intrinsically invalid; mode 1 has actual translucent source
/// surfaces over a transparent backdrop, invalid for the Contrast extension.
pub fn build_invalid(case: u32, phase: u32, mode: u32) -> Result<Frame> {
    if mode > 1 {
        return Err(ChartError("unknown invalid border profile mode".into()));
    }
    build_impl(case, phase, Some(mode))
}
fn build_impl(case: u32, phase: u32, invalid: Option<u32>) -> Result<Frame> {
    if case >= CASE_COUNT || phase > 3 {
        return Err(ChartError("unknown border-role case or phase".into()));
    }
    let mut c = config(phase);
    let mut role = profile();
    if invalid == Some(0) {
        role = InheritedColor::FromContext(vec![ColorModifier::Opacity(f64::NAN)]);
    }
    if invalid == Some(1) {
        role = contrast();
        c.theme.background = Color(255, 255, 255, 0);
        for color in &mut c.theme.colors {
            color.3 = 64;
        }
    }
    if case < 5 {
        build_bar(case, phase, c, role, invalid)
    } else {
        build_heat(case, phase, c, role, invalid)
    }
}
fn build_bar(
    case: u32,
    phase: u32,
    mut c: ChartConfig,
    role: InheritedColor,
    invalid: Option<u32>,
) -> Result<Frame> {
    let categories = ["A", "B", "C"].map(str::to_string);
    let data = if phase == 0 {
        vec![
            BarSeries::new("late", vec![Some(2.), Some(6.), Some(10.)]),
            BarSeries::new("first", vec![None, Some(4.), Some(8.)]),
        ]
    } else {
        vec![
            BarSeries::new("late", vec![Some(4.), Some(8.), Some(6.)]),
            BarSeries::new("first", vec![None, Some(2.), Some(10.)]),
        ]
    };
    let mut o = BarOptions {
        border: Some(fixed(if case == 1 {
            StrokeCap::Square
        } else {
            StrokeCap::Round
        })),
        border_color_profile: Some(role),
        value_domain: Some([0., 10.]),
        ..Default::default()
    };
    o.value_axis.visible = false;
    o.value_axis.grid = false;
    o.category_axis.visible = false;
    let mut style = BarStyleOptions {
        legend: false,
        series_order: Some(vec!["first".into(), "late".into()]),
        ..Default::default()
    };
    if case == 1 {
        style.color_by = BarColorBy::Category;
        style.colors = categories
            .iter()
            .enumerate()
            .map(|(i, id)| (id.clone(), c.theme.colors[i]))
            .collect();
    }
    if case == 4 {
        c.theme.colors = vec![Color(0, 0, 0, 255), Color(255, 255, 255, 255)];
        if invalid == Some(1) {
            for color in &mut c.theme.colors {
                color.3 = 64;
            }
        }
        if invalid.is_none() {
            o.border_color_profile = Some(contrast());
        }
    }
    if case == 3 {
        style.labels.enabled = true;
        style.labels.color_profile =
            Some(InheritedColor::FromContext(vec![ColorModifier::Darker(1.)]));
    }
    let events = RefCell::new(Vec::<Value>::new());
    let (scene, binding, numeric, base, active, restored) = if case == 0 {
        let scene = bar(&c, &categories, &data, &o)?;
        let sites = scene
            .marks
            .iter()
            .filter_map(|m| {
                let Geometry::Rect {
                    x,
                    y,
                    width,
                    height,
                } = &m.geometry
                else {
                    return None;
                };
                Some(InteractionSite {
                    mark_id: m.id.clone(),
                    series: m.series.clone(),
                    label: m.label.clone(),
                    value: Some(m.value),
                    formatted_value: Some(ValueFormat::Auto.format(m.value).ok()?),
                    color: m.fill?,
                    position: [x + width / 2., y + height / 2.],
                })
            })
            .collect();
        let binding = InteractionBinding {
            sites,
            ..Default::default()
        };
        binding.validate(&scene)?;
        let numeric = scene
            .marks
            .iter()
            .map(|m| json!({"id":m.id,"numericSourceRGBA":m.fill.map(rgba)}))
            .collect::<Vec<_>>();
        let snapshot = scene_facts(&scene);
        (
            scene,
            binding,
            numeric,
            snapshot.clone(),
            snapshot.clone(),
            snapshot,
        )
    } else {
        // Numeric baseline uses the exact public styled path and has no factory.
        let ordinary = bar_layout_with_style(&c, &categories, &data, &o, &style)?;
        let numeric=ordinary.bars.iter().map(|b|json!({"id":b.id,"sourceSeries":b.source_series,"sourceCategory":b.source_category,"series":b.series,"category":b.category,"rawValue":b.raw_value,"numericSourceRGBA":rgba(b.color),"interval":b.interval,"band":b.band,"visibleRect":[b.visible_rect.x,b.visible_rect.y,b.visible_rect.width,b.visible_rect.height],"labelIndex":b.label_index})).collect::<Vec<_>>();
        let mut geometry = |ctx: &BarDatumContext<'_>| {
            events.borrow_mut().push(json!({"factory":"bar","id":ctx.mark_id,"sourceSeries":ctx.source_series,"sourceCategory":ctx.source_category,"rawValue":ctx.raw_value,"position":ctx.position,"numericSourceRGBA":rgba(ctx.color),"inheritedStroke":stroke_facts(ctx.stroke)}));
            let mut ink = ctx.inherit(ctx.geometry.clone());
            ink.fill = Some(Color(0, 180, 70, 255));
            ink.opacity = 0.5;
            if case == 3 {
                ink.stroke = Some(GraphicStroke {
                    color: Color(200, 20, 180, 190),
                    width: 4.,
                    cap: StrokeCap::Square,
                });
            }
            Ok(ink)
        };
        let mut label = |ctx: &BarLabelContext<'_>| {
            events.borrow_mut().push(json!({"factory":"label","id":ctx.mark_id,"rawValue":ctx.raw_value,"sourceCategory":ctx.source_category,"position":ctx.position,"labelPosition":ctx.label_position,"defaultText":ctx.default_text}));
            Ok(format!("{}:{}", ctx.category, ctx.default_text))
        };
        let mut layout = bar_layout_with_factories(
            &c,
            &categories,
            &data,
            &o,
            &style,
            BarFactories {
                bar: if case == 2 || case == 3 {
                    Some(&mut geometry)
                } else {
                    None
                },
                label: if case == 3 { Some(&mut label) } else { None },
            },
        )?;
        let base = scene_facts(&layout.layout.scene);
        let id = layout.layout.bars[0].id.clone();
        layout.set_active(Some(&id), 0.2)?;
        let active = scene_facts(&layout.layout.scene);
        layout.set_active(None, 0.2)?;
        let restored = scene_facts(&layout.layout.scene);
        if phase == 2 {
            layout.set_active(Some(&id), 0.2)?;
        }
        let binding = layout.interaction_binding()?;
        (
            layout.layout.scene,
            binding,
            numeric,
            base,
            active,
            restored,
        )
    };
    let facts = json!({"case":case,"name":CASE_NAMES[case as usize],"phase":phase,"family":"bar","source":{"categories":categories,"series":data.iter().map(|s|json!({"id":s.id,"values":s.values})).collect::<Vec<_>>()},"numeric":numeric,"options":{"border":stroke_facts(o.border),"borderProfile":format!("{:?}",o.border_color_profile),"colorBy":format!("{:?}",style.color_by),"seriesOrder":if case==0 {None}else{style.series_order.clone()},"palette":c.theme.colors.iter().copied().map(rgba).collect::<Vec<_>>(),"valueDomain":o.value_domain},"factoryEvents":events.into_inner(),"base":base,"active":active,"restored":restored,"scene":scene_facts(&scene),"binding":binding_facts(Some(&binding)),"backendScope":"shared actual Rust public layout; host choice is external"});
    let layout = SceneInteraction {
        scene,
        interaction: Some(binding),
        plot: Some(ClipRect::plot(&c)),
    };
    layout.validate()?;
    Ok(Frame { layout, facts })
}
fn build_heat(
    case: u32,
    phase: u32,
    c: ChartConfig,
    role: InheritedColor,
    invalid: Option<u32>,
) -> Result<Frame> {
    let data = vec![
        HeatCell {
            row: "row".into(),
            column: "null".into(),
            value: None,
        },
        HeatCell::new("row", "zero", 0.),
        HeatCell::new("row", "high", if phase == 0 { 10. } else { 5. }),
    ];
    let mut colors = palette(phase);
    if invalid == Some(1) {
        for color in &mut colors {
            color.3 = 64;
        }
    }
    let mut o = HeatmapLayoutOptions {
        axes: HeatmapAxes::none(),
        force_square: false,
        shape: if case == 6 {
            HeatmapShape::Circle
        } else {
            HeatmapShape::Rect
        },
        corner_radius: if case == 5 { 10. } else { 0. },
        column_order: Some(vec!["null".into(), "zero".into(), "high".into()]),
        border: if case == 9 {
            None
        } else {
            Some(fixed(if case == 6 {
                StrokeCap::Square
            } else {
                StrokeCap::Round
            }))
        },
        border_color_profile: Some(role),
        labels: case == 7,
        label_color_profile: Some(InheritedColor::Fixed(Color(30, 40, 50, 101))),
        empty_label: "NULL".into(),
        opacity: 0.8,
        active_opacity: 0.9,
        inactive_opacity: 0.2,
        hover_target: HeatmapHoverTarget::Cell,
        colors: ColorScaleOptions {
            domain: Some([0., 10.]),
            colors: vec![colors[0], colors[1]],
            unknown: colors[2],
            ..Default::default()
        },
        ..Default::default()
    };
    if case == 8 {
        o.size_variation = Some(HeatmapSizeVariation {
            domain: Some([0., 10.]),
            sizes: [0., 1.],
        });
    }
    let events = RefCell::new(Vec::<Value>::new());
    let mut cell = |ctx: &HeatmapCellContext<'_>| {
        events.borrow_mut().push(json!({"factory":"cell","id":ctx.mark_id,"sourceIndex":ctx.source_index,"value":ctx.datum.value,"numericSourceRGBA":rgba(ctx.color),"inheritedFill":ctx.fill.map(rgba),"inheritedStroke":stroke_facts(ctx.stroke),"position":ctx.position}));
        let mut ink = ctx.inherit(ctx.geometry.clone());
        ink.fill = Some(Color(0, 180, 70, 255));
        ink.opacity = 0.5;
        Ok(ink)
    };
    let mut label = |ctx: &HeatmapLabelContext<'_>| {
        events.borrow_mut().push(json!({"factory":"label","id":ctx.mark_id,"sourceIndex":ctx.source_index,"value":ctx.datum.value,"defaultText":ctx.default_text,"position":ctx.position,"labelPosition":ctx.label_position}));
        Ok(format!("{}:{}", ctx.datum.column, ctx.default_text))
    };
    let mut layout = heatmap_layout_with_factories(
        &c,
        &data,
        &o,
        HeatmapFactories {
            cell: if case == 7 { Some(&mut cell) } else { None },
            label: if case == 7 { Some(&mut label) } else { None },
        },
    )?;
    let numeric=layout.cells.iter().map(|cell|json!({"id":cell.mark_id,"row":cell.key.row,"column":cell.key.column,"value":cell.value,"formattedValue":cell.formatted_value,"originalTooltip":cell.label,"numericSourceRGBA":rgba(layout.color_scale.map(cell.value)),"metadataRGBA":rgba(cell.color),"visible":cell.visible,"band":cell.band.map(|r|[r.x,r.y,r.width,r.height]),"logicalPosition":cell.center,"geometry":cell.geometry.as_ref().map(geometry_facts)})).collect::<Vec<_>>();
    let base = scene_facts(&layout.scene);
    layout.set_active(Some(HeatmapCellKey::new("row", "zero")))?;
    let active = scene_facts(&layout.scene);
    layout.set_active(None)?;
    let restored = scene_facts(&layout.scene);
    if phase == 2 {
        layout.set_active(Some(HeatmapCellKey::new("row", "zero")))?;
    }
    let binding = layout.interaction_binding()?;
    let facts = json!({"case":case,"name":CASE_NAMES[case as usize],"phase":phase,"family":"heatmap","source":data.iter().map(|s|json!({"row":s.row,"column":s.column,"value":s.value})).collect::<Vec<_>>(),"numeric":numeric,"options":{"border":stroke_facts(o.border),"borderProfile":format!("{:?}",o.border_color_profile),"shape":format!("{:?}",o.shape),"sizeVariation":format!("{:?}",o.size_variation),"colorDomain":o.colors.domain,"palette":colors.iter().copied().map(rgba).collect::<Vec<_>>(),"opacities":[o.opacity,o.active_opacity,o.inactive_opacity]},"factoryEvents":events.into_inner(),"base":base,"active":active,"restored":restored,"scene":scene_facts(&layout.scene),"binding":binding_facts(Some(&binding)),"backendScope":"shared actual Rust public layout; host choice is external"});
    let published = SceneInteraction {
        scene: layout.scene,
        interaction: Some(binding),
        plot: Some(layout.plot),
    };
    published.validate()?;
    Ok(Frame {
        layout: published,
        facts,
    })
}
