use kartis::*;

fn label(text: &str) -> Label {
    Label {
        text: text.into(),
        size: 100.,
        x: 100.,
        y: 140.,
        anchor: TextAnchor::Start,
        rotation: 0.,
        color: Color(0, 0, 0, 255),
    }
}

// These are supplied protocol values for testing coordinate transformations;
// they are never inferred by Kartis or claimed as a real host's line metrics.
fn supplied_metrics() -> SemanticTextMetrics {
    SemanticTextMetrics {
        advance: 60.,
        line_height: 100.,
        baseline_from_top: 90.,
        ink_bounds: Some(LayoutRect {
            x: 2.,
            y: -70.,
            width: 40.,
            height: 70.,
        }),
    }
}

fn request(label: &Label) -> SemanticTextRequest {
    SemanticTextProfile::default()
        .request(label, &ResolvedFont::default())
        .unwrap()
}

fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-8, "{actual} != {expected}");
}

#[test]
fn request_identity_covers_each_font_resource_and_layout_input() {
    let current = request(&label("B"));
    let measured = current.measured(supplied_metrics()).unwrap();
    assert_eq!(
        measured.validate_for(&current).unwrap(),
        &supplied_metrics()
    );
    let variants = [
        SemanticTextRequest {
            text: "A".into(),
            ..current.clone()
        },
        SemanticTextRequest {
            size: 101.,
            ..current.clone()
        },
        SemanticTextRequest {
            line_height: 100.5,
            ..current.clone()
        },
        SemanticTextRequest {
            revision: 1,
            ..current.clone()
        },
        SemanticTextRequest {
            font: ResolvedFont {
                weight: 700.,
                ..current.font.clone()
            },
            ..current.clone()
        },
        SemanticTextRequest {
            font: ResolvedFont {
                family: vec![FontFamily::Serif],
                ..current.font.clone()
            },
            ..current.clone()
        },
    ];
    for stale in variants {
        stale.validate().unwrap();
        assert!(measured.validate_for(&stale).is_err());
    }
    assert_eq!(measured.request(), &current);
    assert_eq!(measured.metrics(), &supplied_metrics());
}

#[test]
fn physical_anchors_and_repositioning_reuse_the_same_baseline_measurement() {
    let original = label("B");
    let current = request(&original);
    let measured = current.measured(supplied_metrics()).unwrap();
    for (anchor, left) in [
        (TextAnchor::Start, 100.),
        (TextAnchor::Middle, 70.),
        (TextAnchor::End, 40.),
    ] {
        let moved = Label {
            anchor,
            color: Color(255, 0, 0, 128),
            ..original.clone()
        };
        assert_eq!(request(&moved), current);
        let placed = measured.place_label(&moved).unwrap();
        assert_eq!(
            placed.line_box,
            LayoutRect {
                x: left,
                y: 50.,
                width: 60.,
                height: 100.
            }
        );
        assert_eq!(
            placed.ink_bounds,
            Some(LayoutRect {
                x: left + 2.,
                y: 70.,
                width: 40.,
                height: 70.
            })
        );
        assert_eq!(placed.alphabetic_anchor, [100., 140.]);
        close(
            placed.line_box.y + measured.metrics().baseline_from_top,
            moved.y,
        );
    }
    let moved = Label {
        x: 240.,
        y: 200.,
        anchor: TextAnchor::End,
        ..original
    };
    assert_eq!(request(&moved), current);
    assert_eq!(measured.place_label(&moved).unwrap().line_box.x, 180.);
}

#[test]
fn collision_moves_original_line_box_by_actual_ink_delta() {
    let placed = request(&label("B"))
        .measured(supplied_metrics())
        .unwrap()
        .place_label(&Label {
            anchor: TextAnchor::End,
            ..label("B")
        })
        .unwrap();
    let accepted = placed
        .with_placed_ink_bounds(LayoutRect {
            x: 72.,
            y: 80.,
            width: 40.,
            height: 70.,
        })
        .unwrap();
    assert_eq!(
        accepted.line_box,
        LayoutRect {
            x: 70.,
            y: 60.,
            width: 60.,
            height: 100.
        }
    );
    assert_eq!(accepted.alphabetic_anchor, [130., 150.]);
    assert_eq!(accepted.ink_bounds.unwrap().x, 72.);
    assert!(
        placed
            .with_placed_ink_bounds(LayoutRect {
                width: 41.,
                ..placed.ink_bounds.unwrap()
            })
            .is_err()
    );
    assert_eq!(placed.line_box.x, 40.);
    assert_eq!(placed.line_box.height, 100.);
}

