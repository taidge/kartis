use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::*;

fn label(text: &str) -> Label {
    Label {
        text: text.into(),
        x: 100.,
        y: 100.,
        size: 10.,
        rotation: 0.,
        anchor: TextAnchor::End,
        color: Color(0, 0, 0, 255),
    }
}

// Supplied protocol fixtures, not width estimates or a production measurer.
fn supplied(request: &SemanticTextRequest, advance: f64) -> Result<SemanticTextMeasurement> {
    request.measured(SemanticTextMetrics {
        advance,
        line_height: request.line_height,
        baseline_from_top: 8.,
        ink_bounds: None,
    })
}

#[test]
fn measurement_table_never_accepts_a_broken_extended_grapheme() {
    let clusters = ["e\u{301}", "🇨🇳", "👩‍💻"];
    let source = clusters.concat();
    let options = TextWrapOptions {
        max_width: 1.,
        mode: TextWrapMode::Graphemes,
        whitespace: TextWrapWhitespace::Preserve,
        long_word: TextWrapLongWord::Error,
        ..Default::default()
    };
    let wrapped = wrap_label(
        &label(&source),
        &ResolvedFont::default(),
        &options,
        &mut |request| {
            let advance = match request.text.as_str() {
                "" => 0.,
                "e\u{301}" | "🇨🇳" | "👩‍💻" => 1.,
                "e\u{301}🇨🇳" | "🇨🇳👩‍💻" => 2.,
                "e\u{301}🇨🇳👩‍💻" => 3.,
                other => panic!("engine submitted a broken grapheme: {other:?}"),
            };
            supplied(request, advance)
        },
    )
    .unwrap();
    assert_eq!(
        wrapped
            .labels
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>(),
        clusters
    );
}

#[test]
fn crlf_preserves_one_hard_break_and_the_empty_line() {
    let wrapped = wrap_label(
        &label("alpha\r\n\r\nbeta"),
        &ResolvedFont::default(),
        &TextWrapOptions::default(),
        &mut |r| supplied(r, 12.),
    )
    .unwrap();
    assert_eq!(
        wrapped
            .labels
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>(),
        ["alpha", "", "beta"]
    );
}

#[test]
fn a_fitting_word_does_not_overflow_because_of_discarded_trailing_space() {
    let options = TextWrapOptions {
        max_width: 1.,
        long_word: TextWrapLongWord::Error,
        ..Default::default()
    };
    let wrapped = wrap_label(
        &label("a b"),
        &ResolvedFont::default(),
        &options,
        &mut |r| {
            supplied(
                r,
                match r.text.as_str() {
                    "" => 0.,
                    "a" | "b" => 1.,
                    "a " => 2.,
                    "a b" => 3.,
                    other => panic!("unexpected measured candidate {other:?}"),
                },
            )
        },
    )
    .unwrap();
    assert_eq!(
        wrapped
            .labels
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
}

#[test]
fn rotated_lines_expand_every_owned_slot_at_the_original_paint_position() {
    let mut scene = ChartConfig::default().scene().unwrap();
    let mut middle = label("alpha\nbeta\ngamma");
    middle.rotation = 90.;
    scene.labels = vec![label("before"), middle, label("after")];
    scene.marks.push(Mark::new(
        "dot",
        Geometry::Circle {
            x: 50.,
            y: 50.,
            radius: 2.,
        },
        Color(255, 0, 0, 255),
    ));
    scene.draw_order = vec![
        DrawItem::Label(0),
        DrawItem::Mark(0),
        DrawItem::Label(1),
        DrawItem::Label(2),
    ];
    scene.typography.labels.insert(
        1,
        FontOptions {
            weight: Some(700.),
            ..Default::default()
        },
    );
    scene.typography.labels.insert(
        2,
        FontOptions {
            weight: Some(300.),
            ..Default::default()
        },
    );
    scene.label_series.insert(1, "middle".into());
    scene.label_series.insert(2, "last".into());
    let clip = ClipPath::circle(100., 100., 80.);
    scene.label_clips.insert(1, clip.clone());
    let options = TextWrapOptions {
        line_height: 2.,
        ..Default::default()
    };
    wrap_scene_label(&mut scene, 1, &options, &mut |r| {
        assert_eq!(r.font.weight, 700.);
        supplied(r, 12.)
    })
    .unwrap();
    scene.validate().unwrap();
    assert_eq!(scene.labels.len(), 5);
    assert_eq!(scene.labels[4].text, "after");
    for (i, x) in [(1, 100.), (2, 80.), (3, 60.)] {
        assert!((scene.labels[i].x - x).abs() < 1e-10);
        assert!((scene.labels[i].y - 100.).abs() < 1e-10);
        assert_eq!(scene.labels[i].anchor, TextAnchor::End);
        assert_eq!(scene.resolved_label_font(i).weight, 700.);
        assert_eq!(scene.label_series[&i], "middle");
        assert_eq!(scene.label_clips[&i], clip);
    }
    assert_eq!(scene.resolved_label_font(4).weight, 300.);
    assert_eq!(scene.label_series[&4], "last");
    assert_eq!(
        scene.draw_order,
        vec![
            DrawItem::Label(0),
            DrawItem::Mark(0),
            DrawItem::Label(1),
            DrawItem::Label(2),
            DrawItem::Label(3),
            DrawItem::Label(4)
        ]
    );
}

#[test]
fn stale_measurements_and_exhausted_work_leave_scene_unchanged() {
    let mut scene = ChartConfig::default().scene().unwrap();
    scene.labels.push(label("alpha\nbeta"));
    let before = scene.clone();
    assert!(
        wrap_scene_label(&mut scene, 0, &TextWrapOptions::default(), &mut |r| {
            let mut stale = r.clone();
            stale.revision += 1;
            supplied(&stale, 12.)
        })
        .is_err()
    );
    assert_eq!(scene, before);
    assert!(
        wrap_scene_label(
            &mut scene,
            0,
            &TextWrapOptions {
                max_measurements: 1,
                ..Default::default()
            },
            &mut |r| supplied(r, 12.)
        )
        .is_err()
    );
    assert_eq!(scene, before);
}

#[test]
fn expanded_clip_budget_rejects_without_publishing_a_partial_scene() {
    let mut scene = ChartConfig::default().scene().unwrap();
    scene.labels.push(label(&"x\n".repeat(50)));
    let mut commands = Vec::new();
    for _ in 0..1000 {
        commands.extend([
            C::Move(P::new(0., 0.)),
            C::Line(P::new(10., 0.)),
            C::Line(P::new(0., 10.)),
            C::Close,
        ]);
    }
    scene.label_clips.insert(
        0,
        ClipPath {
            geometry: Geometry::Path(commands),
            rule: ClipRule::NonZero,
        },
    );
    scene.validate().unwrap();
    let before = scene.clone();
    let error = wrap_scene_label(&mut scene, 0, &TextWrapOptions::default(), &mut |r| {
        supplied(r, 12.)
    })
    .unwrap_err();
    assert!(error.0.contains("clip"), "{error}");
    assert_eq!(scene, before);
}
