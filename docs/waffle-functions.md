# Waffle group functions and owned application data

This development API is unreleased. Waffle supports borrowed group color,
numeric value-format and border-color functions. Their results feed the actual
cell or area paint, group metadata, legends and interaction sites. An optional
owned frame keeps arbitrary application data alongside the numeric layout.
[中文](waffle-functions.zh-CN.md)

## Group functions

Use `waffle_layout_with_factories` with a borrowed `WaffleFactories` bundle.
Unset functions inherit `WaffleLayoutOptions`; existing `waffle_layout` calls
keep their ordinary behavior. No function is retained after construction.

| Field | Input | Output |
| --- | --- | --- |
| `color` | `&WaffleGroupColorContext` | `Result<Color>` |
| `formatted_value` | Original projected `f64` value | `Result<String>` |
| `border_color` | `&WaffleBorderColorContext` | `Result<Color>` |

`WaffleGroupColorContext` supplies the original numeric `RadialDatum`, its
`source_index`, the palette's `default_color`, plot and centered grid bounds,
total and unit. The unit is total divided by the number of grid slots. The
datum reference need not have the same address as the caller's input slice.

`WaffleBorderColorContext` supplies the same layout bounds, total and unit,
an optional default color, and one of two targets:

- `WaffleBorderTarget::Group(&WaffleGroup)` has the resolved fill and formatted
  value, hidden state, original requested range and capacity-clipped occupied
  range. At this phase `border_color` temporarily equals the new fill color,
  `polygons` is empty and `area_mark_id` is `None`. It is an early style input,
  rather than the final polygon-bearing group.
- `WaffleBorderTarget::Empty(WaffleEmptyStyle { color, opacity })` describes
  one shared empty-cell style. It has no source datum, group index or physical
  cell coordinate.

For each source group, configured functions run in this order: color, value
formatter, then border color. All groups participate in original source order,
including hidden groups, zero values, negative zero and groups whose requested
range exceeds grid capacity. The empty-style border function runs once after
all groups, even if there are no empty cells or the source is empty. Without
a configured border function there is no application callback for that role.

```rust
use kartis::*;

fn main() -> Result<()> {
    let mut color = |c: &WaffleGroupColorContext<'_>| {
        Ok(if c.datum.id == "late" {
            Color(210, 70, 35, 255)
        } else {
            c.default_color
        })
    };
    let mut format = |value: f64| Ok(format!("{value:.1} units"));
    let mut border = |c: &WaffleBorderColorContext<'_>| {
        Ok(c.default_color.unwrap_or(Color(30, 30, 30, 255)))
    };
    let layout = waffle_layout_with_factories(
        &ChartConfig::default(),
        &[RadialDatum::new("late", 30.), RadialDatum::new("ready", 50.)],
        &WaffleLayoutOptions {
            total: Some(100.),
            border_width: 2.,
            ..Default::default()
        },
        &mut WaffleFactories {
            color: Some(&mut color),
            formatted_value: Some(&mut format),
            border_color: Some(&mut border),
        },
    )?;
    assert_eq!(layout.groups[0].formatted_value, "30.0 units");
    assert_eq!(layout.groups[0].color, Color(210, 70, 35, 255));
    Ok(())
}
```

The complete [export example](../examples/waffle_factories.rs) writes Cells and
Areas SVGs plus a table containing original value bits, formatted values,
ranges and hidden state. It takes an explicit output directory:

```text
cargo run --example waffle_factories -- artifacts/waffle-factories-example
```

## Retaining original application data

`WaffleDatum<D>` contains `data: D` and `projected: RadialDatum`. The application
supplies the numeric projection explicitly. Only that projection determines
identity, allocation, numeric values and labels; an unrelated value inside
`D` is preserved without reinterpretation.

`WaffleDataFrame<D>::new` captures a source vector and its checked layout.
`with_factories` adds a borrowed `WaffleDataFactories<D>` bundle:

- Color receives `WaffleDataColorContext { data: &D, computed }`.
- Border receives `WaffleDataBorderContext { data: Option<&D>, computed }`;
  `data` is `None` for the empty-style template.
- The formatter still receives the projected number, not `D`.

The `computed` fields contain the corresponding base contexts described above.
Source order and callback counts are unchanged.

