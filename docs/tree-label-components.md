# Tree label components (Kartis 0.2.0)

[简体中文](tree-label-components.zh-CN.md)

Available in Kartis 0.2.0, this API adds real Glory label Widgets alongside Tree node and link
Widgets. Existing Tree drawing
and the older `TreeWidgetData` node/link API keep their defaults.

`tree_components` returns one `SceneWidgetsData<TreeComponentData>` packet with
`Node`, `Link` and `Label` roles in actual Scene paint order. Return `Ok(None)`
from placement to keep a role as ordinary Scene ink. `tree_label_widgets` selects
only labels and returns strongly typed `TreeLabelWidgetData` instead. Both accept
a revision and a borrowed placement function. The combined limit is 128 slots.

A label role references its real Scene label, not a synthetic node. Its key is
`tree-label:<original source ID>`, independent of the label's current ordinal.
Its context includes the original flat source and datum, computed node, actual
label, resolved font, whitespace policy, label clip and plot. Owned data includes
the full source, node, link and label pools. Labels with empty text, transparent
color, or a zero-size node remain real roles. An unowned annotation is omitted;
multiple managed labels for the same owner are rejected before placement runs.
No base or activity style function is replayed to build the component packet.

For example, select only labels and use a measured box relative to each label's
actual anchor:

```rust
use kartis::*;

fn label_components(layout: &TreeLayout) -> Result<SceneWidgetsData<TreeLabelWidgetData>> {
    tree_label_widgets(layout, 0, |_label| {
        Ok(Some(PlotWidgetPlacement::AtDatum {
            offset: [-24., -9.],
            measured: TextSize { width: 48., height: 18. },
        }))
    })
}
```

Wrap the packet in `SceneWidgetsComposition`; `replace_ink` contains **slot keys**.
Replacing a label removes only that label's painted text. Node/link geometry,
business values, anchors and physical hit targets remain. Source label metadata
remains available to the component. Overlay composition places children above
the ordinary surface; source-ordered composition inserts each child at its actual
source item and supports label-only replacement between other paint items.

`Chart::scene_widgets_composed` mounts axis-aligned components in semantic/native
hosts. BrowserChart has `mount_with_scene_widgets_composed` and
`mount_with_scene_widgets_in_scene`, with corresponding setters on its handle.
Their factory receives `SceneWidgetCompositionContext<T>` containing a retained
Bond of `TypedSceneWidgetFrame<T>`. The frame is `Mark` or `Label`; the application
can return any Glory Widget. A stable key/revision preserves its local state while
accepted frame data changes. Changing a frame between Mark and Label requires a
new revision. Node and link roles are both Mark frames. All roles share one
prepare/commit transaction; factory/build/placement/paint errors
before adoption preserve the old accepted frame and child data.

BrowserTree's `mount_with_components` and `mount_with_factory_and_components`
connect all three roles to the existing owner and activity transaction. Placement
receives `TreeActivityComponentContext`; the component factory receives
`SceneWidgetCompositionContext<TreeActivityComponentData>`. Label activity is its
source node's activity. The older `mount_with_widgets` API remains available.

Measurements describe the component's local box. To fill that box with a button,
set its root to `Display::Block` and its width/height to `Length::Percent(100.)`.
An inline button participates in a text line's baseline layout and can be offset
inside an otherwise correctly placed carrier.

Automatic affine placement requires resolved CSS dimensions for the source
label carrier. Under `display:none`, percentages such as `100%` can remain
unresolved and reject a joint source candidate. The accepted source/component
data are retained; restore visibility and explicitly refresh the owner. This is
separate from a `None` or unmanaged [Tree tooltip](tree-tooltip.md), which does
not itself require a display box. Persistent source errors do not permit a
tooltip-only fallback around the joint transaction.

`AtDatum` label placement rotates the whole measured child box and its offset
around the actual label anchor, then applies the current camera. The browser
uses a real CSS affine transform on the child subtree and resolves independent
horizontal/vertical surface scaling; canvas DPR is separate. `Rect` requests an
explicit axis-aligned box and does not inherit label rotation. Native hosts reject
automatically transformed label placement because their current ordinary Widget
carrier cannot rotate a subtree. Neither an axis-aligned bounding box nor a text
callback is treated as a rotated Widget implementation.

For a persistent pointer opt-in, declare `PointerEvents::Auto` in the child's
Glory Style or an application stylesheet. Direct edits to a Widget's managed
inline style can be replaced by its next reactive Style update.

The label wrapper defaults to `pointer-events:none`, matching the inspected Nivo
SVG Labels container. This does not mean `inert`: fully visible custom content may
receive keyboard focus, and its child can explicitly opt into pointer events.
Partially clipped content remains passive. Kartis does not automatically attach
node datum handlers to a custom label. The caller must use actual frame visibility
and metadata when adding its own behavior.

Nivo's SVG `labelComponent` receives a computed label plus animated position and
rotation. Kartis provides accepted static/live frames, without React/Spring exit
animation. Nivo Canvas's separate `renderLabel(ctx, {label, theme})` is still a
different extension point: a Glory DOM overlay over Canvas does not turn into a
raw Canvas renderer or an exported SVG/raster control. Existing Rust labels keep
their alphabetic baseline and projection inset; no Nivo baseline parity is claimed.
The later [native Canvas program](tree-canvas.md) implements that separate
callback extension without converting retained DOM Widgets into Canvas pixels.

See [Tree Widgets](tree-widgets.md), [Tree activity](tree-activity.md) and
[verification](verification.md) for the existing contracts and recorded checks.

The inspected upstream contract is recorded in [Labels.tsx](https://github.com/plouc/nivo/blob/0dae2c32052a573f9f1d66ec1b453b572119b265/packages/tree/src/Labels.tsx) and [TreeCanvas.tsx](https://github.com/plouc/nivo/blob/0dae2c32052a573f9f1d66ec1b453b572119b265/packages/tree/src/TreeCanvas.tsx).
