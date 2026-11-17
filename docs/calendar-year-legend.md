# Calendar year labels

[简体中文](calendar-year-legend.zh-CN.md)

These APIs are available in Kartis 0.2.0.

`calendar_layout_with_year_legend` adds explicit FullYears year-label placement
and borrowed, fallible plain-text formatting. These are the year labels beside
the calendar panels, not `Scene.legend` rows. Existing `calendar_layout` and
`calendar_layout_with_color_profile` calls retain their complete previous output.
No fields were added to `CalendarLayoutOptions`.

```rust
use kartis::*;

let config = ChartConfig::default();
let options = CalendarLayoutOptions {
    mode: CalendarLayoutMode::FullYears,
    start: Some((2024, 1, 1)),
    end: Some((2024, 12, 31)),
    ..Default::default()
};
let placement = CalendarYearLegendOptions {
    position: CalendarLegendPosition::After,
    offset: 18.,
};
let prefix = String::from("Fiscal year");
let mut text = |year: &CalendarYearLegendContext| {
    Ok(format!("{prefix} {}", year.year))
};
let layout = calendar_layout_with_year_legend(
    &config, &[], &options, Some(&placement), Some(&mut text),
)?;
# Ok::<(), ChartError>(())
```

The last two arguments are independent. `None` placement keeps the old header
label position, anchor and rotation; `None` formatter displays the integer year.
Passing both as `None` reproduces the existing FullYears layout, including its
scene, panels, days, months and color scale. The new entry requires FullYears
even when both arguments are `None`. Compact remains available through the
existing entries; Nivo TimeRange also excludes the year-legend configuration.

For color-space or scheme selection, use
`calendar_layout_with_color_profile_and_year_legend(config, data, options,
color_profile, placement, formatter)`. It accepts the same color profile as the
existing color-profile entry. Zoned data may be explicitly aggregated with the
existing calendar aggregation functions before using this civil-date entry;
there is no new zoned convenience entry.

## Placement and style

`CalendarYearLegendOptions::default()` selects `Before` with a 10-pixel offset.
Negative offsets move the origin toward the panel interior, as in Nivo's
placement formulas; a sufficiently large magnitude can cross the opposite edge.
For the **actual oriented** `CalendarPanel.grid = (x, y, width, height)`, the
unshifted placement origin and base rotation are:

| Orientation | Position | Origin | Base rotation |
| --- | --- | --- | --- |
| Horizontal | Before | `(x - offset, y + height / 2)` | −90° |
| Horizontal | After | `(x + width + offset, y + height / 2)` | −90° |
| Vertical | Before | `(x + width / 2, y - offset)` | 0° |
| Vertical | After | `(x + width / 2, y + height + offset)` | 0° |

Explicit placement uses `TextAnchor::Middle`. It inherits `year_style.size` and
`year_style.color`, falling back to the theme as before. Its rotation is the base
rotation plus `year_style.rotation`, normalized to `[−180°, 180°)`. The extra
rotation is a Kartis extension; the referenced Nivo year-label configuration has
no corresponding rotation option.

Kartis retains its existing alphabetic-baseline conversion: for font size `s`
and final rotation `r`, the stored `Label` position is
`(origin.x - sin(r) * s / 3, origin.y + cos(r) * s / 3)`. SVG, Canvas and CPU
renderers consume that same label. This approximates centering; it does not
promise font-specific visual-baseline equality with Nivo.

The profile only relocates labels. It retains the existing year-header space,
day geometry, month bounds, picking identities, label order and `Scene.legend`.
Positive `month_spacing` is reflected in the actual panel bounds used by the
placement formula. No additional margin is allocated: labels outside the chart
viewport can be clipped, so choose the chart margins and offset together.

## Formatter and validation

`CalendarYearLegendTextFactory<'a>` is a borrowed
`FnMut(&CalendarYearLegendContext) -> Result<String>`. The context exposes `year:
i32`, chronological panel `index`, and actual oriented `grid`. It formats plain
text; it is not an arbitrary React component or Glory widget factory.

The formatter runs once per emitted year label, after placement inputs validate.
`year_labels: Some(false)` disables the labels and skips the formatter. As with
the existing calendar, `hidden_years` hides that year's days and owned labels
but retains shared year decoration; its year formatter therefore still runs.
An empty result retains the empty label's slot and does not suppress or reorder
other labels. Display text never changes `Scene.legend` metadata.

Offsets must be finite and within `-1024..=1024` pixels. Each formatter result must
fit `MAX_LABEL_BYTES` (currently 1024 UTF-8 bytes), including an exactly bounded
Unicode string. Existing limits still apply: at most 32 years and 12,000 days,
with validated sizes, rotations and geometry. Invalid inputs, over-budget text
and formatter errors return `Err` without returning a partial layout. External
side effects performed by the formatter are not rolled back.

## Reference and verification

The reference is Nivo commit `0dae2c32052a573f9f1d66ec1b453b572119b265`:

- `packages/calendar/src/types.ts`: `yearLegend`, `yearLegendOffset`,
  `yearLegendPosition`; `TimeRangeSvgProps` excludes these properties.
- `packages/calendar/src/props.ts`: integer-year formatter, `before`, offset 10.
- `packages/calendar/src/hooks.ts`: `useYearLegends` calls
  `computeYearLegendPositions` from `compute/calendar.ts`.
- `packages/calendar/src/CalendarYearLegends.tsx`: middle text anchor and
  translate/rotate transform, without `dy` or an explicit text-baseline attribute.
  `packages/text/src/Text.tsx` forwards attributes and theme styles.
- `packages/calendar/src/CalendarCanvas.tsx`: center alignment and `middle`
  baseline before translating and rotating each label.

These source paths describe the upstream repository; they are not required
dependencies for using this API. Placement follows the original four formulas
relative to Kartis's panel bounds. The existing occupied-week allocation,
header reservation and baseline policy remain Kartis policies; this is not a
claim of full prop-for-prop Nivo layout or font-rendering equality.

`tests/calendar_year_legend.rs` checks both orientations and positions, negative,
zero and both boundary offsets, month spacing, inherited style and composed rotation, unchanged
default layouts and color profiles, hidden/disabled labels, empty and escaped
Unicode text, byte limits, formatter failure, SVG/Canvas serialization and
actual Lato-font CPU raster output. Run:

```sh
cargo test --test calendar_year_legend --features raster
```