#[test]
fn actual_line_box_quantization_and_negative_leading_are_not_replaced_by_em_guesses() {
    let requested = SemanticTextProfile {
        line_height: 1.2,
        revision: 7,
    }
    .request(&label("B"), &ResolvedFont::default())
    .unwrap();
    assert_eq!(requested.line_height, 120.);
    let measured = requested
        .measured(SemanticTextMetrics {
            line_height: 119.984375,
            baseline_from_top: 97.125,
            ..supplied_metrics()
        })
        .unwrap();
    let placed = measured.place_label(&label("B")).unwrap();
    assert_eq!(placed.line_box.height, 119.984375);
    assert_eq!(placed.line_box.y, 42.875);
    for baseline in [72., -12.] {
        let measured = requested
            .measured(SemanticTextMetrics {
                line_height: 50.,
                baseline_from_top: baseline,
                ..supplied_metrics()
            })
            .unwrap();
        assert_eq!(
            measured.place_label(&label("B")).unwrap().line_box.y,
            140. - baseline
        );
    }
}

#[test]
fn whitespace_and_empty_runs_retain_line_identity_without_collision_ink() {
    for (text, advance, height, baseline) in [(" ", 30., 100., 90.), ("", 0., 0., 0.)] {
        let source = label(text);
        let measured = request(&source)
            .measured(SemanticTextMetrics {
                advance,
                line_height: height,
                baseline_from_top: baseline,
                ink_bounds: None,
            })
            .unwrap();
        let placed = measured.place_label(&source).unwrap();
        assert_eq!(placed.line_box.width, advance);
        assert_eq!(placed.line_box.height, height);
        assert_eq!(placed.ink_bounds, None);
        let moved = placed.translated(4., 9.).unwrap();
        assert_eq!(moved.alphabetic_anchor, [104., 149.]);
        assert!(moved.with_placed_ink_bounds(LayoutRect::default()).is_err());
    }
}

#[test]
fn invalid_measurements_and_profiles_never_construct_an_accepted_capability() {
    for line_height in [f64::NAN, f64::INFINITY, 0.499, 4.001] {
        assert!(
            SemanticTextProfile {
                line_height,
                ..Default::default()
            }
            .request(&label("B"), &ResolvedFont::default())
            .is_err()
        );
    }
    for size in [0., 256.01, f64::NAN, f64::INFINITY] {
        assert!(
            SemanticTextProfile::default()
                .request(&Label { size, ..label("B") }, &ResolvedFont::default())
                .is_err()
        );
    }
    assert!(
        SemanticTextProfile::default()
            .request(&label(&"x".repeat(1025)), &ResolvedFont::default())
            .is_err()
    );
    let current = request(&label("B"));
    for metrics in [
        SemanticTextMetrics {
            advance: -1.,
            ..supplied_metrics()
        },
        SemanticTextMetrics {
            advance: f64::NAN,
            ..supplied_metrics()
        },
        SemanticTextMetrics {
            line_height: -1.,
            ..supplied_metrics()
        },
        SemanticTextMetrics {
            line_height: f64::INFINITY,
            ..supplied_metrics()
        },
        SemanticTextMetrics {
            baseline_from_top: 1_000_001.,
            ..supplied_metrics()
        },
        SemanticTextMetrics {
            baseline_from_top: f64::NAN,
            ..supplied_metrics()
        },
        SemanticTextMetrics {
            ink_bounds: Some(LayoutRect {
                x: 999_999.,
                width: 2.,
                ..LayoutRect::default()
            }),
            ..supplied_metrics()
        },
    ] {
        assert!(current.measured(metrics).is_err());
    }
    let measured = current.measured(supplied_metrics()).unwrap();
    for invalid in [
        Label {
            rotation: 45.,
            ..label("B")
        },
        Label {
            x: f64::INFINITY,
            ..label("B")
        },
        Label {
            text: "A".into(),
            ..label("B")
        },
        Label {
            size: 99.,
            ..label("B")
        },
    ] {
        assert!(measured.place_label(&invalid).is_err());
    }
    let placed = measured.place_label(&label("B")).unwrap();
    assert!(placed.translated(f64::NAN, 0.).is_err());
    assert!(placed.translated(1_000_000., 0.).is_err());
    assert_eq!(measured.request(), &current);
}