```rust
use kartis::*;

// Clone, Debug and PartialEq are intentionally not required on application data.
struct Job {
    overdue: bool,
    note: String,
}

fn main() -> Result<()> {
    let source = vec![
        WaffleDatum::new(
            Job { overdue: true, note: "original business record".into() },
            RadialDatum::new("late", 34.),
        ),
        WaffleDatum::new(
            Job { overdue: false, note: "retained even without cells".into() },
            RadialDatum::new("zero", -0.),
        ),
    ];
    let mut color = |c: &WaffleDataColorContext<'_, Job>| {
        Ok(if c.data.overdue { Color(210, 70, 35, 255) }
           else { c.computed.default_color })
    };
    let frame = WaffleDataFrame::with_factories(
        &ChartConfig::default(),
        source,
        &WaffleLayoutOptions {
            rows: 2, columns: 3, total: Some(100.),
            ..Default::default()
        },
        &mut WaffleDataFactories { color: Some(&mut color), ..Default::default() },
    )?;
    assert_eq!(frame.data(0).unwrap().note, "original business record");
    assert_eq!(frame.source()[1].projected.value.to_bits(), (-0f64).to_bits());

    let packet = frame.cell_widgets(3, |c| {
        let b = c.computed.cell.bounds;
        Ok(Some(PlotWidgetPlacement::Rect(LayoutRect {
            x: b.x, y: b.y, width: b.width, height: b.height,
        })))
    })?;
    assert!(packet.data.values().any(|row| row.data().is_some_and(|d| d.overdue)));
    assert!(packet.data.values().all(|row| row.frame() == &frame));
    Ok(())
}
```

The frame owns one source vector through shared storage. Cloning the frame or
its typed Widget payloads does not clone `D`. The base frame requires no
`Clone`, `PartialEq`, `Debug`, `Send` or `Sync` bound on `D`; actual thread
transfer still follows the contained types. `cell_widgets` requires `D: 'static`
for retained Widget payloads. Other frame methods do not impose that bound.

Frame equality means capture identity. A clone equals its source frame; an
independently built frame is different even when numeric layout and paint are
equal. Thus a raw-only update remains visible to Widget data comparisons,
including data whose own equality ignores fields or considers signed zeros
equal. The API preserves supplied values; it does not freeze application-owned
interior mutability.

## Typed Widgets and interaction

`frame.cell_widgets(revision, placement)` returns
`PlotWidgetsData<WaffleDataCellWidgetData<D>>`. Placement receives the actual
cell/Mark context and optional raw data. Each payload exposes `computed()`,
`data()` and `frame()`. Empty cells have no group or raw datum. Payloads retain
the entire source cohort, including groups with no occupied cell. Numeric
groups shared by multiple cells retain the existing shared group snapshot.

This adapter selects actual cell Marks. In Areas mode occupied cells belong
to union Marks, so only empty-cell Marks are eligible for cell Widgets.
It does not fabricate an area Widget. Existing measured-slot validation and
the 128-slot budget still apply. See [typed plot Widgets](typed-plot-widgets.md)
for mounting a packet with real Glory components.

`frame.interaction()` clones the numeric `SceneInteraction`.
`frame.area_interaction()` creates the checked, full-slot group event regions
described in [Waffle group areas](waffle-regions.md). Neither return value
contains `D`: keep the frame or a typed Widget payload when raw data must remain
available. Calling these methods does not rerun group functions.

## Combining custom cell geometry

Use `waffle_layout_with_factories_and_cell_factory`, or
`WaffleDataFrame::with_factories_and_cell_factory` for raw-data access. All
group functions and the empty-style template finish before the first cell
callback. The raw cell context supplies `data: Option<&D>` and the final
`computed: WaffleCellContext`; empty cells have `None`.

The existing cell factory is invoked only for actually drawn cell Marks, in
traversal order. It sees resolved fill, stroke and group formatting. Group
functions are not replayed per cell. Returning `computed.inherit(...)` preserves
these styles. Deliberately changing custom ink can differ from group/legend
colors, as documented in [custom Waffle cells](waffle-custom.md); the actual
Mark remains authoritative for physical paint and custom-ink picking.

## Validation and failure boundaries

Static source, options, palette/default-source and ordinary geometry validation
precede user functions. A color function cannot rescue an invalid source or
palette configuration. A formatter cannot bypass invalid configured formatting.
Formatter output is limited to `MAX_LABEL_BYTES` (1024 UTF-8 bytes, not Unicode
character count). Raw-frame sources are bounded by `MAX_MARKS`.

An explicit border function may handle an unresolved inherited contrast color:
`default_color` is `None` when the role and backdrop cannot resolve the default.
The function must return its own valid color. Without an explicit border
function the existing resolution error remains, including for empty-cell style.

An error returns no candidate layout or frame. Borrowed base inputs are not
modified. Raw-frame construction consumes its source vector; on error the
candidate is dropped, not returned to the caller. A previously held frame and
its Widget data remain unchanged. Writes to logs, counters, shared application
objects or other callback-owned side effects cannot be rolled back.

These constructors do not themselves publish a browser transaction or create
an automatic Waffle owner. When using a Chart host, prepare a candidate layout
and its matching Widget packet before requesting host adoption; apply that
host's validation and lifecycle rules. No React component, React Spring time
integration, native Waffle-specific mouse handlers or GPU parity is implied.

## Verification scope

The native suite covers legacy structure/SVG preservation, callback ordering,
hidden/zero/overflow groups, style consumers, owned raw identity/lifetime,
Widget sharing and failure cases. The independently extracted upstream
reference checks computation and function-call inputs; it is not a browser
rendering or React lifecycle test. Browser renderer, component state, pixel
checks and their acceptance boundaries are recorded separately in
[verification](verification.md#development-waffle-functions-and-owned-data).
