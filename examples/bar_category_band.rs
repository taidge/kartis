use std::fmt::Write;
use std::path::PathBuf;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(std::env::args().nth(1).ok_or("output directory")?);
    std::fs::create_dir_all(&out)?;
    let mut html =
        String::from("<!doctype html><meta charset=utf-8><body><h1>Configured category band</h1>");
    use kartis::*;
    for horizontal in [false, true] {
        for stacked in [false, true] {
            let i = usize::from(horizontal) * 2 + usize::from(stacked);
            let c = ChartConfig {
                width: 181.,
                height: 181.,
                margin: Margin {
                    left: 40.,
                    right: 40.,
                    top: 40.,
                    bottom: 40.,
                },
                ..Default::default()
            };
            let cats = ["A", "B", "C"].map(str::to_string);
            let data = [
                BarSeries::new("s", vec![Some(0.), None, Some(-3.)]),
                BarSeries::new("a", vec![Some(2.), Some(4.), Some(6.)]),
            ];
            let o = BarOptions {
                horizontal,
                layout: if stacked {
                    BarLayout::Stacked
                } else {
                    BarLayout::Grouped
                },
                ..Default::default()
            };
            let style = BarStyleOptions {
                category_reverse: horizontal,
                category_scale: Some(scales::BandScaleOptions {
                    padding_inner: 0.2,
                    padding_outer: 0.2,
                    round: true,
                    ..Default::default()
                }),
                ..Default::default()
            };
            let l = bar_layout_with_style(&c, &cats, &data, &o, &style)?;
            let svg = render::to_svg_with_namespace(&l.scene, &format!("case{i}"))?;
            let js = render::to_canvas_script(&l.scene, &format!("cv{i}"))?;
            write!(
                html,
                "<article data-index=\"{i}\"><h2>horizontal={horizontal}, stacked={stacked}</h2><div>{svg}</div><canvas id=\"cv{i}\"></canvas></article><script>{js}</script>"
            )?;
        }
    }
    html.push_str("</body>");
    std::fs::write(out.join("index.html"), html)?;
    println!("4 actual SVG/Canvas consumers");
    Ok(())
}
