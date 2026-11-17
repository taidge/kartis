//! One nullable source builds Scene, binding and measured live cell data.
use kartis::*;

pub fn source(
    phase: u8,
    width: f64,
    revision: u64,
) -> Result<(
    SceneInteraction,
    PlotWidgetsComposition<HeatmapCellWidgetData>,
)> {
    let config = ChartConfig {
        width,
        height: 320.,
        margin: Margin {
            top: 20.,
            right: 20.,
            bottom: 20.,
            left: 20.,
        },
        ..Default::default()
    };
    let mut data = vec![
        HeatCell::new("south", "value", if phase >= 2 { 23. } else { 12. }),
        HeatCell {
            row: "north:温度".into(),
            column: "null".into(),
            value: if phase >= 2 { Some(0.) } else { None },
        },
        HeatCell::new("north:温度", "value", -4.),
        HeatCell::new("south", "zero", 0.),
        HeatCell::new("north:温度", "zero", 0.),
    ];
    if phase > 0 {
        data.reverse();
    }
    let mut options = HeatmapLayoutOptions {
        axes: HeatmapAxes {
            top: None,
            right: None,
            bottom: None,
            left: None,
        },
        labels: false,
        row_order: Some(if phase >= 3 {
            vec!["south".into(), "north:温度".into()]
        } else {
            vec!["north:温度".into(), "south".into()]
        }),
        column_order: Some(if phase >= 3 {
            vec!["value".into(), "zero".into(), "null".into()]
        } else {
            vec!["zero".into(), "null".into(), "value".into()]
        }),
        x_padding: HeatmapBandPadding {
            inner: 0.08,
            outer: 0.04,
        },
        y_padding: HeatmapBandPadding {
            inner: 0.08,
            outer: 0.04,
        },
        ..Default::default()
    };
    if phase == 7 {
        options.opacity = 0.;
    }
    if phase == 4 {
        options.hidden_rows.insert("south".into());
    }
    let layout = if phase == 5 {
        heatmap_layout_with_cell_factory(
            &config,
            &data,
            &options,
            &mut |ctx: &HeatmapCellContext<'_>| {
                Ok(ctx.inherit(Geometry::Circle {
                    x: ctx.position[0],
                    y: ctx.position[1],
                    radius: 12.,
                }))
            },
        )?
    } else {
        heatmap_layout(&config, &data, &options)?
    };
    let widgets = heatmap_cell_widgets(&layout, revision, |ctx| {
        if phase == 6 && ctx.cell.key.row == "south" {
            return Ok(None);
        }
        let band = ctx.cell.band.expect("visible callback has band");
        Ok(Some(PlotWidgetPlacement::Rect(LayoutRect {
            x: band.x + 4.,
            y: band.y + 4.,
            width: band.width - 8.,
            height: band.height - 8.,
        })))
    })?;
    let replace_ink = widgets.data.keys().cloned().collect();
    Ok((
        layout.into_painted_scene_interaction()?,
        PlotWidgetsComposition {
            widgets,
            replace_ink,
        },
    ))
}
