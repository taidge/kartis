# Repeated piecewise domain knots

[简体中文](piecewise-repeated.zh-CN.md)

**Available in Kartis 0.2.0.** Numeric scales accept repeated knots and fully constant domains.
Line/Scatter integration passes the native and four-browser SVG/Canvas profiles
described below. Published Kartis 0.1.0 does not include this extension.

## Mapping and ties

`PiecewiseScaleOptions` contains matching numeric `domain` and `range` vectors
with 2–4096 stops and the existing `ContinuousScaleOptions`. Domains may be
nondecreasing or nonincreasing, including all-equal stops. A genuine direction
change rejects. Stops remain in authored order.

```rust
use kartis::*;

let scale = ResolvedContinuousScale::piecewise(&PiecewiseScaleOptions {
    domain: vec![0., 1., 1., 2.],
    range: vec![0., 10., 20., 30.],
    ..Default::default()
})?;
assert_eq!(scale.map(1.), 20.);
assert_eq!(scale.invert(15.)?, 1.);
assert_eq!(scale.map(scale.invert(15.)?), 20.);
# Ok::<(), ChartError>(())
```

Repeated knots can create a jump. Selection follows original D3's continuous
scale: transform the domain, reverse both stop arrays if the transformed last
stop is below the first, then bisect right over the interior knots. With range
`[0,10,20,30]`, Linear domain `[0,1,1,2]` maps `1` to `20`; descending domain
`[2,1,1,0]` maps it to `10`. Pow with exponent `-1` and domain `[1,2,2,4]` maps
`2` to `10`, because its native signed-power transform decreases. Internal
clipping coordinates use the same resolved transform, including the reciprocal
normalization; selecting only from the raw domain would lose this distinction.

For a two-stop constant domain, D3's bimap always uses the range midpoint. With
three or more equal stops, polymap chooses the first range pair below the
transformed knot and the last pair at or above it. Thus Linear domain `[2,2,2]`
and range `[0,10,20]` map `1` to `5`, and `2` or `3` to `15`. Negative powers
reverse which raw side is below the transformed knot.

Clamp acts on the **whole raw domain before transformation and segment
selection**. The clamped constant-domain example maps every finite query to
`15`. A repeated edge is not a separate global clamp plateau. Round applies
ECMAScript rounding after interpolation; inverse remains unrounded. Nice changes
only the outer domain stops, Reverse reverses every range stop, and ticks remain
in source-value units.

## Validation and inverse branches

Adjacent equal raw stops may have equal transformed values. Distinct raw stops
that collapse to the same transformed coordinate still reject. Finite arithmetic,
representable spans and existing kind-specific constraints remain checked: Log
requires one nonzero sign, negative powers require one same-sign nonzero branch,
and UTC requires bounded integral epoch milliseconds. UTC inverse retains Date's
truncating TimeClip rule.

`is_invertible()` means a strictly monotonic range selects one inverse branch.
It does not promise a bijection, continuity or `map(invert(pixel)) == pixel`.
Repeated domain intervals can produce a constant algebraic inverse. A turning
or plateau **range** keeps the earlier checked policy: global `invert` rejects;
`invert_segment(index,pixel)` chooses an explicit branch, and a flat range branch
rejects even there. With clamp enabled, the explicit branch inverse clamps to
that branch's domain; global inverse clamps to the whole domain after inverse
transformation.

## Numeric profile

All explicit piecewise scales share fixed `libm` log/log1p/exp/expm1 primitives
and cached transformed interpolation. This policy also applies when nice or a
rebuilt viewport removes the repeated stops. It avoids relying on private
source-history flags that a public `PiecewiseScaleOptions` cannot carry. Ordinary
two-endpoint continuous scales retain their existing arithmetic.

