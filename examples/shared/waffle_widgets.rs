//! Typed real-cell Widgets: allocation, empty slots and occupied area unions.
use std::collections::BTreeSet;

use kartis::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Data {
    pub cell: WaffleCellWidgetData,
    pub note: u32,
}

pub fn source(
    phase: u8,
    width: f64,
    revision: u64,
    note: u32,
    cover_first: bool,
) -> Result<(WaffleLayout, PlotWidgetsComposition<Data>)> {
    let config = ChartConfig {
        width,
        height: 240.,
        margin: Margin {
            left: 20.,
            right: 20.,
            top: 20.,
            bottom: 20.,
        },
        title: "Waffle live cell Widgets".into(),
        ..Default::default()
    };
    let mut data = vec![
        RadialDatum {
            id: "one".into(),
            label: "Original one".into(),
            value: if matches!(phase, 1 | 3) { 33.5 } else { 17.25 },
        },
        RadialDatum::new("two", 25.5),
    ];
    if phase == 7 {
        data.reverse();
    }
    let options = WaffleLayoutOptions {
        rows: 2,
        columns: 3,
        total: Some(100.),
        padding: 8.,
        direction: if phase == 3 {
            WaffleFillDirection::Right
        } else {
            WaffleFillDirection::Bottom
        },
        mode: if phase == 4 {
            WaffleMode::Areas
        } else {
            WaffleMode::Cells
        },
        hidden: if phase == 2 {
            BTreeSet::from(["one".into()])
        } else {
            BTreeSet::new()
        },
        empty_opacity: if phase == 5 { 0. } else { 1. },
        colors: OrdinalColorOptions {
            palette: OrdinalColorPalette::Custom(vec![
                Color(0, 220, 180, 255),
                Color(0, 150, 220, 255),
            ]),
            domain: Some(vec![CategoryKey::from("one"), CategoryKey::from("two")]),
            ..Default::default()
        },
        value_format: ValueFormat::Fixed(1),
        ..Default::default()
    };
    let mut layout = if phase == 6 {
        waffle_layout_with_cell_factory(&config, &data, &options, &mut |c| {
            Ok(c.inherit(Geometry::Circle {
                x: c.position[0],
                y: c.position[1],
                radius: 22.,
            }))
        })?
    } else {
        waffle_layout(&config, &data, &options)?
    };
    layout.interaction.picking_profile = PickingProfile::PaintedGeometry;
    // A real decorative source mark, with no manufactured data/site/label.
    let main = layout
        .scene
        .marks
        .iter()
        .position(|m| m.id == "waffle-cell:0:0");
    let cover_index = layout.scene.marks.len();
    let mut cover = Mark::new(
        "waffle-native-cover",
        Geometry::Rect {
            x: 60.,
            y: 35.,
            width: 40.,
            height: 40.,
        },
        Color(0, 0, 255, 255),
    );
    cover.label.clear();
    layout.scene.marks.push(cover);
    layout.scene.labels.push(Label {
        x: 20.,
        y: 235.,
        text: "Original Waffle caption".into(),
        color: Color(0, 0, 0, 255),
        size: 12.,
        anchor: TextAnchor::Start,
        rotation: 0.,
    });
    if let Some(main) = main {
        layout.scene.draw_order = if cover_first {
            vec![DrawItem::Mark(cover_index), DrawItem::Mark(main)]
        } else {
            vec![DrawItem::Mark(main), DrawItem::Mark(cover_index)]
        };
        layout
            .scene
            .draw_order
            .extend((0..cover_index).filter(|i| *i != main).map(DrawItem::Mark));
        layout.scene.draw_order.push(DrawItem::Label(0));
    }
    let cells = waffle_cell_widgets(&layout, revision, |c| {
        Ok(Some(PlotWidgetPlacement::Rect(LayoutRect {
            x: c.cell.bounds.x + 6.,
            y: c.cell.bounds.y + 6.,
            width: c.cell.bounds.width - 12.,
            height: c.cell.bounds.height - 12.,
        })))
    })?;
    let replace_ink = cells
        .layout
        .slots
        .iter()
        .map(|s| s.mark_id.clone())
        .collect();
    let widgets = PlotWidgetsData {
        layout: cells.layout,
        data: cells
            .data
            .into_iter()
            .map(|(id, cell)| (id, Data { cell, note }))
            .collect(),
    };
    Ok((
        layout,
        PlotWidgetsComposition {
            widgets,
            replace_ink,
        },
    ))
}
