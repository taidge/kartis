# Custom Swarm node geometry

[简体中文](swarm-custom.zh-CN.md)

**Available in Kartis 0.2.0.** Published Kartis 0.1.0 does not include this extension.

`swarm_layout_with_node_factory` replaces a Swarm node's Scene geometry and
paint after the ordinary layout has completed. The resulting Scene is shared
by SVG, Canvas, CPU raster, semantic Chart controls and browser hosts. The
original `swarm_layout` entry point and `SwarmLayoutOptions` remain compatible.

```rust
use kartis::*;

let data = [SwarmPoint::new("sample", "Group", 5.)];
let layout = swarm_layout_with_node_factory(
    &ChartConfig::default(),
    &data,
    &SwarmLayoutOptions::default(),
    |ctx| {
        let [x, y] = ctx.position;
        Ok(ctx.inherit(Geometry::Rect {
            x: x - 6., y: y - 4., width: 12., height: 8.,
        }))
    },
)?;
let frame = layout.into_painted_scene_interaction()?;
# Ok::<(), ChartError>(())
```

The callback receives `SwarmNodeContext`: borrowed original `datum`, computed
`group` and `site`, source/group/site indices, original packed `position`,
`size`, conservative `ink_radius`, resolved `value_scale`, `plot`, inherited
`geometry` and paint. `site_index` follows packed drawing order; `source_index`
still addresses the original input slice. Equal primary coordinates retain the
ordinary ID ordering. The callback may borrow application state and is not
retained after construction.

`SwarmNodeGeometry` aliases `ScatterNodeGeometry`. It supports one Circle, Rect
or compound Path with solid fill, gradient or pattern, stroke, dash and opacity.
Use `ctx.inherit(geometry)` to retain all inherited paint. Returning
`ctx.inherit(ctx.geometry.clone())` preserves the complete default frame,
including the original glyph animation anchor. Circle and Rect geometry must
retain the logical center; a Path can intentionally place its ink elsewhere.
Optional `anchor_commands` chooses Path endpoints that follow later geometry
changes while retaining the original logical position in the accepted frame.

## Packing, visibility and interaction

The factory runs once per original site, after all source validation and
packing succeed. Hidden groups and nodes whose original collision envelope is
wholly culled by `SwarmValueFit::Clip` do not call it. Zero-size, transparent and
zero-opacity nodes retain their ordinary logical sites and do call it. Custom
ink does not resurrect a culled source or add a new business datum.

Custom paint and shape do not recompute collision discs, group bands, scale,
source values, source metadata or legend colors. The original `ink_radius`
therefore describes packing, not the replacement's visible extent. A larger
replacement can overlap neighbors; ink outside the original plot is clipped.
The representative custom paint color updates the site's interaction color.

Exact geometry picking uses the replacement's actual fill/stroke, including
its path offset and dash gaps. Nearest picking uses the original packed data
center, subject to the host's existing opacity and paint eligibility. For
example, a Move-only Path with a retained fill can keep a nearest site without
creating Exact ink. Removing every fill and stroke, or setting opacity to zero,
does not synthesize extra paint or business sites. Keyboard controls and
visibility continue to follow the host's own policy; an existing zero-size
source can retain a semantic control even though Exact has no ink to hit.
Geometry-profile picking follows the existing contour/clip and paint
eligibility rules; it does not sample arbitrary gradient or pattern alpha.

Pass a typed frame to a host, rather than passing only its Scene, to retain
explicit Path anchors and original source values. `layout.into()` preserves
the original Geometry/Nearest profile for Surface and browser hosts.
`layout.into_painted_scene_interaction()?` validates actual painted ink and
selects the Geometry-only profile required by Glory `chart_layout`; Nearest
and slices are unavailable in that profile. Keyboard controls keep the canonical node ID,
source group and label. A custom offset does not change the tooltip's original
business coordinate.

Glory's semantic Graphic supports solid fills, rectangular clips and at most
256 commands per mark. It rejects gradient/pattern fills and arbitrary clip
masks explicitly. SVG, Canvas and RasterSurface support the richer shared
paint; use RasterSurface when those fills are needed alongside live Glory
controls.

## Validation and failure

The final frame has the ordinary Scene limits, including a shared 200,000 Path
command budget that includes axes and decoration, and the shared gradient-stop
budget. Callback results accumulate across nodes; each gradient is validated
before admission. Invalid geometry, paint, dash, anchor support or opacity
rejects the candidate. A callback `Err` stops further callback execution and
returns no partial layout. External side effects in a callback cannot be
rolled back; build a candidate successfully before calling `set_layout`.

The example `swarm_custom` exports horizontal/vertical, updated and resized
frames through actual SVG, Canvas and CPU raster. Its shared browser fixture
checks trusted pointer and keyboard events, original-center Nearest versus
offset-ink Exact picking, rejected factories, source updates and cleanup.
Rejection checks capture source, revision, queued input, paint and controls
synchronously around the failed build. A later RAF or explicit idle `flush`
can advance paint revision independently of candidate admission; the browser
receipt records that telemetry separately from the unchanged accepted frame.

## Relation to Nivo

Nivo 0.99's SwarmPlot SVG `circleComponent` and Canvas `renderCircle` receive
already computed nodes (`types.ts`, `hooks.ts`, `Circles.tsx` and
`SwarmPlotCanvas.tsx`). This Rust API covers their typed geometry/paint role
across the shared renderers. It does not execute React components, arbitrary
imperative Canvas callbacks or live Glory Widget children. Kartis retains its
documented deterministic packing with a fixed value coordinate; it does not
claim equality with Nivo's two-axis force simulation or animation trajectory.
