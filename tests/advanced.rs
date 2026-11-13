use kartis::*;

fn config() -> ChartConfig {
    ChartConfig::default()
}
fn matrix() -> (Vec<String>, Vec<BarSeries>) {
    (
        vec!["A".into(), "B".into(), "zero".into()],
        vec![
            BarSeries::new("one", vec![Some(1.), Some(3.), Some(0.)]),
            BarSeries::new("two", vec![Some(3.), Some(1.), None]),
        ],
    )
}

#[test]
fn marimekko_preserves_area_ratios_and_transposes() {
    let c = config();
    let (categories, data) = matrix();
    for horizontal in [false, true] {
        let scene = marimekko(
            &c,
            &categories,
            &data,
            &MarimekkoOptions {
                horizontal,
                ..MarimekkoOptions::default()
            },
        )
        .unwrap();
        assert_eq!(scene.marks.len(), 4);
        for mark in scene.marks {
            if let Geometry::Rect { width, height, .. } = mark.geometry {
                assert!(
                    (width * height / (c.inner_width() * c.inner_height()) - mark.value / 8.).abs()
                        < 1e-12
                );
            } else {
                panic!("expected rectangle");
            }
        }
    }
    let variable = vec![BarSeries::new("one", vec![Some(1.), Some(3.)])];
    let scene = marimekko(
        &c,
        &categories[..2],
        &variable,
        &MarimekkoOptions::default(),
    )
    .unwrap();
    let widths = scene
        .marks
        .iter()
        .map(|m| match m.geometry {
            Geometry::Rect { width, .. } => width,
            _ => 0.,
        })
        .collect::<Vec<_>>();
    assert!((widths[1] / widths[0] - 3.).abs() < 1e-12);
    assert!(
        marimekko(
            &c,
            &["A".into()],
            &[BarSeries::new("bad", vec![Some(-1.)])],
            &MarimekkoOptions::default()
        )
        .is_err()
    );
}

#[test]
fn swarm_is_deterministic_collision_free_and_keeps_exact_values() {
    let c = config();
    let mut data = (0..24)
        .map(|i| SwarmDatum {
            id: format!("p{i:02}"),
            group: "first".into(),
            value: 10.,
            radius: 2. + (i % 3) as f64,
        })
        .collect::<Vec<_>>();
    for horizontal in [true, false] {
        let options = SwarmOptions {
            horizontal,
            ..SwarmOptions::default()
        };
        let scene = swarmplot(&c, &data, &options).unwrap();
        scene.validate().unwrap();
        for (i, a) in scene.marks.iter().enumerate() {
            for b in scene.marks.iter().skip(i + 1) {
                match (&a.geometry, &b.geometry) {
                    (
                        Geometry::Circle {
                            x: ax,
                            y: ay,
                            radius: ar,
                        },
                        Geometry::Circle {
                            x: bx,
                            y: by,
                            radius: br,
                        },
                    ) => {
                        assert!((ax - bx).hypot(ay - by) + 1e-8 >= ar + br + options.gap);
                        if horizontal {
                            assert_eq!(ax, bx);
                        } else {
                            assert_eq!(ay, by);
                        }
                    }
                    _ => panic!("expected circles"),
                }
            }
        }
        data.reverse();
        assert_eq!(scene, swarmplot(&c, &data, &options).unwrap());
    }
    let many = (0..100)
        .map(|i| SwarmDatum::new(format!("p{i}"), "crowded", 10.))
        .collect::<Vec<_>>();
    assert!(
        swarmplot(&c, &many, &SwarmOptions::default()).is_err(),
        "overflow must reject instead of introducing collisions"
    );
    assert!(
        swarmplot(
            &c,
            &[
                SwarmDatum::new("same", "g", 0.),
                SwarmDatum::new("same", "g", 1.)
            ],
            &SwarmOptions::default()
        )
        .is_err()
    );
}

#[test]
fn swarm_mixed_values_and_groups_stay_within_plot() {
    let c = config();
    let data = (0..80)
        .map(|i| SwarmDatum::new(format!("d{i}"), format!("group{}", i % 4), (i % 7) as f64))
        .collect::<Vec<_>>();
    let o = SwarmOptions::default();
    let scene = swarmplot(&c, &data, &o).unwrap();
    for a in &scene.marks {
        if let Geometry::Circle { x, y, radius } = a.geometry {
            assert!(
                x - radius >= c.margin.left - 1e-9 && x + radius <= c.width - c.margin.right + 1e-9
            );
            assert!(
                y - radius >= c.margin.top - 1e-9
                    && y + radius <= c.height - c.margin.bottom + 1e-9
            );
        }
        for b in &scene.marks {
            if a.id == b.id {
                continue;
            }
            if let (
                Geometry::Circle {
                    x: ax,
                    y: ay,
                    radius: ar,
                },
                Geometry::Circle {
                    x: bx,
                    y: by,
                    radius: br,
                },
            ) = (&a.geometry, &b.geometry)
            {
                assert!((ax - bx).hypot(ay - by) + 1e-8 >= ar + br);
            }
        }
    }
}

#[test]
fn polar_bar_layouts_holes_and_area_scaling() {
    let c = config();
    let (categories, data) = matrix();
    for layout in [BarLayout::Grouped, BarLayout::Stacked, BarLayout::Percent] {
        for area_scale in [false, true] {
            let scene = polar_bar(
                &c,
                &categories,
                &data,
                &PolarBarOptions {
                    layout,
                    area_scale,
                    inner_radius: 0.4,
                    ..PolarBarOptions::default()
                },
            )
            .unwrap();
            assert_eq!(
                scene
                    .marks
                    .iter()
                    .filter(|m| m.id.starts_with("polar-bar-"))
                    .count(),
                4
            );
            assert!(
                scene
                    .marks
                    .iter()
                    .all(|m| m.geometry.commands().len() <= 256)
            );
            let cx = c.margin.left + c.inner_width() / 2.;
            let cy = c.margin.top + c.inner_height() / 2.;
            assert!(
                scene.hit_test(cx, cy).is_none(),
                "annular hole must remain empty"
            );
        }
    }
    assert!(
        polar_bar(
            &c,
            &categories,
            &data,
            &PolarBarOptions {
                pad_angle: 4.,
                ..PolarBarOptions::default()
            }
        )
        .is_err()
    );
    let scene = polar_bar(
        &c,
        &["all".into()],
        &[BarSeries::new("one", vec![Some(1.)])],
        &PolarBarOptions {
            inner_radius: 0.,
            pad_angle: 0.,
            ..PolarBarOptions::default()
        },
    )
    .unwrap();
    let cx = c.margin.left + c.inner_width() / 2.;
    let cy = c.margin.top + c.inner_height() / 2.;
    assert_eq!(scene.hit_test(cx + 1., cy).unwrap().series, "one");
}
