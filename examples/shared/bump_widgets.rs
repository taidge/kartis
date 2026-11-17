//! Bump point roles with typed categories, gaps and original owned series.
use std::collections::BTreeSet;

use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::*;
#[derive(Clone, Debug)]
pub struct Source {
    pub frame: SceneInteraction,
    pub composition: PlotWidgetsComposition<BumpPointWidgetData>,
}
/// 0 base; 1 gap becomes rank0 and rank update; 2 hidden south; 3 offset paths;
/// 4 all zero sizes; 5 transparent custom point; 6 omitted component;
/// 7 reversed scales; 8 retained source lines and endpoint labels.
pub fn source(phase: u8, width: f64, revision: u64) -> Result<Source> {
    let config = ChartConfig {
        width,
        height: 260.,
        margin: Margin {
            left: 40.,
            right: 40.,
            top: 30.,
            bottom: 30.,
        },
        title: "Bump live point Widgets".into(),
        ..Default::default()
    };
    let north = BumpSeries {
        id: "north".into(),
        label: "Original north".into(),
        data: vec![
            BumpDatum::new(1_f64, if phase == 1 { 3.5 } else { 2. }),
            BumpDatum::new("1", -0.),
            if phase == 1 {
                BumpDatum::new("middle", 0.)
            } else {
                BumpDatum::gap("middle")
            },
            BumpDatum::new("tail", -2.),
        ],
    };
    let south = BumpSeries {
        id: "south".into(),
        label: "Original south".into(),
        data: vec![
            BumpDatum::new(1_f64, 5.),
            BumpDatum::new("1", 4.),
            BumpDatum::new("middle", -1.),
            BumpDatum::new("tail", 7.),
        ],
    };
    let options = BumpLayoutOptions {
        axes: BumpAxes {
            top: None,
            right: None,
            bottom: None,
            left: None,
        },
        x_scale: PointScaleOptions {
            padding: 0.6,
            reverse: phase == 7,
            ..Default::default()
        },
        rank_scale: PointScaleOptions {
            padding: 0.6,
            reverse: phase == 7,
            ..Default::default()
        },
        style: BumpSeriesStyle {
            width: if phase == 8 { 2. } else { 0. },
            point: BumpPointStyle {
                size: if phase == 4 { 0. } else { 18. },
                ..Default::default()
            },
            ..Default::default()
        },
        series_styles: [
            (
                "north".into(),
                BumpSeriesStyle {
                    color: Some(Color(0, 100, 255, 255)),
                    width: if phase == 8 { 2. } else { 0. },
                    point: BumpPointStyle {
                        size: if phase == 4 { 0. } else { 18. },
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ),
            (
                "south".into(),
                BumpSeriesStyle {
                    color: Some(Color(255, 180, 0, 255)),
                    width: if phase == 8 { 2. } else { 0. },
                    point: BumpPointStyle {
                        size: if phase == 4 { 0. } else { 18. },
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ),
        ]
        .into(),
        hidden: if phase == 2 {
            BTreeSet::from(["south".into()])
        } else {
            BTreeSet::new()
        },
        labels: BumpEndpointLabels {
            start: phase == 8,
            end: phase == 8,
            padding: 4.,
            ..Default::default()
        },
        rank_format: ValueFormat::Fixed(1),
        legend: false,
        ..Default::default()
    };
    let layout = bump_layout_with_point_factory(&config, &[north, south], &options, |c| {
        let [x, y] = c.position;
        let geometry = if matches!(phase, 3 | 5) {
            Geometry::Path(vec![
                C::Move(P::new(x + 10., y - 5.)),
                C::Line(P::new(x + 22., y - 5.)),
                C::Line(P::new(x + 22., y + 5.)),
                C::Line(P::new(x + 10., y + 5.)),
                C::Close,
            ])
        } else {
            Geometry::Circle {
                x,
                y,
                radius: if phase == 4 || c.series_index == 0 && c.source_index == 1 {
                    0.
                } else {
                    9.
                },
            }
        };
        let mut point = c.inherit(geometry);
        if phase == 5 && c.series_index == 0 && c.source_index == 1 {
            point.opacity = 0.;
        }
        Ok(point)
    })?;
    let widgets = bump_point_widgets(&layout, revision, |c| {
        if phase == 6 && c.computed.source_index == 0 && c.point.source_index == 1 {
            return Ok(None);
        }
        Ok(Some(PlotWidgetPlacement::AtDatum {
            offset: [-16., -11.],
            measured: TextSize {
                width: 32.,
                height: 22.,
            },
        }))
    })?;
    let replace_ink = widgets
        .layout
        .slots
        .iter()
        .map(|s| s.mark_id.clone())
        .collect();
    let frame = bump_widget_frame(&layout)?;
    Ok(Source {
        frame,
        composition: PlotWidgetsComposition {
            widgets,
            replace_ink,
        },
    })
}
