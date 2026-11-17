//! Complete original Bar series and real bar component roles for every host.
use std::collections::{BTreeMap, BTreeSet};

use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::*;

#[derive(Clone, Debug)]
pub struct Source {
    pub frame: SceneInteraction,
    pub composition: PlotWidgetsComposition<BarWidgetData>,
}

/// 0 grouped; 1 None becomes zero/value update; 2 hidden south; 3 offset Paths;
/// 4 horizontal; 5 one transparent custom bar; 6 skip a component; 7 stacked;
/// 8 percent; 9 category crop; 10 configured SymLog with value cropping; 11 controlled dim-to-zero;
/// 12 label factory; 13 empty/zero labels; 14 dimmed labels.
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
        title: "Bar live component roles".into(),
        ..Default::default()
    };
    let categories = ["A", "B", "C", "D"].map(str::to_string);
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
    let mut options = BarOptions {
        horizontal: phase == 4,
        layout: match phase {
            7 => BarLayout::Stacked,
            8 => BarLayout::Percent,
            _ => BarLayout::Grouped,
        },
        hidden: if phase == 2 {
            BTreeSet::from(["south".into()])
        } else {
            BTreeSet::new()
        },
        category_window: (phase == 9).then_some([0.2, 0.8]),
        value_domain: if phase == 8 {
            None
        } else if phase == 10 {
            Some([-1., 2.])
        } else {
            Some([-5., 8.])
        },
        ..Default::default()
    };
    options.value_axis.visible = false;
    options.value_axis.grid = false;
    options.category_axis.visible = false;
    options.category_axis.grid = false;
    let mut style = BarStyleOptions {
        colors: BTreeMap::from([
            ("north".into(), Color(0, 100, 255, 255)),
            ("south".into(), Color(255, 180, 0, 255)),
        ]),
        tooltip_format: Some(ValueFormat::Fixed(1)),
        legend: false,
        ..Default::default()
    };
    style.labels.enabled = phase >= 12;
    if phase == 13 {
        style.labels.zero_extent = BarZeroExtentLabels::ValueAxis;
    }
    let mut label_factory = |c: &BarLabelContext<'_>| {
        Ok(if phase == 13 {
            String::new()
        } else {
            format!("Label {} / {}: {:.1}", c.category, c.series.id, c.raw_value)
        })
    };
    let mut factory = |c: &BarDatumContext<'_>| {
        let [x, y] = c.position;
        let mut geometry = c.inherit(Geometry::Path(vec![
            C::Move(P::new(x + 10., y - 5.)),
            C::Line(P::new(x + 22., y - 5.)),
            C::Line(P::new(x + 22., y + 5.)),
            C::Line(P::new(x + 10., y + 5.)),
            C::Close,
        ]));
        if phase == 5 && c.source_series == 0 && c.source_category == 1 {
            geometry.opacity = 0.;
        }
        Ok(geometry)
    };
    let factories = BarFactories {
        bar: matches!(phase, 3 | 5).then_some(&mut factory as &mut BarGeometryFactory<'_>),
        label: (phase >= 12).then_some(&mut label_factory as &mut BarLabelFactory<'_>),
    };
    let mut layout = if phase == 10 {
        bar_layout_with_scale_options_and_factories(
            &config,
            &categories,
            &series,
            &options,
            &style,
            &ContinuousScaleOptions {
                kind: ScaleKind::SymLog { constant: 1. },
                nice: ScaleNice::Off,
                ..Default::default()
            },
            factories,
        )?
        .layout
    } else {
        bar_layout_with_factories(&config, &categories, &series, &options, &style, factories)?
    };
    if matches!(phase, 11 | 14) {
        let active = layout
            .layout
            .bars
            .iter()
            .find(|b| b.source_series == 0 && b.source_category == 2)
            .unwrap()
            .id
            .clone();
        layout.set_active(Some(&active), 0.)?;
    }
    let widgets = bar_widgets(&layout, revision, |c| {
        if phase == 6 && c.bar.source_series == 0 && c.bar.source_category == 1 {
            return Ok(None);
        }
        Ok(Some(PlotWidgetPlacement::AtDatum {
            offset: [-12., -5.],
            measured: TextSize {
                width: 24.,
                height: 10.,
            },
        }))
    })?;
    let replace_ink = widgets
        .layout
        .slots
        .iter()
        .map(|slot| slot.mark_id.clone())
        .collect();
    let frame = bar_widget_frame(&layout)?;
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
