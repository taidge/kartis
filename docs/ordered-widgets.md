# Scene-order live Widgets

[简体中文](ordered-widgets.zh-CN.md)

Development API; absent from the published 0.1.0 package.

`BrowserChart::mount_with_plot_widgets_in_scene(root, scene, options,
composition, factory)` inserts actual Glory Widgets at authored Scene mark
positions. SVG and Canvas share the same order compiler. Its composition and
factory types are the same as [strict composition](widget-composition.md).
`set_plot_widgets_in_scene` updates the packet;
`set_layout_with_plot_widgets_in_scene` updates Scene, interaction binding and
packet together. Both setters are also available on `BrowserHandle`.
A mount retains its metadata type and composition mode, including empty packets.

## Authored order and paint

A slot is inserted immediately after its source Mark. If that ID belongs to
`replace_ink`, the Widget occupies the Mark's position and the original physical
ink is omitted. Labels remain at their original positions; ownership is not
inferred from text or naming. `Scene::draw_order` and the normal default item
order remain authoritative. Applications must explicitly arrange the Scene
when a chart needs a different role order, such as Bullet ranges, measures,
axis, markers and title.

`widget_paint_plan` is a checked, host-independent compiler. Its immutable
`WidgetPaintPlan` contains transparent `Paint` runs, `Widget` IDs and disjoint
`Group` nodes. Run Scenes retain required fonts, text declarations and clips;
they do not repeat the chart background or source group opacity. A host paints
the background once, composites group children, then applies group opacity
once to their combined output. The browser follows this contract for both
renderer profiles, including live Widget content.

A non-Normal source mark becomes a separate run with CSS `mix-blend-mode`,
composited against preceding source and Widget content. Original Mark blend
or opacity does not automatically style a replacement Widget. The Widget
factory owns its appearance; enclosing Scene group opacity applies to both.
Runtime helpers paint last without intercepting pointer input.

## Native browser input

Later source ink covers earlier Widgets in both paint and native browser hit
testing. Each source run has a transparent SVG pointer companion clipped to
its Canvas alpha-positive pixels. Browser hit testing determines real pointer
targets and CSS `:hover`; the host does not synthesize clicks. Transparent fill,
path holes, dash gaps and clip holes pass through. Even noninteractive source
ink and labels can occlude a Widget. Child keyboard and pointer actions belong
to the child and do not also activate the source chart controller.

The hit profile is quantized to the accepted Canvas device-pixel ratio,
including for SVG output. SVG subpixel edge coverage is not an exact hit-test
oracle. Canvas fractional dimensions map their rounded backing dimensions to
the displayed viewport. Font resources must be ready before mounting or
updating; source Canvas text and its SVG counterpart use the same explicit
left-to-right paragraph direction. Inherited language remains available.

Replaced physical geometry is excluded from source picking. The authored
Scene, typed metadata, logical controls and programmatic selection remain.
Partial slots keep the existing visible-but-inert policy. Hidden and zero-alpha
source groups retain Widget state while hiding them and disabling input. They
do not remove the logical source.

## Identity and rejected proposals

Same ID and revision preserve the actual component scope, DOM identity and
local state while order and group parents change. Accepted typed data and
geometry update together. Changing a revision constructs a new child; removing
a slot disposes its child after acceptance.

Candidate renderer surfaces and new children are prepared while hidden.
Factory, child-build, validation, paint or allocation errors reject the whole
candidate, retaining the accepted source, pixels, controls, data and children.
Existing children read accepted data during preparation. Candidate parents
stay alive until retained children have moved successfully; failed placements
restore prior parents and ordering. Application side effects outside candidate
scopes cannot be rolled back. Dynamic child-binding errors observed after
commit are ordinary UI errors and do not revert a committed source.

The plan admits at most 128 Widget slots, 512 source runs, 1,024 total nodes and
200,000 authored geometry commands, including replaced ink. Browser live and
pending paint surfaces, opacity/blend backing and alpha readback share a
67,108,864-pixel budget; hit companions share 200,000 rectangles. Pattern
preparation additionally limits cumulative tile pixels to 16,777,216 and tile
operations to 268,435,456. Existing pending-child quotas also apply. A frame
within one limit may still fail another; rejection preserves the old frame.

## Scope

This API is browser-only. The shared compiler and checked component placement
provide a foundation for other hosts, but precise native same-layer input
occlusion remains unfinished. The existing semantic and CPU-image
`Chart::plot_widgets_composed` API retains its top-layer behavior.

Scene/SVG/PNG exports do not capture arbitrary live Widget content. A Scene
export still includes original authored ink. Nivo automatic springs,
`animatedProps`, GPU rendering and worker rendering remain separate work.
See the [feature gaps](feature-gaps.md) and [verification](verification.md).
