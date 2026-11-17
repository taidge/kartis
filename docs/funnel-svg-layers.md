# Funnel SVG function layers (Kartis 0.2.0)

[简体中文](funnel-svg-layers.zh-CN.md)

This API is available in Kartis 0.2.0. Browser input, paint and lifecycle checks have a
separate scope from native domain tests and WebAssembly compilation. See the
[verification record](verification.md#development-funnel-literal-svg-layers) for
that scope. This API does not promise complete compatibility with Nivo Funnel.

`FunnelSvgFrame` and `FunnelSvgPlan` work in native Rust. `BrowserFunnel` requires
WebAssembly and the `web` feature. It is a dedicated SVG owner; existing Funnel
layout functions, generic `BrowserChart` and Canvas rendering keep their APIs.

## One authoritative source

Construct a frame through a real Funnel builder:

```rust
use kartis::*;

fn source_frame() -> Result<FunnelSvgFrame> {
    let config = ChartConfig {
        width: 480.,
        height: 320.,
        title: "Conversion funnel".into(),
        ..Default::default()
    };
    let source = [
        FunnelDatum::new("visits", 100.),
        FunnelDatum::new("trials", 40.),
        FunnelDatum::new("sales", 10.),
    ];
    let options = FunnelLayoutOptions {
        current: Some("trials".into()),
        current_size_extension: 8.,
        ..Default::default()
    };
    let mut border = |context: &FunnelPartStyleContext<'_>| {
        Ok(if context.part.is_current {
            Color(220, 80, 20, 255)
        } else {
            context.default_color
        })
    };
    FunnelSvgFrame::with_factories(
        &config,
        &source,
        &options,
        &mut FunnelFactories {
            border_color: Some(&mut border),
            ..Default::default()
        },
    )
}
```

The frame owns the original configuration, typed source IDs, raw numeric values,
options and complete layout. `source()`, `parts()`, `separators()`, `styles()`,
`scene()` and `layout()` expose that cohort. A string category `"1"` is distinct
from the numeric category `1`; source numeric bits, including signed zero, are
retained. Frame clones share storage. Equality uses capture identity so a new
cohort is observable even when ordinary floating-point equality would hide a raw
change.

`styles()` retains constructor color, fill opacity, effective border width,
border color/opacity and optional label color. Explicit style-function results
are retained even when the corresponding Mark or Label is absent. Reading the
frame, creating repeated occurrences or changing the browser view does not
replay those functions. Legacy disabled-role rules remain: with labels disabled
and no label-color function, cached `label_color` is `None`; an unused border
with no explicit style function can retain the existing part-color fallback
instead of resolving an otherwise invalid inherited contrast color. This is not
an always-computed Nivo default-style profile.

Use `with_size`, `with_size_and_factories`, `with_stage_factory`,
`with_factories_and_stage_factory` or `with_size_factories_and_stage_factory` for
the corresponding borrowed builder combination. Functions execute only during
that build and are not stored in the frame. Callback-owned external side effects
cannot be rolled back.

`from_layout(layout)` reads private provenance captured by a real Funnel build;
it does not guess options from geometry. Source/part metadata must still match
the captured build or its actual stage-factory result. Legal Scene decoration or
paint edits remain separate from that source. An unowned Label without a
`label_series` owner is Scene decoration, not a fabricated Funnel part.

`funnel_svg_validate_inputs` is only a static-input preflight. It does not call
size/style functions, resolve inherited contrast or validate their final
geometry. A custom color function can legitimately make otherwise unresolvable
default paint valid; a default layout is not a substitute for that preflight.

## Literal occurrences

Each `FunnelSvgLayer` has a unique occurrence key and one kind:
`Separators`, `Parts`, `Labels`, `SceneExtras` or `Custom(registry_key)`.
The vector is the actual paint order. Default order is separators, parts,
labels, then Scene extras. A Parts occurrence contains actual original border
and body items in their source order. Labels and separators remain separate
roles.

```rust
use kartis::*;

fn repeated_plan(frame: &FunnelSvgFrame) -> Result<FunnelSvgPlan> {
    FunnelSvgPlan::new(
        frame,
        vec![
            FunnelSvgLayer::new("separators", FunnelSvgLayerKind::Separators)?,
            FunnelSvgLayer::new("parts-under", FunnelSvgLayerKind::Parts)?,
            FunnelSvgLayer::new("application", FunnelSvgLayerKind::Custom("panel".into()))?,
            FunnelSvgLayer::new("parts-over", FunnelSvgLayerKind::Parts)?,
            FunnelSvgLayer::new("labels", FunnelSvgLayerKind::Labels)?,
            FunnelSvgLayer::new("extras", FunnelSvgLayerKind::SceneExtras)?,
        ],
        FunnelSvgBudget::default(),
        FunnelSvgExtrasPolicy::RequireLayer,
    )
}
```

The pure plan describes a custom registry occurrence; it does not construct a
Widget. The browser program must register that registry before mounting.
Omitting a builtin omits its actual paint. Repeating it paints another view of
the same source; it does not duplicate data, logical controls, source Mark IDs
or function evaluation. Occurrences have separate DOM/resource namespaces.

`mark_indices()` references original Scene indices, with repetitions.
`body_mark_indices()` is the union of actual body Mark indices referenced by
the literal plan. It is a projection descriptor, not an alpha, clip, occlusion
or pixel-coverage oracle. An absent zero-width body stays absent. Borders and
separators never become body data targets.

`SceneExtras` is a Rust extension for actual unowned Scene items. It does not
implement Nivo's bound `annotations` feature. Under `RequireLayer`, nonempty
extras require an explicit Extras occurrence; `FunnelSvgExtrasPolicy::Omit`
permits their omission. A compound opacity group must remain complete,
contiguous and in member order within each occurrence; splitting it across
roles is rejected. Layer, work, source-command and component budgets are
bounded; repeated paint consumes repeated work/resources.

## A real retained Glory layer

An application needs `kartis` with `web`, Glory and `web-sys` with `Element`.
Give the root an explicit nonzero CSS width and height before mounting. The
owner uses measured root dimensions and observes subsequent size changes.
Keep the returned owner alive; its handle and component current capability are
weak.

```rust
#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn mount(root: web_sys::Element) -> kartis::Result<kartis::browser::BrowserFunnel> {
    use glory::Widget;
    use glory::render::EventKind;
    use glory::style::{Length, PointerEvents, Style};
    use kartis::browser::{
        BrowserFunnel, BrowserFunnelSvgLayerContext, BrowserFunnelSvgProgram,
    };
    use kartis::*;

    let program = BrowserFunnelSvgProgram::new(vec![
        FunnelSvgLayer::new("separators", FunnelSvgLayerKind::Separators)?,
        FunnelSvgLayer::new("parts", FunnelSvgLayerKind::Parts)?,
        FunnelSvgLayer::new("summary", FunnelSvgLayerKind::Custom("summary-ui".into()))?,
        FunnelSvgLayer::new("labels", FunnelSvgLayerKind::Labels)?,
        FunnelSvgLayer::new("extras", FunnelSvgLayerKind::SceneExtras)?,
    ])?
    .custom("summary-ui", 0, |context: BrowserFunnelSvgLayerContext| {
        let frame = context.frame;
        let current = context.current;
        Ok(glory::ui::widget(move |scope| {
            let current = current.clone();
            let count = scope.cage(0u32);
            let error = scope.cage(String::new());
            let sales = CategoryKey::from("sales");
            glory::ui::button()
                .style(
                    Style::new()
                        .width(Length::Px(180.))
                        .height(Length::Px(32.))
                        .pointer_events(PointerEvents::Auto),
                )
                .label(scope.bond(move || {
                    frame.get().map_or_else(
                        || "Preparing".to_owned(),
                        |accepted| format!(
                            "{} parts; clicks={}; {}",
                            accepted.data.parts().len(), count.get(), error.get(),
                        ),
                    )
                }))
                .on(EventKind::Activate, move |_| {
                    match current.set_current_part(Some(&sales)) {
                        Ok(()) => count.revise(|value| *value += 1),
                        Err(reason) => error.revise(|value| *value = reason.to_string()),
                    }
                })
                .show_in(scope)
        }))
    })?;

    BrowserFunnel::mount_with_factory_and_svg_layers(
        root,
        vec![FunnelDatum::new("visits", 100.), FunnelDatum::new("sales", 20.)],
        ChartConfig::default(),
        FunnelLayoutOptions::default(),
        program,
        |context| FunnelSvgFrame::new(context.config, context.source, context.options),
    )
}
```

The whole layer is a real Glory subtree in an XHTML host within SVG
`foreignObject`. It can contain controls and Glory `Graphic` drawing; it is not
a React component, raw SVG-node factory, string callback or top overlay outside
the literal order. Its host origin is plot-local. The fixed plot clip and camera
transform are applied once; do not add the plot margin a second time. Native
Funnel profile coordinates remain full-Scene coordinates. Axis-aligned CSS
scaling is distinct from the logical camera and DPR.

Carrier hosts are pointer-passive. Interactive custom children explicitly opt
into pointer events, as the button does above. A child above a native body can
block that body; a lower child can be covered by later native paint.
`.interactive(false)` disables the owner's automatic native body-hover route;
it is not a global ban on an application's custom controls or explicit current
commands.

## Data and component lifecycle

`BrowserFunnelSvgLayerContext` provides `key`, `revision`, tentative `initial`,
`frame: Bond<Option<BrowserFunnelSvgLayerFrame>>` and `current`. During
construction the Bond is `None`; `initial` is not an accepted snapshot. The
accepted frame contains the complete `FunnelSvgFrame`, source generation,
publication, occurrence key/index, view, hidden series and interactive setting.
`svg_layer_frame(key)` reads the accepted frame for any literal occurrence,
including builtins; an absent key returns `None`.

One complete Cage backs each occurrence's frame in its own Glory runtime. All
candidate occurrence data and owner state adopt before runtime pumping. This
keeps fields together without pretending that independently tracked Bonds from
different WebApps share a runtime. Ordinary component code should track its own
frame; deliberate cross-runtime inspection uses explicit untracked reads.

The same occurrence key, registry and revision retain the real child Scope and
local state through source updates, reordering and camera/hidden changes. A new
revision rebuilds that child. Registering a different function under the same
retained identity does not request a rebuild. Removal, replacement, stop or owner
drop retires the old Scope; retained current capabilities then fail.

`current.set_current_part(Some(&id))` and `None` enqueue real owner work. They do
not synchronously change the Bond or flush inside a component callback. The next
RAF processes the queue, or application code can call `owner.flush()` outside
the callback. Unknown IDs, constructor/pending use, stopped/retired ownership
and reentrant mutation return errors. Read-only snapshots can observe the
accepted cohort; setters are guarded while a build/commit is busy.

Source/config/options/current/resize changes and `refresh()` rebuild the source
through the supplied builder. Program-only, camera (`set_view`) and
`set_hidden` changes reuse the accepted source frame and do not replay layout,
size, style or retained custom factories. Source generation advances with an
accepted source rebuild; publication advances with an accepted presentation
update. `BrowserFunnelSvgHit` records the historical frame/generation/publication
and actual occurrence for its native observation; later adoption does not retag
that record.

Hidden entries are actual `FunnelPart.series` strings, not their display labels
or untyped datum IDs. Read them from the accepted frame. The camera is supplied
through `ZoomTransform`; this API does not add generic Chart pan/zoom gestures.

## Input and failure boundaries

Automatic current changes originate from actual accepted, connected owned
native SVG body paths. The browser uses SVG `visiblePainted` hit behavior, not a
solid-only picking profile or a nonzero-alpha approximation. In particular,
transparent SVG paint is not universally equivalent to `pointer-events:none`.
Gradient/pattern paint is retained. Omitted Parts, missing bodies, hidden
series, separators, borders, labels and arbitrary custom HTML do not fabricate
body hits. Explicit component current commands are separate application actions.

Fallible source/factory/owned-carrier placement work precedes adoption. Rejection
preserves the accepted source/presentation and consumes the rejected batch;
later valid current commands start from accepted inputs rather than retrying
failed desired configuration. `snapshot().error` reports rejected work. User
callback side effects cannot be undone.

UI pumping follows adoption. A postcommit Glory UI error is reported in
`ui_errors`, retains accepted metadata and hides the failed custom view; it is
not presented as a precommit rollback. The failed instance stays hidden with
`display:none` and inert, including after later successful source updates or
Glory batches. Increase its custom revision to construct a new instance and
restore presentation; that remount resets the component's local state. Journaled owned-carrier writes do not claim an upstream Glory API
for transactionally flushing every managed backend operation before adoption.

WebKit can suppress every window-exit input event when a stationary hovered SVG
path is replaced before the pointer leaves the window. A plain SVG replacement
control reproduces this. The owner clears current for delivered input; it cannot
infer an exit for which the browser supplies no event. The outside-window test
first delivers a real Move to the accepted replacement body before checking its
actual trusted Out. Stationary replacement without further delivered input is a
remaining input limitation; no operating-system event equivalence is claimed.

## Remaining scope

Four public native part mouse hooks and standard/retained Glory tooltips are
implemented and have passed their separate browser acceptance. See
[events](funnel-events.md), [tooltips](funnel-tooltip.md) and the
[verification record](verification.md#development-funnel-native-events-and-tooltips)
for the tested configuration and boundaries.

This API does not yet provide Source Widgets interleaved within Funnel builtin
runs, part-Element handler binding, bound annotations, automatic Funnel
Spring/path interpolation and exit transitions, a generic application raw-datum
type, or arbitrary mutable D3 area/line generator objects. Existing per-stage
geometry functions and Funnel Widget adapters remain separate APIs.

`original_body_geometry` and `original_border_geometry` adapt the captured
four-sample profile using the captured interpolation. Read `body_mark` and
`border_mark` for actual custom or edited ink. No sampled custom body is
represented as its original profile. The source `scene()` is not a flattened
export of repeated occurrences or arbitrary Glory subtrees; portable custom-UI
export is not promised by this feature.
