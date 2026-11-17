# Native Tree Canvas renderers

[中文](tree-canvas.zh-CN.md)

These APIs are **Unreleased** and are not part of crates.io version 0.1.0.
The native renderer requires `canvas` on WebAssembly; the retained browser
owner requires `web`.

`TreeCanvasCallbacks` accepts real `CanvasRenderingContext2d` callbacks for
nodes, links, labels and named custom layers. All invocations in one paint
share one native context. Transforms, paths, clipping, alpha and composite
operations carry between user callbacks. The host does not wrap each user
callback in `save`/`restore`.

`TreeCanvasPlan` preserves literal layer order, including repeated builtins
and repeated custom keys. Its defaults are links, nodes, labels and mesh.
An omitted layer does not paint. A mesh diagnostic is computed only when its
debug, interactive and use-mesh gates are enabled; polygon diagnostics allow
at most 512 visible sites. Ordinary nearest-node queries have a separate
bounded spatial index.

## Complete data and label qualification

Use `tree_canvas_layout`, `tree_canvas_layout_with_factories` or
`tree_canvas_layout_with_activity` to compute a complete Canvas capture.
Each node's label function runs once, in breadth-first order, after final
activity sizing and coloring. Disabled labels remain in the computed pool;
the ordinary Scene still contains only eligible labels. Disabled, transparent
and zero-width links retain their resolved thickness and color.

Build `TreeCanvasFrame::from_all_computed` or
`TreeCanvasFrame::from_all_computed_activity` from that capture and the actual
caller configuration. Contexts expose original source values, topology,
computed nodes, link endpoints, activity, theme and current Scene paint.
Original signed-zero values are preserved. No renderer replays layout factories
to reconstruct discarded metadata.

The label context distinguishes its canonical computed label/font/clip from
the optional current `scene_label` and `scene_index`. Editing a Scene label
does not rewrite the constructor's computed pool. `TreeCanvasFrame::new` and
`with_activity` explicitly select `CurrentScene` label qualification instead;
they do not recover labels omitted by an older constructor.

## Direct painting and sealed preparation

`draw_tree_canvas` draws into the caller's actual Canvas. An error may leave
that destination partially painted.

`prepare_tree_canvas` paints a private pending Canvas, validates its dimensions
and origin-clean readback, then copies it to a sealed bitmap. The sealed context
is never handed to application callbacks. `PreparedTreeCanvas::to_png_data_url`
exports those pixels without running callbacks again. Pending native contexts
are retired after both success and failure; retaining one cannot change the
sealed bitmap.

The initial context applies DPR and camera once. Coordinates already include
Kartis plot margins; do not translate those margins a second time. Entering a
links layer binds its native link generator, including an empty links layer.
Entering an enabled labels layer initializes the Rust theme font once.

The link generator accepts arbitrary finite endpoints and supports a mutable
native context, curve and orientation. Without a bound context it returns SVG
path data; with a context it appends a path without beginning or stroking it.
Radial endpoints use the explicitly named polar methods.

## Retained browser owner

`BrowserTree::mount_with_canvas` retains a `BrowserTreeCanvasProgram` separately
from the layout builder. Factory and typed-component variants combine an
authoritative activity builder with the same program. `refresh` reevaluates an
authored candidate; `set_canvas_program` replaces renderers/layers even when
the Scene is unchanged.

Canvas preparation runs outside Engine state borrows. A paint callback can
read its owner's accepted snapshot. Mutating or flushing that same owner from
its candidate callbacks rejects; stopping it prevents further callbacks and
publication. Failure recovery uses accepted sealed pixels and never replays
stateful callbacks to reconstruct the old frame.

Native view, DPR and hidden changes prepare fresh pixels without rerunning
layout factories. Idle owner flushes do not redraw. Canvas PNG export includes
custom paint, excludes HTML components, and does not reevaluate renderers.
Exporting `frame.layout().scene` as SVG, CPU raster or CanvasScript exports
the source Scene; those formats do not execute or reproduce native callbacks.

The raw profile requires Canvas, controlled highlighting and no source
crosshair. Typed components must be strict top overlays with
`source_ordered: false` and `replace_ink: false`. Unsupported combinations
reject; arbitrary native compositing cannot erase or read a retained HTML
Widget. The ordinary SVG and source-ordered component APIs remain separate.

Picking uses source Scene geometry or the separately configured node-center
activity mesh. Painting a different shape does not create new physical hit
geometry. Source-ID hiding omits nodes, labels and incident links. Mesh debug
settings do not replace `BrowserTreeOptions.picking`.

## Bounds and differences from Nivo

- Plans admit at most 128 layer occurrences and 200,000 scheduled role/source
  operations by default. These are separate from the 128 measured Widget slots.
- Live Canvas backing admission includes pending/sealed buffers and caller
  reservations. Default pattern tiles and isolated Mark scratch consume the
  remaining paint budget. Individual isolation also has an 8,192-side/16-Mpixel
  cap. A callback-authored transform can cause a late bounded allocation error.
- Callbacks are trusted application code. Their own allocations, infinite loops,
  DOM mutations and other external side effects cannot be rolled back or metered
  by an unrestricted native-context API.
- Default renderers preserve rich Rust Scene geometry, paints and typography.
  They restore their own styles; they do not duplicate every Nivo default
  renderer's style leakage. Default link geometry follows its current Mark,
  rather than later mutations of the generic link generator.
- Rust retains its alphabetic text baseline, theme subset, projection defaults,
  flat source IDs, and vertical-step geometry. Every paint starts a fresh link
  generator; cross-paint D3 context identity is not preserved.
- This is not React/Spring execution, arbitrary JavaScript accessors, D3 numeric
  identity, custom-pixel picking, Widget export, GPU or worker rendering.

Acceptance results and remaining work are recorded separately in
[verification](verification.md) and the [feature gaps](feature-gaps.md).
