use glory::render::{GraphicStroke, StrokeCap};
use kartis::*;

pub const NAME: &str = "Area · isolated multiply blending";

pub fn scene(c: &ChartConfig) -> Result<(Scene, MarkBlends)> {
    let observed = Series::new(
        "Observed",
        [16., 36., 42., 32., 45., 53.]
            .into_iter()
            .enumerate()
            .map(|(i, value)| Point::new(i as f64, value))
            .collect(),
    );
    let expected = Series::new(
        "Expected",
        [26., 23., 37., 48., 41., 35.]
            .into_iter()
            .enumerate()
            .map(|(i, value)| Point::new(i as f64, value))
            .collect(),
    );
    let frame = ChartConfig {
        theme: Theme {
            colors: vec![Color(39, 113, 181, 255), Color(220, 103, 73, 255)],
            background: Color(250, 247, 237, 255),
            ..c.theme.clone()
        },
        ..c.clone()
    };
    let mut scene = line(
        &frame,
        &[observed, expected],
        &LineOptions {
            area: true,
            curve: Curve::MonotoneX,
            y_domain: Some([0., 60.]),
            ..Default::default()
        },
    )?;
    for mark in scene
        .marks
        .iter_mut()
        .filter(|mark| mark.id.starts_with("area-"))
    {
        if let Some(Color(r, g, b, _)) = mark.fill {
            mark.fill = Some(Color(r, g, b, 155));
            mark.stroke = Some(GraphicStroke {
                color: Color(r, g, b, 200),
                width: 2.,
                cap: StrokeCap::Round,
            });
            mark.opacity = 0.82;
        }
    }
    let blends = MarkBlends::new(
        &scene,
        scene
            .marks
            .iter()
            .filter(|mark| mark.id.starts_with("area-"))
            .map(|mark| (mark.id.clone(), BlendMode::Multiply)),
    )?;
    Ok((scene, blends))
}
