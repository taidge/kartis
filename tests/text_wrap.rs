use kartis::*;

fn label(text: &str) -> Label {
    Label {
        x: 20.,
        y: 30.,
        text: text.into(),
        size: 10.,
        rotation: 0.,
        anchor: TextAnchor::Start,
        color: Color(0, 0, 0, 255),
    }
}
fn measure(request: &SemanticTextRequest) -> Result<SemanticTextMeasurement> {
    let mut advance = 0.;
    for word in request.text.split_inclusive(' ') {
        advance += match word {
            "A " => 12.,
            "B " => 14.,
            "A" => 8.,
            "B" => 10.,
            "C" => 7.,
            "…" => 5.,
            _ => request.text.len() as f64 * 5.,
        };
    }
    request.measured(SemanticTextMetrics {
        advance,
        line_height: request.line_height,
        baseline_from_top: 8.,
        ink_bounds: None,
    })
}
#[test]
fn words_use_actual_candidate_measurements() {
    let result = wrap_label(
        &label("A B C"),
        &ResolvedFont::default(),
        &TextWrapOptions {
            max_width: 15.,
            ..Default::default()
        },
        &mut measure,
    )
    .unwrap();
    assert_eq!(
        result
            .labels
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>(),
        ["A", "B", "C"]
    );
    assert_eq!(result.labels[1].y, 42.);
    assert!(!result.truncated);
}
#[test]
fn nonmonotonic_advances_do_not_cause_premature_breaks() {
    let result = wrap_label(
        &label("fi"),
        &ResolvedFont::default(),
        &TextWrapOptions {
            max_width: 10.,
            mode: TextWrapMode::Graphemes,
            ..Default::default()
        },
        &mut |r| {
            r.measured(SemanticTextMetrics {
                advance: if r.text == "f" { 12. } else { 8. },
                line_height: r.line_height,
                baseline_from_top: 8.,
                ink_bounds: None,
            })
        },
    )
    .unwrap();
    assert_eq!(result.labels[0].text, "fi");
    assert_eq!(result.labels.len(), 1);
}
#[test]
fn clipping_and_measured_ellipsis_are_explicit() {
    let options = TextWrapOptions {
        max_width: 15.,
        max_lines: 1,
        overflow: TextWrapOverflow::Ellipsis,
        ..Default::default()
    };
    let result = wrap_label(
        &label("A B C"),
        &ResolvedFont::default(),
        &options,
        &mut |r| {
            r.measured(SemanticTextMetrics {
                advance: match r.text.as_str() {
                    "A" => 8.,
                    "A " => 12.,
                    "…" => 5.,
                    "A…" => 13.,
                    _ => 30.,
                },
                line_height: r.line_height,
                baseline_from_top: 8.,
                ink_bounds: None,
            })
        },
    )
    .unwrap();
    assert_eq!(result.labels[0].text, "A…");
    assert!(result.truncated);
    let result = wrap_label(
        &label("A B"),
        &ResolvedFont::default(),
        &TextWrapOptions {
            overflow: TextWrapOverflow::Clip,
            ..options
        },
        &mut measure,
    )
    .unwrap();
    assert_eq!(result.labels[0].text, "A");
    assert!(result.truncated);
}
#[test]
fn callback_and_work_failures_preserve_the_scene() {
    let mut scene = ChartConfig::default().scene().unwrap();
    scene.labels.push(label("A B C"));
    let before = scene.clone();
    assert!(
        wrap_scene_label(
            &mut scene,
            0,
            &TextWrapOptions {
                max_measurements: 1,
                ..Default::default()
            },
            &mut measure
        )
        .is_err()
    );
    assert_eq!(scene, before);
    assert!(
        wrap_scene_label(&mut scene, 0, &TextWrapOptions::default(), &mut |_| Err(
            ChartError("font unavailable".into())
        ))
        .is_err()
    );
    assert_eq!(scene, before);
}
#[test]
fn stale_font_resource_measurements_reject() {
    let other = SemanticTextRequest::for_label(
        &label("stale"),
        &ResolvedFont::default(),
        &SemanticTextProfile::default(),
    )
    .unwrap();
    let stale = measure(&other).unwrap();
    assert!(
        wrap_label(
            &label("A"),
            &ResolvedFont::default(),
            &TextWrapOptions {
                revision: 1,
                ..Default::default()
            },
            &mut |_| Ok(stale.clone())
        )
        .is_err()
    );
}
#[test]
fn one_cluster_wider_than_width_is_not_split() {
    let options = TextWrapOptions {
        max_width: 1.,
        mode: TextWrapMode::Graphemes,
        ..Default::default()
    };
    let text = "👩🏽‍🚀";
    assert!(
        wrap_label(
            &label(text),
            &ResolvedFont::default(),
            &options,
            &mut measure
        )
        .is_err()
    );
    let result = wrap_label(
        &label(text),
        &ResolvedFont::default(),
        &TextWrapOptions {
            long_word: TextWrapLongWord::Overflow,
            ..options
        },
        &mut measure,
    )
    .unwrap();
    assert_eq!(result.labels[0].text, text);
}
