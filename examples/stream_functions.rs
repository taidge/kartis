//! A complete Glory application using Stream's eight borrowed functions.
use std::cell::Cell;

use glory::App;
use kartis::stack::{StackOffset, StackOptions};
use kartis::*;
fn source(value: f64) -> Result<SceneInteraction> {
    let categories = [CategoryKey::from("first"), CategoryKey::from("last")];
    let series = [
        BarSeries::new("sales", vec![Some(10.), Some(value)]),
        BarSeries::new("returns", vec![Some(2.), None]),
    ];
    let options = StreamLayoutOptions {
        stack: StackOptions {
            offset: StackOffset::Zero,
            ..Default::default()
        },
        dots: Some(StreamDotOptions::default()),
        legend: true,
        ..Default::default()
    };
    let formats = Cell::new(0);
    let mut label =
        |ctx: &StreamLayerPathContext<'_>| Ok(format!("{} ({})", ctx.id, ctx.series.values.len()));
    let mut color = |ctx: &StreamLayerPathContext<'_>| {
        Ok(if ctx.id == "sales" {
            Color(30, 140, 90, 255)
        } else {
            Color(190, 80, 40, 255)
        })
    };
    let mut format = |v: f64| {
        formats.set(formats.get() + 1);
        Ok(format!("{v:.0} tickets"))
    };
    let mut border = |ctx: &StreamLayerStyleContext<'_>| Ok(ctx.layer.color);
    let mut size = |ctx: &StreamDotStyleContext<'_>| Ok(8. + ctx.sample.source_index as f64 * 2.);
    let mut dot_color = |ctx: &StreamDotStyleContext<'_>| Ok(ctx.layer.color);
    let mut width = |_: &StreamDotStyleContext<'_>| Ok(2.);
    let mut dot_border = |_: &StreamDotStyleContext<'_>| Ok(Color(255, 255, 255, 255));
    let layout = stream_layout_with_factories(
        &ChartConfig::default(),
        &categories,
        &series,
        &options,
        &mut StreamFactories {
            label: Some(&mut label),
            color: Some(&mut color),
            formatted_value: Some(&mut format),
            border_color: Some(&mut border),
            dot_size: Some(&mut size),
            dot_color: Some(&mut dot_color),
            dot_border_width: Some(&mut width),
            dot_border_color: Some(&mut dot_border),
        },
    )?;
    assert_eq!(formats.get(), 3); // Three Some values; area summaries reuse these strings.
    assert_eq!(layout.layers[1].samples[1].formatted_value, None);
    assert_eq!(
        layout.layers[0].samples[1].formatted_value,
        Some(format!("{value:.0} tickets"))
    );
    let mut frame = layout.into_scene_interaction()?;
    // Bound Glory charts explicitly pick the actual painted geometry.
    frame.interaction.as_mut().unwrap().picking_profile = PickingProfile::PaintedGeometry;
    kartis::surface::SurfaceModel::new_with_interaction(
        frame.scene.clone(),
        kartis::surface::SurfaceOptions {
            plot: frame.plot,
            ..Default::default()
        },
        frame.interaction.clone(),
    )?;
    Ok(frame)
}
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let app = App::memory();
    let input = app.scope().cage(source(20.)?);
    let frame = app.scope().bond(move || input.get());
    app.mount(chart_layout(frame).on_reject(|e| eprintln!("Stream example rejected: {e:?}")))?;
    app.flush()?;
    input.revise(|old| *old = source(30.).unwrap());
    app.flush()?;
    assert!(input.with(|s| {
        s.interaction
            .as_ref()
            .unwrap()
            .sites
            .iter()
            .any(|s| s.formatted_value.as_deref() == Some("30 tickets"))
    }));
    app.clear()?;
    Ok(())
}
