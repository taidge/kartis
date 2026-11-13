use kartis::*;

fn values(s: &str) -> Vec<f64> {
    s.split(',').map(|v| v.parse().unwrap()).collect()
}
fn near(a: f64, b: f64) {
    assert!(
        (a - b).abs() <= 2e-11 * a.abs().max(b.abs()).max(1.),
        "{a} != {b}"
    );
}
fn profile(row: &[&str]) -> PiecewiseScaleOptions {
    let parameter = row[1].parse().unwrap();
    PiecewiseScaleOptions {
        domain: values(row[2]),
        range: values(row[3]),
        continuous: ContinuousScaleOptions {
            kind: match row[0] {
                "linear" => ScaleKind::Linear,
                "log" => ScaleKind::Log,
                "symlog" => ScaleKind::SymLog {
                    constant: parameter,
                },
                "pow" => ScaleKind::Pow {
                    exponent: parameter,
                },
                "sqrt" => ScaleKind::Sqrt,
                "utc" => ScaleKind::Utc,
                _ => unreachable!(),
            },
            log_base: if row[0] == "log" { parameter } else { 10. },
            nice: if row[4] == "-1" {
                ScaleNice::Off
            } else {
                ScaleNice::Count(3)
            },
            reverse: row[5] == "1",
            round: row[6] == "1",
            ..Default::default()
        },
    }
}

#[test]
fn full_breakpoint_viewports_match_original_d3_zoom_rescale_in_both_directions() {
    let frame = ChartConfig {
        width: 420.5,
        height: 300.5,
        margin: Margin {
            left: 60.,
            right: 20.,
            top: 20.,
            bottom: 40.,
        },
        ..Default::default()
    };
    let mut count = 0;
    for line in include_str!("fixtures/piecewise-viewport-d3.csv")
        .lines()
        .skip(1)
    {
        let row: Vec<_> = line.split('|').collect();
        let vertical = row[7] == "y";
        let start: f64 = row[8].parse().unwrap();
        let span: f64 = row[9].parse().unwrap();
        let p = profile(&row);
        // Empty source allows this test to exercise projection metadata for
        // all branches without inventing invalid observations on a log axis.
        let scales = PiecewiseLineScaleOptions {
            x: (!vertical).then(|| p.clone()),
            y: vertical.then(|| p.clone()),
            ..Default::default()
        };
        let layout = line_layout_with_piecewise_scale_options(
            &frame,
            &[],
            &LineOptions::default(),
            &LineStyleOptions::default(),
            &scales,
        )
        .unwrap_or_else(|e| panic!("{line}: {e}"));
        let displayed = if vertical {
            &layout.y_scale
        } else {
            &layout.x_scale
        };
        let actual = displayed.display_domain_stops();
        let expected = values(row[10]);
        assert_eq!(actual.len(), expected.len());
        for (&a, b) in actual.iter().zip(expected) {
            near(a, b);
        }
        let mut controller = layout
            .viewport_controller(ViewportLimits::default())
            .unwrap();
        let original = controller.clone();
        let plot = controller.plot();
        let k = 1. / span;
        let request = ViewportRequest::TransformXY {
            transform: ViewportTransform {
                scale_x: if vertical { 1. } else { k },
                scale_y: if vertical { k } else { 1. },
                translate_x: if vertical {
                    0.
                } else {
                    -start * plot.width * k
                },
                translate_y: if vertical {
                    -start * plot.height * k
                } else {
                    0.
                },
            },
            axes: if vertical {
                ViewportSelection::Y
            } else {
                ViewportSelection::X
            },
        };
        let preview = controller.resolve_piecewise(request).unwrap();
        assert_eq!(controller, original);
        controller
            .apply(request)
            .unwrap_or_else(|e| panic!("{line}: {e}"));
        assert_eq!(controller.piecewise_domains().unwrap(), preview);
        let stops = if vertical {
            preview.left.as_ref().unwrap()
        } else {
            preview.x.as_ref().unwrap()
        };
        let expected = values(row[11]);
        assert_eq!(stops.len(), expected.len());
        for (&a, b) in stops.iter().zip(expected) {
            near(a, b);
        }
        let next = layout.viewport_scale_options(&preview).unwrap();
        let accepted = controller.domains();
        let reflow = line_layout_with_piecewise_scale_options(
            &frame,
            &[],
            &LineOptions {
                x_domain: Some(accepted.x),
                y_domain: accepted.left,
                ..Default::default()
            },
            &LineStyleOptions::default(),
            &next,
        )
        .unwrap_or_else(|e| panic!("{line}: {e}"));
        let new_scale = if vertical {
            &reflow.y_scale
        } else {
            &reflow.x_scale
        };
        assert_eq!(new_scale.display_domain_stops(), stops);
        let accepted_stops = controller.piecewise_domains().unwrap();
        controller
            .resize(ClipRect {
                x: 90.,
                y: 40.,
                width: 500.,
                height: 100.,
            })
            .unwrap();
        assert_eq!(controller.piecewise_domains().unwrap(), accepted_stops);
        controller.apply(ViewportRequest::Reset).unwrap();
        assert_eq!(
            controller.piecewise_domains().unwrap(),
            original.piecewise_domains().unwrap()
        );
        count += 1;
    }
    assert_eq!(count, 432);
}
