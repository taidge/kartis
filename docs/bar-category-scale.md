# Configured category bands

`BandScale::with_options` provides checked D3 band rounding, alignment and separate inner/outer padding. `BarStyleOptions::category_scale` applies the same scale to bar positions, category ticks, totals and returned category metadata.

```rust
use kartis::*;
use kartis::scales::{BandScale, BandScaleOptions};

let categories = ["A", "B", "C"].map(str::to_string);
let profile = BandScaleOptions {
    padding_inner: 0.2,
    padding_outer: 0.2,
    round: true,
    ..Default::default()
};
let scale = BandScale::with_options(categories.to_vec(), [0., 101.], &profile)?;
assert_eq!(scale.map("B"), Some(38.));
assert_eq!(scale.bandwidth, 25.);
assert_eq!(scale.step(), 31.);

let layout = bar_layout_with_style(
    &ChartConfig::default(),
    &categories,
    &[BarSeries::new("values", vec![Some(0.), None, Some(3.)])],
    &BarOptions::default(),
    &BarStyleOptions {
        category_scale: Some(profile),
        ..Default::default()
    },
)?;
let svg = render::to_svg(&layout.scene)?;
# Ok::<(), kartis::ChartError>(())
```

The explicit profile resolves rounding over the entire category range. A category window then projects those resolved positions continuously; it does not round them again. `StyledBarLayout::category_scale` exposes the resolved, windowed scale. Grouped, stacked and percent layouts use its step and bandwidth; grouped-series pixel gaps remain controlled by `inner_padding`.

Set `category_reverse: true` with an explicit profile to reverse the category range while preserving original category/source identities. This provides Nivo's reversed y-band direction for horizontal bars without reordering input data. `category_reverse` requires `category_scale`. An explicit profile cannot be combined with the older `category_inner_padding` or `category_outer_padding` overrides. `category_scale: None` and `BandScale::new` retain their existing behavior.

Inner padding and alignment must be finite in [0,1], and outer padding finite in [0,4]. Domains contain at most 20,000 unique nonempty strings of at most 1,024 bytes; ranges and projected results must be representable. Inner padding 1 produces zero bandwidth. `invert` follows the existing band-interval convention, including exact logical positions for zero-width bands; it is not a painted-geometry hit test. A grouped pixel gap that exceeds available bandwidth is rejected.

Run `cargo run --example bar_category_band -- output-directory` for four actual SVG/Canvas scenes covering horizontal/vertical and grouped/stacked bars. Original D3/Nivo numeric fixtures and their complete licenses are retained under `tests/fixtures/bar-category-band`. Adding public options/layout fields expands the source API; use `..Default::default()` when constructing options.
