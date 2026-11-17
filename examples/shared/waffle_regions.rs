//! Full occupied-slot interaction profiles, with unmodified source ink.
use std::collections::BTreeSet;

use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::*;

/// 0 Cells; 1 transparent cells; 2 hidden alpha; 3 Areas; 4 offset Paths;
/// 5 Left; 6 Right; 7 Bottom. Other profiles keep the default Top traversal.
pub fn source(phase: u8, width: f64) -> Result<WaffleLayout> {
    let config = ChartConfig {
        width,
        height: 240.,
        margin: Margin {
            left: 20.,
            right: 20.,
            top: 20.,
            bottom: 20.,
        },
        title: "Waffle occupied group regions".into(),
        ..Default::default()
    };
    let options = WaffleLayoutOptions {
        rows: 3,
        columns: 5,
        total: Some(15.),
        padding: 8.,
        border_width: 0.,
        mode: if phase == 3 {
            WaffleMode::Areas
        } else {
            WaffleMode::Cells
        },
        direction: match phase {
            5 => WaffleFillDirection::Left,
            6 => WaffleFillDirection::Right,
            7 => WaffleFillDirection::Bottom,
            _ => WaffleFillDirection::Top,
        },
        hidden: if phase == 2 {
            BTreeSet::from(["alpha".into()])
        } else {
            BTreeSet::new()
        },
        value_format: ValueFormat::Fixed(1),
        ..Default::default()
    };
    let data = [
        RadialDatum {
            id: "alpha".into(),
            label: "Original alpha".into(),
            value: 5.,
        },
        RadialDatum {
            id: "beta".into(),
            label: "Original beta".into(),
            value: 4.,
        },
        RadialDatum::new("zero", 0.),
    ];
    if matches!(phase, 1 | 4) {
        waffle_layout_with_cell_factory(&config, &data, &options, &mut |context| {
            let [x, y] = context.position;
            let mut geometry = context.inherit(if phase == 4 {
                Geometry::Path(vec![
                    C::Move(P::new(x + 10., y - 6.)),
                    C::Line(P::new(x + 22., y - 6.)),
                    C::Line(P::new(x + 22., y + 6.)),
                    C::Line(P::new(x + 10., y + 6.)),
                    C::Close,
                ])
            } else {
                context.geometry.clone()
            });
            if phase == 1 {
                geometry.opacity = 0.;
            }
            Ok(geometry)
        })
    } else {
        waffle_layout(&config, &data, &options)
    }
}
