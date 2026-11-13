//! Actual shared Scene color legends. Discrete bins have hard edges.
//! A continuous ramp retains every sample in one renderer gradient.
//! SequentialQuantile uses that same rectangle with ordered duplicate stops,
//! retaining exact data jumps and isolated endpoint colors without band seams.
use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};

use crate::color_scale::color_value;
use crate::scene::ensure;
use crate::*;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColorLegendOrientation {
    #[default]
    Horizontal,
    Vertical,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ColorLegendOptions {
    pub id: String,
    /// Horizontal domain start is left; vertical domain start is bottom.
    pub position: [f64; 2],
    pub length: f64,
    pub thickness: f64,
    pub orientation: ColorLegendOrientation,
    /// Continuous sample budget, 2..=256. SequentialQuantile instead retains
    /// every effective jump within the existing 512-stop paint budget.
    pub steps: usize,
    pub tick_count: usize,
    pub tick_values: Option<Vec<f64>>,
    pub tick_size: f64,
    pub label_offset: f64,
    pub text_size: f64,
    pub text_color: Color,
    pub format: ValueFormat,
    pub title: String,
    pub unknown_label: Option<String>,
}
impl Default for ColorLegendOptions {
    fn default() -> Self {
        Self {
            id: "color-legend".into(),
            position: [24., 24.],
            length: 200.,
            thickness: 14.,
            orientation: Default::default(),
            steps: 16,
            tick_count: 5,
            tick_values: None,
            tick_size: 4.,
            label_offset: 4.,
            text_size: 12.,
            text_color: Color(45, 52, 64, 255),
            format: ValueFormat::Auto,
            title: String::new(),
            unknown_label: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ColorLegendTick {
    pub value: f64,
    pub color: Color,
    pub position: [f64; 2],
    pub label: String,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ColorLegendLayout {
    pub stops: Vec<ColorScaleStop>,
    pub ticks: Vec<ColorLegendTick>,
    pub unknown: Option<ClipRect>,
}
/// Transactional append; unknown/gradient swatches are decorations rather
/// than data controls. Existing draw order, label ownership and clips survive.
pub fn append_color_legend(
    scene: &mut Scene,
    scale: &ColorScale,
    o: &ColorLegendOptions,
) -> Result<ColorLegendLayout> {
    scene.validate()?;
    ensure(
        !o.id.trim().is_empty() && o.id.len() <= 256,
        "invalid color legend ID",
    )?;
    ensure(
        o.position
            .iter()
            .all(|v| v.is_finite() && v.abs() <= 1_000_000.)
            && o.length.is_finite()
            && (1. ..=16384.).contains(&o.length)
            && o.thickness.is_finite()
            && (1. ..=4096.).contains(&o.thickness)
            && o.tick_count <= 32
            && o.tick_values.as_ref().is_none_or(|v| v.len() <= 32)
            && o.tick_size.is_finite()
            && (0. ..=256.).contains(&o.tick_size)
            && o.label_offset.is_finite()
            && (0. ..=256.).contains(&o.label_offset)
            && o.text_size.is_finite()
            && (1. ..=256.).contains(&o.text_size),
        "invalid color legend dimensions or tick budget",
    )?;
    ensure(
        o.title.len() <= scene::MAX_LABEL_BYTES
            && o.unknown_label
                .as_ref()
                .is_none_or(|s| s.len() <= scene::MAX_LABEL_BYTES),
        "color legend text budget exceeded",
    )?;
    o.format.validate()?;
    let stops = scale.stops(o.steps)?;
    let [a, b] = scale.domain();
    let mut values = if let Some(values) = &o.tick_values {
        ensure(
            values
                .iter()
                .all(|v| v.is_finite() && *v >= a.min(b) && *v <= a.max(b))
                && values
                    .iter()
                    .enumerate()
                    .all(|(i, v)| !values[..i].contains(v)),
            "color legend ticks must be unique and within domain",
        )?;
        values.clone()
    } else if o.tick_count == 0 {
        vec![]
    } else if a == b || o.tick_count == 1 {
        vec![color_value(a, b, 0.5)]
    } else {
        (0..o.tick_count)
            .map(|i| scale.value_at(i as f64 / (o.tick_count - 1) as f64))
            .collect::<Result<Vec<_>>>()?
    };
    // A finite interval may contain fewer representable f64 values than the
    // requested tick count. Keep its endpoints without overprinting duplicates.
    if o.tick_values.is_none() {
        values.dedup();
    }
    let horizontal = o.orientation == ColorLegendOrientation::Horizontal;
    let point = |t: f64| {
        if horizontal {
            [o.position[0] + t * o.length, o.position[1] + o.thickness]
        } else {
            [
                o.position[0] + o.thickness,
                o.position[1] + (1. - t) * o.length,
            ]
        }
    };
    let mut candidate = scene.clone();
    let old_marks = candidate.marks.len();
    let old_labels = candidate.labels.len();
    let prefix = format!("color-legend:{}:{}", o.id.len(), o.id);
    let band = |i: usize, left: f64, right: f64, color: Color| {
        let (x, y, width, height) = if horizontal {
            (
                o.position[0] + left * o.length,
                o.position[1],
                (right - left) * o.length,
                o.thickness,
            )
        } else {
            (
                o.position[0],
                o.position[1] + (1. - right) * o.length,
                o.thickness,
                (right - left) * o.length,
            )
        };
        let mut mark = Mark::new(
            format!("{prefix}:band:{i}"),
            Geometry::Rect {
                x,
                y,
                width,
                height,
            },
            color,
        );
        mark.label.clear();
        mark
    };
    if scale.kind().is_discrete() {
        for (i, pair) in stops.windows(2).enumerate() {
            let [left, right] = [pair[0], pair[1]];
            if right.offset > left.offset {
                candidate
                    .marks
                    .push(band(i, left.offset, right.offset, left.color));
            }
        }
    } else {
        // A single painted rectangle avoids AA seams and double opacity at
        // fractional sample boundaries in both vector and raster targets.
        let first = stops[0].color;
        let mut mark = band(0, 0., 1., first);
        if stops.iter().any(|stop| stop.color != first) {
            mark.fill_paint = Some(FillPaint::Linear {
                start: point(0.),
                end: point(1.),
                stops: stops
                    .iter()
                    .map(|stop| GradientStop::new(stop.offset, stop.color))
                    .collect(),
            });
        }
        candidate.marks.push(mark);
    }
    let mut ticks = Vec::with_capacity(values.len());
    for (i, value) in values.into_iter().enumerate() {
        let offset = scale.position(value);
        ensure(
            offset.is_some(),
            "color legend tick position is not representable",
        )?;
        let position = point(offset.unwrap());
        let text = o.format.format(value)?;
        if o.tick_size > 0. {
            let end = if horizontal {
                [position[0], position[1] + o.tick_size]
            } else {
                [position[0] + o.tick_size, position[1]]
            };
            let mut tick = Mark::new(
                format!("{prefix}:tick:{i}"),
                Geometry::Path(vec![
                    C::Move(P::new(position[0], position[1])),
                    C::Line(P::new(end[0], end[1])),
                ]),
                o.text_color,
            );
            tick.label.clear();
            tick.fill = None;
            tick.stroke = Some(GraphicStroke {
                color: o.text_color,
                width: 1.,
                cap: StrokeCap::Butt,
            });
            candidate.marks.push(tick);
        }
        candidate.labels.push(Label {
            x: if horizontal {
                position[0]
            } else {
                position[0] + o.tick_size + o.label_offset
            },
            y: if horizontal {
                position[1] + o.tick_size + o.label_offset + o.text_size
            } else {
                position[1] + o.text_size * 0.35
            },
            text: text.clone(),
            color: o.text_color,
            size: o.text_size,
            anchor: if horizontal {
                TextAnchor::Middle
            } else {
                TextAnchor::Start
            },
            rotation: 0.,
        });
        ticks.push(ColorLegendTick {
            value,
            color: scale.map(Some(value)),
            position,
            label: text,
        });
    }
    if !o.title.is_empty() {
        candidate.labels.push(Label {
            x: o.position[0],
            y: o.position[1] - o.label_offset,
            text: o.title.clone(),
            color: o.text_color,
            size: o.text_size,
            anchor: TextAnchor::Start,
            rotation: 0.,
        });
    }
    let unknown = if let Some(label) = &o.unknown_label {
        let rect = ClipRect {
            x: o.position[0] + if horizontal { o.length + 16. } else { 0. },
            y: o.position[1] + if horizontal { 0. } else { o.length + 16. },
            width: o.thickness,
            height: o.thickness,
        };
        let mut mark = Mark::new(
            format!("{prefix}:unknown"),
            Geometry::Rect {
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height,
            },
            scale.unknown(),
        );
        mark.label.clear();
        candidate.marks.push(mark);
        candidate.labels.push(Label {
            x: rect.x + rect.width + 6.,
            y: rect.y + rect.height / 2. + o.text_size * 0.35,
            text: label.clone(),
            color: o.text_color,
            size: o.text_size,
            anchor: TextAnchor::Start,
            rotation: 0.,
        });
        Some(rect)
    } else {
        None
    };
    if !candidate.draw_order.is_empty() {
        candidate
            .draw_order
            .extend((old_marks..candidate.marks.len()).map(DrawItem::Mark));
        candidate
            .draw_order
            .extend((old_labels..candidate.labels.len()).map(DrawItem::Label));
    }
    candidate.validate()?;
    *scene = candidate;
    Ok(ColorLegendLayout {
        stops,
        ticks,
        unknown,
    })
}
