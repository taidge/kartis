use kartis::*;
#[derive(Default)]
struct Recorder {
    log: Vec<&'static str>,
}
impl render::Canvas for Recorder {
    fn begin(&mut self, _: f64, _: f64, _: Color) -> Result<()> {
        self.log.push("begin");
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
        self.log.push("fill");
        Ok(())
    }
    fn stroke(&mut self, _: glory::render::GraphicStroke, _: f64) -> Result<()> {
        Ok(())
    }
    fn text(&mut self, _: &Label) -> Result<()> {
        self.log.push("text");
        Ok(())
    }
    fn finish(&mut self) -> Result<()> {
        self.log.push("finish");
        Ok(())
    }
}
fn scene() -> Scene {
    let c = ChartConfig::default();
    let mut s = c.scene().unwrap();
    for (i, id) in ["a", "b"].iter().enumerate() {
        let mut m = Mark::new(
            *id,
            Geometry::Rect {
                x: 10.,
                y: 10.,
                width: 100.,
                height: 100.,
            },
            c.theme.color(i),
        );
        m.series = (*id).into();
        s.marks.push(m);
        s.labels.push(Label {
            rotation: 0.,
            x: 50.,
            y: 50.,
            text: format!("caption-{id}"),
            color: c.theme.text,
            size: 12.,
            anchor: TextAnchor::Middle,
        });
        s.label_series.insert(i, (*id).into());
    }
    s.draw_order = vec![
        DrawItem::Mark(0),
        DrawItem::Label(0),
        DrawItem::Mark(1),
        DrawItem::Label(1),
    ];
    s
}
#[test]
fn arbitrary_paint_order_drives_svg_canvas_and_hit_testing() {
    let mut s = scene();
    s.validate().unwrap();
    let svg = render::to_svg(&s).unwrap();
    assert!(svg.find("caption-a").unwrap() < svg.find("data-id=\"b\"").unwrap());
    let mut recorder = Recorder::default();
    render::draw(&s, &mut recorder).unwrap();
    assert_eq!(
        recorder.log,
        vec!["begin", "fill", "text", "fill", "text", "finish"]
    );
    assert_eq!(s.hit_test(50., 50.).unwrap().id, "b");
    s.draw_order.swap(0, 2);
    assert_eq!(s.hit_test(50., 50.).unwrap().id, "a");
}
#[test]
fn invalid_order_is_rejected_before_paint_and_owned_labels_filter_atomically() {
    let s = scene();
    let filtered = s.without_series(&["a".into()].into_iter().collect());
    filtered.validate().unwrap();
    assert_eq!(filtered.marks.len(), 1);
    assert_eq!(filtered.labels.len(), 1);
    assert_eq!(filtered.labels[0].text, "caption-b");
    assert_eq!(
        filtered.draw_items(),
        vec![DrawItem::Mark(0), DrawItem::Label(0)]
    );
    let mut invalid = s.clone();
    invalid.draw_order[0] = DrawItem::Mark(1);
    let mut recorder = Recorder::default();
    assert!(render::draw(&invalid, &mut recorder).is_err());
    assert!(recorder.log.is_empty());
    invalid = s;
    invalid.label_series.insert(99, "none".into());
    assert!(invalid.validate().is_err());
}
