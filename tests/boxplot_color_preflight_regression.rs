use std::cell::Cell;
use std::collections::BTreeMap;

use kartis::*;

fn combined_factory_uses_final_legend_budget(shared_scale: bool) {
    let chart = ChartConfig {
        width: 600.,
        height: 320.,
        ..Default::default()
    };
    let groups: Vec<_> = (0..256).map(|i| format!("g{i:03}")).collect();
    let data: Vec<_> = groups
        .iter()
        .enumerate()
        .flat_map(|(i, g)| {
            (0..8).map(move |sub| {
                BoxObservation::new(format!("row{i}/{sub}"), g, format!("s{sub}"), Some(2.))
            })
        })
        .collect();
    let captions: BTreeMap<_, _> = groups
        .iter()
        .enumerate()
        .map(|(i, g)| (g.clone(), format!("c{i:03}{}", "c".repeat(1020))))
        .collect();
    let options = BoxPlotLayoutOptions {
        groups: Some(groups.clone()),
        subgroups: Some((0..8).map(|i| format!("s{i}")).collect()),
        inner_padding: 0.,
        color_by: BoxPlotColorBy::Group,
        legend_mode: BoxPlotLegendMode::ByLabel,
        legend_labels: Some(captions),
        value_axis: AxisOptions {
            visible: false,
            grid: false,
            ..Default::default()
        },
        group_axis: CategoryAxisOptions {
            visible: false,
            grid: false,
            ..Default::default()
        },
        ..Default::default()
    };
    // The provisional captions are 256 valid distinct rows; the actual factory
    // sees all 2048 summaries and collapses them to one final representative.
    assert_eq!(
        boxplot_layout(&chart, &data, &options)
            .unwrap()
            .scene
            .legend
            .len(),
        256
    );
    // A borrowed label factory validly collapses these candidates to one row.
    let old =
        boxplot_layout_with_legend_label_factory(
            &chart,
            &data,
            &options,
            &mut |_| Ok("one".into()),
        )
        .unwrap();
    assert_eq!(old.scene.legend.len(), 1);
    let color_calls = Cell::new(0);
    let label_calls = Cell::new(0);
    let mut color = |_: &BoxPlotColorContext<'_>| {
        color_calls.set(color_calls.get() + 1);
        Ok(Color(12, 34, 56, 255))
    };
    let mut label = |_: &BoxPlotLegendContext<'_>| {
        label_calls.set(label_calls.get() + 1);
        Ok("one".into())
    };
    let factories = BoxPlotFactories {
        color: Some(&mut color),
        legend_label: Some(&mut label),
    };
    let layout = if shared_scale {
        boxplot_layout_with_scale_options_and_factories(
            &chart,
            &data,
            &options,
            &ContinuousScaleOptions::default(),
            factories,
        )
        .unwrap()
        .layout
    } else {
        boxplot_layout_with_factories(&chart, &data, &options, factories).unwrap()
    };
    assert_eq!(color_calls.get(), 2048);
    assert_eq!(label_calls.get(), 2048);
    assert_eq!(layout.scene.legend.len(), 1);
    assert_eq!(layout.scene.legend[0].color, Color(12, 34, 56, 255));
    assert_eq!(
        layout.scene.legend_target(&layout.scene.legend[0].id),
        Some(LegendTarget::Category(groups[0].clone()))
    );
}

#[test]
fn ordinary_combined_factory_budgets_final_deduplicated_rows() {
    combined_factory_uses_final_legend_budget(false);
}

#[test]
fn configured_combined_factory_budgets_final_deduplicated_rows() {
    combined_factory_uses_final_legend_budget(true);
}
