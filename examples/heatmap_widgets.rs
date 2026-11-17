//! Real nullable cell controls; replace App::memory with an application host.
use glory::render::EventKind;
use glory::{App, Widget};
use kartis::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let source = [
        HeatCell {
            row: "temperature".into(),
            column: "missing".into(),
            value: None,
        },
        HeatCell::new("temperature", "zero", 0.),
    ];
    let layout = heatmap_layout(
        &ChartConfig::default(),
        &source,
        &HeatmapLayoutOptions {
            axes: HeatmapAxes::none(),
            labels: false,
            ..Default::default()
        },
    )?;
    let widgets = heatmap_cell_widgets(&layout, 0, |context| {
        let b = context.cell.band.unwrap();
        Ok(Some(PlotWidgetPlacement::Rect(LayoutRect {
            x: b.x + 4.,
            y: b.y + 4.,
            width: b.width - 8.,
            height: b.height - 8.,
        })))
    })?;
    let composition = PlotWidgetsComposition {
        replace_ink: widgets.data.keys().cloned().collect(),
        widgets,
    };
    let app = App::memory();
    app.mount(
        chart_layout(layout.into_painted_scene_interaction()?)
            .legend(false)
            .data_controls(false)
            .plot_widgets_composed(composition, |context| {
                let frame = context.frame;
                Ok(glory::ui::widget(move |scope| {
                    let clicks = scope.cage(0usize);
                    glory::ui::button()
                        .class("heatmap-demo-cell")
                        .label(scope.bond(move || {
                            frame.with(|f| {
                                format!(
                                    "{} / {:?} / {} clicks",
                                    f.data.datum.column,
                                    f.data.datum.value,
                                    clicks.get()
                                )
                            })
                        }))
                        .on(EventKind::Activate, move |_| clicks.revise(|n| *n += 1))
                        .show_in(scope)
                }))
            }),
    )?;
    app.flush()?;
    println!("Mounted two real HeatMap cell controls: None and Some(0)");
    app.clear()?;
    Ok(())
}
