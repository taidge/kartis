# BoxPlot custom-color reference

[简体中文](README.zh-CN.md)

This fixture executes the complete pinned Nivo `getOrdinalColorScale`, BoxPlot hooks, summary and generation functions, inherited-color resolver, transition hook and legend helper. The legend helper uses genuine Lodash 4.17.21. Source bodies are unchanged; TypeScript annotations and imports are adapted for execution. React hooks use a cold recording adapter. Component JSX is pinned for source provenance and is not rendered.

Regenerate or check from the package root:

```text
node scripts/box-colors-function-reference.mjs <released-D3-node_modules>
node scripts/box-colors-function-reference.mjs <released-D3-node_modules> --check
```

The producer checks released D3 package versions and pins their complete JavaScript/JSON/license resources. `reference.json` contains original source hashes, runtime hashes and adapter limits. `values.json` contains 32 primary profiles and 16 qualifications. There are 49 primary cold snapshots because the active-state qualification explicitly invokes the original hook twice; 32 additional repeated invocations check deterministic output. Counts of boxes, color calls and legend rows describe the primary snapshots only.

The matrix covers Group/SubGroup identities, both orientations, constant/group/subgroup/mean/quantile-value/cell-dependent functions and zero/half alpha. Each real callback input and output is recorded. `partial-domain` calls come from the original hook's complete-domain warm-up, while `summary` calls receive original summary objects. The custom-function ordinal branch returns the actual supplied function and does not create `.scale`. Exact-label deduplication and stable full-color-index sorting are the real Lodash operations; representative indices refer to the retained original row objects.

The qualifications preserve these boundaries:

- A function reading `values[2]` without a guard genuinely throws during the first partial-domain callback, before any summary. A borrowed Rust summary factory need not invent this partial callback contract.
- Native `Number(null)` creates zero values and null-only summaries. Rust `None` is omitted from statistics; raw observation indices still retain its source position. Native hidden examples use an explicit upstream filter and retain full configured domains.
- Ordinary native summarization drops arbitrary raw properties. The real precomputed-summary spread can retain them, and actual Lodash datum paths observe the difference. Undefined, numeric and invalid-CSS results remain native qualifications, not valid typed Rust colors.
- Original summary `quantiles` are configured probabilities; `values` are computed statistics. Projected geometry and raw observation slices are provenance annotations, not extra fields supplied by Nivo to its color callback.
- CSS alpha zero can lose hidden RGB through D3 modifier parsing; the opacity-restoration qualification records the resulting black. Typed Rust RGBA deliberately preserves its original RGB. The exact CSS half opacity and its rounded 8-bit representation remain distinct recorded data.
- Fixed border, median and whisker colors are independent of the custom numeric color. Nivo's pinned BoxPlot component has no outlier marks. Rust default and fixed outlier behavior requires Rust regression tests rather than an invented native renderer oracle.
- The explicit active-state example is two cold original invocations. It is not a React hover, memo, commit or lifecycle measurement. Rust accepted-color caching may avoid re-invoking its borrowed factory during hover.

The fixture does not prove browser, GPU, worker, spring animation, renderer pixels or widget behavior. `ByLabel` first-representative color and canonical target are recorded; it does not claim that Rust legacy `Keys` reserved rows match Nivo's actual-box-only legends.
