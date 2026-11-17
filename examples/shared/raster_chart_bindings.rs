//! Typed CPU-image fixture with actual aliases and nullable original values.
use std::collections::BTreeMap;

use glory::render::{GraphicPoint, GraphicStroke, PathCommand, StrokeCap};
use kartis::*;

pub const OWNER: &str = "owner";
pub const ALIAS: &str = "long-alias";
pub const TAIL: &str = "tail-alias";
pub const INK: Color = Color(230, 20, 80, 255);
pub const ALIAS_INK: Color = Color(20, 180, 100, 255);
pub const TAIL_INK: Color = Color(180, 90, 220, 255);

#[derive(Clone, Debug, PartialEq)]
pub struct Data {
    pub value: Option<f64>,
    pub note: u32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Source {
    pub frame: SceneInteraction,
    pub widgets: PlotWidgetsData<Data>,
}

pub fn source(value: Option<f64>, shift: f64, note: u32, revision: u64) -> Result<Source> {
    let mut scene = ChartConfig {
        width: 240.,
        height: 160.,
        margin: Margin {
            left: 0.,
            right: 0.,
            top: 0.,
            bottom: 0.,
        },
        title: "Typed CPU image and physical aliases".into(),
        ..Default::default()
    }
    .scene()?;
    let mut owner = Mark::new(
        OWNER,
        Geometry::Circle {
            x: 40. + shift,
            y: 40.,
            radius: 9.,
        },
        INK,
    );
    owner.series = "route".into();
    owner.label = "Nullable measurement".into();
    // Nullable binding is authoritative; the live scalar is deliberately not 0.
    owner.value = value.unwrap_or(-913.);
    let mut commands = vec![PathCommand::Move(GraphicPoint::new(90. + shift, 80.))];
    commands.extend((1..=300).map(|i| {
        PathCommand::Line(GraphicPoint::new(
            90. + shift + 40. * f64::from(i) / 300.,
            80.,
        ))
    }));
    let mut alias = Mark::new(ALIAS, Geometry::Path(commands), ALIAS_INK);
    alias.series = "route".into();
    alias.label.clear();
    alias.value = -713.;
    alias.fill = None;
    alias.stroke = Some(GraphicStroke {
        color: ALIAS_INK,
        width: 6.,
        cap: StrokeCap::Butt,
    });
    let mut tail = Mark::new(
        TAIL,
        Geometry::Circle {
            x: 160. + shift,
            y: 105.,
            radius: 8.,
        },
        TAIL_INK,
    );
    tail.series = "route".into();
    tail.label.clear();
    tail.value = -813.;
    let mut other = Mark::new(
        "other",
        Geometry::Circle {
            x: 200.,
            y: 115.,
            radius: 8.,
        },
        Color(30, 70, 210, 255),
    );
    other.series = "other".into();
    other.label = "Other measurement".into();
    other.value = 7.;
    scene.marks = vec![owner, alias, tail, other];
    scene.legend = vec![
        LegendItem {
            id: "route".into(),
            label: "Route".into(),
            color: INK,
        },
        LegendItem {
            id: "other".into(),
            label: "Other".into(),
            color: Color(30, 70, 210, 255),
        },
    ];
    let binding = InteractionBinding {
        sites: vec![
            InteractionSite {
                mark_id: OWNER.into(),
                series: "route".into(),
                label: "Nullable measurement".into(),
                value,
                formatted_value: Some(value.map_or_else(|| "missing".into(), |v| format!("{v}"))),
                color: INK,
                position: [40. + shift, 40.],
            },
            InteractionSite {
                mark_id: "other".into(),
                series: "other".into(),
                label: "Other measurement".into(),
                value: Some(7.),
                formatted_value: Some("seven".into()),
                color: Color(30, 70, 210, 255),
                position: [200., 115.],
            },
        ],
        ink_owners: [(ALIAS.into(), OWNER.into()), (TAIL.into(), OWNER.into())].into(),
        part_data: [(
            TAIL.into(),
            InteractionPartDatum {
                source_id: format!("raw-part-{note}"),
                value: 99. + f64::from(note),
            },
        )]
        .into(),
        picking_profile: PickingProfile::PaintedGeometry,
        ..Default::default()
    };
    let frame = SceneInteraction {
        scene,
        interaction: Some(binding),
        plot: Some(ClipRect {
            x: 0.,
            y: 0.,
            width: 240.,
            height: 160.,
        }),
    };
    frame.validate()?;
    Ok(Source {
        frame,
        widgets: PlotWidgetsData {
            layout: PlotWidgetsLayout {
                plot: Some(ClipRect {
                    x: 0.,
                    y: 0.,
                    width: 240.,
                    height: 160.,
                }),
                slots: vec![PlotWidgetSlot {
                    mark_id: OWNER.into(),
                    revision,
                    placement: PlotWidgetPlacement::AtDatum {
                        offset: [-16., -14.],
                        measured: TextSize {
                            width: 32.,
                            height: 24.,
                        },
                    },
                }],
            },
            data: BTreeMap::from([(OWNER.into(), Data { value, note })]),
        },
    })
}
