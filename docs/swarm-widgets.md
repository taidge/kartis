# Live Swarm node Widgets

Unreleased. `swarm_node_widgets` creates measured, typed Glory components for actual packed Swarm nodes. Use `swarm_widget_frame` for an explicit painted-geometry frame in semantic, CPU-image or SVG/Canvas browser hosts. Ordinary Swarm rendering and its legacy conversion retain their previous behavior.

```rust,ignore
let packet = swarm_node_widgets(&layout, &original_points, revision, |node| {
    Ok(Some(PlotWidgetPlacement::AtDatum {
        offset: [-16.0, -11.0],
        measured: TextSize { width: 32.0, height: 22.0 },
    }))
})?;
```

Each `SwarmNodeWidgetData` contains one shared `Arc<Vec<SwarmPoint>>`, original `source_index`, packed `site_index`, current `SwarmSite` and `SwarmGroupLayout`, exact plot and resolved value scale. Read the current datum through `source[source_index]`; read actual paint/geometry from the host's accepted `TypedPlotWidgetFrame.frame.mark`. Original numeric value and optional size equality preserves signed-zero bits, including hidden or unselected source rows. Ordinary business identity and geometry equality remain numeric. Fresh equal source allocations compare equal; selected nodes share one owned input copy.

The borrowed placement context supplies the layout, original input/datum, group, packed site and actual Mark. All source rows, groups, metadata, value projection, packed order, actual marks and anchors are checked before the first callback. Returning `None` skips a role; errors return no packet. Callback-owned effects are not rolled back. The host validates placement/clip support and the 128 selected-slot budget. An empty packet retains the full plot.

Hidden groups and fully culled nodes have no role. Actual size-zero, transparent, zero-opacity and unpainted marks retain roles; the host decides physical input and visibility from actual accepted eligibility. Callbacks follow packed drawing order, which can differ from input order. Custom geometry retains the original collision disc and packed datum position. Offset Paths use typed anchors; Exact picking follows ink, not a guessed Path bounding-box center.

Source provenance is explicit: pass the original points alongside the public layout. This verifies agreement with its current metadata; the layout does not retain original orientation, packing spacing, shape, fit or formatter configuration. A fully coherent public edit is legal. The frame helper has no original-input argument and checks current structure/binding only; the packet adapter additionally checks all supplied original records. This does not reconstruct a force simulation or the original configuration.

## A complete application

This runnable Memory application keeps the actual Button's identity and click count across a source-only signed-zero change and an input reorder:

