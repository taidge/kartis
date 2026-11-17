# Tree property springs (Unreleased)

[简体中文](tree-motion.zh-CN.md)

`TreeMotion` supplies explicit-clock typed animation properties.
Motion-capable `BrowserTree` mounts add SVG presentation, automatic RAF,
retained Glory components and shared source/layer/tooltip adoption. The target
data and the physical paint remain separately accessible. Nivo Canvas Tree
renders its current computed data directly; Canvas springs, Radial animation,
GPU and worker rendering remain outside this SVG profile.

## Data and properties

Construct a `TreeEventFrame` from a validated Tree layout, then use
`TreeMotion::new(frame, TreeMotionOptions::default())`. `retarget(next)` adopts a
new immutable target, and `tick(seconds)` returns a cheap shared
`TreeMotionFrame`. Clock steps must be finite and within `0..=60` seconds.
The complete program is [tree_motion_data.rs](../examples/tree_motion_data.rs):

```sh
cargo run --example tree_motion_data
```

| Role | Animated properties | Leave target |
| --- | --- | --- |
| Node | X, Y, diameter and RGBA | Prior target X/Y/color, diameter zero |
| Link | Source/target X/Y, thickness and RGBA | Prior target properties |
| Label | X, Y and rotation | Prior target properties |

New roles enter directly at their target properties. A settled departing Link
or Label retires immediately. A moving departure continues towards its prior
target until all its tracks settle. `finish()` and reduced motion converge live
properties and remove leaving rows.

