# Tree function configuration

This API is available in Kartis 0.2.0.
`tree_layout_with_factories` accepts five synchronous borrowed `FnMut` functions
in `TreeFactories`. Unspecified functions retain the static options. Neither
the functions nor their captured state is retained by the returned layout.
[中文](tree-functions.zh-CN.md)

| Field | Context | Result |
| --- | --- | --- |
| `node_size` | `TreeNodeFunctionContext` | Diameter in Scene pixels |
| `node_color` | `TreeNodeFunctionContext` | Actual node fill `Color` |
| `link_thickness` | `TreeLinkFunctionContext` | Stroke width in Scene pixels |
| `link_color` | `TreeLinkFunctionContext` | Stroke `Color` |
| `label` | `TreeLabelFunctionContext` | Drawing-label `String` |

All functions return `Result`. Node input contains the complete flat source,
original datum and index, parent and children indices, depth, height, aggregate
value, projected position and plot. The original weight, including its sign bit,
remains in `datum.value`. Size and color share this intermediate context, which
does not pretend to contain a final size or color. Link input supplies the
computed link and both final computed endpoint nodes, along with source and
plot. Label input supplies the original datum and final computed node.

## Complete application

The [standalone example](../examples/tree_functions.rs) mounts a Glory Memory
chart and rebuilds its accepted source. Memory exercises the host protocol;
operating-system input is a separate concern.

```rust
//! A complete Glory application with Tree's five borrowed base functions.
use glory::App;
use kartis::*;

fn source(visits: f64) -> Result<SceneInteraction> {
    let data = [
        HierarchyDatum::new("visits", Some("root".into()), visits),
        HierarchyDatum::new("root", None, 0.),
        HierarchyDatum::new("sales", Some("root".into()), 10.),
    ];
    let options = TreeOptions::default();
    // Size is a diameter; node.radius is half the accepted result.
    let mut size = |ctx: &TreeNodeFunctionContext<'_>| Ok(16. + ctx.depth as f64 * 8.);
    let mut color = |ctx: &TreeNodeFunctionContext<'_>| {
        Ok(if ctx.children.is_empty() {
            Color(30, 140, 90, 255)
        } else {
            Color(40, 100, 180, 255)
        })
    };
    let mut thickness =
        |ctx: &TreeLinkFunctionContext<'_>| Ok(if ctx.target_node.value >= 20. { 4. } else { 2. });
    let mut link_color = |ctx: &TreeLinkFunctionContext<'_>| {
        Ok(ctx.target_node.color.unwrap_or(Color(100, 100, 100, 255)))
    };
    let mut label =
        |ctx: &TreeLabelFunctionContext<'_>| Ok(format!("{}: {:.0}", ctx.datum.id, ctx.node.value));
    let layout = tree_layout_with_factories(
        &ChartConfig::default(),
        &data,
        &options,
        &mut TreeFactories {
            node_size: Some(&mut size),
            node_color: Some(&mut color),
            link_thickness: Some(&mut thickness),
            link_color: Some(&mut link_color),
            label: Some(&mut label),
        },
    )?;
    assert_eq!(layout.nodes[0].radius, 12.);
    assert_eq!(layout.nodes[1].value, visits + 10.);
    assert!(
        layout
            .scene
            .labels
            .iter()
            .any(|l| l.text == format!("visits: {visits:.0}"))
    );
    // Drawing labels do not rewrite original source or business-control labels.
    let frame = tree_widget_frame(&layout)?;
    assert_eq!(frame.interaction.as_ref().unwrap().sites[0].label, "visits");
    Ok(frame)
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let app = App::memory();
    let input = app.scope().cage(source(20.)?);
    let frame = app.scope().bond(move || input.get());
    app.mount(chart_layout(frame).on_reject(|e| eprintln!("Tree example rejected: {e:?}")))?;
    app.flush()?;
    input.revise(|old| *old = source(30.).unwrap());
    app.flush()?;
    assert!(input.with(|s| s.scene.labels.iter().any(|l| l.text == "visits: 30")));
    app.clear()?;
    Ok(())
}
```

## Evaluation and drawing

Static options, raw hierarchy, all positions and original paths are checked
before the first function runs. Nodes execute in breadth-first hierarchy order,
with size before color for each node; source indices continue to address the
original flat input, even when a child precedes its parent. After all nodes are
resolved, links execute thickness before color with complete endpoint data.
Disabled links still execute these functions and retain topology metadata, but
produce no Mark. Zero-width enabled links retain their Mark without a stroke.

Label functions run after node and link resolution, only when hierarchy labels
and the relevant All/Leaves/Internal visibility permit drawing. Empty returned
strings retain their actual Label entry and ownership. These strings change
drawn labels; they do not overwrite the raw datum, node Mark's business label
or canonical controls. [Tree Widget adapters](tree-widgets.md) receive the same
accepted label pool and computed node colors/radii.

Size results are diameters, unlike `TreeOptions.node_radius`. They update
`TreeNode.radius` and its resolved style. An explicit color function overrides
Palette, Fixed and None fill declarations. Missing functions preserve those
declarations. Zero-size and transparent nodes remain real computed roles.
Invalid output or a function error returns no candidate layout. User-owned
side effects from functions are not rolled back; perform host publication only
after construction succeeds.

Diameters must be finite in `0..=32768`. Link widths must be zero or finite in
`0.000001..=10000`. Function labels use the existing 1024-byte UTF-8 budget.

## Nivo correspondence and limits

The frozen reference executes the original five function props, passthrough
helpers, computed-node/link order and label recipes. It separately records
active/inactive modifiers, fixed-depth color inheritance and SVG/Canvas label
gates. Its scales, hierarchy coordinates, hooks and Spring are explicit
dependency probes, rather than D3 numerical or React lifecycle evidence.

Rust preserves its existing projection inset derived from declared static node
styles. Function sizes do not reproject positions. Nivo projects independently
of node sizes; therefore changing a static radius can produce a different inset
in Rust. Function output still controls actual node geometry and label spacing.
These existing Scene constructors use one visibility-gated label calculation for both renderers;
original Nivo Canvas computes labels even when their paint layer is disabled.
The later [native Canvas constructors](tree-canvas.md) opt into a complete,
once-computed label pool without changing these existing constructors.

Nested generic JavaScript data and path UIDs differ from Rust's flat, globally
unique IDs and explicit raw/aggregate weights. Active/inactive node-size and
link-thickness modifiers and fixed-depth color inheritance are available through
[Tree activity](tree-activity.md). [Label components](tree-label-components.md)
and [native Canvas layers](tree-canvas.md) have separate later APIs. Arbitrary
SVG function layers and automatic retiring animations remain open.
GPU/worker and arbitrary live Widget export remain deferred. These functions
do not claim those broader behaviors or full-frame pixel equivalence.
