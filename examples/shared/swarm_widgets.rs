//! Actual packed Swarm node roles with owned original input.
use std::collections::BTreeSet;

use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::*;
#[derive(Clone, Debug)]
pub struct Source {
    pub frame: SceneInteraction,
    pub composition: PlotWidgetsComposition<SwarmNodeWidgetData>,
}
/// Base; metadata/value; hidden south; custom offset ink; all zero sizes;
/// transparent c; skip b; culled values; raw-zero bits only; source reorder;
/// vertical/reversed; empty source.
pub fn source(phase: u8, width: f64, revision: u64) -> Result<Source> {
    let config = ChartConfig {
        width,
        height: 260.,
        margin: Margin {
            left: 40.,
            right: 40.,
            top: 30.,
            bottom: 30.,
        },
        title: "Swarm live node Widgets".into(),
        ..Default::default()
    };
    let make = |id: &str, group: &str, value: f64, size: f64, label: &str| {
        let mut d = SwarmPoint::new(id, group, value);
        d.size = Some(size);
        d.label = Some(label.into());
        d
    };
    let mut data = vec![
        make(
            "a",
            "north",
            if phase == 1 { 2.5 } else { 2. },
            14.,
            if phase == 1 { "Alpha updated" } else { "Alpha" },
        ),
        make(
            "b",
            "north",
            if phase == 8 { 0. } else { -0. },
            0.,
            "Zero original",
        ),
        make("c", "north", 6., 12., "Charlie"),
        make("d", "south", 3., 18., "Delta"),
        make("e", "south", 8., 10., "Echo"),
        make("h", "ghost", 9., 10., "Hidden original"),
    ];
    if phase == 4 {
        for d in &mut data {
            d.size = Some(0.);
        }
    }
    if phase == 9 {
        data.reverse();
    }
    if phase == 11 {
        data.clear();
    }
    let mut north = SwarmGroup::new("north");
    north.label = "North source".into();
    north.color = Some(Color(40, 90, 180, 255));
    let mut south = SwarmGroup::new("south");
    south.label = "South source".into();
    south.color = Some(Color(230, 150, 40, 255));
    let options = SwarmLayoutOptions {
        orientation: if phase == 10 {
            SwarmOrientation::Vertical
        } else {
            SwarmOrientation::Horizontal
        },
        groups: Some(vec![north, south, SwarmGroup::new("ghost")]),
        hidden: if phase == 2 {
            BTreeSet::from(["south".into(), "ghost".into()])
        } else {
            BTreeSet::from(["ghost".into()])
        },
        value_domain: Some(if phase == 7 { [1., 7.] } else { [-1., 10.] }),
        value_fit: if phase == 7 {
            SwarmValueFit::Clip
        } else {
            SwarmValueFit::Inset
        },
        value_scale: ContinuousScaleOptions {
            nice: ScaleNice::Off,
            reverse: phase == 10,
            ..Default::default()
        },
        value_axis: None,
        group_axis: None,
        value_format: ValueFormat::Fixed(1),
        ..Default::default()
    };
    let mut layout = if matches!(phase, 3 | 5) {
        swarm_layout_with_node_factory(&config, &data, &options, |c| {
            let [x, y] = c.position;
            let mut ink = c.inherit(Geometry::Path(vec![
                C::Move(P::new(x + 20., y - 5.)),
                C::Line(P::new(x + 32., y - 5.)),
                C::Line(P::new(x + 32., y + 5.)),
                C::Line(P::new(x + 20., y + 5.)),
                C::Close,
            ]));
            if phase == 5 && c.datum.id == "c" {
                ink.opacity = 0.;
            }
            Ok(ink)
        })?
    } else {
        swarm_layout(&config, &data, &options)?
    };
    // Stable nonbusiness decoration leaves a real source plane in every phase.
    layout.scene.marks.insert(
        0,
        Mark::new(
            "swarm-reference",
            Geometry::Rect {
                x: width - 28.,
                y: 236.,
                width: 12.,
                height: 12.,
            },
            Color(90, 100, 120, 255),
        ),
    );
    for site in &mut layout.sites {
        site.mark_index += 1;
    }
    let widgets = swarm_node_widgets(&layout, &data, revision, |c| {
        if phase == 6 && c.datum.id == "b" {
            return Ok(None);
        }
        Ok(Some(PlotWidgetPlacement::AtDatum {
            offset: [-16., -11.],
            measured: TextSize {
                width: 32.,
                height: 22.,
            },
        }))
    })?;
    let replace_ink = widgets
        .layout
        .slots
        .iter()
        .map(|s| s.mark_id.clone())
        .collect();
    Ok(Source {
        frame: swarm_widget_frame(&layout)?,
        composition: PlotWidgetsComposition {
            widgets,
            replace_ink,
        },
    })
}
