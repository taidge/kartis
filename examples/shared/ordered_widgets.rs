//! Authored paint-order fixture shared by actual SVG and Canvas live hosts.
use std::collections::{BTreeMap, BTreeSet};

use glory::render::{GraphicPoint, GraphicStroke, PathCommand, StrokeCap};
use kartis::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Data {
    pub note: u32,
    pub source: String,
}

pub fn source(
    order: u8,
    width: f64,
    revision: u64,
    note: u32,
    variant: u8,
) -> Result<(Scene, PlotWidgetsComposition<Data>)> {
    let mut scene = ChartConfig {
        width,
        height: 260.,
        margin: Margin {
            top: 0.,
            right: 0.,
            bottom: 0.,
            left: 0.,
        },
        title: "Actual Scene-ordered Widgets".into(),
        ..Default::default()
    }
    .scene()?;
    scene.background = Color(255, 255, 255, 255);
    let rect = |x, y, width, height| Geometry::Rect {
        x,
        y,
        width,
        height,
    };
    let mut main = Mark::new("stage", rect(60., 60., 160., 80.), Color(0, 220, 0, 255));
    main.series = "ordered".into();
    main.label = "Stage source".into();
    main.value = 42. + f64::from(note);
    let mut upper = Mark::new(
        "decorative-blue",
        rect(120., 70., 80., 60.),
        Color(0, 0, 255, 255),
    );
    upper.label = "Decorative blue paint".into();
    match variant {
        1 => upper.opacity = 0.5,
        2 => upper.opacity = 0.,
        3 => {
            let mut commands = rect(120., 70., 80., 60.).commands();
            commands.extend(rect(140., 80., 40., 30.).commands());
            upper.clip_path = Some(Box::new(ClipPath {
                geometry: Geometry::Path(commands),
                rule: ClipRule::EvenOdd,
            }));
        }
        4 => {
            upper.geometry = Geometry::Path(vec![
                PathCommand::Move(GraphicPoint::new(120., 100.)),
                PathCommand::Line(GraphicPoint::new(200., 100.)),
            ]);
            upper.fill = None;
            upper.stroke = Some(GraphicStroke {
                color: Color(0, 0, 255, 255),
                width: 12.,
                cap: StrokeCap::Butt,
            });
            upper.stroke_dash = Some(StrokeDash {
                segments: vec![16., 16.],
                offset: 0.,
            });
        }
        5 => {
            upper.clip = Some(ClipRect {
                x: 120.,
                y: 70.,
                width: 20.,
                height: 30.,
            })
        }
        6 => upper.opacity = 0.,
        _ => {}
    }
    let mut secondary = Mark::new(
        "secondary",
        rect(60., 170., 100., 40.),
        Color(0, 220, 0, 255),
    );
    secondary.series = "secondary".into();
    secondary.label = "Second source".into();
    secondary.value = 7.;
    let mut raw = Mark::new(
        "clipped-original",
        rect(270., 60., 60., 70.),
        Color(255, 0, 255, 255),
    );
    raw.clip = Some(ClipRect {
        x: 280.,
        y: 70.,
        width: 30.,
        height: 40.,
    });
    scene.marks = vec![main, upper, secondary, raw];
    scene.labels.push(Label {
        x: 60.,
        y: 35.,
        text: "Authored label: 日期".into(),
        color: Color(0, 0, 0, 255),
        size: 16.,
        anchor: TextAnchor::Start,
        rotation: 0.,
    });
    if variant == 6 {
        scene.labels.push(Label {
            x: 124.,
            y: 110.,
            text: "MMMM".into(),
            color: Color(0, 0, 255, 255),
            size: 32.,
            anchor: TextAnchor::Start,
            rotation: 0.,
        });
        // The occlusion fixture needs a genuine 3x3 opaque glyph interior on
        // every desktop font backend, rather than assuming regular M stems
        // are at least three fully covered pixels wide.
        scene.typography.labels.insert(
            1,
            FontOptions {
                weight: Some(700.),
                ..Default::default()
            },
        );
    }
    let blue_first = order % 2 == 1;
    scene.draw_order = if blue_first {
        vec![DrawItem::Mark(1), DrawItem::Mark(0)]
    } else {
        vec![DrawItem::Mark(0), DrawItem::Mark(1)]
    };
    scene
        .draw_order
        .extend([DrawItem::Mark(2), DrawItem::Mark(3)]);
    scene
        .draw_order
        .extend((0..scene.labels.len()).map(DrawItem::Label));
    if order >= 2 {
        let members = if blue_first {
            vec!["decorative-blue".into(), "stage".into()]
        } else {
            vec!["stage".into(), "decorative-blue".into()]
        };
        scene.opacity_groups.push(MarkOpacityGroup {
            id: format!("source-group-{order}"),
            members,
            opacity: if order == 4 { 0. } else { 0.5 },
        });
    }
    let widgets = PlotWidgetsData {
        // Reverse slot order deliberately: the Scene owns visual ordering.
        layout: PlotWidgetsLayout {
            plot: None,
            slots: vec![
                PlotWidgetSlot {
                    mark_id: "secondary".into(),
                    revision,
                    placement: PlotWidgetPlacement::Rect(LayoutRect {
                        x: 60.,
                        y: 170.,
                        width: 100.,
                        height: 40.,
                    }),
                },
                PlotWidgetSlot {
                    mark_id: "stage".into(),
                    revision,
                    placement: PlotWidgetPlacement::Rect(LayoutRect {
                        x: 60.,
                        y: 60.,
                        width: 160.,
                        height: 80.,
                    }),
                },
            ],
        },
        data: ["stage", "secondary"]
            .map(|id| {
                (
                    id.into(),
                    Data {
                        note,
                        source: id.into(),
                    },
                )
            })
            .into_iter()
            .collect::<BTreeMap<_, _>>(),
    };
    scene.validate()?;
    Ok((
        scene,
        PlotWidgetsComposition {
            widgets,
            replace_ink: BTreeSet::from(["stage".into(), "secondary".into()]),
        },
    ))
}