The default mass/stiffness/damping is `1/120/14`, following the named
[React Spring gentle preset](https://www.react-spring.dev/docs/advanced/config).
Kartis uses its analytical physical solver and its own explicit rest thresholds;
this does not establish identical React Spring integration, color interpolation,
expiration or scheduling. Rotation follows the direct numeric path.

## Identity, provenance and limits

Node and Label keys use the existing globally unique flat business IDs. Link
keys are structural `(source_id, target_id)` pairs, so delimiters cannot cause
key collisions. Reparenting preserves a Node/Label instance and changes its
incoming Link key. This extends the existing Rust identity contract; Nivo uses
ancestor-path UIDs.

Each row exposes `phase()`, `instance()`, typed `props()` and independent raw
`states()`. The 7/9/3 state arrays contain value, target and velocity. Reentry
before retirement preserves the instance, sampled position and velocity. Entry
after retirement receives a new instance at the target properties.

Live rows share the new complete `TreeEventFrame`. Leaving rows retain their
prior immutable frame, including raw signed zero, source indices and original
Mark/Label metadata. Even a raw-only update refreshes the target cohort. Labels
resolve through actual `Scene::label_series` owners; unowned annotations are
listed separately, and ambiguous duplicate owners are rejected.

`target_serial` advances on each successful retarget. `sample_serial` advances
on nonzero unsettled ticks and explicit convergence. A zero step does not
advance physics. Per-family budgets cover both live and leaving rows; invalid
time, configuration and union budgets reject without changing accepted state.
Candidate cloning and allocation precede some budget checks.

`set_options(next)` changes spring parameters and union limits atomically. It
preserves each current value, target, velocity and instance; new rest thresholds
take effect on the next tick. A configuration change alone does not publish a
new sampled frame. Frame equality identifies the shared immutable capture,
including captures whose raw data differs only by signed zero.

## Explicit native presentation

`tree_motion_paint(motion.frame())` builds a validated `TreeMotionPaintFrame` for
the four Cartesian orientations. Its Scene contains sampled node geometry,
solid node colors, link endpoints, thickness and color, and label positions and
rotation. The original target remains available through `target()`. Node
borders, authored paints, clipping, opacity and text remain target properties.
The target's mixed draw order and unowned annotations are preserved.

Leaving ink receives a private paint ID. `target_item` returns `None` for a
departure; current-source index maps and instance maps are separate. Historical
label typography, clipping and source data survive until retirement. These maps
describe provenance; a standalone painted Scene does not install input handlers.

Default projection rejects edited native geometry, edited link strokes, Radial
orientation and multi-Mark opacity groups. Use
`tree_motion_paint_with_renderers` for explicit per-frame geometry or stroke
recipes. Mark renderers must preserve business metadata, and the complete
candidate is validated before return. Renderer side effects cannot be rolled
back if a subsequent recipe fails. Unsupported orientation and opacity groups
reject before invoking any renderer, including when retained departures refer
to an older source.

Presentation bounds clamp individual numeric channels, including nonnegative
diameter/thickness and RGBA. These clamps preserve independent raw physical
values and velocities. They do not validate composed custom geometry. The
original target Scene, including arbitrary Mark paths, stays unchanged and is
available through `frame().target().scene()`; it is not an animated paint Scene.
Computed no-Mark, zero and disabled roles gain no painting or input permission
from having animation properties.

## Browser scheduling and input

Select `BrowserRenderer::Svg`, then mount with `mount_with_motion`,
`mount_with_factory_and_motion` or `mount_with_svg_layers_and_motion`.
`BrowserTreeOptions::default()` selects the chart's general default renderer;
the SVG choice is explicit. Existing static mounts retain their contract and
reject enabling automatic motion later.

The default `BrowserTreeMotionClock::Automatic` uses RAF. `Manual` plus
`advance_motion(seconds)` provides exact caller-supplied steps through the same
paint and component adoption path. `motion_frame()` reports adopted numeric
data, source generation, publication and configuration serials. Failed
candidates preserve the accepted physical state and clock. An idle automatic
owner rebases time before animating a new target. A held native press pauses
automatic replacement; hidden, detached or zero-size surfaces stop recurring
animation requests. Showing the surface resumes through ResizeObserver;
explicit `refresh()` can also resume a reattached owner.

`set_motion_options(Some(next))` preserves velocity while changing parameters.
`None` snaps to the current target, retires departures and preserves live Glory
scopes. Enabling motion and changing the target in the same batch animates from
the prior pose. System reduced-motion preferences are honored.

Native geometry and input eligibility use sampled live ink. Event and tooltip
data retain current target metadata, including a zero target link width while
its sampled stroke is still visible. Node fixed anchors use the target
position; link cursor anchors use the actual pointer. Plot bounds and hidden
series still constrain automatic selection. A frontmost Mesh occurrence
captures input for nearest-node selection; select `BrowserTreePicking::Geometry`
or omit Mesh to interact with the native link paths beneath it. Departures
never receive automatic current-source input.

## Glory components and literal layers

Use `mount_with_motion_and_animated_components` or
`mount_with_factory_svg_layers_motion_and_animated_components` to construct real
Glory components. `BrowserTreeAnimatedWidgetContext` provides tentative
`initial` data and two Bonds, `frame` and `animated_props`, derived from one
adopted snapshot. They start at `None` during construction. The application
positions its content with the typed Node, Link or Label properties. The host
provides a full-Scene carrier, applies camera once, and clips to the fixed plot.
External CSS scaling remains an outer browser transform.

Each occurrence/instance/revision retains its actual Scope and local state.
Leaving frames retain their historical `data_generation`, while
`source_generation` identifies the current target. Retained departures are
inert; reentry before retirement revives the same Scope. Entry after retirement
constructs a new one. Removing an occurrence retires its scopes immediately;
adding an occurrence does not construct components for existing departures.
Disabled Links do not create Link components. Zero-sized or transparent enabled
roles may be explicitly represented by application UI, without fabricated native
ink or automatic input permission.

Literal occurrences preserve native/UI mixed order. A whole-layer
`.custom_animated` component receives target layer data and the shared numeric
motion through Bonds from one snapshot. Plain `.custom` layers keep their target
data contract. Source/style factories run only for target builds; component
factories run for new instances or revisions. Sampling, camera and hidden-series
updates retain existing components. Live and tentative scopes share bounded
admission; owned precommit failures restore the prior publication. Deferred
Glory backend failures retain the existing postcommit reporting boundary.

Default native projection is deliberately strict about edited geometry. The
`mount_with_motion_renderer` variants accept an explicit per-frame presenter,
whose returned paint must reference that exact candidate `TreeMotionFrame`.
Ordinary mounts reject noncanonical source draw order before adoption; literal
SVG programs provide explicit ordering. Numerical properties share one owner
clock across occurrences, rather than creating separate React Spring engines.

See [verification](verification.md) for the independent native, extracted Nivo
recipe and actual browser records. These establish the declared Rust profile,
not identical React Spring time integration, arbitrary Widget export, all input
devices or GPU/worker behavior.
