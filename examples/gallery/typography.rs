use kartis::{
    render::raster::{FaceDescriptor, FontRegistry, MissingGlyphPolicy},
    *,
};

fn fixture_family() -> FontFamily {
    FontFamily::Named("Kartis authored geometric symbols".into())
}
fn caption_font() -> FontOptions {
    FontOptions {
        family: Some(vec![FontFamily::Named("Kartis gallery captions".into())]),
        weight: Some(400.),
    }
}
fn fonts() -> Result<FontRegistry> {
    let mut fonts = FontRegistry::new();
    for (bytes, family, weight) in [
        (
            include_bytes!("../../tests/fonts/KartisFixtureTypography-Regular.ttf").as_slice(),
            fixture_family(),
            400.,
        ),
        (
            include_bytes!("../../tests/fonts/KartisFixtureTypography-Bold.ttf").as_slice(),
            fixture_family(),
            700.,
        ),
        (
            include_bytes!("../../tests/fonts/ABeeZee-Regular.ttf").as_slice(),
            FontFamily::Named("Kartis gallery captions".into()),
            400.,
        ),
    ] {
        let id = fonts.register(bytes, 0)?;
        fonts.bind_face(id, FaceDescriptor { family, weight })?;
    }
    Ok(fonts)
}
fn caption(scene: &mut Scene, text: &str, x: f64, y: f64) {
    scene
        .typography
        .labels
        .insert(scene.labels.len(), caption_font());
    scene.labels.push(Label {
        text: text.into(),
        x,
        y,
        size: 14.,
        color: Color(45, 52, 64, 255),
        anchor: TextAnchor::Start,
        rotation: 0.,
    });
}

pub fn scenes(c: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let fonts = fonts()?;
    let mut scenes = Vec::new();
    for mixed in [false, true] {
        let mut scene = c.scene()?;
        scene.typography.default = FontOptions {
            family: Some(if mixed {
                vec![
                    fixture_family(),
                    FontFamily::Named("Kartis gallery captions".into()),
                ]
            } else {
                vec![fixture_family()]
            }),
            weight: Some(400.),
        };
        scene.typography.legend = caption_font();
        let x = c.margin.left + 18.;
        let size = (c.inner_height() * 0.23).clamp(24., 76.);
        for (row, weight, color) in [
            (0, 400., Color(42, 107, 162, 255)),
            (1, 700., Color(176, 79, 41, 255)),
        ] {
            let y = c.margin.top + c.inner_height() * (0.31 + f64::from(row) * 0.40);
            let owner = format!("authored-{weight}");
            let index = scene.labels.len();
            scene.labels.push(Label {
                text: if mixed { "AB C" } else { "AB" }.into(),
                x,
                y,
                size,
                color,
                anchor: TextAnchor::Start,
                rotation: 0.,
            });
            scene.typography.labels.insert(
                index,
                FontOptions {
                    weight: Some(weight),
                    ..Default::default()
                },
            );
            scene.label_series.insert(index, owner.clone());
            let advance = fonts
                .measure_with_font(
                    &scene.labels[index].text,
                    size,
                    &scene.resolved_label_font(index),
                    MissingGlyphPolicy::Error,
                )?
                .advance;
            let mut rule = Mark::new(
                format!("advance-{weight}"),
                Geometry::Rect {
                    x,
                    y: y + 7.,
                    width: advance,
                    height: 4.,
                },
                color,
            );
            rule.series = owner.clone();
            rule.label = format!("Actual {weight} face advance");
            rule.value = advance;
            scene.marks.push(rule);
            scene.legend.push(LegendItem {
                id: owner,
                label: format!("Authored {weight}"),
                color,
            });
            caption(
                &mut scene,
                &format!("Static {weight} / {advance:.1} px advance"),
                x + c.inner_width() * 0.52,
                y - size * 0.25,
            );
        }
        caption(
            &mut scene,
            if mixed {
                "A/B: authored symbols; C: explicit ABeeZee fallback (regular face)."
            } else {
                "Original geometric A/B fixtures; two actual outlines and OS/2 weights."
            },
            x,
            c.margin.top + c.inner_height() * 0.94,
        );
        scenes.push((
            if mixed {
                "Typography · explicit family fallback"
            } else {
                "Typography · authored static 400/700 faces"
            },
            scene_with_painted_labels(
                &scene,
                &fonts,
                &Default::default(),
                if mixed {
                    "gallery-typography-fallback"
                } else {
                    "gallery-typography-weights"
                },
            )?,
        ));
    }
    Ok(scenes)
}
