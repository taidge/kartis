use std::cell::Cell;

use kartis::*;

fn legacy_rejection_message(shared: bool) {
    let c = ChartConfig::default();
    let data = [BoxObservation::new("a", "A", "x", Some(2.))];
    let options = BoxPlotLayoutOptions {
        legend_mode: BoxPlotLegendMode::ByLabel,
        ..Default::default()
    };
    for combined_without_color in [false, true] {
        let calls = Cell::new(0);
        let mut label = |_: &BoxPlotLegendContext<'_>| {
            calls.set(calls.get() + 1);
            Ok("one".into())
        };
        let mut controller = if combined_without_color {
            BoxPlotInteraction::with_factories(
                &c,
                &data,
                &options,
                &BoxPlotInteractionOptions::default(),
                BoxPlotFactories {
                    color: None,
                    legend_label: Some(&mut label),
                },
            )
        } else {
            BoxPlotInteraction::with_legend_label_factory(
                &c,
                &data,
                &options,
                &BoxPlotInteractionOptions::default(),
                &mut label,
            )
        }
        .unwrap();
        let before = format!("{controller:?}");
        let error = if shared {
            controller.replace_source_with_scale_options(
                &c,
                &data,
                &options,
                &ContinuousScaleOptions::default(),
            )
        } else {
            controller.replace_source(&c, &data, &options)
        }
        .unwrap_err();
        assert_eq!(
            error.0,
            "factory-built BoxPlot source replacement requires a legend factory"
        );
        assert_eq!(calls.get(), 1);
        assert_eq!(format!("{controller:?}"), before);
    }
}

#[test]
fn ordinary_legacy_and_none_color_factory_keep_original_error_and_state() {
    legacy_rejection_message(false);
}

#[test]
fn configured_legacy_and_none_color_factory_keep_original_error_and_state() {
    legacy_rejection_message(true);
}
