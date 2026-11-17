# Tree node tooltips (Kartis 0.2.0)

[简体中文](tree-tooltip.zh-CN.md)

The BrowserTree presentation API requires `web` on WebAssembly;
`TreeTooltipData` is also available as pure Rust data. These APIs
are available in Kartis 0.2.0. The tested
opt-in browser profile covers Chrome, Firefox, WebKit, Edge and bundled Chromium
at DPR1/2, with SVG/Canvas and native Canvas source programs. See
[verification](verification.md) for the bounded runtime evidence.

`BrowserTree` has an opt-in tooltip presentation independent of the shared
chart's painted hits. Existing mounts keep their current tooltip defaults.
Call `set_node_tooltip_options` on its handle to enable safe Tree text, or
`set_node_tooltip_component` to supply a real retained Glory Widget. The latter
also enables the default Tree tooltip options. `BrowserOptions.tooltip` must
remain enabled.

## Selection and complete data

A **new trusted primary mouse or pen Move** after enabling establishes the
target. Enabling does not reuse an earlier unmanaged pointer or infer a target
from activity focus. `BrowserTreePicking::Nearest` uses accepted computed node
centers and its configured distance; `Geometry` uses the existing strict source
interaction. Leave requests clearing through the same acceptance boundary.

Tooltip selection is separate from relation activity, focus, selection and
`SurfacePhysicalHit`. Explicitly setting activity does not create a popup, and
dispatching a shared-chart input does not impersonate native tooltip intent.
Nearest mode can present zero-radius, transparent or empty-label nodes without
adding a Mark, canonical control or artificial physical hit. Hidden source IDs
are excluded. Changing source order does not redirect an existing source ID to
another index.

`TreeTooltipData` owns a validated source/layout/activity snapshot. Its accessors
include `source`, `source_index`, `datum`, `node`, `layout`, `activity_layout`,
`activity`, `config`, `anchor` and `is_active`. The layout exposes the complete
node/link pools, resolved link styles and any constructor-computed Canvas labels.
No layout factory is replayed to recover data.

`datum().value` is the original input, including its signed-zero bits;
`node().value` is the computed aggregate. Safe `text(&TooltipOptions)` uses the
aggregate, matching existing Tree text tooltips. Templates remain plain text.
Raw source changes, including an unselected sibling's zero sign, remain
observable in typed data.

## A retained component

This function configures an already mounted owner. Keep the `BrowserTree` alive;
its handle is weak. Setters enqueue work, and `flush` processes it synchronously.
The example uses verified public signatures. The retained frame protocol is
covered by the opt-in browser profile described here.

```rust
use glory::Widget;
use kartis::browser::{
    BrowserTreeHandle, BrowserTreeTooltipAnchor, BrowserTreeTooltipOptions,
    BrowserTreeTooltipPosition,
};

fn configure_tooltip(owner: &BrowserTreeHandle) -> kartis::Result<()> {
    owner.set_node_tooltip_options(BrowserTreeTooltipOptions {
        position: BrowserTreeTooltipPosition::Cursor,
        anchor: BrowserTreeTooltipAnchor::Top,
        gap: 14.,
    })?;
    owner.set_node_tooltip_component(0, |context| {
        let frame = context.frame;
        Ok(glory::ui::widget(move |scope| {
            glory::ui::text()
                .content(scope.bond(move || {
                    frame.get().map_or_else(String::new, |frame| {
                        format!(
                            "{}: {} (raw bits: {:016x})",
                            frame.data.datum().label,
                            frame.data.node().value,
                            frame.data.datum().value.to_bits(),
                        )
                    })
                }))
                .show_in(scope)
        }))
    })?;
    owner.flush()
}
```

The factory receives `BrowserTreeTooltipComponentContext { revision, frame }`.
`frame` is a `Bond<Option<TreeTooltipFrame>>`, initially `None`. A frame carries
typed data, source generation, last native client position, logical viewport
anchor and alignment. Handle `None` in every reactive view: it also covers an
absent target or unavailable presentation box.

Changing nodes or source values retains the component's local state. The same
revision retains its existing factory and Widget; a new revision rebuilds it.
`clear_node_tooltip_component` returns to safe text. Once that clear is accepted,
installing the former revision constructs a new component.
`reset_node_tooltip` clears managed tooltip selection and restores the existing
Core tooltip profile, without setting or clearing activity.

Requested configuration stays separate from the accepted configuration. After a
new component or configuration is rejected, native Leave/re-entry uses the
accepted one; an explicit source update or `refresh` retries the requested one.
A rejected reset does not clear the accepted selection or cache. Only adoption
of the reset switches to Unmanaged.

The wrapper defaults to `pointer-events:none`. Custom content does not acquire
node activation handlers automatically. Keyboard focus and direct input do not
establish native tooltip intent; no touch or OS-focus contract is added.

## Position and publication

Defaults are `Fixed`, `Top` and a 14-local-CSS-pixel gap. Fixed positioning follows
the computed node center through the accepted camera; cursor positioning follows
the last delivered native pointer. Natural content size is bounded by the
surface-derived maximum width/height. The nine alignments are `Top`, `TopRight`,
`Right`, `BottomRight`, `Bottom`, `BottomLeft`, `Left`, `TopLeft` and `Center`.
Center uses no directional gap.

Anchor coordinates and gap are rounded together in local CSS space. Axis-aligned
scaling on a common ancestor or the surface is accounted for before placement;
DPR is separate. Arbitrary CSS rotation, skew and perspective are not covered.
There is no automatic collision avoidance: the box can extend outside the plot.

A `None` or unmanaged tooltip does not itself require a resolved CSS box.
Joint source label components have a separate requirement: their automatic
affine carrier needs resolved CSS dimensions. `display:none` can leave sizes
such as `100%` unresolved and reject a joint source candidate. Accepted data
remain in place; make the source visible and explicitly `refresh` to recover.

Moving within the same target reuses its immutable data cache and updates tooltip
presentation without replaying source factories, source placements or native
Canvas renderers. Programmatic current changes remain distinct from the native
tooltip target. The original press lifecycle still defers automatic updates
through release and its derived click.

Source and tooltip data share an adoption boundary with retained source
components. Before adoption, construction/placement/paint errors preserve the
accepted source and tooltip. After adoption, dynamic Widget errors are reported
on the accepted data; a failed presentation is hidden rather than displayed as
old-source content. External application side effects cannot be rolled back.
Persistent source-builder, source-placement or native-renderer errors preserve
the jointly accepted source and tooltip. Native Leave does not bypass those
errors through a tooltip-only fallback; repair the source error and refresh.
`node_tooltip()` reads accepted logical data, not a guarantee that the popup is
currently visible. Empty safe text can hide a popup while custom content still
uses the typed frame. Stop retires the owned component.

Tooltip Widgets are HTML presentation, including over a raw Canvas program.
They are excluded from native Canvas PNG export and source Scene exports.

This adapts Nivo's optional `nodeTooltip({ node })` content contract. It does not
execute React/Spring, recover arbitrary generic Datum fields or nested path UIDs,
or promise Delaunay tie behavior. Rust retains flat source IDs, bounded nearest
distance and its existing plot input domain. GPU/worker work remains deferred.
See [Tree activity](tree-activity.md), [native Canvas programs](tree-canvas.md)
and [verification](verification.md) for the separate contracts and evidence.
