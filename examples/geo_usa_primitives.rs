//! Diagnostics for projection/postclip primitives; full map integration is pending.
use std::fmt::Write;
use std::path::PathBuf;

use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use kartis::render::raster::{FontRegistry, RasterOptions, render_raster};
use kartis::{render, *};

fn xy([x, y]: [f64; 2]) -> [f64; 2] {
    [50. + 15. * x, 40. + 15. * y]
}
fn commands(rings: &[Vec<[f64; 2]>], close: bool) -> Vec<C> {
    let mut out = Vec::new();
    for ring in rings {
        if let Some((head, tail)) = ring.split_first() {
            let p = xy(*head);
            out.push(C::Move(P::new(p[0], p[1])));
            out.extend(tail.iter().map(|p| {
                let p = xy(*p);
                C::Line(P::new(p[0], p[1]))
            }));
            if close {
                out.push(C::Close);
            }
        }
    }
    out
}
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("provide an output directory")?,
    );
    std::fs::create_dir_all(&out)?;
    let config = ChartConfig {
        width: 320.,
        height: 240.,
        ..Default::default()
    };
    let rect = ClipRect {
        x: 0.,
        y: 0.,
        width: 10.,
        height: 10.,
    };
    let outer = vec![[-5., -5.], [15., -5.], [15., 15.], [-5., 15.]];
    let mut html = String::from(
        "<!doctype html><meta charset=\"utf-8\"><title>USA projection and rectangle clip diagnostics</title><style>section{display:flex}svg,canvas{width:320px;height:240px}</style>",
    );
    let mut frames = Vec::new();
    let color = Color(30, 90, 160, 255);
    let cases = [
        (
            "inside",
            vec![vec![[2., 2.], [8., 2.], [8., 8.], [2., 8.]]],
            vec![
                ([5., 5.], true),
                ([1., 5.], false),
                ([9., 5.], false),
                ([5., 9.], false),
            ],
        ),
        (
            "cover",
            vec![outer.clone()],
            vec![
                ([1., 1.], true),
                ([5., 5.], true),
                ([9., 9.], true),
                ([11., 5.], false),
            ],
        ),
        (
            "hole",
            vec![outer.clone(), vec![[3., 3.], [3., 7.], [7., 7.], [7., 3.]]],
            vec![
                ([2., 5.], true),
                ([5., 5.], false),
                ([8., 5.], true),
                ([5., 8.], true),
            ],
        ),
        (
            "notch",
            vec![outer, vec![[-2., 3.], [-2., 7.], [7., 7.], [7., 3.]]],
            vec![
                ([2., 5.], false),
                ([8., 5.], true),
                ([2., 2.], true),
                ([8., 8.], true),
            ],
        ),
        (
            "concave",
            vec![vec![
                [-5., 1.],
                [8., 1.],
                [8., 3.],
                [-2., 3.],
                [-2., 7.],
                [8., 7.],
                [8., 9.],
                [-5., 9.],
            ]],
            vec![
                ([5., 2.], true),
                ([5., 5.], false),
                ([5., 8.], true),
                ([9., 2.], false),
            ],
        ),
        (
            "no-area",
            vec![
                vec![[-10., -10.], [20., -10.], [20., 20.], [-10., 20.]],
                vec![[-2., -2.], [-2., 12.], [12., 12.], [12., -2.]],
            ],
            vec![
                ([1., 1.], false),
                ([5., 5.], false),
                ([9., 9.], false),
                ([2., 5.], false),
            ],
        ),
    ];
    let mut scenes = Vec::new();
    for (name, rings, probes) in cases {
        let result = clip_geo_planar_polygon(&rings, rect, &Default::default())?;
        let mut scene = config.scene()?;
        let path = commands(&result.rings, true);
        if !path.is_empty() {
            scene
                .marks
                .push(Mark::new(name, Geometry::Path(path), color));
        }
        scenes.push((
            name.to_owned(),
            scene,
            probes
                .into_iter()
                .map(|(p, hit)| (xy(p), hit))
                .collect::<Vec<_>>(),
        ));
    }
    let result = clip_geo_planar_lines(
        &[vec![[-5., 2.], [15., 2.], [15., 8.], [-5., 8.]]],
        rect,
        &Default::default(),
    )?;
    let mut scene = config.scene()?;
    let mut mark = Mark::new(
        "reenter",
        Geometry::Path(commands(&result.lines, false)),
        color,
    );
    mark.fill = None;
    mark.stroke = Some(GraphicStroke {
        color,
        width: 4.,
        cap: StrokeCap::Butt,
    });
    scene.marks.push(mark);
    scenes.push((
        "reenter".into(),
        scene,
        vec![
            (xy([5., 2.]), true),
            (xy([5., 8.]), true),
            (xy([5., 5.]), false),
            (xy([9., 5.]), false),
        ],
    ));
    let projection = GeoAlbersUsaProjection::new(300., [160., 100.])?;
    for region in GeoUsaRegion::ALL {
        let clip = projection.region_clip(region);
        let point = [clip.x + clip.width - 1., clip.y + clip.height / 2.];
        let geographic = projection
            .invert_in_region(region, point)?
            .ok_or("region inverse failed")?;
        let mapped = projection
            .project_in_region(region, geographic)?
            .ok_or("region point failed")?;
        assert!((mapped[0] - point[0]).abs() < 1e-7 && (mapped[1] - point[1]).abs() < 1e-7);
        let name = format!("region-{region:?}");
        let mut scene = config.scene()?;
        let mut mark = Mark::new(
            &name,
            Geometry::Circle {
                x: point[0],
                y: point[1],
                radius: 6.,
            },
            color,
        );
        mark.clip = Some(clip);
        scene.marks.push(mark);
        scenes.push((
            name,
            scene,
            vec![
                (point, true),
                ([point[0] - 3., point[1]], true),
                ([clip.x + clip.width + 3., point[1]], false),
            ],
        ));
    }
    for (name, scene, probes) in scenes {
        scene.validate()?;
        let image = render_raster(&scene, &FontRegistry::new(), &RasterOptions::default())?;
        let mut samples = Vec::new();
        for (point, hit) in probes {
            assert_eq!(
                scene.hit_test(point[0], point[1]).is_some(),
                hit,
                "{name} at {point:?}"
            );
            let i =
                (point[1].floor() as usize * image.width as usize + point[0].floor() as usize) * 4;
            samples.push(serde_json::json!({"point":point,"rgba":image.rgba[i..i+4],"hit":hit}));
        }
        let svg = render::to_svg_with_namespace(&scene, &name)?;
        std::fs::write(out.join(format!("{name}.svg")), &svg)?;
        std::fs::write(out.join(format!("{name}.png")), image.png()?)?;
        write!(
            html,
            "<section id=\"{name}\">{svg}<canvas id=\"canvas-{name}\"></canvas></section><script>{}</script>",
            render::to_canvas_script(&scene, &format!("canvas-{name}"))?
        )?;
        frames.push(serde_json::json!({"name":name,"samples":samples}));
    }
    write!(
        html,
        "<script id=\"frames\" type=\"application/json\">{}</script>",
        serde_json::to_string(&frames)?
    )?;
    std::fs::write(out.join("index.html"), html)?;
    std::fs::write(out.join("frames.json"), serde_json::to_vec_pretty(&frames)?)?;
    Ok(())
}
