use glory::render::GraphicStroke;
use kartis::render::{self, Canvas};
use kartis::*;

fn scene() -> Scene {
    let mut scene = ChartConfig::default().scene().unwrap();
    scene.labels.push(Label {
        rotation: 15.,
        x: 40.,
        y: 50.,
        text: "  A\t<&\"  \r\n\u{a0}".into(),
        color: Color(1, 2, 3, 255),
        size: 16.,
        anchor: TextAnchor::End,
    });
    scene
}

#[test]
fn svg_preserves_only_opted_slots_and_legacy_bytes_restore() {
    let mut scene = scene();
    let before = render::to_svg(&scene).unwrap();
    assert!(!before.contains("white-space"));
    scene
        .typography
        .label_whitespace
        .insert(0, LabelWhitespace::Legacy);
    assert_eq!(render::to_svg(&scene).unwrap(), before);
    scene
        .typography
        .label_whitespace
        .insert(0, LabelWhitespace::Preserve);
    let svg = render::to_svg(&scene).unwrap();
    assert!(svg.contains("style=\"white-space:pre\""));
    assert!(svg.contains("  A &lt;&amp;&quot;    \u{a0}"));
    assert!(svg.contains("rotate(15 40 50)"));
    assert!(!svg.contains('\t') && !svg.contains('\r') && !svg.contains('\n'));
    assert!(svg.contains('\u{a0}'));
    scene.typography.label_whitespace.remove(&0);
    assert_eq!(render::to_svg(&scene).unwrap(), before);
}

#[derive(Default)]
struct Recorder {
    opted: bool,
    began: bool,
    labels: Vec<Label>,
}
impl Canvas for Recorder {
    fn supports_text_rotation(&self) -> bool {
        true
    }
    fn supports_text_whitespace(&self) -> bool {
        self.opted
    }
    fn begin(&mut self, _: f64, _: f64, _: Color) -> Result<()> {
        self.began = true;
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
        self.labels.push(label.clone());
        Ok(())
    }
    fn finish(&mut self) -> Result<()> {
        Ok(())
    }
}

#[test]
fn custom_canvas_must_opt_in_before_any_mutation() {
    let mut scene = scene();
    scene
        .typography
        .label_whitespace
        .insert(0, LabelWhitespace::Preserve);
    let mut target = Recorder::default();
    assert!(
        render::draw(&scene, &mut target)
            .unwrap_err()
            .0
            .contains("whitespace")
    );
    assert!(!target.began && target.labels.is_empty());
    target.opted = true;
    render::draw(&scene, &mut target).unwrap();
    assert_eq!(target.labels[0].text, "  A <&\"    \u{a0}");
    assert_eq!(target.labels[0].rotation, scene.labels[0].rotation);
    assert_eq!(scene.labels[0].text, "  A\t<&\"  \r\n\u{a0}");
}

#[test]
fn legacy_custom_canvas_and_javascript_keep_default_behavior() {
    let mut scene = scene();
    let mut target = Recorder::default();
    render::draw(&scene, &mut target).unwrap();
    assert_eq!(target.labels, scene.labels);
    let original = render::to_canvas_script(&scene, "canvas").unwrap();
    scene
        .typography
        .label_whitespace
        .insert(0, LabelWhitespace::Legacy);
    assert_eq!(
        render::to_canvas_script(&scene, "canvas").unwrap(),
        original
    );
    scene
        .typography
        .label_whitespace
        .insert(0, LabelWhitespace::Preserve);
    let preserved = render::to_canvas_script(&scene, "canvas").unwrap();
    assert_ne!(original, preserved);
    assert!(!preserved.contains("\\t") && !preserved.contains("\\r") && !preserved.contains("\\n"));
}
