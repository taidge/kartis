//! Live Glory components for Bump ranking points.
use glory::{App, Widget};
use kartis::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let series = [BumpSeries::new(
        "north",
        vec![
            BumpDatum::new(1_f64, 2.),
            BumpDatum::gap("middle"),
            BumpDatum::new("tail", 0.),
        ],
    )];
    let layout = bump_layout(
        &ChartConfig::default(),
        &series,
        &BumpLayoutOptions::default(),
    )?;
    let widgets = bump_point_widgets(&layout, 0, |_| {
        Ok(Some(PlotWidgetPlacement::AtDatum {
            offset: [-20., -10.],
            measured: TextSize {
                width: 40.,
                height: 20.,
            },
        }))
    })?;
    let replace_ink = widgets
        .layout
        .slots
        .iter()
        .map(|slot| slot.mark_id.clone())
        .collect();
    let frame = bump_widget_frame(&layout)?;
    let app = App::memory();
    app.mount(chart_layout(frame).plot_widgets_composed(
        PlotWidgetsComposition {
            widgets,
            replace_ink,
        },
        |context: PlotWidgetCompositionContext<BumpPointWidgetData>| {
            let frame = context.frame;
            Ok(glory::ui::widget(move |scope| {
                let count = scope.cage(0u32);
                glory::ui::button()
                    .label(scope.bond(move || {
                        frame.with(|f| {
                            format!(
                                "{}: {:?}, rank {:?} ({} clicks)",
                                f.data.series.label,
                                f.data.point.raw_x,
                                f.data.point.rank,
                                count.get()
                            )
                        })
                    }))
                    .on(glory::render::EventKind::Activate, move |_| {
                        count.revise(|v| *v += 1)
                    })
                    .show_in(scope)
            }))
        },
    ))?;
    app.flush()?;
    app.clear()?;
    Ok(())
}
