# Funnel bound annotations (Unreleased)

[简体中文](funnel-annotations.zh-CN.md)

This additive API binds annotation specifications to the complete computed
Funnel parts. Verification scope and limits are recorded in the
[verification record](verification.md#development-funnel-bound-annotations). Existing
[literal SVG layers](funnel-svg-layers.md), [mouse hooks](funnel-events.md) and
[tooltips](funnel-tooltip.md) keep their separate documented acceptance scopes.

## Bind through an authoritative Frame

`FunnelSvgFrame::with_annotations` returns a new immutable Frame. It preserves
the original source, part styles and constructor capture and adds a distinct
annotation bundle. `annotations()` returns that bundle; `annotation_source()`
returns its base Frame. This is not an alias for manually adding Scene extras.

```rust
use kartis::{
    CategoryKey, FunnelAnnotationFactories, FunnelAnnotationKind, FunnelAnnotationMatch,
    FunnelAnnotationNote, FunnelAnnotationPosition, FunnelAnnotationSpec, FunnelAnnotationStyle,
    FunnelSvgFrame,
};

pub fn annotate_part(frame: &FunnelSvgFrame, id: CategoryKey) -> kartis::Result<FunnelSvgFrame> {
    let mut spec = FunnelAnnotationSpec::new(
        FunnelAnnotationMatch::Id(id),
        FunnelAnnotationKind::Circle,
        FunnelAnnotationNote::Text("Review this stage".into()),
        FunnelAnnotationPosition::Relative(60.),
        FunnelAnnotationPosition::Relative(-30.),
    );
    spec.offset = 6.;
    frame.with_annotations(
        &[spec],
        &mut FunnelAnnotationFactories::default(),
        &FunnelAnnotationStyle::from_chart(&frame.config().theme),
    )
}
```

Matchers are `All`, typed `Id(CategoryKey)` or `Predicate(String)`. Predicate
keys dispatch to the supplied borrowed matcher; string and numeric IDs remain
different identities. Bindings are ordered by specification, then computed
source order. A missing match emits no binding. Read-only bindings retain
spec/part/source indices, typed ID, shape, note, center, effective dimensions
and the computed connector/text instructions. The bundle also exposes its
original source, specifications, theme and transparent annotation Scene.

Binding uses `part.center` and original `part.bounds`, including for parts with
no body ink. It does not infer dimensions from an edited path or a current-size
extension. Derived size is `max(width, height)`. Circle/rect offset adds twice
the offset to all derived dimensions; dot ignores offset. A nonzero explicit
size overrides the derived size, while explicit zero falls back to it, following
the upstream JavaScript `size || derived` behavior. Circle/dot size is diameter.
Derived position and rect dimensions override optional specification x/y and
width/height, preserving the upstream binder's property order. Invalid final
dimensions reject rather than silently clamping.

Note X and Y independently use `Relative` or `Absolute`. Relative values are
offsets from the part center. Absolute values are plot-local coordinates; the
Frame adds chart margins exactly once. Binding outputs use full Scene
coordinates. Default note width is 120, text offset 8 and rect radius 6.

The connector has exactly three points: shape boundary, note joint, horizontal
note-line endpoint. Circle starts on its radius in the note direction; dot
starts at its center. Rect uses the upstream eight-direction side/corner rule,
not a nearest-point projection onto the rectangle. Text is placed above the
note line and extends left or right according to the upstream angle rule.

## Borrowed predicates and custom note drawing

All predicate binding finishes before custom-note callbacks begin. Callbacks
are borrowed for this build and not retained or replayed by rendering. The
predicate receives the full Frame, complete parts collection and the current
spec/part/source context. A custom note receives the bound part, original datum
and already computed full-Scene text position.

```rust
use kartis::{
    Color, FunnelAnnotationFactories, FunnelAnnotationKind, FunnelAnnotationMatch,
    FunnelAnnotationMatchContext, FunnelAnnotationNote, FunnelAnnotationNoteContext,
    FunnelAnnotationPosition, FunnelAnnotationSpec, FunnelAnnotationStyle, FunnelSvgFrame,
    Geometry, Mark,
};

pub fn custom_notes(frame: &FunnelSvgFrame) -> kartis::Result<FunnelSvgFrame> {
    let spec = FunnelAnnotationSpec::new(
        FunnelAnnotationMatch::Predicate("first-two".into()),
        FunnelAnnotationKind::Dot,
        FunnelAnnotationNote::Custom("marker".into()),
        FunnelAnnotationPosition::Relative(50.),
        FunnelAnnotationPosition::Relative(-20.),
    );
    let mut matcher = |context: FunnelAnnotationMatchContext<'_>| {
        Ok(context.key == "first-two" && context.source_index < 2)
    };
    let mut note = |context: FunnelAnnotationNoteContext<'_>| {
        let mut scene = context.scene()?;
        let mut mark = Mark::new(
            "note-marker",
            Geometry::Rect {
                x: context.x,
                y: context.y - 10.,
                width: 14.,
                height: 12.,
            },
            Color(20, 180, 210, 255),
        );
        mark.label.clear();
        scene.marks.push(mark);
        Ok(scene)
    };
    frame.with_annotations(
        &[spec],
        &mut FunnelAnnotationFactories {
            matcher: Some(&mut matcher),
            note: Some(&mut note),
        },
        &FunnelAnnotationStyle::from_chart(&frame.config().theme),
    )
}
```

The returned custom Scene must be transparent, match the source viewport and
introduce no business series, label series or legend targets. Actual mixed
paint order, clip, font, text stroke and opacity groups are retained under
private IDs. There is no implicit translation after the callback. A custom note
is Scene drawing, not a retained React/Glory per-note component. A retained
whole-layer Glory component remains available through the literal Custom layer
API and can read the complete annotated Frame.

`FunnelAnnotationStyle` (also named `FunnelAnnotationTheme`) controls outline,
symbol, link and text separately. Drawing follows SVG annotation order:
optional background connector, shape halo, shape, foreground connector, note.
Text outline uses real stroked text before foreground text. Semi-transparent
text keeps the actual two draws; it is not reduced to an opaque approximation.
This is the SVG composition projected through shared Scene renderers, not a
claim that Nivo's independent Canvas helper has the same drawing sequence.
Circles and rounded rectangles can be lowered to finite Scene paths; this does
not promise identical edge rasterization to Nivo's native SVG primitives.

The browser fixture records the matcher's complete base-Frame DTO and its
spec/source execution order. Its custom-note log observes the actual key,
spec/source indices, raw value bits and computed x/y; it does not claim to read
back every NoteContext field in the browser. The shared native model tests
cover that complete borrowed context separately.

Thin-dot pixel checks use one fixed reference method for every author pose and
browser. The portable [reference helper](../scripts/acceptance-funnel-dot-reference.mjs)
executes the pinned original Nivo `DotAnnotationOutline.tsx` through a cold
numeric Spring/JSX adapter and draws its actual native SVG circles in a separate
browser process. The original source, license, commit and hashes are retained in
[the source index](../tests/fixtures/funnel-annotations/original/source-index.json).
This executes the original drawing function; it does not measure React or Spring
time integration.

Reference inputs come only from the author model: dot center, the diameter-12
and diameter-6 paint stack, background body color or hidden/omitted white,
literal occurrence count, camera, external CSS scale and DPR. Each pose is cached
by these inputs and source hashes, with HTML, DOM, full PNG and process receipts.
Product paths and pixels never determine the reference or select a sample.
The same two whole physical cells selected by the original geometric annulus
rule remain checked, with a maximum per-channel difference of 1. A tooltip's
opaque coverage is applied separately by the independent overlay model.

Pure native controls reproduced Firefox DPR1 antialiasing in one of these cells
for both a native circle and an independently constructed cubic path; a flat
pink expectation was therefore incorrect. The fixed circle reference retains
that actual native blending rather than changing the sample, shape or tolerance.
This bounded comparison does not promise identical antialiasing for all SVG
primitives, path lowering, fonts or arbitrary poses. The native reference
runner's algorithm is the same across engines; it has no passing-color fallback.

A Chromium plain-SVG/foreignObject control found a one-channel, one-level
difference after a local glyph stroke change was restored, with identical DOM
and geometry. Attribute and style updates both reproduced it in two fixed
roundtrips. The glyph paint-contribution experiment therefore restores the
original attributes, hides the same complete SVG stage for two RAF callbacks,
restores its original style and waits two more RAF callbacks before comparing
the complete viewport exactly. It runs before peer focus and verifies unchanged
node identity, owner and peer Cages. This explicit experiment cleanup is not a
production rollback or a relaxed rejection test: source/function/budget
rejections still require an exact unchanged viewport. Original controls are in
`artifacts/verification/funnel-annotations-glyph-native-control-first` and
`artifacts/verification/funnel-annotations-glyph-native-control-second`.

## Literal layer, updates and input

`FunnelSvgLayerKind::Annotations` is independent of Parts, Labels and
SceneExtras. Default layers place it after Labels and before SceneExtras. An
explicit layer list must request it: omission does not erase binding metadata,
and repetition paints the same immutable annotation source at each occurrence.
Each occurrence has its own resource namespace and budgeted work. Empty
annotations add no source bodies or fake business sites.

Annotations bind all computed source parts. Owner hidden series do not filter
these independent decorations. Source/config/options/current rebuilds can
produce a new binding cohort; program/camera/hidden-only presentation updates
do not replay matcher or note functions. The complete source, layer Cages and
tooltip presentation adopt together before component runtimes are pumped.

Native annotation paint follows SVG `visiblePainted` and can cover a body.
Its path or text does not receive a Funnel part identity or part mouse hook.
Parts painted above it can still be actual body targets. This distinction also
applies to custom notes and to transparent paint that remains a native SVG
pointer target; annotation presence is not a shortcut to body picking.

## Budgets and failure boundaries

`funnel_annotations` and `funnel_annotations_with_budget` build standalone
bundles. Frame methods `with_annotations_with_budget` and
`with_annotation_bundle` provide one-pass composition. A precomputed bundle
must belong to the same constructor capture; equal-looking data from a
different Frame is not sufficient authority. Reapplying annotations replaces
the prior annotation layer instead of chaining old paint or replaying factories.

`FunnelAnnotationBudget` bounds specs, binding work, binding count, Marks,
Labels and path commands. Static validation precedes callbacks where possible;
actual predicate matches and custom drawing require further checked work.
Do not assume a later callback failure means earlier user callbacks did not run.
An error produces no accepted candidate Frame. In the browser owner, a rejected
build or owned placement preserves the prior accepted data, presentation and
peer Cages. External callback side effects cannot be rolled back. Postcommit
Glory UI failures remain a separate host failure boundary.

This feature does not add annotation Spring/exit transitions, arbitrary React
note elements, per-note retained Widgets or automatic viewport collision avoidance.
Opaque application data is available through the additive
[typed Funnel data API](funnel-data.md). Existing WebKit undelivered
window-exit limitations remain described in the [native event
guide](funnel-events.md).
