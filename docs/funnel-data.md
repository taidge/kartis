# Funnel owned application data (Unreleased)

[简体中文](funnel-data.zh-CN.md)

The typed API retains opaque application data throughout the captured Funnel
source. Verification scope is recorded in the
[verification record](verification.md#development-funnel-owned-application-data).
Existing `FunnelDatum`, `FunnelSvgFrame`, layouts and untyped browser mounts keep
their APIs. GPU/worker work remains deferred.

## Opaque data and explicit projection

`FunnelDataDatum<D>` owns `data: D` beside `projected: FunnelDatum`. The
application chooses the numeric/layout projection; arbitrary extra fields are
not coerced into a Scene value or serialized by the library. `D` does not need
Clone, PartialEq, Debug, Serialize, Send or Sync. Pure frames can hold borrowed
nonstatic data; a retained browser owner requires `D: 'static`. Interior
mutability in D remains application responsibility: changing a Cell does not
itself enqueue a new accepted source.

```rust
use std::{cell::Cell, rc::Rc};
use kartis::*;

pub struct BusinessData {
    pub status: String,
    pub raw_metric: f64,
    pub local: Rc<Cell<u32>>,
}

pub fn build_data() -> Result<FunnelDataFrame<BusinessData>> {
    let source = vec![FunnelDataDatum::new(
        BusinessData {
            status: "priority".into(),
            raw_metric: -0.,
            local: Rc::new(Cell::new(0)),
        },
        FunnelDatum::new("leads", 8.),
    )];
    let mut color = |context: &FunnelDataColorContext<'_, BusinessData>| {
        Ok(if context.data.status == "priority" {
            Color(210, 70, 40, 255)
        } else {
            context.computed.default_color
        })
    };
    FunnelDataFrame::with_factories(
        &ChartConfig::default(),
        source,
        &FunnelLayoutOptions::default(),
        &mut FunnelDataFactories {
            color: Some(&mut color),
            ..Default::default()
        },
    )
}
```

`FunnelDataFrame<D>` owns one raw source Arc and a checked `FunnelSvgFrame`.
`source()` exposes the owned data/projection rows; `data(source_index)` returns
an optional raw reference. `computed()`, `config()`, `options()`, `layout()` and
`scene()` expose the validated projection. Frame Clone and Debug do not clone or
format D; the existing pure `FunnelSvgFrame` gains no browser Any payload and
retains its own Send/Sync properties.

Full-frame equality is capture identity. A new build is a new frame even when
all projected fields and paint bytes match. `shares_source()` only reports a
shared raw source Arc: separate config/annotation captures can share that pool
without being equal. `from_shared` and `with_shared_factories` accept an
`Arc<[FunnelDataDatum<D>]>`, permitting rebuilds without D:Clone. A browser
builder must use the exact source Arc supplied by its build context, rather
than manufacture an equal-looking replacement.

`with_computed` accepts a real constructed projection for explicit legal ink or
Scene-extra edits. It checks every projected source field, including numeric
bits and nested paint coordinates; it does not reconstruct D from Marks or
certify that an arbitrary user drawing function depends only on D.

## Factory phases and bound annotations

`FunnelDataFactories` contains optional `color`, `border_color`, `label_color`,
`size` and `stage` callbacks. Each receives its raw datum, the whole raw source
slice and the original `computed` role context. The number-only
`formatted_value` callback keeps the existing formatter contract. Functions
are borrowed for one build and never stored in the result.

Size/source validation precedes style evaluation. Color, format and stage-style
roles retain their existing source-order phases; the stage geometry callback
runs for originally drawable bodies. No final future styled collection is
invented at an early color/size phase. Explicit style results for absent ink
remain in the computed cache. Optional roles do not force legacy default roles
through additional callback passes.

At the annotation phase, typed matcher/note contexts contain the complete raw
frame and existing computed context. The matcher can inspect the full computed
collection and resolve each source index to D. A custom note returns an actual
transparent full-size Scene, not just a formatter result:

```rust
use kartis::*;

pub struct NoteData {
    pub flagged: bool,
    pub caption: String,
}

pub fn bind_notes(frame: &FunnelDataFrame<NoteData>) -> Result<FunnelDataFrame<NoteData>> {
    let mut spec = FunnelAnnotationSpec::new(
        FunnelAnnotationMatch::Predicate("flagged".into()),
        FunnelAnnotationKind::Dot,
        FunnelAnnotationNote::Custom("caption".into()),
        FunnelAnnotationPosition::Relative(30.),
        FunnelAnnotationPosition::Relative(-20.),
    );
    spec.size = Some(8.);
    let mut matcher = |context: FunnelDataAnnotationMatchContext<'_, NoteData>| {
        Ok(context.data.flagged)
    };
    let mut note = |context: FunnelDataAnnotationNoteContext<'_, NoteData>| {
        let mut scene = context.scene()?;
        scene.labels.push(Label {
            rotation: 0.,
            x: context.computed.x,
            y: context.computed.y,
            text: context.data.caption.clone(),
            color: Color(35, 45, 65, 255),
            size: 13.,
            anchor: TextAnchor::Start,
        });
        Ok(scene)
    };
    frame.with_annotations(
        &[spec],
        &mut FunnelDataAnnotationFactories {
            matcher: Some(&mut matcher),
            note: Some(&mut note),
        },
        &FunnelAnnotationStyle::default(),
    )
}
```

`with_annotations_with_budget` applies the same policy with explicit budgets.
Bindings always use the annotation-free computed base, so replacement does not
accumulate old annotation paint. Raw and projected source ownership is retained
without replaying layout/style functions. `with_annotation_bundle` accepts a
bundle for the same original computed source capture without another callback
pass. `annotation_data(binding_index)` resolves the corresponding raw datum.
The [annotation guide](funnel-annotations.md) defines geometry, paint order and
static rendering boundaries.

`widgets(revision, placement)` adds typed payloads to the existing legacy Mark
roles. It uses the original unannotated role projection and carries the complete
typed frame. Closing separators keep `data: None`; annotation decoration does
not become a business role. This does not add per-item Widgets inside the
literal SVG owner's builtin layers.

## Typed browser owner

`BrowserFunnelData<D>` is an additive SVG owner; keep it alive while using its
weak typed handle. Give the root a positive CSS box before mounting. The
following example mounts a real retained whole-layer component, a typed Click
hook and a real tooltip component:

```rust
#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub fn mount_data(
    root: web_sys::Element,
) -> kartis::Result<kartis::browser::BrowserFunnelData<AppData>> {
    use glory::Widget;
    use kartis::browser::*;
    use kartis::*;

    let program = BrowserFunnelDataSvgProgram::<AppData>::new(vec![
        FunnelSvgLayer::new("parts", FunnelSvgLayerKind::Parts)?,
        FunnelSvgLayer::new("labels", FunnelSvgLayerKind::Labels)?,
        FunnelSvgLayer::new("annotations", FunnelSvgLayerKind::Annotations)?,
        FunnelSvgLayer::new("extras", FunnelSvgLayerKind::SceneExtras)?,
        FunnelSvgLayer::new("panel", FunnelSvgLayerKind::Custom("summary".into()))?,
    ])?
    .custom("summary", 0, |context| {
        let frame = context.frame;
        Ok(glory::ui::widget(move |scope| {
            glory::ui::text()
                .content(scope.bond(move || {
                    frame.get().and_then(|f| f.data.data(0).map(|d| d.name.clone()))
                        .unwrap_or_default()
                }))
                .show_in(scope)
        }))
    })?;
    let owner = BrowserFunnelData::mount_with_factory_and_svg_layers(
        root,
        vec![FunnelDataDatum::new(
            AppData { name: "Qualified leads".into() },
            FunnelDatum::new("leads", 8.),
        )],
        ChartConfig::default(),
        FunnelLayoutOptions::default(),
        program,
        |context| FunnelDataFrame::with_shared_factories(
            context.config,
            context.source.clone(),
            context.options,
            &mut FunnelDataFactories::default(),
        ),
    )?;
    let handle = owner.handle();
    handle.set_mouse_handlers(BrowserFunnelDataMouseHandlers {
        on_click: Some(Box::new(|event| {
            let data = event.data.data(event.source_index)
                .ok_or_else(|| ChartError("clicked raw source is missing".into()))?;
            let _name = &data.name;
            Ok(())
        })),
        ..Default::default()
    })?;
    handle.set_tooltip_component(0, |context| {
        let frame = context.frame;
        Ok(glory::ui::widget(move |scope| {
            glory::ui::text()
                .content(scope.bond(move || {
                    frame.get().and_then(|f| f.datum().ok().map(|d| d.name.clone()))
                        .unwrap_or_default()
                }))
                .show_in(scope)
        }))
    })?;
    handle.flush()?;
    Ok(owner)
}

pub struct AppData {
    pub name: String,
}
```

The builder's `source` is an Arc reference. The returned frame must retain that
exact raw pool and its actual checked projection. Snapshot, layer, native hook
and tooltip types carry `FunnelDataFrame<D>` plus the accepted source generation
and publication; native input also retains its real event/occurrence metadata.
Layer constructors have an explicit tentative `initial`; their accepted Bond
starts None. Tooltip constructors likewise start None. Accepted layer/tooltip
Bonds derive from complete private source snapshots, not independently adopted
raw and projected data. Observe another runtime's Bond with untracked reads;
Glory effects do not subscribe across independent runtimes.

`set_source(Vec<FunnelDataDatum<D>>)` queues a source update, including a raw-only
change with identical projection. Such a build still yields a new full cohort
and updates retained typed views; stable revision scopes retain their local
state. Program/view/hidden-only work does not replay source factories. Setters
enqueue and `flush()` processes the common transaction; native callbacks must
queue work rather than flush reentrantly.

A captured native event owns the historical frame that was accepted for that
actual event. Do not reinterpret it by looking up its business ID in a later raw
source. Dispatch admission rejects a stale source/presentation lease. Tooltip
selection instead rebinds its existing business/occurrence selection to the
newly accepted candidate and updates its data/generation/publication together,
or clears if its target is removed or ineligible. Standard tooltip text still
uses the projected label/formatted value; a custom typed component can display
arbitrary raw fields. These are different lifecycles, not competing snapshots.

## Rejections and remaining scope

A rejected build, typed-source certificate, predicate, note or precommit owned
placement does not replace the accepted raw/projection/layer/tooltip cohort.
User function side effects are external and cannot be rolled back. Managed
backend failures after adoption remain postcommit UI errors with the existing
failed-view isolation policy; this API does not invent an upstream backend-only
transaction guarantee. New revision/remount and cleanup rules remain those of
the dedicated Funnel owner.

Property springs are provided by the separate [motion API](funnel-motion.md);
Nivo's Funnel has no exit transition. Arbitrary mutable D3
generators, per-item Source Widget mixing, arbitrary live UI export and wider OS
input equivalence remain separate. Canvas can consume the pure computed Scene through existing
APIs; the new dedicated typed browser owner is SVG-only. GPU/worker remains deferred.

NativeComponent activity retains the existing native-event boundary. Omitting a
body does not invent a Leave or clear a controlled current part; tooltip
eligibility can still clear its presentation. The browser fixture explicitly
focuses the real retained button before trusted keyboard activation, because
click-to-focus behavior differs between engines. These setups do not synthesize
business mouse callbacks. See the [native event guide](funnel-events.md).
