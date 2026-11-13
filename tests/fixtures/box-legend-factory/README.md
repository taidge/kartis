# BoxPlot ByLabel: original Nivo reference

This fixture executes Nivo 0.99.0 commit `0dae2c32052a573f9f1d66ec1b453b572119b265`. It contains 32 profiles and eight qualifications produced by the complete original BoxPlot hooks, stratification, summary, generation and legend functions. The original property accessor and genuine Lodash 4.17.21 execute; no substitute deduplication, sorting or property lookup algorithm generates expected legend rows.

```sh
node scripts/box-legend-factory-reference.mjs node_modules --check
```

The argument supplies released D3 packages: d3-scale 4.0.2, d3-interpolate 3.0.1, d3-array 3.2.5, d3-color 3.1.0 and d3-format 3.1.3, plus their d3-time, d3-time-format and internmap dependencies. `reference.json` records the source/runtime hashes and the generator hash. `vendor/source-pins.json` records original paths and provenance; complete upstream licenses accompany the frozen sources.

The adapter erases imports and TypeScript types, then imports the complete original bodies. Cold `useMemo` evaluates immediately; state is null; state setters throw. The original transition hook runs through a recording adapter. UI placeholders and unsupported scale/scheme branches throw. The supplied formatter function is returned unchanged by its adapter. This demonstrates original synchronous data computation, not mounted React behavior, physical rendering, animation or lifecycle callback counts.

## Machine-readable input and expected output

`values.json` has `profiles[]` and `qualifications[]`. Each entry has `id`, `input` and `actual`. A qualification also records its precise scope difference.

| Field | Meaning |
| --- | --- |
| `input.rawData` | Original observations, in original caller order. `v: null` remains null. |
| `input.effectiveData` | Actual data passed to the original hook. Explicit hidden qualifications filter upstream; all configured domain slots remain. |
| `input.groups`, `input.subGroups` | Complete configured domains, including missing/reserved slots and authored order. |
| `input.label` | An actual callback input policy: pair, constant, raw-key or empty. The nested-property qualification passes an original property path instead. |
| `actual.ordinalDomain` | The real original hook's prewarmed D3 ordinal domain. |
| `actual.boxes` | Actual generated box identity, full indices, source summary, geometry, original numeric color/RGBA and opacity. |
| `actual.callbackContexts` | Actual callback arguments and returned labels. `data` is the whole original summary. Source observation indices are explicitly annotated separately. |
| `actual.expectedLegend` | Actual hook legend rows, annotated with source representative and typed raw target. Expected labels, colors and ordering come from the original hook. |
| `actual.originalLegendOperations` | Actual Lodash input/output rows and selected input-object indices. Wrappers delegate to genuine Lodash and preserve the returned object references. |
| `actual.transitions` | Actual original transition-hook records, not sampled animation frames. |

Each expected row retains native numeric `nativeId`, `fullColorIndex`, `label`, `color`, `rgba`, `representativeInputIndex` and the representative summary's complete identity. `target.kind` is Category for Group or Series for SubGroup, with `rawKey` kept separate from display text. This annotation does not invent canonical Rust series encoding or transport row IDs.

The producer observes the original `uniqBy(label)` output references, then the original stable `sortBy(id)` output references. It finds the selected original mapped row by object identity. Same-key/multiple-label rows can have duplicate native numeric IDs. Cross-key/same-label rows retain only the first representative and its color/target; they do not union targets. Empty text produces a genuine retained empty legend row.

`data.quantiles` and `quantileProbabilities` are the configured probabilities `[0.1, 0.25, 0.5, 0.75, 0.9]`. `data.values` and `computedStatistics` are the five actual statistical values. These meanings must not be exchanged when constructing a Rust factory context.

## Rust test mapping and explicit differences

Use `input` to build the complete source/domain and derive the configured callback policy. Compare Rust ByLabel output with `actual.expectedLegend` after translating each recorded raw target to the Rust typed target; do not use display text as identity. Compare representative source identity and full color index as well as label/color/order. Keep the legacy Keys path's full SubGroup domain and `Samples` fallback as separate compatibility behavior.

The 32 main profiles cover Group/SubGroup, four callback policies, full and shuffled domains, sparse cells with real constant zero, and an empty reserved domain. The eight qualifications record nullable data, upstream hidden filtering, raw empty subgroup, disabled legends, and actual nested `values[2]` property access.

Nivo calls `Number(null)`, producing a real zero-valued summary even for a null-only cell. Rust omits `None` from statistics and has no None-only summary. Test this difference explicitly. Hidden filtering is an input adapter, not a native hook hidden API. The numeric nested-property label is native output; Rust's String factory requires explicit conversion. Alpha-zero palette CSS preserves the original supplied RGB here; other inherited-color parser paths may lose transparent RGB and must not force changes to Rust raw colors.

Counts in `reference.json` describe 40 archived primary cold snapshots. Another 32 complete cold evaluations assert deterministic equality. They are not a promise about React, multiple legend configurations, or the number of times a Rust builder invokes a callback.
