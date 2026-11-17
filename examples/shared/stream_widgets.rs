//! Original nullable series and actual Stream dot roles for all live hosts.
use std::collections::BTreeSet;

use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::stack::{StackOffset, StackOptions, StackOrder};
use kartis::*;

#[derive(Clone, Debug)]
pub struct Source {
    pub frame: SceneInteraction,
    pub composition: PlotWidgetsComposition<StreamDotWidgetData>,
}

/// 0: nullable/signed; 1: None becomes zero and values update; 2: hidden layer;
/// 3: offset Path ink; 4: zero size; 5: zero-opacity actual dot; 6: skip one
/// selected role; 7: disabled dots (slice placeholders only).
pub fn source(phase: u8, width: f64, revision: u64) -> Result<Source> {
    let config = ChartConfig {
        width,
        height: 240.,
        margin: Margin {
            left: 24.,
            right: 24.,
            top: 24.,
            bottom: 24.,
        },
        title: "Stream live dot Widgets".into(),
        ..Default::default()
    };
    let categories = [
        CategoryKey::from(1.),
        CategoryKey::from("1"),
        CategoryKey::from("middle"),
        CategoryKey::from("tail"),
    ];
    let series = [
        BarSeries::new(
            "north",
            vec![
                if phase == 1 { Some(0.) } else { None },
                Some(0.),
                Some(if phase == 1 { 5. } else { 3. }),
                Some(-2.),
            ],
        ),
        BarSeries::new("south", vec![Some(2.), Some(1.), Some(-1.), Some(4.)]),
    ];
    let mut options = StreamLayoutOptions {
        stack: StackOptions {
            order: StackOrder::Reverse,
            offset: StackOffset::Diverging,
        },
        curve: Curve::Linear,
        y_bounds: [Some(-5.), Some(8.)],
        style: StreamLayerStyle {
            fill_opacity: 0.15,
            ..Default::default()
        },
        dots: (phase != 7).then(|| StreamDotOptions {
            size: if phase == 4 { 0. } else { 12. },
            ..Default::default()
        }),
        value_format: ValueFormat::Fixed(1),
        hidden: if phase == 2 {
            BTreeSet::from(["south".into()])
        } else {
            BTreeSet::new()
        },
        ..Default::default()
    };
    options
        .labels
        .insert("north".into(), "Original north".into());
    options
        .labels
        .insert("south".into(), "Original south".into());
    options.axes.x.visible = false;
    options.axes.x.grid = false;
    options.axes.y.visible = false;
    options.axes.y.grid = false;
    let layout = if matches!(phase, 3 | 5) {
        stream_layout_with_dot_factory(&config, &categories, &series, &options, |c| {
            let [x, y] = c.position;
            let mut geometry = c.inherit(Geometry::Path(vec![
                C::Move(P::new(x + 10., y - 5.)),
                C::Line(P::new(x + 22., y - 5.)),
                C::Line(P::new(x + 22., y + 5.)),
                C::Line(P::new(x + 10., y + 5.)),
                C::Close,
            ]));
            if phase == 5 && c.series_index == 0 && c.sample.source_index == 2 {
                geometry.opacity = 0.;
            }
            Ok(geometry)
        })?
    } else {
        stream_layout(&config, &categories, &series, &options)?
    };
    let widgets = stream_dot_widgets(&layout, revision, |c| {
        if phase == 6 && c.series_index == 0 && c.sample.source_index == 1 {
            return Ok(None);
        }
        Ok(Some(PlotWidgetPlacement::AtDatum {
            offset: [-18., -12.],
            measured: TextSize {
                width: 36.,
                height: 24.,
            },
        }))
    })?;
    let replace_ink = widgets
        .layout
        .slots
        .iter()
        .map(|s| s.mark_id.clone())
        .collect();
    let mut frame = layout.into_slice_interaction()?;
    frame.interaction.as_mut().unwrap().picking_profile = PickingProfile::PaintedGeometry;
    kartis::surface::SurfaceModel::new_with_interaction(
        frame.scene.clone(),
        kartis::surface::SurfaceOptions {
            plot: frame.plot,
            ..Default::default()
        },
        frame.interaction.clone(),
    )?;
    Ok(Source {
        frame,
        composition: PlotWidgetsComposition {
            widgets,
            replace_ink,
        },
    })
}
