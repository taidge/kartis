//! Group factories determine shared ink, legends and typed source metadata.
use std::path::PathBuf;

use kartis::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let out = PathBuf::from(
        std::env::args()
            .nth(1)
            .ok_or("provide an output directory")?,
    );
    std::fs::create_dir_all(&out)?;
    let data = [
        RadialDatum::new("ready", 45.),
        RadialDatum::new("waiting", 20.),
        RadialDatum::new("hidden", 15.),
        RadialDatum::new("zero", -0.),
    ];
    let config = ChartConfig {
        width: 360.,
        height: 260.,
        ..Default::default()
    };
    let mut summary = String::from("mode\tid\traw_bits\tformatted\trequested\toccupied\thidden\n");
    for mode in [WaffleMode::Cells, WaffleMode::Areas] {
        let mut color = |c: &WaffleGroupColorContext<'_>| {
            Ok(match c.datum.id.as_str() {
                "ready" => Color(25, 140, 80, 255),
                "waiting" => Color(230, 120, 30, 255),
                _ => c.default_color,
            })
        };
        let mut format = |value: f64| Ok(format!("{value:.0} / 100"));
        let mut border = |c: &WaffleBorderColorContext<'_>| {
            Ok(match c.target {
                WaffleBorderTarget::Group(g) if g.hidden => Color(120, 120, 120, 255),
                _ => c.default_color.unwrap_or(Color(30, 30, 30, 255)),
            })
        };
        let layout = waffle_layout_with_factories(
            &config,
            &data,
            &WaffleLayoutOptions {
                rows: 4,
                columns: 5,
                total: Some(100.),
                mode,
                border_width: 2.,
                hidden: ["hidden".into()].into(),
                ..Default::default()
            },
            &mut WaffleFactories {
                color: Some(&mut color),
                formatted_value: Some(&mut format),
                border_color: Some(&mut border),
            },
        )?;
        std::fs::write(
            out.join(format!("{mode:?}.svg")),
            render::to_svg(&layout.scene)?,
        )?;
        for group in &layout.groups {
            use std::fmt::Write;
            writeln!(
                summary,
                "{mode:?}\t{}\t{:016x}\t{}\t{:?}\t{:?}\t{}",
                group.source.id,
                group.source.value.to_bits(),
                group.formatted_value,
                group.requested,
                group.occupied,
                group.hidden
            )?;
        }
    }
    std::fs::write(out.join("groups.tsv"), summary)?;
    Ok(())
}