#[cfg(feature = "raster")]
mod font_ink {
    use kartis::render::raster::{FaceDescriptor, FontRegistry, MissingGlyphPolicy};

    use super::*;

    fn registry() -> (FontRegistry, ResolvedFont) {
        let mut fonts = FontRegistry::new();
        let id = fonts
            .register(include_bytes!("fonts/ABeeZee-Regular.ttf").as_slice(), 0)
            .unwrap();
        let font = ResolvedFont {
            family: vec![FontFamily::Named("ABeeZee".into())],
            weight: 400.,
        };
        fonts
            .bind_face(
                id,
                FaceDescriptor {
                    family: font.family[0].clone(),
                    weight: font.weight,
                },
            )
            .unwrap();
        (fonts, font)
    }

    #[test]
    fn actual_negative_bearing_and_descender_ink_stays_relative_to_alphabetic_anchor() {
        let (fonts, font) = registry();
        let actual = fonts
            .measure_layout_with_font("j", 100., &font, MissingGlyphPolicy::Error)
            .unwrap();
        let ink = actual.ink_bounds.unwrap();
        // Independent FontTools glyf/hmtx readback: unitsPerEm=1000, advance
        // 261, xMin=-85, yMin=-232, xMax=197, yMax=735.
        close(actual.advance, 26.1);
        close(ink.x, -8.5);
        close(ink.y, -73.5);
        close(ink.width, 28.2);
        close(ink.height, 96.7);
        let source = label("j");
        let request = SemanticTextProfile::default()
            .request(&source, &font)
            .unwrap();
        let measured = request
            .measured(SemanticTextMetrics {
                advance: actual.advance,
                // Line metrics are independent host inputs in this pure test.
                line_height: 100.,
                baseline_from_top: 90.,
                ink_bounds: Some(LayoutRect {
                    x: ink.x,
                    y: ink.y,
                    width: ink.width,
                    height: ink.height,
                }),
            })
            .unwrap();
        for anchor in [TextAnchor::Start, TextAnchor::Middle, TextAnchor::End] {
            let placed = measured
                .place_label(&Label {
                    anchor,
                    ..source.clone()
                })
                .unwrap();
            let ink = placed.ink_bounds.unwrap();
            close(ink.x, placed.line_box.x - 8.5);
            close(ink.y + ink.height, source.y + 23.2);
            close(
                placed.line_box.y + measured.metrics().baseline_from_top,
                source.y,
            );
        }
    }

    #[test]
    fn real_space_has_advance_without_ink_and_combining_mark_has_ink_without_advance() {
        let (fonts, font) = registry();
        for text in [" ", "\u{301}"] {
            let actual = fonts
                .measure_layout_with_font(text, 100., &font, MissingGlyphPolicy::Error)
                .unwrap();
            if text == " " {
                close(actual.advance, 30.);
                assert!(actual.ink_bounds.is_none());
            } else {
                close(actual.advance, 0.);
                assert!(actual.ink_bounds.is_some());
            }
            let source = label(text);
            let request = SemanticTextProfile::default()
                .request(&source, &font)
                .unwrap();
            let measured = request
                .measured(SemanticTextMetrics {
                    advance: actual.advance,
                    line_height: 100.,
                    baseline_from_top: 90.,
                    ink_bounds: actual.ink_bounds.map(|ink| LayoutRect {
                        x: ink.x,
                        y: ink.y,
                        width: ink.width,
                        height: ink.height,
                    }),
                })
                .unwrap();
            let placed = measured.place_label(&source).unwrap();
            assert_eq!(placed.line_box.width, actual.advance);
            assert_eq!(placed.ink_bounds.is_some(), text != " ");
        }
    }
}
