use std::collections::{BTreeMap, BTreeSet};

use glory::render::GraphicStroke;
use kartis::render::{Canvas, draw, to_svg};
use kartis::*;

fn halo() -> LabelStroke {
    LabelStroke {
        color: Color(40, 70, 190, 160),
        width: 3.,
    }
}
fn scene() -> Scene {
    let mut scene = ChartConfig::default().scene().unwrap();
    scene.labels = ["hidden", "A B", "extra"]
        .map(|text| Label {
            x: 30.,
            y: 40.,
            text: text.into(),
            size: 14.,
            color: Color(0, 0, 0, 0),
            rotation: 0.,
            anchor: TextAnchor::Start,
        })
        .into();
    scene.label_series.insert(0, "hidden".into());
    scene.typography.label_strokes = BTreeMap::from([(0, halo()), (1, halo())]);
    scene.draw_order = vec![DrawItem::Label(2), DrawItem::Label(0), DrawItem::Label(1)];
    scene
}

#[test]
fn hidden_and_sparse_remap_preserve_only_the_correct_slots() {
    let source = scene();
    let hidden = source.without_series(&BTreeSet::from(["hidden".into()]));
    hidden.validate().unwrap();
    assert_eq!(
        hidden
            .labels
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>(),
        ["A B", "extra"]
    );
    assert_eq!(
        hidden.typography.label_strokes,
        BTreeMap::from([(0, halo())])
    );
    assert_eq!(hidden.draw_order, [DrawItem::Label(1), DrawItem::Label(0)]);
    assert_eq!(source.typography.label_strokes.len(), 2);
    let mut typography = source.typography.clone();
    typography.remap_labels(&BTreeMap::from([(1, 5), (2, 0)]));
    assert_eq!(typography.label_strokes, BTreeMap::from([(5, halo())]));
}

#[test]
fn wrapping_copies_stroke_to_each_line_and_shifts_later_slots() {
    let mut source = scene();
    source.typography.label_strokes.insert(
        2,
        LabelStroke {
            width: 5.,
            ..halo()
        },
    );
    let wrapped = wrap_scene_label(
        &mut source,
        1,
        &TextWrapOptions {
            max_width: 11.,
            ..Default::default()
        },
        &mut |request| {
            request.measured(SemanticTextMetrics {
                advance: request.text.len() as f64 * 10.,
                line_height: request.line_height,
                baseline_from_top: 10.,
                ink_bounds: None,
            })
        },
    )
    .unwrap();
    assert_eq!(wrapped.labels.len(), 2);
    assert_eq!(
        source
            .labels
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>(),
        ["hidden", "A", "B", "extra"]
    );
    assert_eq!(source.typography.label_strokes.get(&1), Some(&halo()));
    assert_eq!(source.typography.label_strokes.get(&2), Some(&halo()));
    assert_eq!(source.typography.label_strokes.get(&3).unwrap().width, 5.);
    assert_eq!(
        source.draw_order,
        [
            DrawItem::Label(3),
            DrawItem::Label(0),
            DrawItem::Label(1),
            DrawItem::Label(2)
        ]
    );
    source.validate().unwrap();
}

#[test]
fn widget_native_run_uses_its_own_label_ordinals() {
    let source = scene();
    let plan = widget_paint_plan::scene_widget_paint_plan(
        &source,
        &SceneWidgetsLayout {
            plot: None,
            slots: vec![],
        },
        &BTreeSet::new(),
        &MarkBlends::default(),
    )
    .unwrap();
    let widget_paint_plan::WidgetPaintNode::Paint(run) = &plan.nodes()[0] else {
        panic!("native paint run");
    };
    assert_eq!(
        run.scene()
            .labels
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>(),
        ["extra", "hidden", "A B"]
    );
    assert_eq!(
        run.scene().typography.label_strokes,
        BTreeMap::from([(1, halo()), (2, halo())])
    );
    run.scene().validate().unwrap();
}

#[derive(Default)]
struct Target {
    begins: usize,
    text_calls: usize,
    strokes: Vec<(Color, LabelStroke)>,
    supported: bool,
}
impl Canvas for Target {
    fn supports_text_stroke(&self) -> bool {
        self.supported
    }
    fn begin(&mut self, _: f64, _: f64, _: Color) -> Result<()> {
        self.begins += 1;
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
        self.text_calls += 1;
        Ok(())
    }
    fn text_with_stroke(
        &mut self,
        label: &Label,
        _: &ResolvedFont,
        _: bool,
        stroke: LabelStroke,
    ) -> Result<()> {
        self.strokes.push((label.color, stroke));
        Ok(())
    }
    fn finish(&mut self) -> Result<()> {
        Ok(())
    }
}

#[test]
fn unsupported_target_rejects_before_begin_but_transparent_fill_keeps_stroke() {
    let source = scene();
    let mut target = Target::default();
    assert!(draw(&source, &mut target).is_err());
    assert_eq!(target.begins, 0);
    assert_eq!(target.text_calls, 0);
    target.supported = true;
    draw(&source, &mut target).unwrap();
    assert_eq!(target.begins, 1);
    assert_eq!(target.strokes, [(Color(0, 0, 0, 0), halo()); 2]);
    assert!(to_svg(&source).unwrap().contains("stroke-width=\"3\""));
}

#[test]
fn absent_stroke_keeps_the_legacy_canvas_and_svg_protocol() {
    let mut source = scene();
    source.typography.label_strokes.clear();
    for label in &mut source.labels {
        label.color = Color(30, 40, 50, 255);
    }
    let mut target = Target::default();
    draw(&source, &mut target).unwrap();
    assert_eq!(target.text_calls, 3);
    assert!(target.strokes.is_empty());
    let svg = to_svg(&source).unwrap();
    assert!(!svg.contains("stroke-width="));
    assert_eq!(svg, to_svg(&source.clone()).unwrap());
}

