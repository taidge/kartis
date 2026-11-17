# Network link component contracts

This fixture preserves eight complete Nivo 0.99.0 source files, its package
manifest and full MIT license. Verification checks the original source,
license, producer and existing pinned TSX compiler hashes before executing
the extracted original recipes.

The original hooks shallow-copy raw links and resolve their endpoints, then
augment them with computed nodes, indices, thickness and color. Coordinates,
indices and computed node styles are authored probe inputs, not force results.
Thirty-two profiles cover animation, an ignored interactivity input, two blend
modes and initial/from/enter/update phases, producing 224 actual original
`createElement` configurations. Default SVG JSX and 28 record-only Canvas
command profiles are executed. Dependency hooks, JSX and transition scalars
are explicit synchronous probes; React/Spring, D3 force, native Canvas
property validation, browser delivery and framebuffer equivalence are not run.

The parent requires the links layer and a non-null computed links array; an
empty array creates no components. Zero thickness, transparent colors, self
links, parallel links and off-viewport endpoints are not filtered. Creation
passes exactly `key`, `link`, `animated` and `blendMode`, with no mouse handlers.
The initial/enter/update recipe uses complete endpoints and opacity one;
from collapses the target onto the source with opacity zero. The transition
configuration has no explicit leave recipe. Default SVG uses a round cap and
the supplied blend mode. Canvas sets stroke style and width, then draws the
segment; it does not set alpha, line cap or blend state. A command recorder
does not emulate native Canvas rules for assignments such as zero line width.

Original keys concatenate endpoint IDs with a dot. Parallel links and some
dotted endpoint names therefore collide. This behavior is recorded, not
repaired. Kartis uses distinct `force-link:index` slots: identity follows the
edge order, and edge reordering does not promise retained state for the same
semantic edge. Its payload contains current finite force-node and edge states,
not arbitrary Nivo raw-link properties. Custom ink bounds do not replace the
original endpoint midpoint, and live components do not create link business
controls or source force-drag targets.

After installing pinned project packages, verification needs no checkout,
Git, network or filesystem writes:

```sh
node scripts/network-link-widget-reference.mjs --frozen
```

An optional fixture directory follows `--frozen`. Regeneration is separate:

```sh
node scripts/network-link-widget-reference.mjs <nivo-checkout> [output-directory]
```
