# Original Calendar month-spacing reference

`node scripts/calendar-month-spacing-reference.mjs <released-node_modules>` generates the reference; append `--check` to verify it. No dependency installation occurs. The producer runs a child with explicit `TZ=UTC`, then executes the complete unmodified Nivo0.99 `computeLayout`, `bindDaysData`, year/month legend-position functions and original `alignBox`. Real Lodash4.17.21 and complete original D3 UMD distributions are used. Original source and licenses are retained in [vendor](vendor/source-pins.json). Runtime package source hashes are in [reference.json](reference.json).

Pinned existing packages: d3-time3.1.1, d3-time-format4.1.1 and d3-array3.2.5. The dist-only package form is deliberate; no translated calendar/time implementation substitutes for the original functions.

Ten profiles contain5120 days and168 months across both orientations, leap/multiple years and spacing0/6/8/12/22. Raw native day positions, paths/bounds and year/month label positions remain intact. The same-week Jan31/Feb1 witness differs by exactly8px for spacing8. Three raw invalid native profiles retain negative, NaN and Infinity results; nonfinite values use `n:NaN`/`n:Infinity` strings. They are separate from checked Rust's strict finite/nonnegative profile.

## Occupied-week qualification

The original Calendar reserves `max(timeWeeks(yearStart,nextYear).length)+1`; the existing Rust layout uses `max(ceil((civilDayCount+January1Weekday)/7))`. These are independently evaluated using original D3-time civil-year facts. In the2023 multiple-year profiles, native reserves54 columns while53 are occupied. Height limits both cell sizes to10px. Native origin and occupied-column origin are independently computed from the authored width, twelve month gaps and their respective column counts; the resulting5px translation is recorded explicitly. The other eight profiles have no origin correction. Nothing is fitted to candidate output or color.

The Rust reference test checks all5120 day rectangles,168 month bounds and both directions of actual path-boundary vertex distance at1e-9. Corrected profiles use the declared legacy occupied-week alignment only; global native prop equality is not claimed. Raw native records remain unchanged. Arbitrary first weekdays, rectangular cells, padded month unions, header/baseline placement and explicit-zone aggregation are tested as Rust behavior separately.

Source replay is not a renderer/browser/device acceptance result.
