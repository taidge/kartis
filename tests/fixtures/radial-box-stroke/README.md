# Original RadialBar and BoxPlot stroke references

The producer executes complete original Nivo 0.99.0 hooks, transitions, inherited and ordinal colour generators, defs binding, and JSX render component bodies from commit `0dae2c32052a573f9f1d66ec1b453b572119b265`. Original sources and the full Nivo licence are pinned under `vendor`. Existing BoxPlot compute/scale and colour fixtures and their D3 licences are reused by relative repository paths.

```sh
node scripts/radial-box-stroke-reference.mjs <scale-node_modules> <shape-node_modules> --check
```

Supply the released packages recorded in `reference.json`. The project-pinned Playwright 1.56.1 provides its existing Babel TypeScript/JSX transform; no React application, animation engine, browser, dependency installation, or pixel renderer is started by this producer.

React memo/callback helpers run synchronously. State is an explicit cold snapshot. Spring transitions record the original phase functions and use the update result for an explicit JSX/createElement recording adapter. Property paths are limited to ordinary dotted keys; primitive `uniqBy`/`sortBy`, band tick, tooltip and scheme adapters have the exact scopes documented in the producer. Original role algorithms and error branches are retained; unsupported adapter branches reject rather than pretending a full runtime.

`values.json.radial[].bars[]` records actual category/group identity, nullable source, numeric palette, bound fill, border result, transition phases and original path render attributes. `values.json.box[].boxes[]` records original summaries, group/subgroup colour, all three role results, and original group/rectangle/line attributes. Main cases contain 32 Radial configurations/128 computed segments and 28 BoxPlot configurations/140 boxes/420 role outputs. Eleven additional cases cover empty inputs, upstream filtering, native null coercion, zero extent, disabled widths, actual computed active-item snapshots and nested precomputed properties. Six real native errors and three property-access boundaries are retained.

Compare Rust only within the explicit eligibility flags. Native Radial nulls still create computed transition data; Rust `None` does not invent a segment. Native BoxPlot converts a raw null value with `Number(null)`, contributing zero. Upstream filtering is an adapter, not a native hidden-series API. Native category colours follow first occurrence; group-mode BoxPlot colours are an independently observed API gap because current Rust uses subgroup colours. Paint URLs replace displayed fill while roles still resolve original `datum.color`.

A CSS alpha-zero colour is preserved by a direct unmodified property return. Applying D3 modifiers parses its RGB as NaN and can later produce visible black after an opacity modifier. Rust intentionally preserves typed raw RGB. Such rows carry `cssAlpha0ParserBoundary` and are not false equality fixtures. Theme paths and arbitrary datum property access are original Nivo capabilities; the bounded Rust roles map Theme to each configured fixed stroke. Rust Contrast remains an extension. Widths, opacity and renderer attributes are data, not evidence of pixel parity or React lifecycle behaviour.
