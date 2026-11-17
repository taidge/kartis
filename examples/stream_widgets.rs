//! Owned Stream source data in actual Glory dot components.
#[path = "shared/stream_widgets.rs"]
mod model;

use glory::{App, Widget};
use kartis::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let source = model::source(0, 600., 0)?;
    let app = App::memory();
    app.mount(
        chart_layout(source.frame).plot_widgets_composed(source.composition, |context| {
            let frame = context.frame;
            Ok(glory::ui::widget(move |scope| {
                let count = scope.cage(0u32);
                glory::ui::button()
                    .label(scope.bond(move || {
                        frame.with(|f| {
                            format!(
                                "{}: {} ({} clicks)",
                                f.data.layer_label,
                                f.data
                                    .sample
                                    .formatted_value
                                    .as_deref()
                                    .unwrap_or("missing"),
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
