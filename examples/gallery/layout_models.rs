use kartis::*;
use std::collections::BTreeMap;

pub fn scenes(c: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let funnel = funnel_layout(
        c,
        &[
            FunnelDatum::new("Reached", 100.),
            FunnelDatum::new("Qualified", 72.),
            FunnelDatum::new("Evaluated", 44.),
            FunnelDatum::new("Converted", 22.),
        ],
        &FunnelLayoutOptions {
            direction: FunnelDirection::Horizontal,
            spacing: 16.,
            current: Some("Qualified".into()),
            current_size_extension: 8.,
            ..Default::default()
        },
    )?;

    let audience = [
        RadialDatum::new("Mobile", 21.),
        RadialDatum::new("Desktop", 15.),
        RadialDatum::new("Tablet", 7.),
    ];
    let waffle = WaffleLayoutOptions {
        rows: 5,
        columns: 10,
        total: Some(50.),
        direction: WaffleFillDirection::Right,
        padding: 6.,
        corner_radius: 8.,
        ..Default::default()
    };
    let cells = waffle_layout(c, &audience, &waffle)?;
    let areas = waffle_layout(
        c,
        &audience,
        &WaffleLayoutOptions {
            mode: WaffleMode::Areas,
            ..waffle
        },
    )?;

    let sunburst = sunburst_layout(
        c,
        &[
            HierarchyDatum::new("Portfolio", None, 0.),
            HierarchyDatum::new("Cloud", Some("Portfolio".into()), 4.),
            HierarchyDatum::new("Devices", Some("Portfolio".into()), 0.),
            HierarchyDatum::new("Compute", Some("Cloud".into()), 20.),
            HierarchyDatum::new("Storage", Some("Cloud".into()), 11.),
            HierarchyDatum::new("Phone", Some("Devices".into()), 15.),
            HierarchyDatum::new("Tablet", Some("Devices".into()), 9.),
        ],
        &SunburstLayoutOptions {
            pad_angle: 0.025,
            corner_radius: 6.,
            labels: Some(SunburstLabelOptions::default()),
            ..Default::default()
        },
    )?;

    let date = |offset| 1_709_078_400_000. + 86_400_000. * offset;
    let mut dimensions = ["Budget", "Throughput", "Review date"].map(ParallelDimension::new);
    dimensions[2].axis.format = ValueFormat::UtcDate;
    dimensions[2].axis.tick_values = Some(vec![date(0.), date(1.), date(2.), date(3.)]);
    let parallel = parallel_layout_with_scale_options(
        &ChartConfig {
            margin: Margin {
                right: 106.,
                ..c.margin
            },
            ..c.clone()
        },
        &dimensions,
        &[
            ParallelDatum::new("Cedar", vec![Some(28.), Some(22.), Some(date(1.))]),
            ParallelDatum::new("Maple", vec![Some(42.), Some(60.), Some(date(0.))]),
            ParallelDatum::new("Willow", vec![Some(58.), Some(11.), Some(date(3.))]),
        ],
        &ParallelCoordinatesOptions {
            legend: true,
            ..Default::default()
        },
        &BTreeMap::from([
            (
                "Budget".into(),
                ParallelScaleOptions {
                    bounds: [Some(0.), None],
                    scale: ContinuousScaleOptions {
                        round: true,
                        nice: ScaleNice::Default,
                        ..Default::default()
                    },
                },
            ),
            (
                "Throughput".into(),
                ParallelScaleOptions {
                    scale: ContinuousScaleOptions {
                        kind: ScaleKind::Log,
                        log_base: 2.,
                        nice: ScaleNice::Default,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ),
            (
                "Review date".into(),
                ParallelScaleOptions {
                    scale: ContinuousScaleOptions {
                        kind: ScaleKind::Utc,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ),
        ]),
    )?;
    Ok(vec![
        ("Funnel · spaced and highlighted stages", funnel.scene),
        ("Waffle · rounded cells", cells.scene),
        ("Waffle · contiguous areas", areas.scene),
        ("Sunburst · equal-area radial partition", sunburst.scene),
        (
            "Parallel · rounded, logarithmic and UTC axes",
            parallel.scene,
        ),
    ])
}
