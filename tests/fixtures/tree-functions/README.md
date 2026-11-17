# Tree function configuration source reference

Run `node scripts/tree-function-reference.mjs --frozen` to verify the frozen reference.

The snapshot contains complete original Nivo 0.99.0 Tree hooks, types, label positioning, defaults, SVG/Canvas parents and role renderers; shared getter sources; package metadata; and three full MIT notices. All original source/license/producer/compiler bytes are hashed. Original helper function passthrough, node/link computations, label calculations and SVG/Canvas label scheduling expressions are re-executed.

Node size runs before node color on an intermediate node without its resolved size, color or active state. Finite `fixNodeColorAtDepth` inherits a computed parent color and skips the base callback for deeper descendants. Links receive computed endpoint nodes; thickness precedes color, then active modifiers. Labels receive complete computed nodes, including final sizes; SVG label creation is gated, but Canvas calls the labels hook even when the labels layer is not drawn.

Hierarchy traversal and x/y positions, scales and React hooks are explicit synchronous authored probes. This is not D3 tree numeric layout, React scheduling or Spring animation. Original nested raw object references and signed zero are checked before JSON serialization. Rust flat IDs, aggregate values, existing inset projection, radius conventions and optional radial layout remain explicit differences. Five base callbacks do not imply support for four active/inactive size callbacks, color-depth fixing, arbitrary components or custom layers.

[中文说明](README.zh-CN.md)
