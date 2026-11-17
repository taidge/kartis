# Bar component reference

Eight complete Nivo 0.99.0 source files and the complete MIT license are
preserved in `original/`, including grouped/stacked compute sources. Their hashes
and the producer hash are recorded in `reference.json`.

The producer extracts and executes original `barsWithValue`, bar-label
eligibility, label-layout, transition options, common props, layer gate and
`createElement` callback expressions. It also executes the original data
normalization/coercion helpers, default rectangle dimension projections, fill
choice and event/focus gates. No synthetic replacement component props are used.
`$number: "-0"` records signed zero exactly; `$function` records named function
probes. Undefined object properties remain absent as in JSON.

The reference verifies that null bars are excluded but zero bars survive;
`enableLabel` and `isInteractive` do not remove bar component roles. Vertical
entry/leave collapses height, horizontal entry/leave collapses width, and the
default rectangle projects animated dimensions through `Math.max(value, 0)`.
Style probes are plain values, not real React Spring values. This is not a React
rendering, animation scheduling, browser input, or pixel-equivalence oracle.

Rust Widget roles follow actual emitted bars: missing, hidden and fully cropped
bars have no role. This Rust clipping contract is not a filter in Nivo's original
component callback. Original source rows and categories remain separate from
computed bars, actual paint, and the optional value-label role.

After installing pinned project packages, use Node 24 or newer:

```sh
node scripts/bar-widget-reference.mjs --frozen
```

An optional fixture-directory argument follows `--frozen`. Verification checks
all source and license hashes, reexecutes the original expressions and compares
results exactly. It requires no Git checkout and never rewrites fixtures.
Regeneration is separate:

```sh
node scripts/bar-widget-reference.mjs <nivo-checkout> [output-directory]
```
