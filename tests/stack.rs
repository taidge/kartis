use kartis::BarSeries;
use kartis::stack::*;

fn sample() -> Vec<BarSeries> {
    vec![
        BarSeries::new("a", vec![Some(1.), Some(2.), Some(1.)]),
        BarSeries::new("b", vec![Some(3.), Some(1.), Some(4.)]),
    ]
}
#[test]
fn zero_nullable_alignment_and_original_metadata() {
    let series = vec![
        BarSeries::new("a", vec![Some(2.), None]),
        BarSeries::new("b", vec![Some(3.), Some(4.)]),
    ];
    let stacked = stack(&series, &StackOptions::default()).unwrap();
    assert_eq!(stacked.order, vec![0, 1]);
    assert_eq!(stacked.domain, [0., 5.]);
    assert_eq!(stacked.layers[0].points[1].value, None);
    assert_eq!(stacked.layers[1].points[0].y0, 2.);
    assert_eq!(stacked.layers[1].points[0].y1, 5.);
    let reversed = stack(
        &series,
        &StackOptions {
            order: StackOrder::Reverse,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(reversed.order, vec![1, 0]);
    assert_eq!(reversed.layers[0].original_index, 0);
    assert_eq!(reversed.layers[0].stack_index, 1);
    assert_eq!(reversed.layers[0].points[0].y0, 3.);
}
#[test]
fn diverging_normalization_silhouette_and_overflow() {
    let signed = vec![
        BarSeries::new("a", vec![Some(-2.), Some(3.)]),
        BarSeries::new("b", vec![Some(-4.), Some(-1.)]),
        BarSeries::new("c", vec![Some(5.), Some(2.)]),
    ];
    let d = stack(
        &signed,
        &StackOptions {
            offset: StackOffset::Diverging,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(d.domain, [-6., 5.]);
    assert_eq!(
        (d.layers[1].points[0].y0, d.layers[1].points[0].y1),
        (-6., -2.)
    );
    assert_eq!(
        (d.layers[2].points[1].y0, d.layers[2].points[1].y1),
        (3., 5.)
    );
    assert!(
        stack(
            &signed,
            &StackOptions {
                offset: StackOffset::Wiggle,
                ..Default::default()
            }
        )
        .is_err()
    );
    let e = stack(
        &sample(),
        &StackOptions {
            offset: StackOffset::Expand,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(e.domain, [0., 1.]);
    for p in &e.layers[1].points {
        assert!((p.y1 - 1.).abs() < 1e-12);
    }
    let s = stack(
        &sample(),
        &StackOptions {
            offset: StackOffset::Silhouette,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(s.baseline, vec![-2., -1.5, -2.5]);
    assert!(
        stack(
            &[
                BarSeries::new("a", vec![Some(f64::MAX)]),
                BarSeries::new("b", vec![Some(f64::MAX)])
            ],
            &StackOptions::default()
        )
        .is_err()
    );
}
#[test]
fn wiggle_matches_weighted_derivative_recurrence() {
    // Column 1 derivative: (0.5*2 + 0*1)/3 = 1/3.
    // Column 2 derivative: (-0.5*1 + 0.5*4)/5 = 0.3.
    let w = stack(
        &sample(),
        &StackOptions {
            offset: StackOffset::Wiggle,
            ..Default::default()
        },
    )
    .unwrap();
    assert!((w.baseline[0] - 0.).abs() < 1e-12);
    assert!((w.baseline[1] + 1. / 3.).abs() < 1e-12);
    assert!((w.baseline[2] + 1. / 3. + 0.3).abs() < 1e-12);
    for col in 0..3 {
        for (index, layer) in w.layers.iter().enumerate() {
            let p = &layer.points[col];
            assert!((p.y1 - p.y0 - sample()[index].values[col].unwrap()).abs() < 1e-12);
        }
    }
    let huge = vec![
        BarSeries::new("one", vec![Some(1e200), Some(2e200)]),
        BarSeries::new("two", vec![Some(2e200), Some(1e200)]),
    ];
    assert!(
        stack(
            &huge,
            &StackOptions {
                offset: StackOffset::Wiggle,
                ..Default::default()
            }
        )
        .is_ok(),
        "normalized wiggle weights must not overflow value products"
    );
}
#[test]
fn orders_zero_columns_and_invalid_inputs() {
    let s = sample();
    assert_eq!(
        stack(
            &s,
            &StackOptions {
                order: StackOrder::Ascending,
                ..Default::default()
            }
        )
        .unwrap()
        .order,
        vec![0, 1]
    );
    assert_eq!(
        stack(
            &s,
            &StackOptions {
                order: StackOrder::Descending,
                ..Default::default()
            }
        )
        .unwrap()
        .order,
        vec![1, 0]
    );
    let peaks = vec![
        BarSeries::new("early", vec![Some(9.), Some(0.), Some(0.)]),
        BarSeries::new("middle", vec![Some(0.), Some(4.), Some(0.)]),
        BarSeries::new("late", vec![Some(0.), Some(0.), Some(8.)]),
    ];
    assert_eq!(
        stack(
            &peaks,
            &StackOptions {
                order: StackOrder::InsideOut,
                ..Default::default()
            }
        )
        .unwrap()
        .order,
        vec![0, 1, 2]
    );
    assert_eq!(
        stack(&[], &StackOptions::default()).unwrap().domain,
        [0., 0.]
    );
    assert!(
        stack(
            &[BarSeries::new("empty", vec![])],
            &StackOptions {
                offset: StackOffset::Wiggle,
                ..Default::default()
            }
        )
        .is_ok()
    );
    assert!(
        stack(
            &[BarSeries::new("a", vec![None]), BarSeries::new("b", vec![])],
            &StackOptions::default()
        )
        .is_err()
    );
    assert!(
        stack(
            &[BarSeries::new("a", vec![Some(f64::NAN)])],
            &StackOptions::default()
        )
        .is_err()
    );
}

#[test]
fn every_order_and_offset_preserves_aligned_thickness_and_bounds() {
    let data = vec![
        BarSeries::new("first", vec![Some(3.), Some(0.), Some(2.), None]),
        BarSeries::new("second", vec![Some(1.), Some(0.), Some(7.), Some(4.)]),
        BarSeries::new("third", vec![Some(2.), None, Some(1.), Some(1.)]),
    ];
    for order in [
        StackOrder::None,
        StackOrder::Reverse,
        StackOrder::Ascending,
        StackOrder::Descending,
        StackOrder::InsideOut,
    ] {
        for offset in [
            StackOffset::Zero,
            StackOffset::Diverging,
            StackOffset::Expand,
            StackOffset::Silhouette,
            StackOffset::Wiggle,
        ] {
            let result = stack(&data, &StackOptions { order, offset }).unwrap();
            for (index, layer) in result.layers.iter().enumerate() {
                assert_eq!(layer.original_index, index);
                assert_eq!(result.order[layer.stack_index], index);
                for p in &layer.points {
                    let total = data
                        .iter()
                        .map(|s| s.values[p.column].unwrap_or(0.))
                        .sum::<f64>();
                    let expected = if offset == StackOffset::Expand && total > 0. {
                        p.value.unwrap_or(0.) / total
                    } else {
                        p.value.unwrap_or(0.)
                    };
                    assert!((p.y1 - p.y0 - expected).abs() < 1e-10);
                    assert!(p.y0 >= result.domain[0] - 1e-10 && p.y1 <= result.domain[1] + 1e-10);
                }
            }
        }
    }
}
