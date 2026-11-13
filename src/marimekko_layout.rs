//! Independent thickness/dimensions Marimekko layout. Reference: Nivo 0.99.0
//! `packages/marimekko/src/{types.ts,hooks.ts,defaults.ts}`. The old
//! `advanced::marimekko` remains a separate area-conserving, expanded model.
use std::collections::{BTreeMap, BTreeSet};

use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};

use crate::scene::ensure;
use crate::stack::{StackOptions, StackResult};
use crate::*;
pub const MAX_MARIMEKKO_CELLS: usize = 20_000;
#[derive(Clone, Debug, PartialEq)]
pub struct MarimekkoCategory {
    pub id: String,
    pub label: String,
    pub thickness: f64,
}
impl MarimekkoCategory {
    pub fn new(id: impl Into<String>, thickness: f64) -> Self {
        let id = id.into();
        Self {
            label: id.clone(),
            id,
            thickness,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct MarimekkoDimensionStyle {
    pub color: Option<Color>,
    pub paint: Option<FillPaint>,
    pub fill_opacity: f64,
    pub border: Option<GraphicStroke>,
}
impl Default for MarimekkoDimensionStyle {
    fn default() -> Self {
        Self {
            color: None,
            paint: None,
            fill_opacity: 1.,
            border: None,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MarimekkoAxes {
    pub top: Option<AxisOptions>,
    pub right: Option<AxisOptions>,
    pub bottom: Option<AxisOptions>,
    pub left: Option<AxisOptions>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct MarimekkoGrid {
    pub enabled: bool,
    pub tick_count: usize,
    pub tick_values: Option<Vec<f64>>,
}
impl Default for MarimekkoGrid {
    fn default() -> Self {
        Self {
            enabled: false,
            tick_count: 10,
            tick_values: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct MarimekkoCategoryAxis {
    pub side: AxisSide,
    pub options: CategoryAxisOptions,
}
#[derive(Clone, Debug, PartialEq)]
pub struct MarimekkoLayoutOptions {
    pub horizontal: bool,
    pub stack: StackOptions,
    /// Logical pixels. All unhidden categories, including zero thickness, count.
    pub outer_padding: f64,
    pub inner_padding: f64,
    /// None retains exact domains. Some(count) applies D3 nice to each domain;
    /// a niced total may leave extra trailing space beyond the reserved padding.
    pub thickness_nice: Option<usize>,
    pub dimensions_nice: Option<usize>,
    pub dimensions_domain: Option<[f64; 2]>,
    pub hidden_categories: BTreeSet<String>,
    pub hidden_dimensions: BTreeSet<String>,
    pub style: MarimekkoDimensionStyle,
    pub styles: BTreeMap<String, MarimekkoDimensionStyle>,
    pub dimension_labels: BTreeMap<String, String>,
    pub labels: BarLabels,
    pub tooltip_format: ValueFormat,
    /// Numeric thickness axes expose the compressed, unpadded linear scale,
    /// matching Nivo. Category ticks below use actual padded band centers.
    pub axes: MarimekkoAxes,
    pub grid_x: MarimekkoGrid,
    pub grid_y: MarimekkoGrid,
    pub category_axis: Option<MarimekkoCategoryAxis>,
    pub legend: bool,
}
impl Default for MarimekkoLayoutOptions {
    fn default() -> Self {
        Self {
            horizontal: false,
            stack: Default::default(),
            outer_padding: 0.,
            inner_padding: 3.,
            thickness_nice: Some(10),
            dimensions_nice: Some(10),
            dimensions_domain: None,
            hidden_categories: Default::default(),
            hidden_dimensions: Default::default(),
            style: Default::default(),
            styles: Default::default(),
            dimension_labels: Default::default(),
            labels: Default::default(),
            tooltip_format: ValueFormat::Auto,
            axes: Default::default(),
            grid_x: Default::default(),
            grid_y: MarimekkoGrid {
                enabled: true,
                ..Default::default()
            },
            category_axis: None,
            legend: true,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ComputedMarimekkoCategory {
    pub id: String,
    pub label: String,
    pub source_index: usize,
    pub thickness: f64,
    pub hidden: bool,
    pub cumulative: Option<[f64; 2]>,
    pub band: Option<[f64; 2]>,
    pub bounds: Option<ClipRect>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct MarimekkoCell {
    pub id: String,
    pub category: String,
    pub dimension: String,
    pub source_category: usize,
    pub source_dimension: usize,
    pub stack_index: usize,
    pub raw_value: Option<f64>,
    /// Nivo's interval difference y1-y0: normalized for Expand and a positive
    /// length for Diverging. raw_value and Mark.value preserve the source sign.
    pub displayed_value: Option<f64>,
    pub interval: [f64; 2],
    pub rect: ClipRect,
    pub visible_rect: Option<ClipRect>,
    pub mark_index: Option<usize>,
    pub label_index: Option<usize>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct MarimekkoLayout {
    pub scene: Scene,
    pub categories: Vec<ComputedMarimekkoCategory>,
    pub cells: Vec<MarimekkoCell>,
    pub stack: StackResult,
    pub dimension_source_indices: Vec<usize>,
    pub category_source_indices: Vec<usize>,
    pub thickness_total: f64,
    pub thickness_domain: [f64; 2],
    pub dimensions_domain: [f64; 2],
    pub thickness_range: [f64; 2],
    pub dimensions_range: [f64; 2],
    pub available_thickness: f64,
}
fn key(s: &str) -> Result<()> {
    ensure(
        !s.trim().is_empty() && s.len() <= 256,
        "invalid Marimekko identity",
    )
}
fn text_label(
    c: &ChartConfig,
    at: [f64; 2],
    text: String,
    style: &BarTextStyle,
    anchor: TextAnchor,
) -> Label {
    Label {
        x: at[0],
        y: at[1],
        text,
        color: style.color.unwrap_or(c.theme.text),
        size: style.size.unwrap_or(c.theme.font_size),
        rotation: style.rotation,
        anchor,
    }
}
fn style_valid(s: &MarimekkoDimensionStyle) -> Result<()> {
    ensure(
        s.fill_opacity.is_finite()
            && (0. ..=1.).contains(&s.fill_opacity)
            && s.border
                .is_none_or(|b| b.width.is_finite() && (0.000001..=10000.).contains(&b.width)),
        "invalid Marimekko paint style",
    )?;
    if let Some(p) = &s.paint {
        p.validate()?;
    }
    Ok(())
}
fn labels_valid(l: &BarLabels) -> Result<()> {
    l.format.validate()?;
    ensure(
        [l.offset, l.skip_width, l.skip_height, l.style.rotation]
            .into_iter()
            .all(f64::is_finite)
            && l.offset.abs() <= 16384.
            && (0. ..=16384.).contains(&l.skip_width)
            && (0. ..=16384.).contains(&l.skip_height)
            && (-360. ..=360.).contains(&l.style.rotation)
            && l.style
                .size
                .is_none_or(|v| v.is_finite() && (1. ..=256.).contains(&v)),
        "invalid Marimekko labels",
    )
}
fn numeric_axis_valid(o: &AxisOptions) -> Result<()> {
    o.validate()?;
    ensure(
        o.tick_calendar.is_none(),
        "Marimekko numeric axes do not accept calendar ticks",
    )
}
fn resolved_domain(domain: [f64; 2], nice: Option<usize>) -> Result<[f64; 2]> {
    let scale = crate::scales::LinearScale::new(domain, [0., 1.])?;
    ensure(domain[0] <= domain[1], "Marimekko domain must be ascending")?;
    let result = nice.map_or(scale, |count| scale.nice(count)).domain;
    ensure(
        result.iter().all(|v| v.is_finite()) && (result[1] - result[0]).is_finite(),
        "Marimekko nice domain overflow",
    )?;
    Ok(result)
}
fn rect_geometry(r: ClipRect) -> Geometry {
    Geometry::Rect {
        x: r.x,
        y: r.y,
        width: r.width,
        height: r.height,
    }
}
fn intersection(a: ClipRect, b: ClipRect) -> Option<ClipRect> {
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    let right = (a.x + a.width).min(b.x + b.width);
    let bottom = (a.y + a.height).min(b.y + b.height);
    (right > x && bottom > y).then_some(ClipRect {
        x,
        y,
        width: right - x,
        height: bottom - y,
    })
}
fn path_mark(scene: &mut Scene, id: String, points: [[f64; 2]; 2], color: Color) {
    let mut m = Mark::new(
        id,
        Geometry::Path(vec![
            C::Move(P::new(points[0][0], points[0][1])),
            C::Line(P::new(points[1][0], points[1][1])),
        ]),
        color,
    );
    m.label.clear();
    m.fill = None;
    m.stroke = Some(GraphicStroke {
        color,
        width: 1.,
        cap: StrokeCap::Butt,
    });
    scene.marks.push(m);
}
fn categories_axis(
    scene: &mut Scene,
    c: &ChartConfig,
    categories: &[ComputedMarimekkoCategory],
    o: &MarimekkoCategoryAxis,
    horizontal: bool,
) -> Result<()> {
    o.options.validate()?;
    ensure(
        if horizontal {
            matches!(o.side, AxisSide::Left | AxisSide::Right)
        } else {
            matches!(o.side, AxisSide::Top | AxisSide::Bottom)
        },
        "Marimekko category axis side disagrees with layout",
    )?;
    if let Some(keys) = &o.options.tick_values {
        let unique = keys.iter().collect::<BTreeSet<_>>();
        ensure(
            unique.len() == keys.len()
                && keys.iter().all(|id| categories.iter().any(|d| &d.id == id)),
            "invalid Marimekko category ticks",
        )?;
    }
    let active = categories
        .iter()
        .filter(|d| !d.hidden && d.band.is_some_and(|b| b[1] > 0.))
        .collect::<Vec<_>>();
    let stride = active.len().div_ceil(o.options.tick_count.max(1)).max(1);
    let side = o.side;
    let baseline = match side {
        AxisSide::Top => c.margin.top,
        AxisSide::Bottom => c.height - c.margin.bottom,
        AxisSide::Left => c.margin.left,
        AxisSide::Right => c.width - c.margin.right,
    };
    let direction = if matches!(side, AxisSide::Left | AxisSide::Top) {
        -1.
    } else {
        1.
    };
    if o.options.visible {
        path_mark(
            scene,
            "mekko-category-axis".into(),
            if horizontal {
                [
                    [baseline, c.margin.top],
                    [baseline, c.height - c.margin.bottom],
                ]
            } else {
                [
                    [c.margin.left, baseline],
                    [c.width - c.margin.right, baseline],
                ]
            },
            c.theme.text,
        );
    }
    for (i, d) in active.iter().enumerate() {
        let selected = o
            .options
            .tick_values
            .as_ref()
            .map_or(o.options.tick_count > 0 && i % stride == 0, |ticks| {
                ticks.contains(&d.id)
            });
        if !selected {
            continue;
        }
        let band = d.band.unwrap();
        let pixel = band[0] + band[1] / 2.;
        if o.options.grid {
            path_mark(
                scene,
                format!("mekko-category-grid:{}:{}", d.id.len(), d.id),
                if horizontal {
                    [[c.margin.left, pixel], [c.width - c.margin.right, pixel]]
                } else {
                    [[pixel, c.margin.top], [pixel, c.height - c.margin.bottom]]
                },
                c.theme.grid,
            );
        }
        if o.options.visible {
            let pos = if horizontal {
                [
                    baseline + direction * o.options.tick_padding,
                    pixel + c.theme.font_size / 3.,
                ]
            } else {
                [
                    pixel,
                    baseline
                        + direction * o.options.tick_padding
                        + if direction > 0. {
                            c.theme.font_size
                        } else {
                            0.
                        },
                ]
            };
            scene.labels.push(text_label(
                c,
                pos,
                d.label.clone(),
                &BarTextStyle {
                    rotation: o.options.tick_rotation,
                    ..Default::default()
                },
                if horizontal {
                    if direction < 0. {
                        TextAnchor::End
                    } else {
                        TextAnchor::Start
                    }
                } else {
                    TextAnchor::Middle
                },
            ));
        }
    }
    if o.options.visible
        && let Some(legend) = &o.options.legend
    {
        let pos = if horizontal {
            [baseline, c.margin.top - c.theme.font_size]
        } else {
            [
                c.margin.left + c.inner_width() / 2.,
                baseline + direction * (o.options.tick_padding + c.theme.font_size * 2.5),
            ]
        };
        scene.labels.push(text_label(
            c,
            pos,
            legend.clone(),
            &Default::default(),
            TextAnchor::Middle,
        ));
    }
    Ok(())
}
pub fn marimekko_with_options(
    c: &ChartConfig,
    categories: &[MarimekkoCategory],
    dimensions: &[BarSeries],
    o: &MarimekkoLayoutOptions,
) -> Result<Scene> {
    Ok(marimekko_layout(c, categories, dimensions, o)?.scene)
}
/// A pure checked proposal. Hidden dimensions affect only stack/height; hidden
/// categories remove a column before both scale and stack calculation. None
/// remains missing metadata and contributes zero. Zero thickness still affects
/// stack/domain and padding, but never emits ink, controls or data labels.
pub fn marimekko_layout(
    c: &ChartConfig,
    categories: &[MarimekkoCategory],
    dimensions: &[BarSeries],
    o: &MarimekkoLayoutOptions,
) -> Result<MarimekkoLayout> {
    let mut scene = c.scene()?;
    ensure(
        categories.len() <= MAX_MARIMEKKO_CELLS
            && dimensions.len() <= 256
            && categories
                .len()
                .checked_mul(dimensions.len())
                .is_some_and(|v| v <= MAX_MARIMEKKO_CELLS),
        "Marimekko source budget exceeded",
    )?;
    ensure(
        o.styles.len() <= 256
            && o.dimension_labels.len() <= 256
            && o.hidden_categories.len() <= MAX_MARIMEKKO_CELLS
            && o.hidden_dimensions.len() <= 256,
        "Marimekko option budget exceeded",
    )?;
    ensure(
        [o.outer_padding, o.inner_padding]
            .into_iter()
            .all(|v| v.is_finite() && (0. ..=16384.).contains(&v))
            && [o.thickness_nice, o.dimensions_nice]
                .into_iter()
                .all(|n| n.is_none_or(|n| (1..=1000).contains(&n))),
        "invalid Marimekko padding or nice count",
    )?;
    style_valid(&o.style)?;
    for s in o.styles.values() {
        style_valid(s)?;
    }
    labels_valid(&o.labels)?;
    o.tooltip_format.validate()?;
    let mut cat_ids = BTreeSet::new();
    for d in categories {
        key(&d.id)?;
        ensure(
            cat_ids.insert(d.id.clone())
                && d.label.len() <= MAX_LABEL_BYTES
                && d.thickness.is_finite()
                && d.thickness >= 0.,
            "invalid/duplicate Marimekko category or thickness",
        )?;
    }
    let mut dim_ids = BTreeSet::new();
    for d in dimensions {
        key(&d.id)?;
        ensure(
            dim_ids.insert(d.id.clone())
                && d.values.len() == categories.len()
                && d.values.iter().flatten().all(|v| v.is_finite()),
            "invalid/duplicate/misaligned Marimekko dimension",
        )?;
    }
    ensure(
        o.styles
            .keys()
            .chain(o.dimension_labels.keys())
            .all(|id| dim_ids.contains(id))
            && o.dimension_labels
                .values()
                .all(|s| s.len() <= MAX_LABEL_BYTES)
            && o.hidden_dimensions
                .iter()
                .all(|id| !id.is_empty() && id.len() <= 256)
            && o.hidden_categories
                .iter()
                .all(|id| !id.is_empty() && id.len() <= 256),
        "invalid Marimekko style/label/hidden key",
    )?;
    let category_source_indices = categories
        .iter()
        .enumerate()
        .filter(|(_, d)| !o.hidden_categories.contains(&d.id))
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    let dimension_source_indices = dimensions
        .iter()
        .enumerate()
        .filter(|(_, d)| !o.hidden_dimensions.contains(&d.id))
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    let visible = dimension_source_indices
        .iter()
        .map(|i| {
            BarSeries::new(
                &dimensions[*i].id,
                category_source_indices
                    .iter()
                    .map(|j| dimensions[*i].values[*j])
                    .collect(),
            )
        })
        .collect::<Vec<_>>();
    let mut stacked = crate::stack::stack(&visible, &o.stack)?;
    if stacked.layers.is_empty() {
        stacked.baseline.resize(category_source_indices.len(), 0.);
    }
    let mut total = 0.;
    for i in &category_source_indices {
        total += categories[*i].thickness;
        ensure(total.is_finite(), "Marimekko thickness total overflow")?;
    }
    let length = if o.horizontal {
        c.inner_height()
    } else {
        c.inner_width()
    };
    let padding = if category_source_indices.is_empty() {
        0.
    } else {
        2. * o.outer_padding
            + category_source_indices.len().saturating_sub(1) as f64 * o.inner_padding
    };
    let available = length - padding;
    ensure(
        available >= 0. && (total == 0. || available > 0.),
        "Marimekko padding exhausts thickness axis",
    )?;
    let thickness_domain = resolved_domain([0., total], o.thickness_nice)?;
    let dimensions_domain = resolved_domain(
        o.dimensions_domain.unwrap_or(stacked.domain),
        o.dimensions_nice,
    )?;
    let thickness_range = if o.horizontal {
        [c.margin.top, c.margin.top + available]
    } else {
        [c.margin.left, c.margin.left + available]
    };
    let dimensions_range = if o.horizontal {
        [c.margin.left, c.width - c.margin.right]
    } else {
        [c.height - c.margin.bottom, c.margin.top]
    };
    let tscale = ChartScale::new(ScaleKind::Linear, thickness_domain, thickness_range)?;
    let dscale = ChartScale::new(ScaleKind::Linear, dimensions_domain, dimensions_range)?;
    let mut computed = categories
        .iter()
        .enumerate()
        .map(|(i, d)| ComputedMarimekkoCategory {
            id: d.id.clone(),
            label: d.label.clone(),
            source_index: i,
            thickness: d.thickness,
            hidden: o.hidden_categories.contains(&d.id),
            cumulative: None,
            band: None,
            bounds: None,
        })
        .collect::<Vec<_>>();
    let mut prefix = 0.;
    let mut position = if o.horizontal {
        c.margin.top
    } else {
        c.margin.left
    } + o.outer_padding;
    for i in &category_source_indices {
        let d = &categories[*i];
        let width = if total == 0. {
            0.
        } else {
            d.thickness / thickness_domain[1] * available
        };
        ensure(
            d.thickness == 0. || width > 0. && position + width > position,
            "positive Marimekko thickness is not representable",
        )?;
        computed[*i].cumulative = Some([prefix, prefix + d.thickness]);
        computed[*i].band = Some([position, width]);
        prefix += d.thickness;
        position += width + o.inner_padding;
    }
    let (x, y) = if o.horizontal {
        (&dscale, &tscale)
    } else {
        (&tscale, &dscale)
    };
    for (name, g, scale, side) in [
        ("x", &o.grid_x, x, AxisSide::Bottom),
        ("y", &o.grid_y, y, AxisSide::Left),
    ] {
        let a = AxisOptions {
            visible: false,
            grid: g.enabled,
            tick_count: g.tick_count,
            tick_values: g.tick_values.clone(),
            ..Default::default()
        };
        numeric_axis_valid(&a)?;
        crate::axes::axis(
            &mut scene,
            c,
            scale,
            side,
            &a,
            &format!("mekko-grid-{name}"),
        )?;
    }
    for (name, side, a, scale) in [
        ("top", AxisSide::Top, &o.axes.top, x),
        ("bottom", AxisSide::Bottom, &o.axes.bottom, x),
        ("left", AxisSide::Left, &o.axes.left, y),
        ("right", AxisSide::Right, &o.axes.right, y),
    ] {
        if let Some(a) = a {
            numeric_axis_valid(a)?;
            crate::axes::axis(&mut scene, c, scale, side, a, &format!("mekko-axis-{name}"))?;
        }
    }
    if let Some(a) = &o.category_axis {
        categories_axis(&mut scene, c, &computed, a, o.horizontal)?;
    }
    let palette = dim_ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.clone(), c.theme.color(i)))
        .collect::<BTreeMap<_, _>>();
    let clip = ClipRect::plot(c);
    let mut cells =
        Vec::with_capacity(category_source_indices.len() * dimension_source_indices.len());
    let mut stops = 0_usize;
    for (si, layer) in stacked.layers.iter().enumerate() {
        let source_dimension = dimension_source_indices[si];
        let dimension = &dimensions[source_dimension];
        let style = o.styles.get(&dimension.id).unwrap_or(&o.style);
        let color = style.color.unwrap_or(palette[&dimension.id]);
        let alias = o
            .dimension_labels
            .get(&dimension.id)
            .unwrap_or(&dimension.id);
        for point in &layer.points {
            let source_category = category_source_indices[point.column];
            let category = &categories[source_category];
            let band = computed[source_category].band.unwrap();
            let a = dscale.map(point.y0);
            let b = dscale.map(point.y1);
            ensure(
                point.value.is_none_or(|v| v == 0.)
                    || band[1] == 0.
                    || point.y0 != point.y1 && a != b,
                "nonzero Marimekko dimension thickness is not representable",
            )?;
            ensure(
                [a, b]
                    .into_iter()
                    .all(|p| p.is_finite() && p.abs() <= 1_000_000.),
                "Marimekko dimensions projection exceeds coordinate profile",
            )?;
            let rect = if o.horizontal {
                ClipRect {
                    x: a.min(b),
                    y: band[0],
                    width: (a - b).abs(),
                    height: band[1],
                }
            } else {
                ClipRect {
                    x: band[0],
                    y: a.min(b),
                    width: band[1],
                    height: (a - b).abs(),
                }
            };
            rect.validate()?;
            let displayed = point.value.map(|_| point.y1 - point.y0);
            ensure(
                displayed.is_none_or(f64::is_finite),
                "Marimekko displayed value overflow",
            )?;
            if band[1] > 0. {
                computed[source_category].bounds = Some(match computed[source_category].bounds {
                    None => rect,
                    Some(old) => {
                        let x = old.x.min(rect.x);
                        let y = old.y.min(rect.y);
                        ClipRect {
                            x,
                            y,
                            width: (old.x + old.width).max(rect.x + rect.width) - x,
                            height: (old.y + old.height).max(rect.y + rect.height) - y,
                        }
                    }
                });
            }
            let id = format!(
                "mekko:{}:{}:{}:{}",
                dimension.id.len(),
                dimension.id,
                category.id.len(),
                category.id
            );
            let mut mark_index = None;
            let mut label_index = None;
            let visible_rect =
                if point.value.is_some_and(|v| v != 0.) && rect.width > 0. && rect.height > 0. {
                    intersection(rect, clip)
                } else {
                    None
                };
            if let Some(visible) = visible_rect {
                ensure(
                    scene.marks.len() + 1 + usize::from(style.border.is_some()) <= MAX_MARKS,
                    "Marimekko output mark budget exceeded",
                )?;
                stops += style.paint.as_ref().map_or(0, |p| p.stops().len());
                ensure(
                    stops <= crate::MAX_SCENE_GRADIENT_STOPS,
                    "Marimekko gradient budget exceeded",
                )?;
                let raw = point.value.unwrap();
                let mut m = Mark::new(&id, rect_geometry(rect), color);
                m.series = dimension.id.clone();
                m.value = raw;
                m.label = format!(
                    "{} · {}: {}",
                    category.label,
                    alias,
                    o.tooltip_format.format(raw)?
                );
                ensure(
                    m.label.len() <= MAX_LABEL_BYTES,
                    "Marimekko tooltip label budget exceeded",
                )?;
                m.fill_paint = style.paint.clone();
                m.opacity = style.fill_opacity;
                m.clip = Some(clip);
                mark_index = Some(scene.marks.len());
                scene.marks.push(m);
                if let Some(border) = style.border {
                    let mut m = Mark::new(format!("{id}:border"), rect_geometry(rect), color);
                    m.fill = None;
                    m.stroke = Some(border);
                    m.series = dimension.id.clone();
                    m.label.clear();
                    m.clip = Some(clip);
                    scene.marks.push(m);
                }
                let l = &o.labels;
                if l.enabled && visible.width >= l.skip_width && visible.height >= l.skip_height {
                    let value = if l.value == BarLabelValue::Raw {
                        raw
                    } else {
                        displayed.unwrap()
                    };
                    let mut pos = if o.horizontal {
                        [
                            rect.x
                                + match l.position {
                                    BarLabelPosition::Start => 0.,
                                    BarLabelPosition::Middle => rect.width / 2.,
                                    BarLabelPosition::End => rect.width,
                                }
                                + l.offset,
                            rect.y + rect.height / 2.,
                        ]
                    } else {
                        [
                            rect.x + rect.width / 2.,
                            rect.y
                                + match l.position {
                                    BarLabelPosition::Start => rect.height,
                                    BarLabelPosition::Middle => rect.height / 2.,
                                    BarLabelPosition::End => 0.,
                                }
                                - l.offset,
                        ]
                    };
                    pos[1] += l.style.size.unwrap_or(c.theme.font_size) / 3.;
                    if !l.clip_to_plot || clip.contains(pos[0], pos[1]) {
                        ensure(
                            scene.labels.len() < MAX_MARKS,
                            "Marimekko label budget exceeded",
                        )?;
                        let index = scene.labels.len();
                        scene.label_series.insert(index, dimension.id.clone());
                        if l.clip_to_plot {
                            scene.label_clips.insert(
                                index,
                                ClipPath {
                                    geometry: rect_geometry(clip),
                                    rule: ClipRule::NonZero,
                                },
                            );
                        }
                        scene.labels.push(text_label(
                            c,
                            pos,
                            l.format.format(value)?,
                            &l.style,
                            TextAnchor::Middle,
                        ));
                        label_index = Some(index);
                    }
                }
            }
            cells.push(MarimekkoCell {
                id,
                category: category.id.clone(),
                dimension: dimension.id.clone(),
                source_category,
                source_dimension,
                stack_index: layer.stack_index,
                raw_value: point.value,
                displayed_value: displayed,
                interval: [point.y0, point.y1],
                rect,
                visible_rect,
                mark_index,
                label_index,
            });
        }
    }
    if o.legend {
        for d in dimensions {
            let style = o.styles.get(&d.id).unwrap_or(&o.style);
            scene.legend.push(LegendItem {
                id: d.id.clone(),
                label: o
                    .dimension_labels
                    .get(&d.id)
                    .cloned()
                    .unwrap_or_else(|| d.id.clone()),
                color: style.color.unwrap_or(palette[&d.id]),
            });
        }
    }
    scene.validate()?;
    Ok(MarimekkoLayout {
        scene,
        categories: computed,
        cells,
        stack: stacked,
        dimension_source_indices,
        category_source_indices,
        thickness_total: total,
        thickness_domain,
        dimensions_domain,
        thickness_range,
        dimensions_range,
        available_thickness: available,
    })
}
