use kartis::*;

pub fn scenes(c: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let radial = stacked_radial_bar_with_arc_style(
        c,
        &["Retail".into(), "Online".into(), "Wholesale".into()],
        &[
            BarSeries::new("North", vec![Some(30.), Some(20.), Some(12.)]),
            BarSeries::new("South", vec![Some(22.), Some(32.), Some(8.)]),
        ],
        &StackedRadialBarOptions {
            inner_radius: 0.2,
            pad_angle: 0.09,
            max_value: Some(80.),
            labels: Some(RadialBarLabels {
                text: RadialBarLabelText::Category,
                skip_angle: 0.35,
                ..Default::default()
            }),
            ..Default::default()
        },
        &StackedRadialArcStyle {
            corner_radius: 14.,
            ..Default::default()
        },
    )?;

    let tree = [
        HierarchyDatum::new("Portfolio", None, 0.),
        HierarchyDatum::new("Cloud", Some("Portfolio".into()), 0.),
        HierarchyDatum::new("Devices", Some("Portfolio".into()), 0.),
        HierarchyDatum::new("Compute", Some("Cloud".into()), 18.),
        HierarchyDatum::new("Storage", Some("Cloud".into()), 12.),
        HierarchyDatum::new("Phone", Some("Devices".into()), 17.),
        HierarchyDatum::new("Tablet", Some("Devices".into()), 7.),
    ];
    let view = HierarchyView::Treemap(TreemapLayoutOptions {
        inner_padding: 4.,
        outer_padding: 5.,
        parent_label_size: 22.,
        ..Default::default()
    });
    let mut focus = HierarchyFocusMotion::new(c, &tree, &view, Default::default())?;
    let initial = focus.scene().clone();
    focus.focus(c, "Cloud", &view)?;
    let sampled = focus.tick(0.075)?.clone();
    focus.finish();
    Ok(vec![
        ("Stacked RadialBar · rounded sectors", radial),
        ("Treemap focus · portfolio", initial),
        ("Treemap focus · sampled motion", sampled),
        ("Treemap focus · cloud", focus.scene().clone()),
    ])
}
