//! Typed custom-layer roles and real Nivo-style function configuration.
use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::*;
pub struct Source {
    pub frame: SceneInteraction,
    pub composition: PlotWidgetsComposition<FunnelWidgetData>,
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
        title: "Funnel functions and live layer roles".into(),
        ..Default::default()
    }
}
pub fn data(phase: u8) -> Vec<FunnelDatum> {
    let mut data = vec![
        FunnelDatum::new("a", 80.),
        FunnelDatum::new(
            if phase == 1 { 0. } else { -0. },
            if phase == 1 { 0. } else { -0. },
        ),
        FunnelDatum::new("c", 30.),
        FunnelDatum::new("d", 10.),
    ];
    data[0].color = Some(Color(99, 99, 99, 255));
    data[1].size = Some(if phase == 1 { 0. } else { -0. });
    for (d, label) in data.iter_mut().zip(["Alpha", "Zero", "Charlie", "Delta"]) {
        d.label = label.into();
    }
    if phase == 2 {
        data.reverse();
    }
    if phase == 7 {
        for d in &mut data {
            d.size = Some(0.);
        }
    }
    if phase == 9 {
        data.clear();
    }
    data
}
fn is(d: Option<&FunnelDatum>, name: &str) -> bool {
    d.is_some_and(|d| matches!(&d.id,CategoryKey::String(id) if id==name))
}
pub fn source(phase: u8, width: f64, revision: u64, profile: u8, failure: u8) -> Result<Source> {
    let c = config(width);
    let data = data(phase);
    let horizontal = phase == 4;
    let o = FunnelLayoutOptions {
        direction: if horizontal {
            FunnelDirection::Horizontal
        } else {
            FunnelDirection::Vertical
        },
        interpolation: if phase == 6 {
            FunnelInterpolation::Smooth
        } else {
            FunnelInterpolation::Linear
        },
        spacing: 8.,
        shape_blending: 0.66,
        border_width: 4.,
        border_opacity: 1.,
        current: (phase == 5).then(|| "a".into()),
        current_size_extension: 8.,
        current_border_width: Some(8.),
        size: if phase == 7 {
            FunnelSize::DatumPixels
        } else {
            FunnelSize::Value
        },
        labels: if phase == 8 {
            None
        } else {
            Some(FunnelLabelOptions {
                size: Some(20.),
                ..Default::default()
            })
        },
        before: Some(FunnelSeparatorOptions {
            length: 80.,
            offset: 10.,
            stroke: None,
        }),
        after: Some(FunnelSeparatorOptions {
            length: 80.,
            offset: 10.,
            stroke: None,
        }),
        legend: false,
        ..Default::default()
    };
    let mut color = |ctx: &FunnelColorContext<'_>| {
        if failure == 7 && ctx.source_index == 2 {
            return Err(ChartError("Funnel color rejected".into()));
        }
        Ok(match &ctx.datum.id {
            CategoryKey::String(id) if id == "a" => Color(40, 120, 90, 255),
            CategoryKey::String(id) if id == "c" => Color(120, 90, 40, 255),
            CategoryKey::String(_) => Color(30, 120, 150, 255),
            _ => Color(70, 100, 180, 255),
        })
    };
    let mut format = |_: f64| {
        if failure == 8 {
            Err(ChartError("Funnel formatter rejected".into()))
        } else {
            Ok("MMMM".into())
        }
    };
    let mut border = |_: &FunnelPartStyleContext<'_>| Ok(Color(230, 110, 20, 255));
    let mut label = |ctx: &FunnelPartStyleContext<'_>| {
        if failure == 9 && ctx.part.source_index == 2 {
            Err(ChartError("Funnel label color rejected".into()))
        } else {
            Ok(Color(100, 20, 180, 255))
        }
    };
    let mut factories = FunnelFactories {
        color: Some(&mut color),
        formatted_value: Some(&mut format),
        border_color: Some(&mut border),
        label_color: Some(&mut label),
    };
    let layout = if phase == 3 {
        funnel_layout_with_factories_and_stage_factory(&c, &data, &o, &mut factories, &mut |ctx| {
            if matches!(&ctx.datum.id,CategoryKey::String(id) if id=="a") {
                let [x, y] = ctx.part.center;
                let mut shape = ctx.inherit(Geometry::Path(vec![
                    C::Move(P::new(x + 45., y - 8.)),
                    C::Line(P::new(x + 65., y - 8.)),
                    C::Line(P::new(x + 65., y + 8.)),
                    C::Line(P::new(x + 45., y + 8.)),
                    C::Close,
                ]));
                shape.body.fill = Some(Color(40, 120, 90, 255));
                shape.body.fill_paint = None;
                Ok(shape)
            } else {
                Ok(ctx.inherit(ctx.geometry.clone()))
            }
        })?
    } else {
        funnel_layout_with_factories(&c, &data, &o, &mut factories)?
    };
    let widgets = funnel_widgets(&layout, &data, revision, |ctx| {
        if failure == 10 && ctx.role == FunnelWidgetRole::AfterSeparator {
            return Err(ChartError("Funnel placement rejected".into()));
        }
        let offset = match ctx.role {
            FunnelWidgetRole::Body if is(ctx.datum, "a") && phase != 10 => {
                if horizontal {
                    [-16., 59.]
                } else {
                    [84., -11.]
                }
            }
            FunnelWidgetRole::Border
                if ctx
                    .datum
                    .is_some_and(|d| matches!(d.id,CategoryKey::Number(n) if n==0.)) =>
            {
                if horizontal {
                    [-16., -91.]
                } else {
                    [ctx.plot.x + 70. - ctx.position[0] - 16., -11.]
                }
            }
            FunnelWidgetRole::BeforeSeparator if is(ctx.datum, "c") => [-16., -11.],
            FunnelWidgetRole::AfterSeparator if ctx.part.is_none() => {
                if horizontal {
                    [-38., -11.]
                } else {
                    [-16., -28.]
                }
            }
            _ => return Ok(None),
        };
        Ok(Some(PlotWidgetPlacement::AtDatum {
            offset,
            measured: TextSize {
                width: 32.,
                height: 22.,
            },
        }))
    })?;
    let replace_ink = if profile.is_multiple_of(2) {
        widgets
            .layout
            .slots
            .iter()
            .map(|s| s.mark_id.clone())
            .collect()
    } else {
        Default::default()
    };
    Ok(Source {
        frame: funnel_widget_frame(&layout)?,
        composition: PlotWidgetsComposition {
            widgets,
            replace_ink,
        },
    })
}
pub fn color(data: &FunnelWidgetData) -> Color {
    if data
        .source
        .iter()
        .find(|d| matches!(d.id,CategoryKey::Number(n) if n==0.))
        .is_some_and(|d| d.value.to_bits() == 0)
    {
        Color(0, 200, 0, 255)
    } else {
        Color(0, 100, 255, 255)
    }
}
