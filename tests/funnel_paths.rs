use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 420.,
        height: 300.,
        margin: Margin {
            left: 31.,
            right: 23.,
            top: 17.,
            bottom: 29.,
        },
        ..Default::default()
    }
}
fn source() -> Vec<FunnelDatum> {
    vec![
        FunnelDatum::new("a", 10.),
        FunnelDatum::new(0., 4.),
        FunnelDatum::new("zero", -0.),
    ]
}

#[test]
fn exact_default_body_and_open_border_paths_across_all_profiles() {
    let source = source();
    for direction in [FunnelDirection::Horizontal, FunnelDirection::Vertical] {
        for interpolation in [FunnelInterpolation::Linear, FunnelInterpolation::Smooth] {
            for shape_blending in [0., 0.66, 1.] {
                for current in [false, true] {
                    let o = FunnelLayoutOptions {
                        direction,
                        interpolation,
                        shape_blending,
                        spacing: 5.3,
                        current: current.then(|| "a".into()),
                        current_size_extension: 8.2,
                        ..Default::default()
                    };
                    let layout = funnel_layout(&config(), &source, &o).unwrap();
                    let original = layout.clone();
                    for part in &layout.parts {
                        let body = funnel_part_geometry(part, interpolation).unwrap();
                        assert_eq!(
                            body.as_ref(),
                            layout
                                .scene
                                .marks
                                .iter()
                                .find(|m| m.id == part.mark_id)
                                .map(|m| &m.geometry)
                        );
                        let border_id =
                            format!("funnel:border:{}", part.datum.id.identity().unwrap());
                        assert_eq!(
                            funnel_part_border_geometry(part, interpolation).unwrap(),
                            layout
                                .scene
                                .marks
                                .iter()
                                .find(|m| m.id == border_id)
                                .unwrap()
                                .geometry
                        );
                        let Geometry::Path(commands) =
                            funnel_part_border_geometry(part, interpolation).unwrap()
                        else {
                            panic!("profile Path")
                        };
                        assert_eq!(
                            commands.iter().filter(|c| matches!(c, C::Move(_))).count(),
                            2
                        );
                        assert!(!commands.iter().any(|c| matches!(c, C::Close)));
                    }
                    assert_eq!(layout, original);
                }
            }
        }
    }
}

#[test]
fn collapsed_profile_is_not_a_body_and_border_paint_eligibility_is_separate() {
    let source = source();
    for interpolation in [FunnelInterpolation::Linear, FunnelInterpolation::Smooth] {
        let l = funnel_layout(
            &config(),
            &source,
            &FunnelLayoutOptions {
                interpolation,
                size: FunnelSize::OrdinalPixels(vec![0.]),
                border_width: 0.,
                ..Default::default()
            },
        )
        .unwrap();
        for part in &l.parts {
            assert_eq!(funnel_part_geometry(part, interpolation).unwrap(), None);
            assert!(matches!(
                funnel_part_border_geometry(part, interpolation).unwrap(),
                Geometry::Path(_)
            ));
        }
        assert!(
            l.scene
                .marks
                .iter()
                .all(|m| m.id.starts_with("funnel:separator:"))
        );
    }
}

#[test]
fn original_profile_geometry_is_distinct_from_actual_offset_custom_stage() {
    let source = source();
    for interpolation in [FunnelInterpolation::Linear, FunnelInterpolation::Smooth] {
        let o = FunnelLayoutOptions {
            interpolation,
            ..Default::default()
        };
        let original = funnel_layout(&config(), &source, &o).unwrap();
        let custom = funnel_layout_with_stage_factory(&config(), &source, &o, &mut |c| {
            let [x, y] = c.part.center;
            Ok(c.inherit(Geometry::Path(vec![
                C::Move(P::new(x + 30., y - 5.)),
                C::Line(P::new(x + 40., y - 5.)),
                C::Line(P::new(x + 40., y + 5.)),
                C::Line(P::new(x + 30., y + 5.)),
                C::Close,
            ])))
        })
        .unwrap();
        for (a, b) in original.parts.iter().zip(&custom.parts) {
            assert_eq!(
                funnel_part_geometry(a, interpolation).unwrap(),
                funnel_part_geometry(b, interpolation).unwrap()
            );
            assert_eq!(
                funnel_part_border_geometry(a, interpolation).unwrap(),
                funnel_part_border_geometry(b, interpolation).unwrap()
            );
            if let Some(actual) = custom.scene.marks.iter().find(|m| m.id == b.mark_id) {
                assert_ne!(
                    funnel_part_geometry(b, interpolation).unwrap().as_ref(),
                    Some(&actual.geometry)
                );
            }
        }
    }
}

#[test]
fn invalid_public_profile_coordinates_reject_before_any_geometry_and_bound_is_inclusive() {
    let layout = funnel_layout(&config(), &source(), &Default::default()).unwrap();
    for slot in 0..4 {
        for axis in 0..2 {
            for before in [false, true] {
                for value in [
                    f64::NAN,
                    f64::INFINITY,
                    f64::NEG_INFINITY,
                    1_000_000.000_1,
                    -1_000_000.000_1,
                ] {
                    let mut part = layout.parts[0].clone();
                    if before {
                        part.profile[slot].before[axis] = value;
                    } else {
                        part.profile[slot].after[axis] = value;
                    }
                    for interpolation in [FunnelInterpolation::Linear, FunnelInterpolation::Smooth]
                    {
                        assert!(funnel_part_geometry(&part, interpolation).is_err());
                        assert!(funnel_part_border_geometry(&part, interpolation).is_err());
                    }
                }
            }
        }
    }
    let mut part = layout.parts[0].clone();
    for p in &mut part.profile {
        p.before = [-1_000_000., 1_000_000.];
        p.after = p.before;
    }
    assert_eq!(
        funnel_part_geometry(&part, FunnelInterpolation::Smooth).unwrap(),
        None
    );
    assert!(funnel_part_border_geometry(&part, FunnelInterpolation::Smooth).is_ok());
    // The same profile is deliberately rendered with the explicitly requested
    // interpolation, not a guessed setting retained on the public part.
    assert_ne!(
        funnel_part_geometry(&layout.parts[0], FunnelInterpolation::Linear).unwrap(),
        funnel_part_geometry(&layout.parts[0], FunnelInterpolation::Smooth).unwrap()
    );
}
