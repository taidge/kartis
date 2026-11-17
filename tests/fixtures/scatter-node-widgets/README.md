# Scatter node component reference

The original Nivo 0.99.0 sources and complete MIT license are preserved in
`original/`. The reference records their hashes and the producer hash.

The producer extracts and executes the actual `computePoints`, node-key,
regular/leave transition, transition-options, `createElement` callback and
default circle-radius functions. It checks raw datum object identity, global
and within-series indices, zero-size and off-plot inputs, unchanged node/style
arguments, and all seven `isInteractive` handler gates. Probe styles are plain
values, rather than real React Spring values. No React lifecycle, animation
scheduling, physical event transport, or renderer pixel equivalence is claimed.

The source `serieIndex` means the datum index within a series. Rust separately
exposes series and datum source indices. Rust hidden-series, missing-size-skip
and omitted Point-domain records have no actual site and therefore no Widget
role; these are explicit Rust eligibility rules, not filters found in Nivo's
original `Nodes` component.

After installing the pinned project Node packages, use Node 24 or newer:

```sh
node scripts/scatter-node-widget-reference.mjs --frozen
```

This mode only reads the frozen fixture, verifies all source and license hashes,
reexecutes the original expressions, and compares every result exactly. An
optional fixture-directory argument follows `--frozen`. Git and a source
checkout are not required for verification. Regeneration is separate:

```sh
node scripts/scatter-node-widget-reference.mjs <nivo-checkout> [output-directory]
```
