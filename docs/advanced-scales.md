# Continuous transforms, piecewise axes and geographic inverse

[简体中文](advanced-scales.zh-CN.md)

## Color inputs

`ColorScaleOptions::transform` selects Linear, Log, Pow, Sqrt or SymLog for Sequential and Diverging colors. Linear remains the default; discrete scales require Linear. This input transform is independent of the selected color interpolation space or named ramp.

HeatMap, calendar and choropleth pass the same profile to their resolved scale. Use it with `append_color_legend` so tick positions, gradient samples and actual data colors agree. A Diverging midpoint remains a data value and can appear away from the geometric middle of the legend.

Log domains require one nonzero sign; negative powers require one same-sign nonzero branch. Source observations remain finite or explicitly missing. Query behavior is separate: finite D3 signed-power extrapolations and clamped zero limits are retained; final nonfinite color parameters use `unknown`.

Empty or all-missing observations keep the general automatic `[0,1]` domain. Give Log and negative powers an explicit valid domain in that case. This checked policy does not invent nonzero calibration bounds or silently discard zero/opposite-sign observations.

`position(value)` exposes the transformed axis fraction. `value_at(offset)` accepts finite offsets in `[0,1]` and returns a checked inverse inside the authored domain. Overflow, underflow outside that domain and nonrepresentable inverse results return an error.

## Full-breakpoint chart axes

`PiecewiseScaleOptions` stores full `domain` and `range` vectors plus `ContinuousScaleOptions`; the existing two-endpoint options retain their Copy API. Numeric and UTC profiles accept 2–4096 matching stops. Domains may be nondecreasing or nonincreasing, including repeated knots and fully constant domains; genuine turns reject. UTC stops must be bounded integral epoch milliseconds. Repeated knots can create jumps and follow D3's native-transformed reverse-then-bisect-right rule, including negative powers. See [repeated domain knots](piecewise-repeated.md) for the constant-domain midpoint and whole-domain clamp rules.

`ResolvedContinuousScale::piecewise` and `ChartScale::with_piecewise_options` use actual output coordinates. Nice extends only the outer domain stops, preserving interior knots; Reverse reverses the full range. Ticks stay in source-value units. Round applies ECMAScript rounding to the computed floating-point result; inverse stays unrounded.

Ranges may turn or contain plateaus. Forward mapping remains available, while `invert` rejects an ambiguous global inverse. `invert_segment(index,pixel)` selects a branch; a flat range branch still rejects. A repeated domain branch can have a constant algebraic inverse. `is_invertible()` identifies unique range-branch selection, rather than a bijection or a guaranteed map/invert round trip.

Actual charts use `PiecewiseLineScaleOptions` with `line_layout_with_piecewise_scale_options`, or `ScatterAxisScale::Piecewise`. Their ranges are plot fractions beginning at 0 and ending at 1. Interior fractions may turn or leave that interval; the ordinary plot clip applies. Axes, paths, factories, raw-data anchors and picking share the resolved mapping.

### Data viewports

**Repeated-knot viewport integration is available in Kartis 0.2.0.** The native Line and Scatter acceptance matrix checks 4,128 original-D3 viewport records, including complete reflow, checked rejection and reset. Browser paint and platform math are separately scoped in the [repeated-knot guide](piecewise-repeated.md).

Line and Scatter layout controllers retain all original displayed knots. Like original D3 `rescaleX`/`rescaleY`, navigation inverts every original range stop through the accumulated pixel transform. Resize preserves accepted domains and changes the dimensions used by later input.

`BrowserViewportLayout::domains` still provides canonical outer endpoints. `piecewise_domains` additionally carries complete authored-order stops:

- Line: use the baseline layout's `viewport_scale_options(&request.piecewise_domains)`.
- Scatter: use `viewport_scale_options_with_piecewise(request.domains,&request.piecewise_domains)`.

Pass the resulting profiles to the next layout, and use outer domains for ordinary axes. Do not rebuild a piecewise mapping from two endpoints. The endpoint-only Scatter helper explicitly rejects piecewise axes. Turning or plateau ranges can be drawn but cannot create a unique data viewport. Static constant domains are supported; navigation still requires a usable outer extent. True turns, invalid transforms and nonrepresentable proposals must preserve the accepted controller and frame.

Repeated-knot navigation must retain the original local pixel range and invert `(rangeStop - tx)/k` in the original mapping. An additional normalized `[0,1]` round trip can change discontinuous Log branch selection or UTC TimeClip by a millisecond. Range direction already includes source and output reversal; resize must preserve accepted domains and original mapping provenance.

## Geographic inverse

All ten existing projection families expose `GeoProjectionContext::invert([x,y]) -> Result<Option<[lon,lat]>>`. It uses the accepted fitted raw center, scale, translation, camera center and roll, including transverse Mercator's frame convention.

As with `project`, `invert` checks the intrinsic projection footprint and explicit cap without implicitly clipping the plot rectangle. `invert_in_plot` adds that rectangle check. Nonfinite or over-budget Scene coordinates return an error; points outside the supported footprint return None. Poles and seams need not have a unique longitude representation.

Browser handles provide `scene_point_at_client`, `invert_geo_at_client` and `invert_geo_in_plot_at_client`. Raster surface handles provide corresponding `*_at_host` helpers taking the current `RasterSurfaceBounds`. Each reuses accepted host conversion and inverts the Scene camera once. Explicit plot helpers check the displayed and authored plots.

Applications retain the projection context together with its accepted frame. A dimensions check detects size mismatches; it does not detect arbitrary stale contexts of the same size.

## Numerical and host boundaries

The independent references execute pinned original D3 functions. Explicit piecewise scales now share fixed `libm` log/log1p/exp/expm1 primitives, including after nice or a viewport rebuild removes repetition; ordinary two-endpoint scales keep their earlier arithmetic. Comparing 31,904 queries from 2,400 earlier strict-domain profiles finds 76 changed raw bit patterns, maximum absolute delta `5.684341886080802e-14`. The only two rounded changes correct the earlier recorded SymLog `208` results to original D3's `207`; the historical values and difference list remain preserved.

General signed powers retain platform `powf` in native builds and standalone `wasm32` core builds without `canvas`. On `wasm32` with `canvas` (including `web`), explicit piecewise scales safely import the actual host's `Math.pow`, selecting the same primitive as original D3 in that runtime. This feature selects browser math; it does not change ordinary two-endpoint scales or the fixed `libm` logarithmic primitives. The initial full WASM matrix retained 108 failures across 72 exponent-`0.25` profiles before that import. The rebuilt browser-math version completed the same 9,496-profile matrix in one Chromium context with no differences or one-ULP rounding exceptions. This measured WASM result is separate from native acceptance, and identical transcendental bits across engines are not promised. At a repeated knot, an ULP can select an entirely different branch and requires diagnosis rather than a general pixel tolerance. Geographic exact-cap and singular cases remain separate from ordinary inverse equivalence and retain the existing projection boundary tolerance.

Browser publication retains its revision telemetry. Unchanged paint attributes are not rewritten. Hosts supporting Canvas2D `reset()` reuse unchanged backing dimensions while clearing drawing state; older hosts retain the dimension-reset path. This does not promise that explicit Scene replacement preserves gesture ownership. The controlled same-frame gradient failure test covers caught renderer errors and restoration of old pixels, not arbitrary user callback side effects.

See [verification](verification.md), [feature gaps](feature-gaps.md) and [third-party notices](third-party-notices.md).
