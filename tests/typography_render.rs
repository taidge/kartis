use glory::render::GraphicStroke;
use glory::style::Family;
use kartis::render::{self, Canvas};
use kartis::*;

fn scene() -> Scene {
    let mut scene = ChartConfig::default().scene().unwrap();
    scene.labels = ["First", "Second"]
        .into_iter()
        .enumerate()
        .map(|(index, text)| Label {
            x: 40.,
            y: 40. + index as f64 * 30.,
            text: text.into(),
            color: Color(0, 0, 0, 255),
            size: 18.,
            rotation: 0.,
            anchor: TextAnchor::Start,
        })
        .collect();
    scene
}

#[test]
fn export_uses_global_family_and_weight_and_independent_label_overrides() {
    let mut scene = scene();
    scene.typography.default = FontOptions {
        family: Some(vec![Family::Named("Chart Body".into()), Family::Sans]),
        weight: Some(700.),
    };
    scene.typography.labels.insert(
        1,
        FontOptions {
            family: Some(vec![Family::Mono]),
            weight: None,
        },
    );
    scene.draw_order = vec![DrawItem::Label(1), DrawItem::Label(0)];
    let svg = render::to_svg(&scene).unwrap();
    let texts = svg.split("<text ").skip(1).collect::<Vec<_>>();
    assert_eq!(texts.len(), 2);
    assert!(texts[0].contains("font-family=\"monospace\""));
    assert!(texts[1].contains("font-family=\"&quot;Chart Body&quot;, sans-serif\""));
    assert!(
        texts
            .iter()
            .all(|text| text.contains("font-weight=\"700\""))
    );
    let canvas = render::to_canvas_script(&scene, "chart").unwrap();
    assert!(canvas.contains("ctx.font=\"700 18px monospace\""));
    assert!(canvas.contains("ctx.font=\"700 18px \\\"Chart Body\\\", sans-serif\""));
    assert!(canvas.find("Second").unwrap() < canvas.find("First").unwrap());
}

#[test]
fn explicit_regular_label_resets_bold_parent_and_default_exports_reset_host_weight() {
    let mut scene = scene();
    scene.typography.default.weight = Some(800.);
    scene.typography.labels.insert(
        1,
        FontOptions {
            family: None,
            weight: Some(400.),
        },
    );
    let svg = render::to_svg(&scene).unwrap();
    assert_eq!(svg.matches("font-weight=\"800\"").count(), 1);
    assert_eq!(svg.matches("font-weight=\"400\"").count(), 1);
    let canvas = render::to_canvas_script(&scene, "chart").unwrap();
    assert!(canvas.contains("ctx.font=\"800 18px sans-serif\""));
    assert!(canvas.contains("ctx.font=\"18px sans-serif\""));
    let default_svg = render::to_svg(&self::scene()).unwrap();
    assert_eq!(default_svg.matches("font-weight=\"400\"").count(), 2);
}

#[test]
fn quoted_face_name_remains_data_in_xml_and_embedded_javascript() {
    let mut scene = scene();
    scene.typography.default.family = Some(vec![Family::Named(
        "serif\"; color: red; \\ </script><foreignObject>&".into(),
    )]);
    let svg = render::to_svg(&scene).unwrap();
    assert!(!svg.contains("<foreignObject>"));
    assert!(!svg.contains("</script>"));
    assert!(svg.contains("&lt;/script&gt;&lt;foreignObject&gt;&amp;"));
    assert!(svg.contains("&quot;"));
    let script = render::to_canvas_script(&scene, "chart").unwrap();
    assert!(!script.contains('<'));
    assert!(script.contains("\\u003c/script\\u003e"));
    assert!(script.contains("\\u0026"));
    // A named CSS keyword stays quoted, distinct from its generic counterpart.
    scene.typography.default.family = Some(vec![Family::Named("serif".into()), Family::Serif]);
    assert!(
        render::to_svg(&scene)
            .unwrap()
            .contains("font-family=\"&quot;serif&quot;, serif\"")
    );
}

#[derive(Default)]
struct LegacyCanvas {
    events: Vec<&'static str>,
}
impl Canvas for LegacyCanvas {
    fn begin(&mut self, _: f64, _: f64, _: Color) -> Result<()> {
        self.events.push("begin");
        Ok(())
    }
    fn begin_path(&mut self) -> Result<()> {
        Ok(())
    }
    fn move_to(&mut self, _: f64, _: f64) -> Result<()> {
        Ok(())
    }
    fn line_to(&mut self, _: f64, _: f64) -> Result<()> {
        Ok(())
    }
    fn cubic_to(&mut self, _: f64, _: f64, _: f64, _: f64, _: f64, _: f64) -> Result<()> {
        Ok(())
    }
    fn close_path(&mut self) -> Result<()> {
        Ok(())
    }
    fn fill(&mut self, _: Color, _: f64) -> Result<()> {
        Ok(())
    }
    fn stroke(&mut self, _: GraphicStroke, _: f64) -> Result<()> {
        Ok(())
    }
    fn text(&mut self, _: &Label) -> Result<()> {
        self.events.push("text");
        Ok(())
    }
    fn finish(&mut self) -> Result<()> {
        self.events.push("finish");
        Ok(())
    }
}