```rust
//! Retained Glory node components with actual packed Swarm source data.
use glory::render::{Event, EventKind, NodeId, Props};
use glory::{App, Widget};
use kartis::*;
#[derive(Clone, Debug)]
struct Source {
    frame: SceneInteraction,
    composition: PlotWidgetsComposition<SwarmNodeWidgetData>,
}
fn source(positive_zero: bool, reverse_input: bool) -> Result<Source> {
    let mut b = SwarmPoint::new("b", "north", if positive_zero { 0. } else { -0. });
    b.size = Some(0.);
    let mut data = vec![SwarmPoint::new("a", "north", 2.), b];
    data[0].label = Some("Alpha".into());
    if reverse_input {
        data.reverse();
    }
    let layout = swarm_layout(
        &ChartConfig {
            width: 600.,
            height: 260.,
            ..Default::default()
        },
        &data,
        &SwarmLayoutOptions {
            orientation: SwarmOrientation::Horizontal,
            value_domain: Some([-1., 10.]),
            value_axis: None,
            group_axis: None,
            ..Default::default()
        },
    )?;
    let widgets = swarm_node_widgets(&layout, &data, 0, |_| {
        Ok(Some(PlotWidgetPlacement::AtDatum {
            offset: [-16., -11.],
            measured: TextSize {
                width: 32.,
                height: 22.,
            },
        }))
    })?;
    let replace_ink = widgets
        .layout
        .slots
        .iter()
        .map(|s| s.mark_id.clone())
        .collect();
    Ok(Source {
        frame: swarm_widget_frame(&layout)?,
        composition: PlotWidgetsComposition {
            widgets,
            replace_ink,
        },
    })
}
fn main_node(app: &App) -> NodeId {
    let tree = app.tree();
    let mut pending = tree.roots().to_vec();
    while let Some(id) = pending.pop() {
        let n = tree.node(id).unwrap();
        if n.descriptor
            .view()
            .classes
            .iter()
            .any(|c| c == "swarm-example-main")
        {
            return id;
        }
        pending.extend(n.children.iter().copied());
    }
    panic!("main node component missing")
}
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let app = App::memory();
    let input = app.scope().cage(source(false, false)?);
    let frame = app.scope().bond(move || input.with(|s| s.frame.clone()));
    let packet = app
        .scope()
        .bond(move || input.with(|s| s.composition.clone()));
    app.mount(
        chart_layout(frame).plot_widgets_composed(packet, |context| {
            let frame = context.frame;
            let main = frame.with(|f| f.data.site.node_id == "a");
            Ok(glory::ui::widget(move |scope| {
                let count = scope.cage(0u32);
                glory::ui::button()
                    .class(if main {
                        "swarm-example-main"
                    } else {
                        "swarm-example-node"
                    })
                    .label(scope.bond(move || {
                        frame.with(|f| {
                            let d = &f.data.source[f.data.source_index];
                            let zero = f.data.source.iter().find(|v| v.id == "b").unwrap();
                            format!(
                                "{} · {}: {} (zero={:016x}, clicks={})",
                                f.data.group.label,
                                d.label.as_deref().unwrap_or(&d.id),
                                f.data.site.formatted_value,
                                zero.value.to_bits(),
                                count.get()
                            )
                        })
                    }))
                    .on(EventKind::Activate, move |_| count.revise(|v| *v += 1))
                    .show_in(scope)
            }))
        }),
    )?;
    let child = main_node(&app);
    app.dispatch(child, Event::Activate)?;
    app.flush()?;
    input.revise(|s| *s = source(true, false).unwrap());
    app.flush()?;
    assert_eq!(main_node(&app), child);
    {
        let tree = app.tree();
        let Props::Button(button) = tree.node(child).unwrap().descriptor.view().props else {
            panic!("not a Button")
        };
        assert!(button.label.contains("zero=0000000000000000, clicks=1"));
    }
    input.revise(|s| *s = source(false, true).unwrap());
    app.flush()?;
    assert_eq!(main_node(&app), child);
    app.clear()?;
    Ok(())
}
```

Run `cargo run --example swarm_widgets`. Overlay hosts retain original source ink. To replace selected ink, explicitly put those mark IDs in `PlotWidgetsComposition.replace_ink` and use `plot_widgets_composed` or BrowserChart's Scene-order composition entry. Replacement excludes that physical source ink while preserving source data and controls. Scene exports contain authored chart marks and labels; they do not export live child Widgets. A new revision rebuilds a child; stable revision updates accepted data and preserves local state.

## Relationship to Nivo

Nivo SwarmPlot's SVG `circleComponent` receives the computed node, animated style, border width and seven mouse handlers. Its transition keys by node ID, clamps radius to zero and can still create zero-size or noninteractive nodes. Canvas separately uses `renderCircle`. Kartis provides actual Glory components through shared typed hosts and a separate geometry factory for exportable ink. Its deterministic one-axis packing, String business IDs, finite numeric/integral UTC values and explicit host events remain distinct from Nivo's force simulation, arbitrary raw fields, React/Spring scheduling and JavaScript Date objects. Controlled hidden/cull eligibility is also explicit; these APIs do not promise automatic sampled-motion or retiring-node components.

The browser fixture uses font-zero solid-color Buttons, independent original input/role oracles, actual DOM geometry within 1/32 logical pixel and foreground samples within ±1 per channel. Rejection compares accepted state, complete root DOM, SVG computed paint styles, native Canvas backing bytes and viewport geometry exactly. Whole-viewport PNG hashes remain separate diagnostics because a script-free native SVG antialias pixel variance is already known. Arbitrary glyph parity, strict compositor stability, native same-layer input and GPU/worker work remain open.

See [geometry factories](swarm-custom.md), [typed packets](typed-plot-widgets.md), [original-value updates](widget-source-values.md) and [中文指南](swarm-widgets.zh-CN.md).
