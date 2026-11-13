//! Configured Bullet layout. References: nivo 0.99.0
//! `packages/bullet/src/{types.ts,hooks.ts,compute.ts,Bullet.tsx}`.
//! Sorted endpoints form adjacent intervals, not overlapping bars from zero.
//! This profile deliberately preserves zero/negative values and source indices.
use std::collections::{BTreeMap, BTreeSet};

use glory::render::GraphicStroke;

use crate::boxplot_layout::{stats_extent, stats_id, stats_path, stats_stroke, stats_stroke_valid};
use crate::scene::ensure;
use crate::*;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BulletDomainMode {
    #[default]
    Shared,
    PerRow,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BulletPosition {
    Before,
    #[default]
    After,
}
#[derive(Clone, Debug, PartialEq)]
pub struct BulletLayoutOptions {
    pub orientation: StatisticsOrientation,
    pub reverse: bool,
    pub domain_mode: BulletDomainMode,
    pub min: Option<f64>,
    pub max: Option<f64>,
    /// None preserves source row order. Explicit order must include all rows.
    pub order: Option<Vec<String>>,
    pub hidden: BTreeSet<String>,
    pub spacing: f64,
    pub measure_size: f64,
    pub marker_size: f64,
    pub range_colors: Vec<Color>,
    pub measure_colors: Vec<Color>,
    pub marker_colors: Vec<Color>,
    pub range_border: Option<GraphicStroke>,
    pub measure_border: Option<GraphicStroke>,
    pub marker_width: f64,
    pub axis: AxisOptions,
    pub axis_position: BulletPosition,
    pub titles: BTreeMap<String, String>,
    pub title_position: BulletPosition,
    pub title_align: TextAnchor,
    pub title_offset: [f64; 2],
    pub title_rotation: f64,
    pub value_format: ValueFormat,
    pub legend: bool,
}
impl Default for BulletLayoutOptions {
    fn default() -> Self {
        Self {
            orientation: StatisticsOrientation::Horizontal,
            reverse: false,
            domain_mode: Default::default(),
            min: None,
            max: None,
            order: None,
            hidden: Default::default(),
            spacing: 32.,
            measure_size: 0.4,
            marker_size: 0.6,
            range_colors: vec![
                Color(225, 229, 235, 255),
                Color(195, 203, 214, 255),
                Color(160, 175, 195, 255),
            ],
            measure_colors: vec![Color(78, 121, 167, 255), Color(55, 85, 130, 255)],
            marker_colors: vec![Color(225, 87, 89, 255)],
            range_border: None,
            measure_border: None,
            marker_width: 2.,
            axis: Default::default(),
            axis_position: BulletPosition::After,
            titles: Default::default(),
            title_position: BulletPosition::Before,
            title_align: TextAnchor::Middle,
            title_offset: [-10., 0.],
            title_rotation: 0.,
            value_format: ValueFormat::Auto,
            legend: true,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BulletPartKind {
    Range,
    Measure,
    Marker,
}
#[derive(Clone, Debug, PartialEq)]
pub struct BulletPart {
    pub id: String,
    pub row: String,
    pub kind: BulletPartKind,
    /// None denotes the synthetic final range ending at the domain maximum.
    pub source_index: Option<usize>,
    pub raw_value: f64,
    pub interval: [f64; 2],
    pub visible_interval: [f64; 2],
    pub geometry: Geometry,
}
#[derive(Clone, Debug, PartialEq)]
pub struct BulletRowLayout {
    pub id: String,
    /// Bounds used to draw the row, after any configured nice extension.
    pub domain: [f64; 2],
    pub track: ClipRect,
    pub parts: Vec<BulletPart>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct BulletLayout {
    pub scene: Scene,
    pub rows: Vec<BulletRowLayout>,
    pub shared_domain: Option<[f64; 2]>,
}
#[derive(Clone, Debug)]
pub struct ScaledBulletLayout {
    pub layout: BulletLayout,
    /// The shared value scale, including when all rows are hidden.
    pub shared_value_scale: Option<ResolvedContinuousScale>,
    /// The actual scale used to draw each visible row.
    pub row_value_scales: BTreeMap<String, ResolvedContinuousScale>,
}
pub fn bullet_with_options(
    c: &ChartConfig,
    data: &[BulletDatum],
    o: &BulletLayoutOptions,
) -> Result<Scene> {
    Ok(bullet_layout(c, data, o)?.scene)
}
fn all_values(d: &BulletDatum) -> impl Iterator<Item = f64> + '_ {
    d.ranges
        .iter()
        .chain(&d.measures)
        .chain(&d.markers)
        .copied()
}
fn domain(
    values: impl Iterator<Item = f64>,
    o: &BulletLayoutOptions,
    kind: ScaleKind,
) -> Result<[f64; 2]> {
    let mut values = values.peekable();
    let e = if kind.requires_nonzero_domain() && values.peek().is_none() {
        match o.min.or(o.max) {
            Some(bound) => [bound, bound],
            None => match &o.axis.tick_values {
                Some(ticks) if !ticks.is_empty() => stats_extent(ticks.iter().copied()),
                _ => [1., 10.],
            },
        }
    } else {
        stats_extent(values)
    };
    let d = [o.min.unwrap_or(e[0]), o.max.unwrap_or(e[1])];
    ensure(
        d.iter().all(|v| v.is_finite()) && d[0] <= d[1],
        "invalid bullet domain",
    )?;
    Ok(d)
}
pub fn bullet_layout(
    c: &ChartConfig,
    data: &[BulletDatum],
    o: &BulletLayoutOptions,
) -> Result<BulletLayout> {
    Ok(build_bullet_layout(c, data, o, None)?.layout)
}

/// Use a shared continuous scale for tracks, ticks and part geometry.
///
/// Both range and measure intervals start at the displayed lower bound, and
/// the final range ends at the displayed upper bound. Log supports either
/// nonzero sign. With no values, its inferred bounds come from an explicit
/// bound, explicit ticks, or the positive interval 1..=10, in that order.
/// UTC uses bounded integral epoch milliseconds and accepts calendar ticks.
/// Ink is clipped to the track even when the query scale extrapolates.
/// The two reverse flags toggle the same direction, so using both cancels.
pub fn bullet_layout_with_scale_options(
    c: &ChartConfig,
    data: &[BulletDatum],
    o: &BulletLayoutOptions,
    value_scale: &ContinuousScaleOptions,
) -> Result<ScaledBulletLayout> {
    value_scale.validate()?;
    build_bullet_layout(c, data, o, Some(value_scale))
}

fn build_bullet_layout(
    c: &ChartConfig,
    data: &[BulletDatum],
    o: &BulletLayoutOptions,
    value_scale: Option<&ContinuousScaleOptions>,
) -> Result<ScaledBulletLayout> {
    let mut scene = c.scene()?;
    let kind = value_scale.map_or(ScaleKind::Linear, |scale| scale.kind);
    ensure(
        data.len() <= 256
            && data
                .iter()
                .all(|d| d.ranges.len() <= 64 && d.measures.len() <= 64 && d.markers.len() <= 64),
        "bullet input budget exceeded",
    )?;
    ensure(
        data.iter()
            .map(|d| d.ranges.len() + d.measures.len() + d.markers.len())
            .sum::<usize>()
            <= MAX_MARKS,
        "bullet source component budget exceeded",
    )?;
    ensure(
        o.spacing.is_finite()
            && (0. ..=16384.).contains(&o.spacing)
            && [o.measure_size, o.marker_size]
                .into_iter()
                .all(|v| v.is_finite() && (0. ..=1.).contains(&v))
            && o.marker_width.is_finite()
            && (0. ..=256.).contains(&o.marker_width)
            && o.title_offset
                .into_iter()
                .all(|v| v.is_finite() && v.abs() <= 16384.)
            && o.title_rotation.is_finite()
            && (-360. ..=360.).contains(&o.title_rotation),
        "invalid bullet geometry",
    )?;
    for colors in [&o.range_colors, &o.measure_colors, &o.marker_colors] {
        ensure(
            !colors.is_empty() && colors.len() <= 256,
            "invalid bullet color palette",
        )?;
    }
    stats_stroke_valid(o.range_border)?;
    stats_stroke_valid(o.measure_border)?;
    o.axis.validate()?;
    o.value_format.validate()?;
    ensure(
        o.axis.tick_calendar.is_none() || kind == ScaleKind::Utc,
        "bullet calendar ticks require a UTC value scale",
    )?;
    let bounded_instant =
        |value: f64| value.fract() == 0. && value.abs() <= MAX_TIMESTAMP_MS as f64;
    let bounds = [o.min, o.max].into_iter().flatten();
    match kind {
        ScaleKind::Log => ensure(
            bounds.clone().all(|value| value != 0.)
                && o.min
                    .zip(o.max)
                    .is_none_or(|(a, b)| a.signum() == b.signum()),
            "bullet log bounds must have one nonzero sign",
        )?,
        ScaleKind::Utc => ensure(
            bounds.clone().all(bounded_instant),
            "bullet UTC bounds must be bounded integral epoch milliseconds",
        )?,
        _ => {}
    }
    let mut by_id = BTreeMap::new();
    for d in data {
        stats_id(&d.id, false)?;
        ensure(by_id.insert(&d.id, d).is_none(), "duplicate bullet row ID")?;
        ensure(
            all_values(d).all(f64::is_finite),
            "bullet values must be finite",
        )?;
        if kind == ScaleKind::Utc {
            ensure(
                all_values(d).all(bounded_instant),
                "bullet UTC values must be bounded integral epoch milliseconds",
            )?;
        }
    }
    ensure(
        o.titles
            .iter()
            .all(|(id, title)| by_id.contains_key(id) && title.len() <= MAX_LABEL_BYTES),
        "unknown or oversized bullet title",
    )?;
    let order = if let Some(order) = &o.order {
        let set = order.iter().collect::<BTreeSet<_>>();
        ensure(
            order.len() == data.len()
                && set.len() == data.len()
                && set.iter().all(|id| by_id.contains_key(*id)),
            "bullet order must contain each row exactly once",
        )?;
        order.clone()
    } else {
        data.iter().map(|d| d.id.clone()).collect()
    };
    let visible = order
        .iter()
        .filter(|id| !o.hidden.contains(*id))
        .map(|id| by_id[id])
        .collect::<Vec<_>>();
    let shared = domain(visible.iter().flat_map(|d| all_values(d)), o, kind)?;
    let horizontal = o.orientation == StatisticsOrientation::Horizontal;
    let mut range = if horizontal {
        [c.margin.left, c.margin.left + c.inner_width()]
    } else {
        [c.margin.top + c.inner_height(), c.margin.top]
    };
    if o.reverse {
        range.reverse();
    }
    let shared_value_scale = value_scale
        .filter(|_| o.domain_mode == BulletDomainMode::Shared)
        .map(|options| ResolvedContinuousScale::new(shared, range, options))
        .transpose()?;
    let cross = if horizontal {
        c.inner_height()
    } else {
        c.inner_width()
    };
    let breadth = if visible.is_empty() {
        0.
    } else {
        (cross - o.spacing * (visible.len() - 1) as f64) / visible.len() as f64
    };
    ensure(
        visible.is_empty() || breadth > 0.,
        "bullet spacing exceeds available tracks",
    )?;
    let mut rows = Vec::new();
    let mut row_value_scales = BTreeMap::new();
    for (i, d) in visible.into_iter().enumerate() {
        let source = if o.domain_mode == BulletDomainMode::Shared {
            shared
        } else {
            domain(all_values(d), o, kind)?
        };
        let start = if horizontal {
            c.margin.top
        } else {
            c.margin.left
        } + i as f64 * (breadth + o.spacing);
        let track = if horizontal {
            ClipRect {
                x: c.margin.left,
                y: start,
                width: c.inner_width(),
                height: breadth,
            }
        } else {
            ClipRect {
                x: start,
                y: c.margin.top,
                width: breadth,
                height: c.inner_height(),
            }
        };
        let resolved = if let Some(shared_scale) = &shared_value_scale {
            Some(shared_scale.clone())
        } else {
            value_scale
                .map(|options| ResolvedContinuousScale::new(source, range, options))
                .transpose()?
        };
        let scale = if let Some(scale) = &resolved {
            scale.chart_scale().clone().clamped(true)
        } else {
            ChartScale::new(ScaleKind::Linear, source, range)?.clamped(true)
        };
        let dom = scale.domain();
        let mut parts = Vec::new();
        for (kind, values, colors, border, size) in [
            (
                BulletPartKind::Range,
                &d.ranges,
                &o.range_colors,
                o.range_border,
                1.,
            ),
            (
                BulletPartKind::Measure,
                &d.measures,
                &o.measure_colors,
                o.measure_border,
                o.measure_size,
            ),
        ] {
            let mut endpoints = values
                .iter()
                .enumerate()
                .map(|(j, v)| (Some(j), *v))
                .collect::<Vec<_>>();
            if kind == BulletPartKind::Range && !values.contains(&dom[1]) {
                endpoints.push((None, dom[1]));
            }
            endpoints.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
            let mut previous = dom[0];
            for (rank, (source_index, raw)) in endpoints.into_iter().enumerate() {
                // Values below/above explicit bounds retain their raw metadata;
                // empty cropped intervals produce no phantom fill or border.
                let v0 = previous;
                previous = raw;
                let clipped = [v0.clamp(dom[0], dom[1]), raw.clamp(dom[0], dom[1])];
                let a = scale.map(clipped[0]);
                let b = scale.map(clipped[1]);
                let geometry = if horizontal {
                    Geometry::Rect {
                        x: a.min(b),
                        y: track.y + track.height * (1. - size) / 2.,
                        width: (b - a).abs(),
                        height: track.height * size,
                    }
                } else {
                    Geometry::Rect {
                        x: track.x + track.width * (1. - size) / 2.,
                        y: a.min(b),
                        width: track.width * size,
                        height: (b - a).abs(),
                    }
                };
                let part = if kind == BulletPartKind::Range {
                    "range"
                } else {
                    "measure"
                };
                let suffix = source_index.map_or_else(|| "domain".into(), |j| j.to_string());
                let id = format!("bullet:{}:{}:{part}:{suffix}", d.id.len(), d.id);
                if resolved.is_some() {
                    ensure(
                        a.is_finite() && b.is_finite(),
                        "bullet interval projection overflow",
                    )?;
                }
                if clipped[0] != clipped[1] && size > 0. && (resolved.is_none() || a != b) {
                    let mut mark =
                        Mark::new(id.clone(), geometry.clone(), colors[rank % colors.len()]);
                    mark.series = d.id.clone();
                    mark.value = raw;
                    mark.label = format!(
                        "{} · {part} {}: {} → {}",
                        d.id,
                        suffix,
                        o.value_format.format(v0)?,
                        o.value_format.format(raw)?
                    );
                    mark.stroke = border;
                    mark.clip = Some(track);
                    scene.marks.push(mark);
                }
                parts.push(BulletPart {
                    id,
                    row: d.id.clone(),
                    kind,
                    source_index,
                    raw_value: raw,
                    interval: [v0, raw],
                    visible_interval: clipped,
                    geometry,
                });
            }
        }
        for (j, raw) in d.markers.iter().copied().enumerate() {
            let clipped = raw.clamp(dom[0], dom[1]);
            let pixel = scale.map(clipped);
            if resolved.is_some() {
                ensure(pixel.is_finite(), "bullet marker projection overflow")?;
            }
            let half = breadth * o.marker_size / 2.;
            let points = if horizontal {
                [
                    [pixel, start + breadth / 2. - half],
                    [pixel, start + breadth / 2. + half],
                ]
            } else {
                [
                    [start + breadth / 2. - half, pixel],
                    [start + breadth / 2. + half, pixel],
                ]
            };
            let geometry = stats_path(&points);
            let id = format!("bullet:{}:{}:marker:{j}", d.id.len(), d.id);
            if raw >= dom[0] && raw <= dom[1] && o.marker_size > 0. && o.marker_width > 0. {
                let mut mark = Mark::new(
                    id.clone(),
                    geometry.clone(),
                    o.marker_colors[j % o.marker_colors.len()],
                );
                mark.fill = None;
                mark.stroke = Some(stats_stroke(
                    o.marker_colors[j % o.marker_colors.len()],
                    o.marker_width,
                ));
                mark.series = d.id.clone();
                mark.value = raw;
                mark.label = format!("{} · marker {j}: {}", d.id, o.value_format.format(raw)?);
                mark.clip = Some(track);
                scene.marks.push(mark);
            }
            parts.push(BulletPart {
                id,
                row: d.id.clone(),
                kind: BulletPartKind::Marker,
                source_index: Some(j),
                raw_value: raw,
                interval: [raw, raw],
                visible_interval: [clipped; 2],
                geometry,
            });
        }
        ensure(
            scene.marks.len() <= MAX_MARKS,
            "bullet mark budget exceeded",
        )?;
        let mut local = c.clone();
        local.margin = Margin {
            left: track.x,
            right: c.width - track.x - track.width,
            top: track.y,
            bottom: c.height - track.y - track.height,
        };
        let side = match (horizontal, o.axis_position) {
            (true, BulletPosition::Before) => AxisSide::Top,
            (true, BulletPosition::After) => AxisSide::Bottom,
            (false, BulletPosition::Before) => AxisSide::Left,
            (false, BulletPosition::After) => AxisSide::Right,
        };
        let labels_start = scene.labels.len();
        let marks_start = scene.marks.len();
        crate::axes::axis(
            &mut scene,
            &local,
            &scale,
            side,
            &o.axis,
            &format!("bullet-axis:{}:{}", d.id.len(), d.id),
        )?;
        for mark in &mut scene.marks[marks_start..] {
            mark.series = d.id.clone();
        }
        for index in labels_start..scene.labels.len() {
            scene.label_series.insert(index, d.id.clone());
        }
        let before = o.title_position == BulletPosition::Before;
        let position = if horizontal {
            [
                if before {
                    track.x
                } else {
                    track.x + track.width
                },
                track.y + track.height / 2.,
            ]
        } else {
            [
                track.x + track.width / 2.,
                if before {
                    track.y
                } else {
                    track.y + track.height
                },
            ]
        };
        scene.label_series.insert(scene.labels.len(), d.id.clone());
        scene.labels.push(Label {
            x: position[0] + o.title_offset[0],
            y: position[1] + o.title_offset[1],
            text: o.titles.get(&d.id).cloned().unwrap_or_else(|| d.id.clone()),
            color: c.theme.text,
            size: c.theme.font_size,
            anchor: o.title_align,
            rotation: o.title_rotation,
        });
        rows.push(BulletRowLayout {
            id: d.id.clone(),
            domain: dom,
            track,
            parts,
        });
        if let Some(scale) = resolved {
            row_value_scales.insert(d.id.clone(), scale);
        }
    }
    if o.legend {
        let mut keys = data.iter().map(|d| &d.id).collect::<Vec<_>>();
        keys.sort();
        for id in keys {
            scene.legend.push(LegendItem {
                id: id.clone(),
                label: o.titles.get(id).cloned().unwrap_or_else(|| id.clone()),
                // A row visibility key, using the first actual measure color;
                // measure/range palettes encode interval rank, not row identity.
                color: o.measure_colors[0],
            });
        }
    }
    scene.validate()?;
    Ok(ScaledBulletLayout {
        layout: BulletLayout {
            scene,
            rows,
            shared_domain: (o.domain_mode == BulletDomainMode::Shared).then(|| {
                shared_value_scale
                    .as_ref()
                    .map_or(shared, ResolvedContinuousScale::display_domain)
            }),
        },
        shared_value_scale,
        row_value_scales,
    })
}
