# Original Nivo BoxPlot caption-map reference

[简体中文](README.zh-CN.md)

This fixture executes complete original Nivo 0.99.0 `useBoxPlot`, `getLegendData` and `useBoxPlotTransition` bodies, their original summary/scale/color helpers, and the original core `getPropertyAccessor` / `usePropertyAccessor`. A JSON caption descriptor is converted to an actual function passed as `legendLabel`. The original accessor returns that same function; the original legend helper calls it on actual summaries. Expected legend rows are the original helper's results, including text-based deduplication.

```text
node scripts/box-legend-labels-reference.mjs <released-D3-node_modules>
node scripts/box-legend-labels-reference.mjs <released-D3-node_modules> --check
```

The optional `--shared-vendor <directory>` argument selects the unchanged [shared original source bundle](../box-colorby-hover/README.md). The producer does not install packages. It uses Node TypeScript erasure and the asserted released D3 modules; complete dependency source hashes are recorded. Nivo source and its [MIT license](../box-colorby-hover/vendor/nivo-LICENSE) are reused. The new core property source is preserved in `vendor/`. Released Lodash 4.17.21 is preserved byte-for-byte as `vendor/lodash.cjs` (only the filename changes from the package's `lodash.js`) with its [license](vendor/lodash-LICENSE). Its real `uniqBy`, `sortBy`, `isFunction`, `get` and `isPlainObject` run; none are replacement algorithms. Source provenance and hashes are in `vendor/source-pins.json`.

The 30 main inputs are Group/SubGroup × unique/duplicate/empty captions × complete/sparse/deleted-group/deleted-subgroup/empty-reserved sources. Every input records the full raw domain, palette, observations and caption map. Baseline, activation using an actual previously computed summary, and restoration each execute the original hook. Two nonempty legend configurations invoke the original helper; an additional direct call cross-checks them. Callback arguments and returned captions are recorded. `activeKeys` comes from the actual original memo return, never a replacement association predicate. Removed input rows retain the explicitly configured full palette domain; this is upstream source deletion, not a fabricated native hidden option.

Seven qualifications cover partial maps, one selected identity producing multiple labels through a non-map callback, explicit empty-subgroup text versus its raw fallback, an unused extra map key, and a caption exceeding Rust's byte limit. The main matrix uses finite numeric observations; the existing shared reference retains the separate nullable/coercion cases.

Rust comparison boundaries remain explicit:

- The proposed Rust map keeps IDs, roles and row count even when captions are equal or empty. Nivo deduplicates by caption, retaining the first source item and its numeric ID/color.
- Native numeric legend IDs are recorded unchanged. `sourceKey` is an adapter annotation translating the numeric index to the configured raw domain; it is not a native field.
- Rust Group legends keep actual eligible groups; legacy SubGroup legends retain the complete domain, including reserved/hidden identities. Native legends only contain actual summaries.
- Missing map entries return the native raw selected key. Rust preserves its existing fallback, including `Samples` for an empty subgroup; an explicit empty caption can request empty text.
- Rust checks every map key against the selected full domain and every caption against 1024 UTF-8 bytes. A native callback may ignore extra map-descriptor entries or return longer strings; no native UI layout is claimed here.
- A key-stable map cannot produce multiple different labels for one selected identity. The recorded non-map callback and arbitrary property accessors remain outside the proposed API. No new series alias or legend action protocol is inferred.

No React mount, event dispatch, Spring integration, original legend component, font layout, SVG/Canvas/CPU pixels or Rust implementation is executed by this producer. Generation and `--check` validate original-function reference output only; current product acceptance requires its own source and functional checks.
