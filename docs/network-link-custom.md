# Custom force-network links

[简体中文](network-link-custom.zh-CN.md)

**Unreleased.** `network_glyph_layout_with_link_factory` composes custom link
geometry and paint with existing node glyphs. The callback runs after physics;
it does not change the simulation, node radii or force endpoints. SVG, Canvas,
CPU raster and solid semantic Graphic paths consume the resulting Scene.

```rust
use kartis::*;
use glory::render::{GraphicPoint as P, PathCommand as C};

let simulation = ForceSimulation::new(
    &[NetworkNode::new("source"), NetworkNode::new("target")],
    &[NetworkLink::new("source", "target", 4.)],
    ForceOptions::default(),
)?;
let frame = network_glyph_layout_with_link_factory(
    &ChartConfig::default(), &simulation, &NetworkGlyphOptions::default(),
    &mut |ctx| {
        let [x0, y0] = ctx.source_position;
        let [x1, y1] = ctx.target_position;
        Ok(ctx.inherit(Geometry::Path(vec![
            C::Move(P::new(x0, y0)),
            C::Cubic {
                first: P::new(x0, y0 - 30.),
                second: P::new(x1, y1 - 30.),
                end: P::new(x1, y1),
            },
        ])))
    },
)?;
let semantic_frame = frame.into_painted_scene_interaction()?;
# Ok::<(), ChartError>(())
```

The borrowed context contains the resolved `ForceEdge`, its input-order index,
the current source/target `ForceNodeState`, original geometry and paint, actual
Scene endpoints, original midpoint, world translation and plot rectangle.
Coordinates already include that translation once. `ctx.inherit` retains the
original paint; return another checked `NetworkLinkGeometry` to customize it.
Paths may have offset, curved or multiple subpaths. Rectangles and circles use
the original midpoint as their logical origin. Link bounds exclude stroke ink.

Factories run in original edge order, including zero weights, parallel links
and self links. Inheriting every original mark preserves the complete glyph
Scene and node binding. Source values, labels, IDs, draw order and node glyphs
remain intact. Bare force links use positional IDs and remain decorative:
custom link ink neither adds node controls nor becomes a node drag target.
`network_glyph_relation_layout_with_link_factory` instead accepts explicit
stable edge IDs and preserves the relation graph, ownership and authored paint
across highlighting. It retains the existing relation adapter's interaction
contract rather than adding implicit nearest-edge data sites.

Native borrowed factories are not stored. A callback error or invalid geometry
rejects the whole candidate without changing the input simulation or an older
frame. All nodes and custom links share the 200,000 path-command and 16,384
gradient-stop Scene budgets; each gradient also keeps its own stop limit.

## Retained browser driver

`BrowserNetwork::mount_with_link_factory` accepts the same glyph configuration
and an explicit retained `'static` factory. It rebuilds from current simulation
states for tick, drag, resize and replacement candidates. The driver publishes
the Scene and typed node binding before committing force positions. Failed
candidates retain the accepted frame and positions, release a failed drag and
can be resumed through the existing reheat/replacement APIs.

Replacement admission can invoke the callback before its later RAF invocation;
idle driver frames may also rebuild it. Do not use invocation counts as a
data-change subscription. External callback effects are not rolled back.
Callbacks can read accepted state or stop their owner; nested link rendering
returns a checked error. Stopping drops the retained callback exactly once,
including a stop requested inside replacement preflight.

## Boundaries and evidence

This is a shared Scene renderer hook for Nivo 0.99's `linkComponent`/`renderLink`
use case. It does not reproduce React components, arbitrary user datum fields,
spring lifecycles or a different force solver's final positions. The actual
Nivo Canvas default renderer inherits `lineCap` and ignores invalid zero-width
assignments through Canvas2D itself; SVG uses round caps. The inherited Rust
profile retains its existing round-cap and zero-weight width behavior.

Semantic Graphic hosts require supported solid paint and at most 256 commands
per mark. Use SVG/Canvas/raster for gradients and larger paths. Live arbitrary
Widget children and their snapshots remain a separate protocol.

Tests include complete default inheritance, source identity, self/parallel
links, relation ownership, aggregate budgets, real Glory pen input and CPU
pixels. The browser suite exercises trusted node dragging under nonuniform CSS
scale, current link endpoints after ticks, replacement/resize, source/paint
rollback, callback reentry and release. Original Nivo source, MIT notices and
real Canvas function probes are preserved in
[reference fixtures](../tests/fixtures/node-link-custom/).
Final scope and first failures are recorded in [verification](verification.md).