#[test]
fn generic_motion_refuses_unsupported_stroke_transactionally() {
    use kartis::motion::{PropertyMotion, PropertyMotionOptions, SpringConfig, SpringMotion};
    let stroked = scene();
    assert!(SpringMotion::new(stroked.clone(), SpringConfig::default()).is_err());
    assert!(PropertyMotion::new(stroked.clone(), PropertyMotionOptions::default()).is_err());
    let mut plain = stroked.clone();
    plain.typography.label_strokes.clear();
    let mut spring = SpringMotion::new(plain.clone(), SpringConfig::default()).unwrap();
    spring.set_reduced_motion(true);
    assert!(spring.retarget(stroked.clone()).is_err());
    assert_eq!(spring.scene(), &plain);
    let mut properties =
        PropertyMotion::new(plain.clone(), PropertyMotionOptions::default()).unwrap();
    assert!(properties.retarget(stroked).is_err());
    assert_eq!(properties.scene(), &plain);
}

#[test]
fn invalid_stroke_is_admitted_before_any_target_mutation() {
    for stroke in [
        LabelStroke {
            width: f64::NAN,
            ..halo()
        },
        LabelStroke {
            width: -1.,
            ..halo()
        },
    ] {
        let mut source = scene();
        source.typography.label_strokes.insert(0, stroke);
        let mut target = Target {
            supported: true,
            ..Default::default()
        };
        assert!(draw(&source, &mut target).is_err());
        assert_eq!(target.begins, 0);
    }
    let mut source = scene();
    source.typography.label_strokes.insert(3, halo());
    assert!(source.validate().is_err());
}

#[test]
fn tree_canvas_and_motion_keep_captured_strokes_on_live_and_departing_labels() {
    let config = ChartConfig::default();
    let rows = vec![
        HierarchyDatum::new("r", None, 0.),
        HierarchyDatum::new("a", Some("r".into()), 3.),
    ];
    let mut options = TreeOptions::default();
    options.hierarchy.labels = true;
    let mut layout = tree_layout(&config, &rows, &options).unwrap();
    let source_slot = *layout
        .scene
        .label_series
        .iter()
        .find(|(_, owner)| owner.as_str() == "a")
        .unwrap()
        .0;
    layout
        .scene
        .typography
        .label_strokes
        .insert(source_slot, halo());
    let canvas = TreeCanvasFrame::new(config.clone(), layout.clone()).unwrap();
    assert_eq!(
        canvas.label_context(source_slot).unwrap().stroke,
        Some(halo())
    );
    let event = TreeEventFrame::new(config.clone(), layout).unwrap();
    let mut motion = TreeMotion::new(event, TreeMotionOptions::default()).unwrap();
    let live = tree_motion_paint(motion.frame()).unwrap();
    let live_slot = *live
        .scene()
        .label_series
        .iter()
        .find(|(_, owner)| owner.as_str() == "a")
        .unwrap()
        .0;
    assert_eq!(
        live.scene().typography.label_strokes.get(&live_slot),
        Some(&halo())
    );
    // A settled Label leaves immediately. Move it first so its retained
    // departure is a real nonzero-velocity historical row.
    let mut moving = tree_layout(&config, &rows, &options).unwrap();
    moving.scene.labels[source_slot].x += 40.;
    moving
        .scene
        .typography
        .label_strokes
        .insert(source_slot, halo());
    motion
        .retarget(TreeEventFrame::new(config.clone(), moving).unwrap())
        .unwrap();
    motion.tick(0.03).unwrap();
    let target = tree_layout(&config, &rows[..1], &options).unwrap();
    motion
        .retarget(TreeEventFrame::new(config, target).unwrap())
        .unwrap();
    motion.tick(0.01).unwrap();
    let leaving = tree_motion_paint(motion.frame()).unwrap();
    let ghost_slot = *leaving
        .scene()
        .label_series
        .iter()
        .find(|(_, owner)| owner.as_str() == "a")
        .unwrap()
        .0;
    assert!(leaving.ghost_label_indices().contains(&ghost_slot));
    assert_eq!(
        leaving.scene().typography.label_strokes.get(&ghost_slot),
        Some(&halo())
    );
    leaving.scene().validate().unwrap();
}

#[cfg(feature = "raster")]
#[test]
fn unsupported_glyph_lowering_refuses_before_font_lookup_even_with_transparent_fill() {
    let source = scene();
    let fonts = render::raster::FontRegistry::new();
    // The legacy outline default selects only rotated labels. Skipping an
    // upright label must leave its native stroke intact, without font lookup.
    let original = source.clone();
    let unselected = text_outline::outline_labels(&source, &fonts, &Default::default()).unwrap();
    assert!(unselected.labels.is_empty());
    assert_eq!(source, original);
    let mut target = Target {
        supported: true,
        ..Default::default()
    };
    draw(&source, &mut target).unwrap();
    assert_eq!(target.strokes, [(Color(0, 0, 0, 0), halo()); 2]);
    let check = |result: Result<()>| {
        assert_eq!(
            result.unwrap_err().to_string(),
            "glyph lowering does not support label strokes"
        );
    };
    check(
        text_outline::outline_labels(
            &source,
            &fonts,
            &OutlineLabelOptions {
                rotated_only: false,
                ..Default::default()
            },
        )
        .map(|_| ()),
    );
    check(text_outline::painted_labels(&source, &fonts, &Default::default()).map(|_| ()));
    check(
        scene_with_painted_labels(&source, &fonts, &Default::default(), "stroke-test").map(|_| ()),
    );
}
