use glory::render::{GraphicPoint, PathCommand};
use kartis::*;

#[test]
fn dimension_identity_containing_grid_keeps_axis_position_style_and_paint_order() {
    let c = ChartConfig {
        width: 600.,
        height: 360.,
        margin: Margin {
            left: 60.,
            right: 60.,
            top: 30.,
            bottom: 50.,
        },
        ..Default::default()
    };
    for layout in [ParallelLayout::Horizontal, ParallelLayout::Vertical] {
        let mut dimensions = ["cost-grid-usd", "quality"].map(ParallelDimension::new);
        for d in &mut dimensions {
            d.domain = Some([0., 100.]);
            d.axis.tick_values = Some(vec![50.]);
            d.axis.grid = true;
            d.style.line_width = 3.;
            d.style.line_color = Some(Color(15, 40, 85, 255));
        }
        let scene = parallel_coordinates_with_options(
            &c,
            &dimensions,
            &[ParallelDatum::new(
                "row-grid-name",
                vec![Some(25.), Some(75.)],
            )],
            &ParallelCoordinatesOptions {
                layout,
                ..Default::default()
            },
        )
        .unwrap();
        let id = "parallel-axis:13:cost-grid-usd";
        let axis = scene.marks.iter().find(|m| m.id == id).unwrap();
        let expected = match layout {
            ParallelLayout::Horizontal => vec![
                PathCommand::Move(GraphicPoint::new(60., 310.)),
                PathCommand::Line(GraphicPoint::new(60., 30.)),
            ],
            ParallelLayout::Vertical => vec![
                PathCommand::Move(GraphicPoint::new(60., 310.)),
                PathCommand::Line(GraphicPoint::new(540., 310.)),
            ],
        };
        assert_eq!(
            axis.geometry,
            Geometry::Path(expected),
            "ID text must not decide whether an axis receives translation"
        );
        assert_eq!(axis.stroke.unwrap().width, 3.);
        assert_eq!(axis.stroke.unwrap().color, Color(15, 40, 85, 255));
        let paint = scene
            .paint_marks()
            .map(|m| m.id.as_str())
            .collect::<Vec<_>>();
        let row = paint
            .iter()
            .position(|id| *id == "parallel:13:row-grid-name")
            .unwrap();
        let axis = paint.iter().position(|candidate| *candidate == id).unwrap();
        assert!(axis > row, "foreground axes must stay above record paths");
        let grid = paint
            .iter()
            .position(|candidate| *candidate == format!("{id}-grid-0"))
            .unwrap();
        assert!(grid < row, "only actual grids belong below record paths");
    }
}
