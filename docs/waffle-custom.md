# Custom Waffle cells

[简体中文](waffle-custom.zh-CN.md)

This is an **Unreleased development API**, not part of crates.io Kartis 0.1.0.
It extends the existing Waffle layout with a borrowed single-mark geometry and
paint factory. It does not implement arbitrary React components or live Widgets.

`waffle_layout_with_cell_factory(config, data, options, &mut factory)` returns a
`WaffleLayout`. The callback borrows a `WaffleCellContext` containing:

- `cell`: the original slot, row/column, owner and mark identity;
- `group`: `Some(&WaffleGroup)` for an occupied slot, `None` for an empty slot;
- `geometry`, `position`, `plot`, `grid`, `padding` and `corner_radius`;
- inherited `fill`, `fill_paint`, `stroke`, `stroke_dash` and `opacity`.

Return `ScatterNodeGeometry`, or start from `ctx.inherit(geometry)` to keep the
original paint. Sources and the callback are borrowed only during construction;
the returned layout does not retain them.

```rust
# use kartis::*;
let data = [RadialDatum::new("Orders", 40.), RadialDatum::new("Other", 20.)];
let layout = waffle_layout_with_cell_factory(
    &ChartConfig::default(),
    &data,
    &WaffleLayoutOptions { rows: 5, columns: 10, total: Some(100.), ..Default::default() },
    &mut |ctx| {
        let [x, y] = ctx.position;
        Ok(ctx.inherit(Geometry::Circle {
            x, y, radius: ctx.cell.bounds.width * 0.25,
        }))
    },
)?;
# Ok::<(), ChartError>(())
```

The factory runs in the layout's cell traversal order for slots with an actual
cell mark. This includes empty cells. In `WaffleMode::Areas`, occupied groups
are solid area paths rather than individual cell marks, so only empty-cell
marks call this factory. Occupied area geometry and paint stay unchanged.
An empty cell never acquires an occupied typed interaction site merely because
its factory returns visible geometry.

## Geometry, identity and interaction

`ctx.inherit(ctx.geometry.clone())` preserves the complete original layout,
including rounded geometry, paints and interaction bindings. Circles and
rectangles must be centered at `position`; use a Path for offset ink. Custom
bodies support the shared solid/gradient/pattern fills, opacity, stroke and
dash fields of `ScatterNodeGeometry`.

Slot bounds, row/column, ownership, group source values, requested/occupied
ranges, polygons, group colors and legend colors keep their original meaning.
They are not recomputed from replacement ink. `cell_at(x, y)` still identifies
the original full grid slot, including padding; it is not an exact-ink query.

Exact geometry picking follows the replacement mark and its clip. Nearest
picking retains the original slot center. Occupied interaction sites resolve
the actual custom paint's representative color, while group and legend source
colors remain unchanged. A custom Path can therefore paint away from its
logical data site without moving that data site or reallocating the grid.

## Validation and scope

All callbacks and candidate validation complete before a layout is returned.
Invalid geometry, paint, opacity, dash or anchor data and exceeded cumulative
Scene budgets reject the candidate. The factory does not mutate an already
accepted layout. Callback-owned external side effects cannot be undone if a
later callback fails; keep application publication outside the callback.

The result uses the shared Scene SVG, Canvas, CPU and Glory rendering paths.
Its typed binding can be used through the existing Scene interaction APIs.
Concrete backend acceptance is recorded separately; returning a valid layout
does not establish physical-device input or arbitrary Widget lifecycle support.

The Nivo reference exposes `cellComponent` through
`packages/waffle/src/types.ts` and `WaffleCells.tsx`, including empty cells,
padding and border props. This Rust entry point provides a bounded geometry
factory instead of that React component protocol. Nivo's areas layer is a
transparent interaction layer over cells (`WaffleArea.tsx` uses zero fill
opacity); Kartis `WaffleMode::Areas` draws
solid group areas. These are different contracts, so an empty-cell factory in
Areas mode is not a complete port of Nivo areas or custom layers.

For occupied-group events across cell padding, use the explicit [Waffle region binding](waffle-regions.md); it preserves the factory's actual paint and reports physical ink separately.
