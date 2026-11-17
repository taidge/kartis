# Funnel native mouse events (Unreleased)

[简体中文](funnel-events.zh-CN.md)

This Unreleased API extends the dedicated SVG `BrowserFunnel` owner. Its native
events and tooltip browser acceptance has passed: 10 configurations, 40 profiles,
2,000 frames, 160 rejections and 20 real DOM negative controls. See the
[verification record](verification.md#development-funnel-native-events-and-tooltips)
for artifacts and the fixed Chromium `--disable-gpu` test configuration. See
[literal SVG layers](funnel-svg-layers.md) for mounting and source transactions,
and [tooltips](funnel-tooltip.md) for standard text and retained Glory components.
These hooks do not add Canvas picking, Mesh routing or arbitrary element binding.

## Install four independent hooks

`BrowserFunnel` and its weak `BrowserFunnelHandle` expose
`set_mouse_handlers(BrowserFunnelMouseHandlers)` and `clear_mouse_handlers()`.
Installing a bundle immediately replaces all four slots; it does not rebuild the
source. Keep the owning `BrowserFunnel` alive. These examples use the browser API
on the `wasm32-unknown-unknown` target.

```rust
use kartis::ChartError;
use kartis::browser::{
    BrowserFunnelHandle, BrowserFunnelMouseActivity, BrowserFunnelMouseEvent,
    BrowserFunnelMouseHandlers,
};

fn observe(event: BrowserFunnelMouseEvent) -> kartis::Result<()> {
    let datum = event
        .data
        .source()
        .get(event.source_index)
        .ok_or_else(|| ChartError("missing event source row".into()))?;
    let part = event
        .data
        .parts()
        .get(event.part_index)
        .ok_or_else(|| ChartError("missing event part".into()))?;
    let original_event = event.input.event.clone();
    let _observation = (
        datum,
        part,
        original_event,
        event.source_generation,
        event.publication,
        event.input.occurrence,
    );
    Ok(())
}

pub fn install_handlers(owner: &BrowserFunnelHandle) -> kartis::Result<()> {
    owner.set_mouse_handlers(BrowserFunnelMouseHandlers {
        on_mouse_enter: Some(Box::new(observe)),
        on_mouse_move: Some(Box::new(observe)),
        on_mouse_leave: Some(Box::new(observe)),
        on_click: Some(Box::new(observe)),
        activity: BrowserFunnelMouseActivity::NativeComponent,
    })
}
```

| Slot | Semantic kind | Original DOM event |
| --- | --- | --- |
| `on_mouse_enter` | `Enter` | `pointerover` |
| `on_mouse_move` | `Move` | `pointermove` |
| `on_mouse_leave` | `Leave` | `pointerout` |
| `on_click` | `Click` | `click` |

Each callback receives the actual `web_sys::MouseEvent` object, including its
`PointerEvent` identity when applicable. Click is not reconstructed from Up,
Core activation or another hook. Eligible events are captured at Window capture
and delivered synchronously at Window bubble: during the callback,
`event.event_phase()` is 3 and `current_target()` is Window. A changed
registration or accepted presentation before delivery invalidates the captured
admission; the owner does not attach an old event to a newer frame. Normal DOM
propagation still applies: a stopped event need not reach Window bubble.

## Original data and actual targets

`BrowserFunnelMouseEvent` contains the complete immutable `FunnelSvgFrame`,
`source_generation`, `publication`, computed `part_index`, original
`source_index` and `input`. The frame provides captured config, original source,
options, layout, computed styles and actual Scene ink. Indices refer to this
frame, not a later owner snapshot. Cloning the frame does not replay factories.

`input` contains `kind`, the original `event`, `client`, logical `viewport`,
camera-inverted `scene`, optional pointer facts (`id`, `kind`, `primary`,
`captured`), and `occurrence { key, index, publication }`. Coordinates use the
geometry accepted when that event was captured. Literal repetitions share the
business source but retain distinct occurrence keys and indices. Source
generation identifies an accepted source rebuild; publication identifies the
accepted SVG source/layer presentation. Neither is an event sequence number.

Only an actual connected body path in the accepted occurrence can supply a
part. Borders, separators and labels remain passive. A custom layer's real
button can block the body beneath it. Omitted Parts, missing bodies, hidden
series and disabled interaction do not fabricate targets. SVG `visiblePainted`
rules apply: opacity-zero paint can still receive native pointer input. This is
not a nonzero-alpha pixel hit test or a solid-only approximation.

Pointer hover phases require a primary, non-touch pointer when pointer metadata
is present. Click excludes touch but does not reject `isPrimary == false`:
browser-generated mouse clicks and keyboard clicks can have that value. A
keyboard click still needs an actual accepted body target. There is no
`isTrusted` security gate; connected owned DOM events retain ordinary DOM
semantics, and `is_trusted()` reports the original fact. Physical acceptance
uses real trusted input rather than promoting synthetic input to trusted input.

## Current, tooltip and stable hover identity

Existing mounts with neither handlers nor a configured tooltip retain legacy
`Owner` activity. `BrowserFunnelMouseHandlers::default()` selects
`NativeComponent`. Configuring a tooltip also selects that policy unless a
handler bundle explicitly selects `Owner`. Clearing handlers restores the
tooltip policy if configured, otherwise legacy Owner behavior.

Under `NativeComponent`, Enter queues current and tooltip selection before the
application hook; matching Leave queues clearing both. Move updates tooltip
selection only and preserves manual current. Click invokes only the hook: it
does not select current, show a tooltip or move one. These reservations enqueue
work; accepted getters inside the callback still expose the accepted snapshot.
Owner retains its previous automatic-current/recovery behavior, including a
real Move retry after a rejected native-current batch. That recovery does not
invent a typed Enter.

Physical hover identity is the occurrence key plus original body Mark ID.
Source-driven DOM replacement can produce another raw `pointerover` for that
same business role. While its hover lease remains live, that Over does not emit
another typed Enter or repeat Enter's automatic current/tooltip side effects.
A subsequent real Move remains an independent Move. A different occurrence or
a genuine Leave followed by reentry can Enter again. Move is never rewritten
into Enter, and raw delivered events remain observable independently of these
typed observations.

## Queue updates from a callback

Setters may enqueue changes during a hook. `flush()` inside a hook returns an
error; it must not adopt data or pump component runtimes halfway through native
dispatch. RAF processes queued work after dispatch, or application code can
flush outside the callback. The next example replaces the entire handler bundle
with a Click updater; it is an alternative installation to the first example.

```rust
use kartis::ChartError;
use kartis::browser::{BrowserFunnelHandle, BrowserFunnelMouseHandlers};

pub fn install_click_update(owner: &BrowserFunnelHandle) -> kartis::Result<()> {
    let queued_owner = owner.clone();
    owner.set_mouse_handlers(BrowserFunnelMouseHandlers {
        on_click: Some(Box::new(move |event| {
            let mut next = event.data.source().to_vec();
            let datum = next
                .get_mut(event.source_index)
                .ok_or_else(|| ChartError("missing clicked source row".into()))?;
            datum.value *= 0.9;
            queued_owner.set_source(next)
        })),
        ..BrowserFunnelMouseHandlers::default()
    })
}
```

The queued source uses the historical Click frame intentionally; applications
that merge concurrent edits should choose their own conflict policy. The
callback's frame never changes when the queued update is later accepted.
Source and tooltip commands in the same batch accept or reject together. A
rejected source batch cannot be bypassed by accepting its tooltip changes.
External callback side effects are not rolled back.

A callback can replace or clear handlers without the returning old callback
overwriting that new registration. Recursive dispatch of another owned native
event is rejected before its automatic intents or callback are delivered; the
owner does not stop unrelated DOM listeners. Returning `Err` is recorded in the
bounded `snapshot().event_errors` log. Later successful flushes do not erase that
log. Source rejection (`error`) and postcommit UI errors (`ui_errors`) are
separate channels; Rust/JavaScript traps are not promised to behave as a returned
`Result::Err`.

`stop()` is allowed from a callback and is idempotent. Stop/drop removes native
listeners, pending envelope cleanup, RAF/resize work and tooltip presentation.
No further callbacks are delivered; mutators reject stopped ownership. An
inactive snapshot remains readable and `tooltip()` returns `Ok(None)` after
stop. A weak handle cannot keep a dropped owner alive.

## Tooltip time and browser limits

A hook is historical: its Frame, generation, publication and occurrence belong
to its original native capture. A tooltip is current presentation: its last
selected typed part ID and occurrence are rebound against the newly accepted
source/plan. Its frame and certificates therefore refresh after source or
presentation changes; removed/omitted bodies clear selection. This explicit
rebasing differs from retaining Nivo's captured part object. Pointer-only
tooltip updates retain the source generation and SVG publication; cursor
position is not a new source/layer epoch. See [tooltip timing and
placement](funnel-tooltip.md).

WebKit can deliver no window-exit event at all after a hovered path is replaced
under a stationary pointer. A plain-SVG replacement control reproduces this;
there is no original Out for the owner to observe. Delivered Out from a current
accepted body can clear the matching lease even with coordinates outside the
viewport. Tests first deliver a real Move to the replacement body before
checking its actual Out. No timeout or fabricated global fallback claims to
recover an undelivered event. The [SVG input boundary](funnel-svg-layers.md)
documents this remaining limitation; this API makes no operating-system event
delivery equivalence claim.
