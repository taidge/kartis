# Bump point component reference

`original/` preserves eight complete Nivo 0.99.0 Bump/point-scale files and the
complete Bump and Scales MIT licenses. The reference records their hashes, the
producer hash and the installed pinned D3 point-scale dependency hashes.

The producer executes the original `computeSeries`, `castPointScale`, series
and point style recipes, point mapping, `createElement` argument mapping, layer
gates, default Point spring input and radius clamp. Synchronous hook adapters
evaluate source expressions; they do not implement React memoization,
scheduling, component lifecycle or actual Spring values. Color accessors are
instrumented probes, not an inherited-color math oracle. The line `.defined`
predicate is executed, but no D3 line-path drawing equivalence is claimed.

`computeSeries` removes null-rank points before the point mapping hook. Its
line points independently retain null separators. Original datum indices remain
in point IDs, zero ranks are valid, and zero point sizes still produce component
calls. The point layer is independent of `isInteractive`/`useMesh`; only the Mesh
layer needs both. Active series flags and active point size selection use
different ID arrays. Default Point has `pointerEvents: none` and does not apply
the computed series opacity as a circle attribute.

`manualHookInputDiagnostic` separately feeds an invented null-coordinate point
directly to the downstream hook. That hook does not add a second null filter;
the diagnostic is outside `computeSeries` output and is not an additional
normal-pipeline component or a Rust role obligation.

`points.tsv` is an original-function-generated sidecar for the focused Rust
numeric/role test. It compares the shared typed input subset at an absolute
coordinate tolerance of `1e-12`. It is not a React or arbitrary extra-props test.

After installing pinned project packages, use Node 24 or newer:

```sh
node scripts/bump-point-widget-reference.mjs --frozen
```

An optional fixture directory follows `--frozen`. This mode only reads frozen
sources and installed dependencies, verifies source/license hashes, reexecutes
original expressions, and compares all results and the point sidecar exactly.
It neither invokes Git nor rewrites files. Regeneration is separate:

```sh
node scripts/bump-point-widget-reference.mjs <nivo-checkout> [output-directory]
```
