# Tree native mouse hooks (Unreleased)

[简体中文](tree-events.zh-CN.md)

The browser adapter provides seven independent node handlers and seven link
handlers. The native input contract has passed the complete matrix across five browser
configurations and two DPRs. See the [verification record](verification.md#development-tree-native-mouse-hooks)
for the tested profiles and boundaries. Existing constructors,
`BrowserTreeOptions`, `on_event` and `on_update` keep their interfaces.

## Register handlers

```rust
use kartis::Result;
use kartis::browser::{
    BrowserTreeHandle, BrowserTreeMouseHandlers, BrowserTreeNodeMouseActivity,
};

fn display_node_clicks(
    tree: &BrowserTreeHandle,
    mut display: impl FnMut(String) + 'static,
) -> Result<()> {
    let mut handlers = BrowserTreeMouseHandlers {
        node_activity: BrowserTreeNodeMouseActivity::NativeComponent,
        ..Default::default()
    };
    handlers.nodes.on_click = Some(Box::new(move |event| {
        display(format!(
            "{}: raw={}, aggregate={}, position={:?}",
            event.data.datum().label,
            event.data.datum().value,
            event.data.node().value,
            event.input.scene,
        ));
        Ok(())
    }));
    tree.set_mouse_handlers(handlers)
}
```

Both `handlers.nodes` and `handlers.links` have `on_mouse_enter`,
`on_mouse_move`, `on_mouse_leave`, `on_mouse_down`, `on_mouse_up`, `on_click`,
and `on_double_click`. Each is an independent optional boxed `FnMut` returning
`kartis::Result<()>`. Replacing this bundle does not replace `on_event` or
`on_update`, rebuild the source, or replay component factories.
`clear_mouse_handlers()` removes this channel. A bundle with no callbacks does
not enable the new input policy.

## Event data and timing

`BrowserTreeNodeMouseEvent` and `BrowserTreeLinkMouseEvent` contain immutable
typed `data`, the accepted `source_generation`, and `input`. `TreeEventFrame`
shares the complete raw source, computed nodes and links, resolved link styles,
activity, theme, configuration and source Scene. Raw values preserve signed
zero; aggregates are separate. Node indices use original flat source order;
computed link order and endpoint source indices are distinct. Selecting or
cloning a target shares this pool without replaying layout or style functions.

An optional `data.mark()` is an original source Mark, not proof of a physical
hit. Unpainted, zero-sized or empty-label nodes and disabled links can retain
valid typed data without a canonical interaction site. Current source paint
edits do not rewrite constructor-computed centers or link styles.

`tree.source_generation()` reads the currently adopted source generation
without rebuilding data. It includes accepted activity updates and is distinct
from Core revision and literal-layer publication. Compare it with a saved
event's generation before applying an asynchronous result. The getter rejects
after stop or drop; it can read an adopted frame during a component effect.

`input` carries the actual `web_sys::MouseEvent`, semantic `kind`, exact client,
logical viewport and inverse-camera Scene coordinates, optional pointer
identity, explicit routing `origin`, and optional literal SVG occurrence
key/index/publication. Pointer events are MouseEvent subclasses; their actual
native type remains observable. A derived Mesh Leave can therefore carry an
actual `pointermove` event. Repeated occurrences do not duplicate business IDs.
Coordinates and pointer-capture flags describe the envelope's native capture
snapshot. Chart transport may establish capture before the late callback, so a
Down snapshot can report `captured == false` even though the Surface already
owns capture when the callback runs. The original event remains a live DOM
object; its `current_target()` is not a persistent snapshot after dispatch.

Delivery occurs during the same native event's late Window bubble phase after
required chart transport has been reserved, outside owner and callback borrows.
It is not reconstructed from RAF updates, `SurfaceEvent::Activated`, or Up.
The payload captures the event's accepted source before an application callback
can replace it. Automatic current/tooltip intent can already be queued while
the payload still contains the previously rendered computed data. A callback
that flushes does not rewrite its captured payload.

Delivery requires an event from the browser. A default context menu can interrupt
later pointer input; hosts that require secondary-button release callbacks can
handle `contextmenu` and prevent its default action explicitly. Registering Tree
hooks preserves the browser's default menu behavior.

Actual DOM keyboard Click events remain Click events, including a PointerEvent
whose detail is zero and pointer identity is not primary. Programmatic
`SurfaceInput` activation does not fabricate a native Click. Synthetic DOM
events expose `is_trusted() == false`. The legacy Owner/source-model profile can
still update activity from a synthetic DOM Move; managed tooltips retain their
trusted-input rule. In NativeComponent, synthetic native-node/link/blank Move
preserves explicit current and does not establish a trusted tooltip; synthetic
Enter/Leave do not reserve new current or tooltip. These are browser protocols,
not OS pen or touch certification.

## Routing profiles

| Route | Contract |
| --- | --- |
| Ordinary SVG | Actual eligible node/link paths take precedence. Under Nearest, other Surface positions use the explicitly identified source-model nearest route. This hybrid is a Rust profile, not an invisible Nivo Mesh layer. |
| Literal SVG layers | Actual order, omissions, repeats, blockers, hidden/replaced ink and accepted occurrence identity apply. Only a real Mesh target or its matching captured contact enables that Mesh route. Custom UI does not fall through. |
| Mesh / source-model nearest | Native Enter into the surface queries the nearest node and can produce Enter. A first matched Move also produces Enter; same node produces Move; A to B produces Leave(A) without another Enter/Move on that dispatch. Discrete Down/Up/Click/DoubleClick query their actual coordinates and update internal comparison state without setting external current or tooltip. |
| Canvas Geometry | Original painted source geometry defines logical roles. A Move from blank to a node produces Enter; A to B produces Leave(A) then Enter(B); a later same-role Move produces Move; node to blank produces Leave. These are declared source-model transitions, not native per-node DOM events. |
| Canvas / raw Canvas | Node hooks only; registering link hooks or binding a link rejects before replacing the accepted registration. Raw callback pixels do not automatically become hit geometry. |
| Captured Source | Current coordinates are queried again. A blank Geometry Up/Click does not invent a hit on the original Down node. The matching contact retains its SVG occurrence; `CapturedSource` distinguishes this query from the current native DOM target. |
| Glory UI | Unbound components, labels, custom layers, overlays and nested charts own their input. Explicit binding is required to forward a business role. |

Stable business IDs, occurrence keys and binding identities normalize logical
hover across accepted frame updates; DOM repaint alone does not create another
Enter. A source reorder is resolved against the event's current accepted frame.
This differs from Nivo's positional Mesh array cursor and React effect timing.
Removed or retired targets cannot be relabeled as another source.

A real pointer Out can close a previously observed hover after a camera update
moves that node away from the stationary pointer. Its historical DOM path may
already be passive; Leave describes the old hover, not a new hit on that path.
The role must still resolve in the accepted source. This does not authorize
Enter, Move or discrete callbacks from retired or inert paint.

## Current node and double-click

`BrowserTreeNodeMouseActivity::Owner` keeps the existing automatic native-move
activity profile. With active hooks, a real native SVG or explicitly bound
node's Enter/Leave also reserves current and tooltip before its callback.

`NativeComponent` adds a component-style profile for actual SVG nodes and
explicitly bound nodes, including bound Glory UI over Canvas. Enter/Leave change
current; node Move changes only the node tooltip. Link/blank Move in SVG Geometry
does not replace an explicitly set current. A matching captured native-node
gesture retains this policy. Canvas source-model Geometry, Nearest and raw
Canvas retain their owner/model activity behavior. This policy requires an
active callback registration; it is not a separate global chart mode.

`BrowserTreeDoubleClickBehavior::KeepChartReset` is the default, even when a
DoubleClick hook exists. `CallbacksOnlyOnHandledTarget` suppresses Reset only
for an eligible target with an actual DoubleClick callback. Blank targets,
cleared registrations and ordinary generic charts keep their reset behavior.
Two native Clicks and their DoubleClick are separate events.

## Explicit component bindings and lifecycle

Call `bind_node_mouse_element(source_id, element)` or
`bind_link_mouse_element(target_source_id, element)` after the actual element
belongs to an accepted owned subtree. A Tree link is identified by its child
source ID. Retain the returned `BrowserTreeMouseBinding`; dropping it or calling
its idempotent `unbind()` removes the binding. The innermost binding wins.
Disconnected, inactive, cross-owner and nested-chart targets reject. Candidate
constructors cannot bind an unaccepted element. Native Scene labels are passive;
real Glory Label Widgets retain their own UI behavior.

Binding does not fabricate a physical Mark or change canonical selection. Its
origin is `BoundComponent`; data comes from the current accepted source. A
removed element/role, retired occurrence or stopped owner cannot deliver a
stale event. Callbacks can replace their registration or source, flush, unbind,
or stop; a returning old callback cannot overwrite a newer registration.
Successful explicit source replacement retires the old Up/Click lineage;
precommit rejection preserves the accepted gesture.

Callback errors are reported through `BrowserEvent::Error`. They do not roll
back application side effects or native transport already reserved. Propagation
stopped before the late observer does not cause an undelivered envelope to be
replayed later. Cleanup is independent of activity RAF scheduling.

This API does not promise React SyntheticEvent identity, arbitrary generic
Datum fields, D3 tie compatibility, native OS device equivalence, automatic
Spring exits or live Widget export. Glory 0.8's separately documented managed
backend error/remount boundary still applies. GPU/worker rendering remains
separate work.
