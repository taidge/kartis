//! Live Glory Bar children retaining original nullable source series.
#[path = "shared/bar_widgets.rs"]
mod model;
use glory::{App, Widget};
use kartis::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let source = model::source(0, 600., 0)?;
    let app = App::memory();
    app.mount(
        chart_layout(source.frame)
            .legend(false)
            .plot_widgets_composed(source.composition, |context| {
                let frame = context.frame;
                Ok(glory::ui::widget(move |scope| {
                    let count = scope.cage(0u32);
                    glory::ui::button()
                        .label(scope.bond(move || {
                            frame.with(|f| {
                                format!(
                                    "{} / {}: {} ({} clicks)",
                                    f.data.bar.category,
                                    f.data.series.id,
                                    f.data.bar.raw_value,
                                    count.get()
                                )
                            })
                        }))
                        .on(glory::render::EventKind::Activate, move |_| {
                            count.revise(|v| *v += 1)
                        })
                        .show_in(scope)
                }))
            }),
    )?;
    app.flush()?;
    app.clear()?;
    Ok(())
}
