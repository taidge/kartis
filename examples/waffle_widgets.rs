//! Actual data cells and empty slots receive different owned source metadata.
use glory::render::EventKind;
use glory::{App, Widget};
use kartis::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let data = [
        RadialDatum::new("orders", 17.25),
        RadialDatum::new("returns", 25.5),
    ];
    let layout = waffle_layout(
        &ChartConfig::default(),
        &data,
        &WaffleLayoutOptions {
            rows: 2,
            columns: 3,
            total: Some(100.),
            padding: 8.,
            ..Default::default()
        },
    )?;
    let widgets = waffle_cell_widgets(&layout, 0, |context| {
        let b = context.cell.bounds;
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
        chart(layout.scene)
            .legend(false)
            .data_controls(false)
            .plot_widgets_composed(composition, |context| {
                let frame = context.frame;
                Ok(glory::ui::widget(move |scope| {
                    let clicks = scope.cage(0usize);
                    glory::ui::button()
                        .class("waffle-demo-cell")
                        .label(scope.bond(move || {
                            frame.with(|f| {
                                format!(
                                    "cell {} / {:?} / {} clicks",
                                    f.data.cell.index,
                                    f.data.group.as_ref().map(|group| group.source.value),
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
    println!("Mounted real Waffle cell controls, including empty slots");
    app.clear()?;
    Ok(())
}
