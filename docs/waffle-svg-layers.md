# Literal Waffle SVG layers and retained Glory components

Unreleased implementation. Native checks pass 2095 default tests, 2440 all-feature
tests and 18 doctests, with strict native/WASM checks. The complete matrix passes
10 browser/DPR contexts, 40 profiles, 2280 captures, 2160 operations, 280 candidate
rejections and 20 actual DOM negative controls with unchanged source. Independent
readback also passes, decoding 3100 PNGs and checking the complete raw/event and
component evidence. See
[verification](verification.md#development-waffle-literal-svg-layers).
CI wiring is not evidence that remote CI has run. [中文](waffle-svg-layers.zh-CN.md)

`WaffleSvgFrame` captures a complete build. `WaffleSvgPlan` orders native Cells,
transparent Areas, Legends and custom whole layers. `BrowserWaffle` and
`BrowserWaffleData<D>` own their real SVG and Glory subtrees; they do not mount a
hidden second chart to simulate input. Existing layout, Canvas, raster and
generic cell-Widget APIs remain available.

## A real whole-layer component

```rust
use glory::Widget;
use glory::render::EventKind;
use glory::style::{Length, PointerEvents, Style};
use kartis::browser::{BrowserWaffle, BrowserWaffleSvgProgram};
use kartis::*;

fn mount(root: web_sys::Element) -> Result<BrowserWaffle> {
    let program = BrowserWaffleSvgProgram::new(vec![
        WaffleSvgLayer::new("cells", WaffleSvgLayerKind::Cells)?,
        WaffleSvgLayer::new("areas", WaffleSvgLayerKind::Areas)?,
        WaffleSvgLayer::new("summary", WaffleSvgLayerKind::Custom("summary-ui".into()))?,
        WaffleSvgLayer::new("legends", WaffleSvgLayerKind::Legends)?,
    ])?
    .custom("summary-ui", 0, |context| {
        let frame = context.frame;
        Ok(glory::ui::widget(move |scope| {
            let count = scope.cage(0u32);
            glory::ui::button()
                .style(
                    Style::new()
                        .width(Length::Px(160.))
                        .height(Length::Px(32.))
                        .pointer_events(PointerEvents::Auto),
                )
                .label(scope.bond(move || {
                    frame.get().map_or_else(
                        || "Preparing".to_owned(),
                        |f| format!("{} groups; clicks={}", f.data.groups().len(), count.get()),
                    )
                }))
                .on(EventKind::Activate, move |_| count.revise(|n| *n += 1))
                .show_in(scope)
        }))
    })?;
    BrowserWaffle::mount_with_svg_layers(
        root,
        vec![RadialDatum::new("ready", 8.), RadialDatum::new("later", 5.)],
        ChartConfig::default(),
        WaffleLayoutOptions {
            rows: 3,
            columns: 5,
            total: Some(20.),
            allocation: WaffleAllocation::IndependentRound,
            ..Default::default()
        },
        program,
    )
}
```

Keep the returned owner alive. `handle()` is weak; dropping or stopping the
owner removes its listeners, scheduled work, tooltip and component runtimes.
Mutators then reject. Snapshots and historical owned callback frames have their
documented inactive/history semantics; `tooltip()` returns `None` after stop.

Every occurrence has a unique key. Repeated built-ins draw real repeated native
content; repeated custom registries have independent occurrence scopes. The
default sequence is Cells, Areas, Legends. Omission changes actual painting and
input while preserving the complete source pool. Legends are empty by default.
Use `with_legends(Vec<WaffleSvgLegendSpec>)` for actual native legend items.
Each spec supplies `LegendOptions`, `LegendLayoutOptions` and a fixed author
`item_size`; the implementation does not guess text width. Their flow/alignment
settings must agree. This is the supported Rust legend vocabulary, not every
Nivo legend-package prop.

A custom context supplies `key`, `revision`, tentative `initial`, and
`frame: Bond<Option<BrowserWaffleSvgLayerFrame>>`. Its constructor starts with
`None`; `initial` is not an accepted snapshot. An adopted frame contains data,
the actual presentation plan, source generation, publication, occurrence key
and index, view, source-hidden IDs and interaction policy. The same
key/registry/revision retains its real subtree and local state across source,
reorder, resize and camera changes. A revision change or removal retires it.
Restoring a removed occurrence constructs a fresh scope.

Whole-layer coordinates cover the full Scene, including margins. The host
applies the camera once and clips to the viewport, without an extra plot clip.
An empty custom carrier is passive; an application button explicitly enables
pointer events as above. Source-level explicit clipping still applies.

## Three different geometries

**Cells** draw actual captured Marks, including custom paths, stroke, fill paint,
rounding and empty cells. They preserve SVG `visiblePainted` target behavior but
have no group mouse hooks. A painted empty cell can block a lower button without
inventing a zero-valued group. Opacity zero is not a promise of SVG hit-through.

**Areas** are real black-filled paths with `fill-opacity=0`, built from occupied
full-slot group polygons. They include cell padding and remain independent of
offset or nonrectangular custom cell ink. They are native event planes, not
extra canonical source Marks/sites or the colored union from
`WaffleMode::Areas`. Only an actual accepted Area element authorizes a group
event. There is no coordinate fallback from a cell, empty slot, nested chart or
custom button. Omitting Areas removes that group route. Moving Areas above a
button can block it; moving Cells above Areas can block group input on cell ink.

**Legends** have real transparent item-background targets. Symbols and text are
passive; backgrounds can block lower Areas or custom UI without producing group
hooks. `interactive(false)` disables group hooks and tooltips, while Cells,
Areas and legend backgrounds retain their actual native hit/occlusion planes.

The new SVG plan requires `WaffleMode::Cells`. Existing colored Areas mode and
its generic adapters retain their old behavior. New SVG construction/shared-Arc
data entrypoints allow `IndependentRound` requests exceeding grid capacity:
`requested` retains the full range and `occupied` is clipped to available slots.
Their polygon profile also merges corner-touching occupied runs. Legacy layout
and Vec-based data constructors retain their prior total/admission and polygon
rules; these are explicit profiles, not a silent change to old charts.

## Original application data without cloning D

```rust
use std::cell::Cell;
use std::rc::Rc;

use kartis::browser::{
    BrowserWaffleData, BrowserWaffleDataMouseHandlers, BrowserWaffleDataSvgProgram,
};
use kartis::*;

struct Order {
    code: String,
    edits: Rc<Cell<u32>>,
}

fn mount_orders(root: web_sys::Element) -> Result<BrowserWaffleData<Order>> {
    let source = vec![WaffleDatum::new(
        Order {
            code: "PO-7".into(),
            edits: Rc::new(Cell::new(0)),
        },
        RadialDatum::new("ready", 8.),
    )];
    let chart = BrowserWaffleData::mount_with_factory_and_svg_layers(
        root,
        source,
        ChartConfig::default(),
        WaffleLayoutOptions::default(),
        BrowserWaffleDataSvgProgram::defaults(),
        |c| {
            let mut color = |f: &WaffleDataColorContext<'_, Order>| {
                Ok(
                    if f.data.code.starts_with("PO-") && f.data.edits.get() == 0 {
                        Color(30, 120, 180, 255)
                    } else {
                        f.computed.default_color
                    },
                )
            };
            WaffleDataFrame::with_shared_factories(
                c.config,
                c.source.clone(),
                c.options,
                &mut WaffleDataFactories {
                    color: Some(&mut color),
                    ..Default::default()
                },
            )
        },
    )?;
    let queued = chart.handle();
    chart.set_mouse_handlers(BrowserWaffleDataMouseHandlers {
        on_click: Some(Box::new(move |event| {
            let _order = event
                .data
                .data(event.source_index)
                .ok_or_else(|| ChartError("missing original order".into()))?;
            let _original_event = event.input.event.clone();
            queued.refresh()
        })),
        ..Default::default()
    })?;
    Ok(chart)
}
```

The builder context borrows config/options and the captured
`Arc<[WaffleDatum<D>]>`. Use `from_shared`, `with_shared_factories`, or the shared
combined cell-factory constructor. The owner verifies that the result retains
that exact raw source pool and the requested numeric/configuration inputs.
Camera, program and tooltip-position updates do not run this builder. Source,
config/options, explicit refresh, real resize and source-hidden reflow do.

The ResizeObserver reads positive root content dimensions, independently of
external CSS scaling, and queues a rebuild only when accepted dimensions differ.
The real resize controls preserve retained component identity and focus. Stop
disconnects the observer, cancels scheduled work and removes event listeners;
weak-handle mutations and clicks on the former Area cannot revive the owner.

D needs only browser `'static`, without Clone, Eq, Debug, Serialize, Send or
Sync bounds. Raw-only replacement is a new observable capture/generation.
Typed layers, tooltip and native admissions carry the same complete internal
snapshot, not a latest-data lookup or postcommit raw sidecar. The framework
does not deep-freeze application interior mutability or prove that the chosen
numeric projection expresses the application's business meaning.

## Native observations and cursor tooltip

Projected and typed handler bundles expose independent Enter, Move, Leave and
Click slots. They receive complete historical data, source/polygon indices,
GEN/publication, occurrence key/index and the actual original MouseEvent plus
fractional client/viewport/Scene coordinates and pointer facts. Window capture
admits the exact accepted source/stage; Window bubble invokes a matching
registration synchronously. An intervening adoption invalidates the old
admission instead of rewriting its raw data. A replacement-driven same-role
Over does not invent another logical Enter; a Move is never manufactured into
Enter. Dispatched DOM events retain their actual trust value; there is no new
trusted-event privilege claim.

Enter/Move queue a tooltip show, Leave/Click queue hide, then the relevant hook
runs. This is Waffle's Click behavior. Hover does not rebuild source or set a
Funnel-like current part. Primary non-touch pointer hover and non-touch clicks
are separate filters; a derived Click is not rejected merely because its
PointerEvent reports non-primary. Group tooltips work without dummy public
handlers. Standard text/chip uses the computed label, formatted value and group
color; `set_tooltip_component` installs a retained Glory component with first
None and an independently controlled revision.

Setters may enqueue in a hook; synchronous `flush()` there rejects. Handler
replacement does not restore an older registration. Recursive DOM dispatch is
guarded, and callback failures are retained in bounded `event_errors` rather
than erased by a later successful source update. Cursor-only adoption keeps the
SVG source/layer publication; source or presentation changes rebase the current
tip to the accepted cohort. Omitted/hidden/disabled Areas invalidate its lease;
restoring them does not resurrect a stale selection. Historical callback data
stays historical. No software fallback fabricates an exit that a UA never
delivers after replacing its under-pointer DOM target.

## Rejection, budgets and remaining work

Plan work counts every repeated native path/resource/legend item. The shared
component limit counts live, pending and retiring layer/tooltip runtimes before
constructors. Invalid batches discard candidate desired state. Owned native
placement uses a parent/attribute/focus journal; accepted source and all complete
Cages adopt before runtime pumping. User callback side effects are external to
rollback. A post-adoption Widget/backend error is a UI failure: its entire
carrier is latched hidden/inert and recovery requires a new component revision.
A deferred or no-work successful pump does not prove recovery.

An early Chromium rollback attempt exposed twelve differing edge pixels despite
unchanged accepted data and restored DOM. An isolated DOM control reproduced
those same pixels after displaying then hiding the rounded HTML tooltip; moving
and restoring the native path returned its cold image. The native SVG content
now uses `will-change:transform` to keep its paint boundary stable. With that
same configuration, independent cold baselines match the full-history images,
and tooltip and placement round trips remain byte-exact. This is bounded browser
evidence, not a specific GPU/driver diagnosis. Path geometry, native input planes
and the whole-viewport rejection comparison keep their original zero tolerance.

Firefox's native pattern raster can mix colors even at an authored motif-center
sample. The fixed pattern case therefore uses an independent native SVG reference
built only from author inputs in the same browser/DPR and carrier configuration.
It checks both original samples with the existing channel tolerance of one and
the entire authored pattern tile with exact RGBA. Wrong foreground, wrong spacing
and missing motif controls must fail that exact tile comparison. This does not
claim general pattern or primitive antialiasing equivalence across engines.

Source frames are sealed constructor captures; there is no unchecked admission
of an arbitrarily edited public WaffleLayout or guessed original configuration.
Use actual style/cell factories for authored ink. Generic mutable-layout/Scene
APIs and existing cell Widgets remain separate supported entrypoints.

The complete browser matrix and independent readback pass within this tested scope. The acceptance
method and preserved diagnostic limits are in the
[verification record](verification.md#development-waffle-literal-svg-layers).
The new acceptance script is wired after the
source-bound producer in CI for Chromium and both DPRs; raw output/PNG/logs use
the existing always-uploaded artifact directory. No remote run is claimed.
Dedicated Canvas ownership, Waffle Spring/exit tracks, per-item Widgets within
these built-ins, group-region Widget factories, HTML-layer parity and arbitrary
live UI export remain separate work. See [functions and data](waffle-functions.md)
and [existing group regions](waffle-regions.md).
