# Tree SVG function layers (Unreleased)

[简体中文](tree-svg-layers.zh-CN.md)

The browser integration requires WebAssembly and `web`; `TreeSvgFrame`,
`TreeSvgPlan` and the layout builders are also available as pure Rust. The bounded
browser matrix covers five browser configurations, two DPRs and 3,690 frames;
see the [verification record](verification.md#development-tree-svg-function-layers)
for evidence and limits. Existing Tree constructors and browser mounts keep
their defaults.

`BrowserTreeSvgProgram` combines literal Tree drawing occurrences with arbitrary
Glory subtrees. A custom subtree is mounted in an XHTML host inside a real SVG
`foreignObject`. It can contain Glory controls and `Graphic` drawing. This is a
Glory adaptation of Nivo's SVG function layers, not a React component or an API
for inserting arbitrary raw SVG nodes.

## Literal paint order

Each `TreeSvgLayer` has a unique occurrence key and a kind: `Links`, `Nodes`,
`Labels`, `Mesh`, `SceneExtras`, or `Custom(registry_key)`. The supplied vector is
the paint order. Omitting a builtin omits its ink; repeating it paints another
view of the same source. Repeats have separate DOM/resource namespaces and do not
duplicate business Marks, canonical controls, or source IDs. The background is
painted once.

`BrowserTreeSvgProgram::defaults()` includes extras, links, nodes, labels and mesh
in that order. `new(vec![])` is an empty program. Unowned Scene annotations must
have a `SceneExtras` occurrence under the default `RequireLayer` policy; use
`with_extras(TreeSvgExtrasPolicy::Omit)` to omit them explicitly. A Scene opacity
group must remain complete and contiguous within an occurrence; a program that
would split it rejects.

Register a whole-layer factory separately from its occurrence keys:

```rust
use glory::Widget;
use kartis::{Result, TreeSvgLayer, TreeSvgLayerKind};
use kartis::browser::{BrowserTreeSvgLayerContext, BrowserTreeSvgProgram};

fn program<W: Widget + 'static>(
    factory: impl FnMut(BrowserTreeSvgLayerContext) -> Result<W> + 'static,
) -> Result<BrowserTreeSvgProgram> {
    BrowserTreeSvgProgram::new(vec![
        TreeSvgLayer::new("links", TreeSvgLayerKind::Links)?,
        TreeSvgLayer::new("under", TreeSvgLayerKind::Custom("panel".into()))?,
        TreeSvgLayer::new("nodes", TreeSvgLayerKind::Nodes)?,
        TreeSvgLayer::new("over", TreeSvgLayerKind::Custom("panel".into()))?,
        TreeSvgLayer::new("labels", TreeSvgLayerKind::Labels)?,
    ])?
    .with_extras(kartis::TreeSvgExtrasPolicy::Omit)
    .custom("panel", 0, factory)
}
```

The two `panel` occurrences get independent child state. A stable occurrence
key, registry key and revision retain a child when the program is reordered.
Changing its revision rebuilds it. Replacing a factory under the same retained
identity does not run that new factory; change the revision to request a rebuild.

## Accepted data and coordinates

The factory receives `BrowserTreeSvgLayerContext`. Its `initial` frame is a
tentative constructor input; `frame` is a retained
`Bond<Option<BrowserTreeSvgLayerFrame>>` that starts at `None`. Render accepted
data from that Bond. A frame contains the occurrence key/index, source generation,
publication, logical camera, hidden IDs, interactive flag and `TreeSvgFrame`.

`TreeSvgFrame` exposes the complete original flat source, computed nodes and
links, resolved link styles, activity, theme, plot, source Mark lookup, extras,
node-ID index and SVG link generator. Source raw values retain their original
signed-zero bits. Node positions are in outer Scene coordinates.
`inner_position(node.position)` subtracts the plot origin for use inside the
custom host. Its box is `inner_size()`; the host applies plot margins and the
accepted camera once. Do not add either transform again.

A Labels occurrence requests normal Tree label calculation. Omitting all Labels
occurrences skips owned label construction and its label function. Existing
Tree label visibility options still apply. Custom layers can use the complete
node/source data independently; they do not acquire fabricated Scene labels.
An authoritative custom builder receives `BrowserTreeSvgBuildContext`, including
`layers` and `labels_required`; use `tree_svg_layout_with_activity` to preserve
this scheduling contract.

## Browser owner and source components

Use `BrowserTree::mount_with_svg_layers` or
`mount_with_factory_and_svg_layers`; change an existing program through
`BrowserTreeHandle::set_svg_program`. `svg_layer_frame(key)` reads the accepted
frame of any declared builtin or custom occurrence, without creating a child
component for a builtin. The renderer must be SVG with controlled highlighting,
transformed source Geometry picking, no crosshair/editable brush, and no forced
animation.

`mount_with_factory_and_svg_layers_and_components` also supports existing
Node/Link/Label Widgets with `source_ordered: true`. Every occurrence has a
full-Scene foreignObject carrier. Pure builtins contain native SVG views; a
builtin containing source components interleaves native SVG runs and Widgets
in source order. Repeated builtins have independent component instances. Custom
content uses an HTML camera inside a fixed viewport-plot clip, with its app box
and coordinates remaining plot-local. Labels are passive; a real painted
decorative link can block a lower
custom subtree even though it has no canonical node control.

## Input and current node

A Mesh occurrence is a real input surface at its literal position. Nearest-node
activity requires that surface, `interactive(true)` and
`BrowserTreePicking::Nearest`. A later custom child can own input above it; a
later mesh can intercept input above a child. Nodes and links also participate
through their projected source paint. Omitting a visual role does not create a
fake hit target. Hidden and replaced source ink is excluded from that projection.
Mesh diagnostics have a separate 512-site Voronoi limit, not a general Tree limit.

`context.current.set_current_node(Some(id))` queues a later owner update;
`None` clears it. This weak capability validates the accepted source generation
and live occurrence. It rejects during construction and after retirement/stop;
it does not flush from a component callback. A first accepted-frame effect may
queue an action. Setting the already requested current node is a no-op.
`interactive(false)` disables automatic source/mesh input but leaves custom
Glory controls and their explicit current-node capability available.

## Transactions, budgets and limits

Accepted source state, source component frames, custom layer frames and managed
tooltip data share one adoption boundary before their update pumps and retired
child cleanup. Constructors and their initial effects still run during candidate
preparation: the custom layer Bond is `None`, and its `initial` input is tentative.
Source components keep their existing candidate-frame construction contract.
Synchronous source preparation, owned mask/carrier DOM writes and semantic
placement checks can reject before adoption and preserve the accepted data,
owned presentation and gesture. Glory 0.8 queues managed renderer writes for a
later runtime pump. This bridge does not provide a rollback transaction for those
managed backend writes: a renderer backend error can leave an applied prefix and
fault its runtime. Remount the owner after such a backend fault. Reactive property
errors are reported on the adopted frame, with the affected presentation hidden;
they have their separate recovery path. Data adoption before effects does not
imply that managed native DOM writes completed before adoption.
A failed desired program is attempted once; ordinary native input continues
with the accepted program. An explicit refresh or authored update can retry it.
Changing only order or repeats does not replay layout/style functions; source
placement is reused when its role selection is unchanged. Camera/hidden retargets
reuse custom children; idle flushing does not reconstruct them.

`TreeSvgBudget` bounds layers, repeated source work, path commands and the shared
live/pending component count. Retained children and a candidate's new children
must fit together; rejection does not retire the accepted children to make room.
Defaults cap layers and components at 128, work at 200,000 and source commands at
2,000,000. These are admission limits, not a promise about arbitrary child cost.

This API does not provide React/Spring transitions, raw React SVG/defs insertion,
or Nivo's seven mouse event kinds for each node/link family (14 SVG handler
props). The SVG link generator
does not bind a Canvas context. Exporting the source Scene does not reproduce
foreignObject Glory subtrees or literal repeated layer views. GPU/worker rendering
and those remaining event/export contracts are separate work.
