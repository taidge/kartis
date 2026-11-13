use kartis::render::raster::{ColorGlyphMode, FontRegistry};
use kartis::*;

pub fn scenes(c: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let mut scenes = Vec::new();
    for (name, palette, foreground_alpha, namespace) in [
        (
            "Color fonts · COLR palette layers",
            0,
            128,
            "gallery-color-palette",
        ),
        (
            "Color fonts · transparent foreground",
            1,
            0,
            "gallery-color-transparent",
        ),
    ] {
        let mut scene = c.scene()?;
        let size = (c.inner_width().min(c.inner_height()) * 0.64).clamp(24., 180.);
        let center = [
            c.margin.left + c.inner_width() / 2.,
            c.margin.top + c.inner_height() / 2.,
        ];
        let owner = format!("Palette {palette}");
        let mut datum = Mark::new(
            "color-font-layers",
            Geometry::Rect {
                x: center[0] - size * 0.7,
                y: center[1] - size * 0.7,
                width: size * 1.4,
                height: size * 1.4,
            },
            Color(235, 240, 247, 255),
        );
        datum.series.clone_from(&owner);
        datum.label = format!("{owner} color layers");
        datum.value = 4.;
        scene.marks.push(datum);
        scene.legend.push(LegendItem {
            id: owner.clone(),
            label: owner.clone(),
            color: if palette == 0 {
                Color(255, 0, 0, 255)
            } else {
                Color(0, 180, 0, 255)
            },
        });
        scene.label_series.insert(0, owner);
        scene.labels.push(Label {
            text: "B".into(),
            x: center[0],
            y: center[1] + size / 2.,
            size,
            color: Color(0, 160, 120, foreground_alpha),
            anchor: TextAnchor::Middle,
            rotation: 0.,
        });
        let mut fonts = FontRegistry::new().with_color_glyphs(ColorGlyphMode::ColrV0 { palette });
        fonts.register(
            include_bytes!("../../tests/fonts/KartisFixtureColrV0.ttf").as_slice(),
            0,
        )?;
        scenes.push((
            name,
            scene_with_painted_labels(&scene, &fonts, &Default::default(), namespace)?,
        ));
    }
    Ok(scenes)
}
