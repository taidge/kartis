# SVG Tree link tooltips (Kartis 0.2.0)

[简体中文](tree-link-tooltip.zh-CN.md)

This SVG adapter is available in Kartis 0.2.0 and has completed the checked browser matrix described
in the [verification record](verification.md#development-tree-svg-link-tooltips).
The API requires `web` on WebAssembly. The pure
`TreeLinkTooltipData` adapter is also available in native Rust.

## Configuration and data

`BrowserTreeHandle` and the owner provide `set_link_tooltip_options`,
`set_link_tooltip_component`, `clear_link_tooltip_component`,
`reset_link_tooltip` and the accepted `link_tooltip` getter. Keep the mounted
`BrowserTree` alive: its handle is weak. Setters enqueue work; `flush()` processes
the shared source and tooltip transaction.

`BrowserTreeLinkTooltipOptions` contains `anchor` and `gap`. The default is
`Top` with a 14 CSS-pixel gap. Placement follows the actual native cursor; this
API has no fixed link-midpoint position. `BrowserOptions.tooltip` remains the
master presentation switch. Canvas, including raw Canvas programs, rejects link
configuration before invoking a link component factory.

Supplying a component enables the link options. Setting options without a
component enables safe plain text with source-then-target rows; this is an
explicit Rust convenience, not a default upstream Nivo link component. Each
text row uses its endpoint's computed aggregate. No scalar value is invented
for the link. `reset_link_tooltip()` removes link management; clearing only the
component retains options and returns to safe text.

`TreeLinkTooltipFrame` carries `data`, accepted `source_generation`, the actual
`pointer_client`, logical `viewport_anchor`, `requested_anchor`, resolved
`anchor`, optional SVG `occurrence`, and the actual route `origin`.
`TreeLinkTooltipData` shares a `TreeLinkEventData` source pool and exposes the
complete configuration, theme, raw source, computed endpoints, original source
indices, computed link index, final constructor style, activity and optional
original Mark. Raw endpoint values preserve signed zero; aggregates are separate.
Cloning or formatting data does not replay layout or style factories.

## A retained Glory component

The component's `Bond<Option<TreeLinkTooltipFrame>>` starts at `None` during
construction. Only shared adoption publishes accepted data. Keeping the revision
retains component state; changing it requests a replacement. Node and link cards
share one adoption boundary and each retain their own component state while the
other role is visible. The accepted getters expose at most one active managed
role. Configuring only a link card preserves the existing generic node tooltip.

```rust
use glory::Widget;
use kartis::browser::{
    BrowserTreeHandle, BrowserTreeLinkTooltipOptions, BrowserTreeTooltipAnchor,
};

fn configure_links(owner: &BrowserTreeHandle) -> kartis::Result<()> {
    owner.set_link_tooltip_options(BrowserTreeLinkTooltipOptions {
        anchor: BrowserTreeTooltipAnchor::Top,
        gap: 14.,
    })?;
    owner.set_link_tooltip_component(1, |context| {
        let frame = context.frame;
        Ok(glory::ui::widget(move |scope| {
            glory::ui::text()
                .content(scope.bond(move || {
                    frame.get().map_or_else(String::new, |frame| {
                        format!(
                            "{} → {} (width {})",
                            frame.data.source_datum().label,
                            frame.data.target_datum().label,
                            frame.data.style().thickness,
                        )
                    })
                }))
                .into_widget()
        }))
    })?;
    owner.flush()
}
```

## Native selection and ordering

Trusted SVG Link Enter/Move reserves link presentation before the independent
application mouse hook. A hook that flushes can observe the newly adopted card;
its captured event payload still refers to the event's original accepted frame.
Link Leave clears only its matching selection lease. Down/Up/Click/DoubleClick
do not establish or move a link card. Link presentation does not set node current
and works without registering a dummy public mouse handler.

Actual eligible native paths, literal order and explicit owned Glory bindings
determine selection. A higher Mesh or custom UI blocks the lower native link.
Omissions, disabled links, zero width, transparent paint and source removal do
not create automatic link hits. Repeated links retain the original business
identity and report their actual occurrence key/index/publication. Captured
native geometry uses current coordinates; it does not replay the Down target.

An explicit owned SVG link binding is a logical component route, identified as
`BoundComponent`; an optional original Mark does not itself prove visible ink.
Default Tree links are decorative Marks with an empty series. Hiding an endpoint
node ID does not hide a still-painted link. This API does not change that existing
series-filter contract or add canonical link Sites.

## Anchors and failure boundaries

The five original anchor values are Top, Right, Bottom, Left and Center. For a
requested Left or Right, the link API follows Nivo's horizontal resolution rule:
compute cursor X in parent CSS space, compare it with half the parent's actual
bounding-box width, then choose Right on the left side and Left otherwise. Both
requested and resolved values are observable. The comparison deliberately keeps
the upstream mixed-unit behavior under scaling; it is not general collision
avoidance. Four corner anchors are fixed Rust extensions. Existing node anchor
semantics do not change.

Cursor coordinates use independent X/Y CSS scaling, with local gap/position
rounding and natural component dimensions. Rotation, skew, perspective, viewport
clamping and automatic Spring exits are separate work.

Source widgets, SVG layers and both tooltip roles adopt together. A rejected
desired component preserves accepted presentation and does not poison later
automatic input. Explicit configuration/refresh can retry it. Stop and retired
source/occurrence/binding leases cannot publish a stale card. Handler errors do
not roll back application side effects.

The separately documented Glory deferred managed-DOM fault/remount boundary
still applies. This feature does not imply Canvas link picking, arbitrary React
nodes or generic Datum fields, OS device certification, live Widget export, GPU
or worker rendering. See the existing [node tooltip](tree-tooltip.md) and
[native event](tree-events.md) contracts for their independent behavior.

## Checked configurations

The matrix runs Chrome, Edge, Firefox, WebKit and Chromium CI at DPR 1 and 2,
with six profiles per context. Chromium contexts explicitly launch with
`--disable-gpu`; every report records the actual launch options. This configuration
bounds the accepted pixel evidence. GPU and worker work remains deferred.

Checks cover actual native SVG paths, retained node/link role state, source and
current-frame coherence, natural shell dimensions, all nine anchors, CSS scaling,
reordering, eligible ink, rejection recovery and cleanup. The new application
callback observation exercises Move with a synchronous public `flush()`; the
existing seven-slot native-event matrix has a separate current-source regression.
The verification record lists the remaining untested routes and preserved
diagnostic failures.
