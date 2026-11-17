# Live hierarchy node components

These APIs are available in Kartis 0.2.0.
`hierarchy_node_widgets` adapts actual Treemap, CirclePacking and Icicle node
marks to typed, measured Glory children. `hierarchy_widget_frame` supplies the
same Scene, accurate plot and explicit painted-geometry interaction.

Each packet shares one owned original flat `Vec<HierarchyDatum>`. Read the
original datum through `source[source_index]`; `node.value` is the computed
subtree aggregate, not that datum's own weight. The packet also carries family,
root/focus and ancestors, current node metadata, formatted aggregate and current
owned labels with clips. Geometry and paint come from the accepted host Mark.
Labels remain independently drawn and owned by Scene; metadata does not remove
or recreate them automatically.

The builder retains a private original source and construction-eligibility
snapshot. Before any placement callback, validation checks source topology,
aggregates, indices, focus ancestry, visibility, actual geometry/marks and managed
label ownership. Hidden, zero-weight, outside-focus and excluded parent/focus
nodes do not invent roles. Transparent or unpainted but actual Marks retain roles.
A frame helper adds real typed sites even for those unpainted roles; painted
picking still reports only physical ink. The old interaction method is unchanged.
Returning `None` skips a component; an error returns no partial packet. External
callback side effects cannot be rolled back. Placements and clip capabilities
are checked by the existing hosts.

Use the current static `HierarchyLayout` or `HierarchyFocusMotion::target_layout`.
Do not combine a target packet with a different sampled motion Scene: this API
does not automatically follow spring samples or retiring marks. Rebuild after
legitimate focus/data/view changes, keeping stable IDs and revisions as needed.
Supported managed label placement/hiding can precede packet construction; stale
or corrupted label ownership is rejected.

The complete Memory-host example uses plot-rectangle label clipping for
CirclePacking: semantic Graphic text supports rectangular clips. SVG/Canvas
and raster hosts retain their separate circular/path label-clip capabilities.

```rust
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
```

Nivo uses different contracts for the three families: Treemap node/animatedProps,
CirclePacking node/style/handlers and Icicle's shared Rect wrapper. Its zero-area
or invisible computed nodes may still create components. This adapter follows
Kartis's actual-Mark eligibility and does not create proxy marks for them.
Rust uses globally unique flat IDs and zero-valued internal-node inputs;
Nivo's nested datum/path identity and aggregation profile differ. Original raw
weights and aggregates are deliberately separate. Frozen component-expression
checks do not execute React reconciliation, spring scheduling or full D3 layout.

Use [transactional composition](widget-composition.md) for common acceptance,
replacement and rollback, or [browser Scene-order composition](ordered-widgets.md)
for paint interleaving. Independent Scene export does not include arbitrary live
Widget snapshots. Native OS input and GPU/worker remain separate work.
The browser color fixture uses actual font0 Buttons with JSON data retained;
this does not establish arbitrary glyph raster equality.

Focused verification covers 24 browser/DPR/family contexts, 624 frames and
20,832 gated RGBA samples. See [verification](verification.md#development-live-hierarchy-node-components)
for exact native/parent/label scope, retained failures and remaining work.
