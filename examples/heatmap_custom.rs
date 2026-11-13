//! Export custom HeatMap cells with --features raster,geojson and an output directory.
use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use kartis::{
    render::{self, raster::*},
    *,
};
use std::{fmt::Write, path::PathBuf};

fn diamond(x: f64, y: f64) -> Geometry {
    Geometry::Path(vec![
        C::Move(P::new(x, y - 20.)),
        C::Line(P::new(x + 20., y)),
        C::Line(P::new(x, y + 20.)),
        C::Line(P::new(x - 20., y)),
        C::Close,
    ])
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let out = PathBuf::from(args.next().ok_or("provide an output directory")?);
    if args.next().is_some() {
        return Err("expected only an output directory".into());
    }
    let config = ChartConfig {
        width: 320.,
        height: 180.,
        margin: Margin {
            left: 20.,
            right: 20.,
            top: 20.,
            bottom: 20.,
        },
        ..Default::default()
    };
    let data = [
        HeatCell {
            row: "r".into(),
            column: "null".into(),
            value: None,
        },
        HeatCell::new("r", "zero", 0.),
        HeatCell::new("r", "value", 7.),
    ];
    let options = HeatmapLayoutOptions {
        axes: HeatmapAxes::none(),
        labels: false,
        hover_target: HeatmapHoverTarget::Cell,
        ..Default::default()
    };
    let base = heatmap_layout_with_cell_factory(&config, &data, &options, |cell| {
        let [x, y] = cell.position;
        let mut ink = cell.inherit(diamond(x, y));
        match cell.datum.value {
            None => {
                ink.fill_paint = Some(FillPaint::Pattern(PatternFill {
                    shape: PatternShape::Lines { width: 4. },
                    spacing: 8.,
                    foreground: Color(200, 30, 40, 255),
                    background: Color(20, 80, 150, 255),
                    ..Default::default()
                }));
            }
            Some(0.) => {
                ink.geometry = Geometry::Circle { x, y, radius: 20. };
                ink.fill = None;
                ink.stroke = Some(GraphicStroke {
                    color: Color(10, 20, 30, 255),
                    width: 2.,
                    cap: StrokeCap::Butt,
                });
            }
            Some(_) => {
                ink.fill_paint = Some(FillPaint::Linear {
                    start: [x - 20., y],
                    end: [x + 20., y],
                    stops: vec![
                        GradientStop::new(0., Color(0, 200, 0, 255)),
                        GradientStop::new(1., Color(0, 0, 200, 255)),
                    ],
                });
            }
        }
        Ok(ink)
    })?;
    let active = base.with_active(Some(HeatmapCellKey::new("r", "null")))?;
    // Binding values retain original nullability, despite the generic Scene's
    // finite scalar compatibility value for an explicit null mark.
    for layout in [&base, &active] {
        let binding = layout.interaction_binding()?;
        let nullable = binding
            .sites
            .iter()
            .map(|site| (site.mark_id.as_str(), site.value))
            .collect::<std::collections::BTreeMap<_, _>>();
        let null_id = HeatmapCellKey::new("r", "null").mark_id();
        let zero_id = HeatmapCellKey::new("r", "zero").mark_id();
        if nullable[null_id.as_str()].is_some() || nullable[zero_id.as_str()] != Some(0.) {
            return Err("typed binding lost original null/zero distinction".into());
        }
    }
    std::fs::create_dir_all(&out)?;
    let mut fonts = FontRegistry::new();
    fonts.register(
        include_bytes!("../tests/fonts/ABeeZee-Regular.ttf").as_slice(),
        0,
    )?;
    let mut html = String::from(
        "<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><title>Custom HeatMap cells</title><style>body{margin:16px;font:16px sans-serif}section{margin-bottom:20px}svg,canvas{border:0;vertical-align:top}</style><h1>Custom HeatMap cells</h1><p>Empty: patterned diamond. Value: gradient diamond. Zero: hollow circle.</p>",
    );
    let mut frames = vec![];
    let mut cpu = vec![];
    for (name, layout) in [("base", &base), ("active", &active)] {
        let svg = render::to_svg(&layout.scene)?;
        let inline = render::to_svg_with_namespace(&layout.scene, &format!("heatmap-{name}"))?;
        let script = render::to_canvas_script(&layout.scene, &format!("canvas-{name}"))?;
        std::fs::write(out.join(format!("{name}.svg")), svg)?;
        std::fs::write(out.join(format!("{name}.js")), &script)?;
        write!(
            html,
            "<section id=\"frame-{name}\" data-frame=\"{name}\">{inline}<canvas id=\"canvas-{name}\"></canvas></section><script>{script}</script>"
        )?;
        for dpr in [1., 2.] {
            let image = render_raster(
                &layout.scene,
                &fonts,
                &RasterOptions {
                    pixel_ratio: dpr,
                    ..Default::default()
                },
            )?;
            let png = image.png()?;
            let mut reader = png::Decoder::new(std::io::Cursor::new(&png)).read_info()?;
            let mut decoded = vec![0; reader.output_buffer_size().ok_or("PNG output size")?];
            let info = reader.next_frame(&mut decoded)?;
            if decoded[..info.buffer_size()] != image.rgba {
                return Err("decoded PNG differs from CPU raster".into());
            }
            std::fs::write(out.join(format!("{name}-cpu-dpr{}.png", dpr as u8)), png)?;
            cpu.push(serde_json::json!({"frame":name,"dpr":dpr,"width":image.width,"height":image.height,"actualDecodedPNG":true}));
        }
        frames.push(serde_json::json!({
            "frame":name,"width":layout.scene.width,"height":layout.scene.height,
            "plot":[layout.plot.x,layout.plot.y,layout.plot.width,layout.plot.height],
            "cells":layout.cells.iter().map(|cell|serde_json::json!({
                "row":cell.key.row,"column":cell.key.column,"raw":cell.value,
                "position":cell.center,"opacity":cell.opacity,"tooltipValue":cell.tooltip_datum().value,
            })).collect::<Vec<_>>()
        }));
    }
    let metadata = serde_json::to_string(&frames)?;
    write!(
        html,
        "<script id=\"heatmap-custom-data\" type=\"application/json\">{metadata}</script></html>"
    )?;
    std::fs::write(out.join("index.html"), html)?;
    std::fs::write(
        out.join("receipt.json"),
        serde_json::to_vec_pretty(&serde_json::json!({
            "passed":true,"scope":"Two actual typed HeatMap factory Scene frames, SVG/CanvasScript and four decoded CPU PNGs.",
            "frames":frames,"cpu":cpu,
            "limits":["One bounded Scene mark per source cell, not an arbitrary Widget component slot.","Chart labels are disabled to isolate geometry and paint; default font parity is separate."],
        }))?,
    )?;
    println!("exported two custom HeatMap frames and four CPU PNGs");
    Ok(())
}
