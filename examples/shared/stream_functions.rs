//! All eight original Stream function roles with explicit Rust ownership.
use std::cell::RefCell;

use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::stack::{StackOffset, StackOptions};
use kartis::*;
pub struct Source {
    pub layout: StreamLayout,
    pub log: Vec<String>,
    pub raw: Vec<BarSeries>,
    pub categories: Vec<CategoryKey>,
}
pub fn config(width: f64) -> ChartConfig {
    ChartConfig {
        width,
        height: 320.,
        margin: Margin {
            left: 40.,
            right: 40.,
            top: 30.,
            bottom: 30.,
        },
        title: "Stream function configuration".into(),
        ..Default::default()
    }
}
pub fn source(phase: u8, width: f64, failure: u8) -> Result<Source> {
    let c = config(width);
    let mut categories = vec![
        CategoryKey::from("first"),
        CategoryKey::from(-0.),
        CategoryKey::from("last"),
    ];
    let mut raw = vec![
        BarSeries::new(
            "a",
            vec![
                Some(30.),
                Some(if phase == 1 { 15. } else { 10. }),
                Some(20.),
            ],
        ),
        BarSeries::new(
            "b",
            vec![Some(10.), None, Some(if phase == 1 { 25. } else { 20. })],
        ),
    ];
    if phase == 2 {
        raw.reverse();
    }
    if phase == 9 {
        raw.clear();
        categories.clear();
    }
    if phase == 10 {
        for s in &mut raw {
            s.values[2] = None;
        }
    }
    let mut o = StreamLayoutOptions {
        stack: StackOptions {
            offset: StackOffset::Zero,
            ..Default::default()
        },
        curve: Curve::Linear,
        x_scale: PointScaleOptions {
            padding: 0.5,
            ..Default::default()
        },
        y_bounds: [Some(0.), Some(60.)],
        y_scale: ContinuousScaleOptions {
            nice: ScaleNice::Off,
            ..Default::default()
        },
        style: StreamLayerStyle {
            border_width: if phase == 7 { 0. } else { 6. },
            fill_opacity: if phase == 12 { 0. } else { 1. },
            ..Default::default()
        },
        dots: (phase != 4).then(|| StreamDotOptions {
            size: 6.,
            glyph: (phase == 11).then(|| GlyphOptions {
                shape: GlyphShape::Filled(FilledGlyph::Square),
                size: 256.,
                ..Default::default()
            }),
            ..Default::default()
        }),
        legend: true,
        ..Default::default()
    };
    o.axes.x.visible = false;
    o.axes.x.grid = false;
    o.axes.y.visible = false;
    o.axes.y.grid = false;
    if phase == 3 {
        o.hidden.insert("b".into());
    }
    let log = RefCell::new(Vec::<String>::new());
    let mut label = |ctx: &StreamLayerPathContext<'_>| {
        log.borrow_mut()
            .push(format!("label:{}:{}", ctx.id, ctx.path.is_some()));
        if failure == 1 && ctx.id == "b" {
            return Err(ChartError("Stream label rejected".into()));
        }
        Ok(if phase == 6 && ctx.id == "b" {
            String::new()
        } else {
            format!("layer-{}", ctx.id)
        })
    };
    let mut color = |ctx: &StreamLayerPathContext<'_>| {
        log.borrow_mut()
            .push(format!("color:{}:{}", ctx.id, ctx.series.values.len()));
        if failure == 2 && ctx.id == "b" {
            return Err(ChartError("Stream color rejected".into()));
        }
        Ok(if ctx.id == "a" {
            Color(40, 120, 90, 255)
        } else {
            Color(30, 120, 150, 255)
        })
    };
    let mut format = |v: f64| {
        log.borrow_mut().push(format!("format:{}", v.to_bits()));
        if failure == 3 && v == 20. {
            return Err(ChartError("Stream formatter rejected".into()));
        }
        Ok(format!("{v:.0} units"))
    };
    let mut border = |ctx: &StreamLayerStyleContext<'_>| {
        log.borrow_mut().push(format!(
            "border:{}:{}:{}",
            ctx.layer.id,
            ctx.layer.samples.len(),
            ctx.layer.label
        ));
        if failure == 4 && ctx.layer.id == "b" {
            return Err(ChartError("Stream border rejected".into()));
        }
        Ok(Color(230, 110, 20, 255))
    };
    let mut size = |ctx: &StreamDotStyleContext<'_>| {
        log.borrow_mut().push(format!(
            "size:{}:{}:{}",
            ctx.layer.id,
            ctx.sample.source_index,
            ctx.layer.samples.len()
        ));
        if failure == 5 {
            return Ok(f64::NAN);
        }
        Ok(if phase == 5 {
            0.
        } else {
            16. + 4. * ctx.sample.source_index as f64
        })
    };
    let mut dot_color = |ctx: &StreamDotStyleContext<'_>| {
        log.borrow_mut().push(format!(
            "dot-color:{}:{}",
            ctx.layer.id, ctx.sample.source_index
        ));
        if failure == 6 {
            return Err(ChartError("Stream dot color rejected".into()));
        }
        Ok(if ctx.layer.id == "a" {
            Color(0, 100, 255, 255)
        } else {
            Color(0, 200, 0, 255)
        })
    };
    let mut width_fn = |ctx: &StreamDotStyleContext<'_>| {
        log.borrow_mut().push(format!(
            "dot-width:{}:{}",
            ctx.layer.id, ctx.sample.source_index
        ));
        Ok(if failure == 7 {
            -1.
        } else if phase == 5 || phase == 7 {
            0.
        } else {
            2.
        })
    };
    let mut dot_border = |ctx: &StreamDotStyleContext<'_>| {
        log.borrow_mut().push(format!(
            "dot-border:{}:{}",
            ctx.layer.id, ctx.sample.source_index
        ));
        if failure == 8 {
            return Err(ChartError("Stream dot border rejected".into()));
        }
        Ok(Color(100, 20, 180, 255))
    };
    let mut factories = StreamFactories {
        label: Some(&mut label),
        color: Some(&mut color),
        formatted_value: Some(&mut format),
        border_color: Some(&mut border),
        dot_size: if phase == 11 { None } else { Some(&mut size) },
        dot_color: Some(&mut dot_color),
        dot_border_width: Some(&mut width_fn),
        dot_border_color: Some(&mut dot_border),
    };
    if failure == 9 {
        o.dots.as_mut().unwrap().glyph = Some(GlyphOptions {
            shape: GlyphShape::Filled(FilledGlyph::Square),
            size: 144.,
            ..Default::default()
        });
    }
    let layout = if phase == 8 {
        stream_layout_with_factories_and_dot_factory(
            &c,
            &categories,
            &raw,
            &o,
            &mut factories,
            |ctx| {
                log.borrow_mut().push(format!(
                    "geometry:{}:{}",
                    ctx.series.id, ctx.sample.source_index
                ));
                if failure == 10 {
                    return Err(ChartError("Stream geometry rejected".into()));
                }
                let [x, y] = ctx.position;
                Ok(ctx.inherit(Geometry::Path(vec![
                    C::Move(P::new(x + 18., y - 7.)),
                    C::Line(P::new(x + 32., y - 7.)),
                    C::Line(P::new(x + 32., y + 7.)),
                    C::Line(P::new(x + 18., y + 7.)),
                    C::Close,
                ])))
            },
        )?
    } else {
        stream_layout_with_factories(&c, &categories, &raw, &o, &mut factories)?
    };
    Ok(Source {
        layout,
        log: log.into_inner(),
        raw,
        categories,
    })
}
