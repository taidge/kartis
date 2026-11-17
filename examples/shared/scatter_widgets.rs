//! Original Scatter series, mixed XY identities and actual node component roles.
use std::collections::BTreeSet;

use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::*;

#[derive(Clone, Debug)]
pub struct Source {
    pub frame: SceneInteraction,
    pub composition: PlotWidgetsComposition<ScatterNodeWidgetData>,
}

/// 0 base/missing size; 1 missing becomes zero; 2 hidden south; 3 offset Paths;
/// 4 zero sizes; 5 one transparent custom node; 6 skip a component;
/// 7 explicit point domain omits two raw X keys.
pub fn source(phase: u8, width: f64, revision: u64) -> Result<Source> {
    let config = ChartConfig {
        width,
        height: 260.,
        margin: Margin {
            left: 30.,
            right: 30.,
            top: 24.,
            bottom: 24.,
        },
        title: "Scatter live node Widgets".into(),
        ..Default::default()
    };
    let datum = |id: &str, x: CategoryKey, y: f64, size: Option<f64>| {
        let mut d = ScatterDatum::new(id, x, y);
        d.size_value = size;
        d.label = Some(format!("Original {id}"));
        d
    };
    let mut north = ScatterSeries::new(
        "north",
        vec![
            datum("a", 1.0.into(), if phase == 1 { 3.5 } else { 2. }, Some(1.)),
            datum("b", "1".into(), 0., Some(0.)),
            datum(
                "c",
                "middle".into(),
                3.,
                if phase == 1 { Some(0.) } else { None },
            ),
            datum("d", "tail".into(), -2., Some(2.)),
        ],
    );
    north.label = "Original north".into();
    north.color = Some(Color(0, 100, 255, 255));
    let mut south = ScatterSeries::new(
        "south",
        vec![
            datum("a", 1.0.into(), 5., Some(2.)),
            datum("b", "1".into(), 4., Some(1.)),
            datum("c", "middle".into(), -1., Some(1.)),
            datum("d", "tail".into(), 7., Some(1.)),
        ],
    );
    south.label = "Original south".into();
    south.color = Some(Color(255, 180, 0, 255));
    let series = [north, south];
    let options = ScatterLayoutOptions {
        axes: ScatterAxes {
            top: None,
            right: None,
            bottom: None,
            left: None,
        },
        scales: ScatterScaleOptions {
            x: ScatterAxisScale::Point {
                options: PointScaleOptions {
                    padding: 0.6,
                    ..Default::default()
                },
                domain: Some(if phase == 7 {
                    vec![1.0.into(), "1".into()]
                } else {
                    vec![1.0.into(), "1".into(), "middle".into(), "tail".into()]
                }),
                window: None,
            },
            y: ScatterAxisScale::Continuous {
                options: ContinuousScaleOptions {
                    nice: ScaleNice::Off,
                    ..Default::default()
                },
                min: ScatterBound::Fixed(-5.),
                max: ScatterBound::Fixed(8.),
            },
        },
        node: ScatterNodeStyle {
            shape: ScatterNodeShape::Circle {
                diameter: ScatterSize::Linear {
                    values: [0., 2.],
                    sizes: if phase == 4 { [0., 0.] } else { [0., 18.] },
                    clamp: true,
                    missing: ScatterMissingSize::Skip,
                },
            },
            ..Default::default()
        },
        hidden: if phase == 2 {
            BTreeSet::from(["south".into()])
        } else {
            BTreeSet::new()
        },
        x_format: ValueFormat::Fixed(1),
        y_format: ValueFormat::Fixed(1),
        ..Default::default()
    };
    let layout = if matches!(phase, 3 | 5) {
        scatter_layout_with_node_factory(&config, &series, &options, |c| {
            let [x, y] = c.position;
            let mut geometry = ScatterNodeGeometry::inherit(
                Geometry::Path(vec![
                    C::Move(P::new(x + 10., y - 5.)),
                    C::Line(P::new(x + 22., y - 5.)),
                    C::Line(P::new(x + 22., y + 5.)),
                    C::Line(P::new(x + 10., y + 5.)),
                    C::Close,
                ]),
                c,
            );
            if phase == 5 && c.series_index == 0 && c.source_index == 1 {
                geometry.opacity = 0.;
            }
            Ok(geometry)
        })?
    } else {
        scatter_layout(&config, &series, &options)?
    };
    let widgets = scatter_node_widgets(&layout, &series, revision, |c| {
        if phase == 6 && c.site.series_index == 0 && c.site.source_index == 1 {
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
    let mut frame: SceneInteraction = layout.into();
    frame.interaction.as_mut().unwrap().picking_profile = PickingProfile::PaintedGeometry;
    surface::SurfaceModel::new_with_interaction(
        frame.scene.clone(),
        surface::SurfaceOptions {
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
