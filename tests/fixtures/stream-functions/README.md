# Stream function configuration source reference

Run `node scripts/stream-function-reference.mjs --frozen` to verify the frozen reference.

The snapshot contains complete Nivo 0.99.0 Stream, shared getter source, package metadata and full MIT licenses. The verifier checks all source/license/producer/compiler hashes and executes the original function passthrough helpers, enhanced-layer/slice/context computations, dot creation, default dot/area JSX and layer gates.

The eight roles are `label`, `colors`, `valueFormat`, `borderColor`, `dotSize`, `dotColor`, `dotBorderWidth`, and `dotBorderColor`. Label and layer color receive the same temporary object containing only `id` and `path`; source-sample formatting precedes them and is not repeated for an area summary. Area border color runs on the complete enhanced layer even with zero border width. Dot functions execute size, color, width, border color for every point, including zero styles; actual layer gates are recorded separately.

Authored stack values, scales, area strings, React hooks, JSX and Spring values are synchronous probes, not D3/React/Spring runtime. Runtime null is recorded separately and is outside the public numeric/string `StreamDatum` contract. Rust nullable values, hidden series and shared glyph sizing require explicit additional policies. These functions are not an invented `layerComponent` prop or full arbitrary custom-layer lifecycle.

[中文说明](README.zh-CN.md)
