//! Shared source for actual range, measure and marker Widget overlays.
use kartis::*;

pub fn source(
    phase: u8,
    width: f64,
    revision: u64,
) -> Result<(Scene, PlotWidgetsData<BulletPartWidgetData>)> {
    let config = ChartConfig {
        width,
        height: 360.,
        margin: Margin {
            top: 20.,
            right: 20.,
            bottom: 20.,
            left: 20.,
        },
        ..Default::default()
    };
    let data = ["north:温度", "south"].map(|id| BulletDatum {
        id: id.into(),
        ranges: vec![0., 25. + f64::from(phase) * 3., 65.],
        measures: vec![0., 42. + f64::from(phase) * 4.],
        markers: vec![78. - f64::from(phase) * 2.],
    });
    let options = BulletLayoutOptions {
        min: Some(0.),
        max: Some(100.),
        spacing: 24.,
        axis: AxisOptions {
            visible: false,
            ..Default::default()
        },
        legend: false,
        order: (phase % 2 == 1).then(|| vec!["south".into(), "north:温度".into()]),
        ..Default::default()
    };
    let layout = bullet_layout(&config, &data, &options)?;
    let packet = bullet_part_widgets(&layout, revision, |ctx| {
        let source = match ctx.part.kind {
            BulletPartKind::Range | BulletPartKind::Measure => Some(1),
            BulletPartKind::Marker => Some(0),
        };
        if ctx.part.source_index != source {
            return Ok(None);
        }
        let band = match ctx.part.kind {
            BulletPartKind::Range => 0.,
            BulletPartKind::Measure => 1.,
            BulletPartKind::Marker => 2.,
        };
        Ok(Some(PlotWidgetPlacement::Rect(LayoutRect {
            x: ctx.row.track.x + (ctx.row.track.width - 100.) * (band + 1.) / 4.,
            y: ctx.row.track.y + 6. + band * 34.,
            width: 100.,
            height: 26.,
        })))
    })?;
    Ok((layout.scene, packet))
}
