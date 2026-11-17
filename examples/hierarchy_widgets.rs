//! Owned original tree and live Glory nodes across all hierarchy views.
use glory::{App, Widget};
use kartis::*;
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let data = vec![
        HierarchyDatum::new("root", None, 0.),
        HierarchyDatum::new("a", Some("root".into()), 2.),
        HierarchyDatum::new("b", Some("root".into()), 3.),
    ];
    for view in [
        HierarchyView::Treemap(Default::default()),
        HierarchyView::CirclePacking(CirclePackingLayoutOptions {
            style: HierarchyStyleOptions {
                leaf_labels: HierarchyLabelStyle {
                    clip_to_node: false,
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        }),
        HierarchyView::Icicle(Default::default()),
    ] {
        let mut controller = HierarchyFocusController::new(&data)?;
        let layout = controller.layout(&ChartConfig::default(), &view)?;
        let widgets = hierarchy_node_widgets(&layout, 0, |_| {
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
            .map(|s| s.mark_id.clone())
            .collect();
        let frame = hierarchy_widget_frame(&layout)?;
        let app = App::memory();
        app.mount(chart_layout(frame).plot_widgets_composed(
            PlotWidgetsComposition {
                widgets,
                replace_ink,
            },
            |ctx: PlotWidgetCompositionContext<HierarchyNodeWidgetData>| {
                let frame = ctx.frame;
                Ok(glory::ui::widget(move |scope| {
                    let clicks = scope.cage(0u32);
                    glory::ui::button()
                        .label(scope.bond(move || {
                            frame.with(|f| {
                                format!(
                                    "{}: raw {}, total {} ({} clicks)",
                                    f.data.node.label,
                                    f.data.source[f.data.source_index].value,
                                    f.data.node.value,
                                    clicks.get()
                                )
                            })
                        }))
                        .on(glory::render::EventKind::Activate, move |_| {
                            clicks.revise(|v| *v += 1)
                        })
                        .show_in(scope)
                }))
            },
        ))?;
        app.flush()?;
        app.clear()?;
    }
    Ok(())
}