#[test]
fn legacy_canvas_keeps_default_text_and_refuses_custom_fonts_before_any_mutation() {
    let mut scene = scene();
    let mut target = LegacyCanvas::default();
    render::draw(&scene, &mut target).unwrap();
    assert_eq!(target.events, ["begin", "text", "text", "finish"]);
    target.events.clear();
    scene.typography.default.weight = Some(400.);
    render::draw(&scene, &mut target).unwrap();
    assert_eq!(target.events, ["begin", "text", "text", "finish"]);
    for font in [
        FontOptions {
            family: Some(vec![Family::Serif]),
            weight: None,
        },
        FontOptions {
            family: None,
            weight: Some(700.),
        },
    ] {
        scene.typography.default = font;
        target.events.clear();
        assert!(render::draw(&scene, &mut target).is_err());
        assert!(target.events.is_empty());
    }
}

#[test]
fn invalid_font_proposals_fail_all_exporters_and_leave_legacy_target_untouched() {
    let mut scene = scene();
    for font in [
        FontOptions {
            family: Some(Vec::new()),
            weight: None,
        },
        FontOptions {
            family: Some(vec![Family::Named("bad\nface".into())]),
            weight: None,
        },
        FontOptions {
            family: None,
            weight: Some(f64::NAN),
        },
        FontOptions {
            family: None,
            weight: Some(1001.),
        },
    ] {
        scene.typography.default = font;
        let mut target = LegacyCanvas::default();
        assert!(render::draw(&scene, &mut target).is_err());
        assert!(target.events.is_empty());
        assert!(render::to_svg(&scene).is_err());
        assert!(render::to_canvas_script(&scene, "chart").is_err());
    }
}

#[derive(Default)]
struct FontCanvas {
    calls: Vec<(String, ResolvedFont)>,
}
impl Canvas for FontCanvas {
    fn supports_text_fonts(&self) -> bool {
        true
    }
    fn begin(&mut self, _: f64, _: f64, _: Color) -> Result<()> {
        Ok(())
    }
    fn begin_path(&mut self) -> Result<()> {
        Ok(())
    }
    fn move_to(&mut self, _: f64, _: f64) -> Result<()> {
        Ok(())
    }
    fn line_to(&mut self, _: f64, _: f64) -> Result<()> {
        Ok(())
    }
    fn cubic_to(&mut self, _: f64, _: f64, _: f64, _: f64, _: f64, _: f64) -> Result<()> {
        Ok(())
    }
    fn close_path(&mut self) -> Result<()> {
        Ok(())
    }
    fn fill(&mut self, _: Color, _: f64) -> Result<()> {
        Ok(())
    }
    fn stroke(&mut self, _: GraphicStroke, _: f64) -> Result<()> {
        Ok(())
    }
    fn text(&mut self, label: &Label) -> Result<()> {
        self.calls
            .push((label.text.clone(), ResolvedFont::default()));
        Ok(())
    }
    fn text_with_font(&mut self, label: &Label, font: &ResolvedFont) -> Result<()> {
        self.calls.push((label.text.clone(), font.clone()));
        Ok(())
    }
    fn finish(&mut self) -> Result<()> {
        Ok(())
    }
}

#[test]
fn canvas_font_protocol_passes_resolved_slots_in_scene_paint_order() {
    let mut scene = scene();
    scene.typography.default.weight = Some(700.);
    scene.typography.labels.insert(
        1,
        FontOptions {
            family: Some(vec![Family::Mono]),
            weight: Some(400.),
        },
    );
    scene.draw_order = vec![DrawItem::Label(1), DrawItem::Label(0)];
    let mut canvas = FontCanvas::default();
    render::draw(&scene, &mut canvas).unwrap();
    assert_eq!(
        canvas.calls,
        vec![
            (
                "Second".into(),
                ResolvedFont {
                    family: vec![Family::Mono],
                    weight: 400.,
                },
            ),
            (
                "First".into(),
                ResolvedFont {
                    family: vec![Family::Sans],
                    weight: 700.,
                },
            ),
        ]
    );
}
