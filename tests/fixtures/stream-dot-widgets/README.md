# Stream dot component reference

`reference.json` records actual Nivo 0.99.0 source execution. The producer
extracts and executes the original `StreamDots` `data.map`, `getDotY`, dot-layer
gate, and `StreamDotsItem` spring input expression. The complete original files
and their MIT license are preserved in `original/`; hashes and the producer hash
are recorded in the reference.

The `createElement` stub records its arguments before React reserved-key
handling. This verifies call order, source datum identity, Start/Center/End
coordinates, size-zero invocation, and default circle radius. It does not verify
React lifecycle, animation, event transport, or renderer pixels. Source-column
indices are React element keys; no category-key lifecycle equivalence is claimed.

Nivo's public computed dot value is numeric. The separately named runtime-null probe
records JavaScript behavior outside that public TypeScript contract. Kartis's
nullable extension excludes null dot roles while preserving their original
nullable series and slice placeholders; enabled numeric zero remains eligible.

Regenerate with Node 24 or newer after installing the pinned project packages:

```sh
node scripts/stream-dot-widget-reference.mjs <nivo-checkout>
```

CI verifies the frozen reference using the preserved original source files:

```sh
node scripts/stream-dot-widget-reference.mjs --frozen
```

An optional fixture-directory argument follows `--frozen`. This mode checks
source and complete license hashes, reexecutes the original functions, and
compares all inputs, calls, positions, styles, gate results, and the separate
runtime-null result exactly. It neither invokes Git nor rewrites fixtures.
The recorded Node version is diagnostic; the executed source and numeric
results remain exact checks across Node patch versions.

Existing `stream-reference.txt` separately covers original Nivo/D3 numeric
stacking, scales, and area curves. Neither reference treats different stack or
rendering algorithms as pixel-equivalent.
