use kartis::scales::LinearScale;

fn close(actual: f64, expected: f64) {
    let tolerance = (actual.abs().max(expected.abs()) * 8. * f64::EPSILON).max(f64::from_bits(1));
    assert!(
        (actual - expected).abs() <= tolerance,
        "{actual:e} != {expected:e}"
    );
}

#[test]
fn nice_matches_independent_d3_domains_including_reversed_and_fractional_steps() {
    let mut queries = 0;
    for line in include_str!("fixtures/linear-nice-d3.csv").lines().skip(2) {
        let fields = line.split('|').collect::<Vec<_>>();
        let domain = [fields[0].parse().unwrap(), fields[1].parse().unwrap()];
        let count = fields[2].parse().unwrap();
        let expected = [fields[3].parse().unwrap(), fields[4].parse().unwrap()];
        let scale = LinearScale::new(domain, [120., -20.])
            .unwrap()
            .clamped(true);
        let result = scale.nice(count);
        close(result.domain[0], expected[0]);
        close(result.domain[1], expected[1]);
        assert_eq!(result.range, scale.range);
        assert!(result.clamp);
        queries += 1;
    }
    assert_eq!(queries, 644);
}

#[test]
fn nice_converges_after_expansion_changes_tick_factor() {
    for (domain, expected) in [
        ([0.01, 0.71], [0., 0.8]),
        ([0.71, 0.01], [0.8, 0.]),
        ([0., 14.1], [0., 16.]),
    ] {
        let nice = LinearScale::new(domain, [0., 1.]).unwrap().nice(10);
        assert_eq!(nice.domain, expected);
        assert_eq!(nice.nice(10), nice);
        assert!(nice.domain[0].min(nice.domain[1]) <= domain[0].min(domain[1]));
        assert!(nice.domain[0].max(nice.domain[1]) >= domain[0].max(domain[1]));
    }
}

#[test]
fn fractional_repeated_rounding_matches_d3_without_assuming_exact_idempotence() {
    let mut scale = LinearScale::new([0.99, 1.1], [0., 1.]).unwrap();
    for expected in [1.11, 1.12, 1.13, 1.13] {
        scale = scale.nice(10);
        assert_eq!(scale.domain, [0.99, expected]);
    }
}

#[test]
fn nice_retains_unrepresentable_extrema_and_bounded_count_policy() {
    for domain in [[0., f64::MAX], [0., f64::from_bits(1)], [7., 7.]] {
        let scale = LinearScale::new(domain, [0., 1.]).unwrap();
        assert_eq!(scale.nice(10).domain, domain);
        assert_eq!(scale.nice(0), scale);
    }
    let scale = LinearScale::new([-1.2345, 5.6789], [0., 1.]).unwrap();
    assert_eq!(scale.nice(usize::MAX), scale.nice(1000));
}
