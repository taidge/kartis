use kartis::*;
pub fn scenes(c: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let mut pie = PieLayoutOptions::default();
    pie.pie.inner_radius = 0.88;
    pie.pie.pad_angle = 0.04;
    pie.pie.style.corner_radius = 6.;
    pie.pie.style.active_ids.insert("Expanded".into());
    pie.pie.style.active_inner_radius_offset = 24.;
    pie.pie.style.active_outer_radius_offset = 6.;
    pie.pie.style.arc_labels.radius_measure = ArcLabelRadiusMeasure::RingThickness;
    pie.pie.style.arc_labels.skip_radius = 32.;
    pie.pie.style.arc_labels.legacy_fit = false;
    let pie = pie_layout(
        c,
        &[
            RadialDatum::new("Expanded", 36.),
            RadialDatum::new("Thin 1", 28.),
            RadialDatum::new("Thin 2", 36.),
        ],
        &pie,
    )?
    .scene;
    let radial = stacked_radial_bar_with_arc_style(
        c,
        &["Retail".into(), "Online".into()],
        &[
            BarSeries::new("North", vec![Some(34.), Some(26.)]),
            BarSeries::new("South", vec![Some(28.), Some(32.)]),
        ],
        &StackedRadialBarOptions {
            inner_radius: 0.2,
            max_value: Some(80.),
            pad_angle: 0.04,
            labels: Some(RadialBarLabels {
                text: RadialBarLabelText::Category,
                radius_measure: ArcLabelRadiusMeasure::RingThickness,
                skip_radius: 32.,
                ..Default::default()
            }),
            ..Default::default()
        },
        &StackedRadialArcStyle {
            corner_radius: 8.,
            ..Default::default()
        },
    )?;
    Ok(vec![
        ("Pie · ring-thickness label gate", pie),
        ("RadialBar · ring-thickness labels", radial),
    ])
}