This matters at discontinuities. For SymLog constant `2`, the original D3
transform distinguishes the knot `-9.3` from the adjacent smaller floating-point
input, while the adjacent larger input genuinely shares the knot's transformed
value. A host `ln_1p` implementation previously merged the smaller input with
the knot as well, selecting a different range segment. Raw-domain bisect would
also be incorrect, because the genuine transformed tie must remain a tie.

The change was measured against the earlier strict-domain numeric behavior:
among 31,904 queries from 2,400 profiles, 76 raw bit patterns change, with maximum
absolute difference `5.684341886080802e-14` and maximum relative difference
`8.574643181216107e-16`. Two previously recorded SymLog rounding differences
change from `208` to `207`, matching original D3. The original reference values
and historical difference list remain preserved.

Native builds and standalone `wasm32` core builds without `canvas` retain platform
`powf` for general signed powers. On `wasm32` with `canvas` (including `web`), explicit
piecewise scales use the actual host's `Math.pow` through a safe wasm-bindgen
import. Original D3 uses that same runtime primitive. This feature therefore
selects browser math instead of standalone core platform math; ordinary
two-endpoint scales keep their earlier arithmetic. The fixed `libm`
log/log1p/exp/expm1 policy is unchanged.

The pinned V8 13.6 native reference uses `std::pow` by default; its legacy fdlibm
pow is a different numeric profile. An initial full 9,496-profile WASM probe
recorded 108 differences across 72 exponent-`0.25` profiles before the browser
math import. All failed values and bulk packets remain preserved. The rebuilt
browser-math version completed that same full matrix in one Chromium
context with no differences or one-ULP rounding exceptions. This is a measured
WASM result, separate from the native pass. Using the same runtime primitive does
not promise identical transcendental bits across operating systems or browser engines. Near a repeated
knot, one ULP may select another branch and cause a material output jump; a
general pixel tolerance must not hide that behavior.

## Charts and viewport acceptance

Line uses `PiecewiseLineScaleOptions` with
`line_layout_with_piecewise_scale_options`; Scatter uses
`ScatterAxisScale::Piecewise`. Chart ranges use plot fractions starting at `0`
and ending at `1`; low-level numeric scales accept actual output coordinates.
Static constant-domain scales are supported. A viewport still needs a usable
outer extent and a strictly monotonic range.

The repeated-knot viewport target is original D3 `rescaleX`/`rescaleY`: retain the
full authored mapping and original local pixel range, then invert each stop
through the actual pixel transform. For baseline extent `L`, window start `a`
and span `b`, `k = 1/b`, `tx = -a*L*k`, and each new knot comes from the original
scale's `invert((rangeStop - tx)/k)`. Range direction already includes source and
output reversal. A separate `[0,1]` normalize/invert round trip can change a
constant Log result or a UTC value by a millisecond. Resize must preserve the
accepted domain and original mapping provenance.

The native matrix checks all 4,128 original-D3 viewport records through both
Line and Scatter. Each consumer rejects 1,216 constant display baselines and
eight native true-turn proposals. Line accepts 2,904 reflows and resets;
Scatter accepts 2,848, while rejecting 56 proposals whose retained point centers
exceed Scene coordinate budgets. Both retain 872 fully constant windows and
reset through the original controller. Four additional unbounded requests
check the authored scale's inverse clamp. Bounded navigation constrains the
pixel window; it does not add a second raw-domain clamp to inverse results.

Chrome, Firefox, WebKit and Edge at DPR 1/2 additionally pass 288 paired
SVG/Canvas frames, 256 trusted pointer/keyboard pick pairs, 96 paired rejected
publisher cases and 16 constant-window/reset cases. Serialized SVG samples
retain the live inherited language so their font selection matches the chart.
The entire-frame pixel mean remains below the unchanged 0.5 gate, including UTC
date labels. This does not imply compositor byte identity or identical math in
every engine. See the [advanced scale guide](advanced-scales.md) for the layout
and callback APIs and [verification](verification.md) for scoped evidence.
