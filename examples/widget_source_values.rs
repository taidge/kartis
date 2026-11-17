//! Observe original-value updates independently of equal chart paint.
use glory::render::Props;
use glory::{App, Widget};
use kartis::*;
fn source(value: f64) -> Result<(SceneInteraction, PlotWidgetsData<BumpPointWidgetData>)> {
    let layout = bump_layout(
        &ChartConfig::default(),
        &[BumpSeries::new("row", vec![BumpDatum::new("point", value)])],
        &BumpLayoutOptions {
            axes: BumpAxes::none(),
            legend: false,
            labels: BumpEndpointLabels {
                start: false,
                end: false,
                ..Default::default()
            },
            ..Default::default()
        },
    )?;
    let packet = bump_point_widgets(&layout, 0, |_| {
        Ok(Some(PlotWidgetPlacement::AtDatum {
            offset: [-40., -12.],
            measured: TextSize {
                width: 80.,
                height: 24.,
            },
        }))
    })?;
    Ok((bump_widget_frame(&layout)?, packet))
}
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let before = source(-0.)?;
    let after = source(0.)?;
    assert_eq!(before.0, after.0); // ordinary numeric Scene equality
    assert_ne!(before.1, after.1); // observable original-source bits
    let app = App::memory();
    let input = app.scope().cage(before);
    let frame = app.scope().bond(move || input.with(|v| v.0.clone()));
    let packet = app.scope().bond(move || input.with(|v| v.1.clone()));
    app.mount(
        chart_layout(frame).plot_widgets_with_data(packet, |context| {
            let frame = context.frame;
            Ok(glory::ui::widget(move |scope| {
                glory::ui::button()
                    .class("raw-example")
                    .label(scope.bond(move || {
                        frame.with(|f| format!("{:016x}", f.data.point.rank.unwrap().to_bits()))
                    }))
                    .show_in(scope)
            }))
        }),
    )?;
    input.revise(|old| *old = after);
    app.flush()?;
    {
        let tree = app.tree();
        let mut pending = tree.roots().to_vec();
        let mut label = None;
        while let Some(id) = pending.pop() {
            let node = tree.node(id).unwrap();
            let view = node.descriptor.view();
            if view.classes.iter().any(|c| c == "raw-example")
                && let Props::Button(p) = view.props
            {
                label = Some(p.label.clone());
            }
            pending.extend(node.children.iter().copied());
        }
        assert_eq!(label.as_deref(), Some("0000000000000000"));
    }
    app.clear()?;
    Ok(())
}
