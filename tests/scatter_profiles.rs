//! Independent actual-Nivo fixtures plus geometric/Surface/CPU contracts.
use glory::render::{GraphicStroke, PathCommand as C, StrokeCap};
use kartis::{surface::*, *};
use std::collections::BTreeMap;
fn frame(width: f64, height: f64) -> ChartConfig {
    ChartConfig {
        width: width + 60.,
        height: height + 60.,
        margin: Margin {
            left: 30.,
            right: 30.,
            top: 30.,
            bottom: 30.,
        },
        ..Default::default()
    }
}
fn raw(kind: &str, value: &str) -> CategoryKey {
    if kind == "s" {
        CategoryKey::String(value.into())
    } else {
        CategoryKey::Number(value.parse().unwrap())
    }
}
fn linear(domain: [f64; 2]) -> ScatterAxisScale {
    ScatterAxisScale::Continuous {
        options: ContinuousScaleOptions::default(),
        min: ScatterBound::Fixed(domain[0]),
        max: ScatterBound::Fixed(domain[1]),
    }
}
fn bare() -> ScatterLayoutOptions {
    ScatterLayoutOptions {
        scales: ScatterScaleOptions {
            x: linear([0., 10.]),
            y: linear([0., 10.]),
        },
        axes: ScatterAxes {
            top: None,
            right: None,
            bottom: None,
            left: None,
        },
        ..Default::default()
    }
}
fn fixture(
    profile: usize,
    width: f64,
    height: f64,
) -> (ChartConfig, Vec<ScatterSeries>, ScatterLayoutOptions) {
    let mut data = Vec::<ScatterSeries>::new();
    for line in include_str!("fixtures/scatter-nivo.csv").lines().skip(2) {
        let r = line.split('|').collect::<Vec<_>>();
        if r[0].parse::<usize>().unwrap() != profile || r[2] != "400" {
            continue;
        }
        let id = r[4].split_once(':').unwrap().1;
        let mut point = ScatterDatum::new(id, raw(r[8], r[9]), raw(r[10], r[11]));
        point.size_value = Some(r[12].parse().unwrap());
        if data.last().is_none_or(|s| s.id != r[7]) {
            data.push(ScatterSeries::new(r[7], vec![]));
        }
        data.last_mut().unwrap().data.push(point);
    }
    let mut options = ScatterLayoutOptions::default();
    if !matches!(profile, 0 | 4 | 6 | 7) {
        options.scales = bare().scales;
    }
    if profile == 4 {
        for axis in [&mut options.scales.x, &mut options.scales.y] {
            if let ScatterAxisScale::Continuous { min, max, .. } = axis {
                *min = ScatterBound::Auto;
                *max = ScatterBound::Auto;
            }
        }
    }
    if profile == 5 {
        for axis in [&mut options.scales.x, &mut options.scales.y] {
            if let ScatterAxisScale::Continuous { options, .. } = axis {
                options.reverse = true;
                options.round = true;
            }
        }
    }
    if profile == 6 {
        options.scales = ScatterScaleOptions {
            x: ScatterAxisScale::Point {
                options: Default::default(),
                domain: None,
                window: None,
            },
            y: ScatterAxisScale::Point {
                options: Default::default(),
                domain: None,
                window: None,
            },
        };
    }
    if profile == 7 {
        options.scales.x = ScatterAxisScale::Continuous {
            options: ContinuousScaleOptions {
                kind: ScaleKind::Utc,
                ..Default::default()
            },
            min: ScatterBound::Auto,
            max: ScatterBound::Auto,
        };
        options.scales.y = linear([0., 10.]);
    }
    let size = match profile {
        1 => Some(([0., 100.], [10., 30.])),
        2 => Some(([5., 5.], [2., 18.])),
        3 => Some(([100., 0.], [8., 24.])),
        8 => Some(([0., 100.], [0., 10.])),
        _ => None,
    };
    if let Some((values, sizes)) = size {
        options.node.shape = ScatterNodeShape::Circle {
            diameter: ScatterSize::Linear {
                values,
                sizes,
                clamp: false,
                missing: ScatterMissingSize::Error,
            },
        };
    }
    (frame(width, height), data, options)
}
fn polygon_area(geometry: &Geometry, origin: [f64; 2]) -> f64 {
    let points = geometry
        .commands()
        .iter()
        .filter_map(|c| match c {
            C::Move(p) | C::Line(p) => Some([p.x - origin[0], p.y - origin[1]]),
            _ => None,
        })
        .collect::<Vec<_>>();
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
        .map(|(a, b)| a[0] * b[1] - b[0] * a[1])
        .sum::<f64>()
        .abs()
        / 2.
}
fn close(a: f64, b: f64) {
    assert!(
        (a - b).abs() <= 1e-8 * a.abs().max(b.abs()).max(1.),
        "{a} != {b}"
    );
}
#[test]
fn actual_nivo_76_points_18_domains_defaults_extrapolation_constant_and_typed_xy() {
    let mut checked = 0;
    for profile in 0..9 {
        for (width, height) in [(400., 240.), (517., 293.)] {
            let (c, data, o) = fixture(profile, width, height);
            let result = scatter_layout(&c, &data, &o);
            if profile == 8 {
                assert!(
                    result.is_err(),
                    "actual Nivo negative diameter must be rejected explicitly"
                );
                continue;
            }
            let layout = result.unwrap();
            for line in include_str!("fixtures/scatter-nivo.csv").lines().skip(2) {
                let r = line.split('|').collect::<Vec<_>>();
                if r[0].parse::<usize>().unwrap() != profile
                    || r[2].parse::<f64>().unwrap() != width
                {
                    continue;
                }
                let node_id = r[4].split_once(':').unwrap().1;
                let p = layout
                    .sites
                    .iter()
                    .find(|s| s.series_id == r[7] && s.node_id == node_id)
                    .unwrap();
                assert_eq!(p.raw_x, raw(r[8], r[9]));
                assert_eq!(p.raw_y, raw(r[10], r[11]));
                assert_eq!(p.global_index, r[5].parse::<usize>().unwrap());
                assert_eq!(p.source_index, r[6].parse::<usize>().unwrap());
                close(p.position[0] - 30., r[13].parse().unwrap());
                close(p.position[1] - 30., r[14].parse().unwrap());
                close(p.node_size, r[15].parse().unwrap());
                assert_eq!(p.size_unit, ScatterSizeUnit::CircleDiameter);
                let Geometry::Circle { radius, .. } = layout.scene.marks[p.mark_index].geometry
                else {
                    panic!("default circle became another glyph")
                };
                close(radius * 2., p.node_size);
                checked += 1;
            }
            for line in include_str!("fixtures/scatter-domains.csv").lines().skip(2) {
                let r = line.split('|').collect::<Vec<_>>();
                if r[0].parse::<usize>().unwrap() != profile
                    || r[2].parse::<f64>().unwrap() != width
                {
                    continue;
                }
                for (axis, field) in [(&layout.x_scale, r[6]), (&layout.y_scale, r[7])] {
                    let values = field
                        .split(',')
                        .map(|x| {
                            let (k, v) = x.split_once(':').unwrap();
                            raw(k, v)
                        })
                        .collect::<Vec<_>>();
                    if let Some(domain) = axis.category_domain() {
                        assert_eq!(domain, values);
                    } else {
                        let d = axis.display_domain().unwrap();
                        for (i, pixel) in d.iter().enumerate() {
                            // Nivo stores reverse in domain order; the additive
                            // Rust profile stores it in range order. Every
                            // actual mapped pixel is compared above.
                            let j = if profile == 5 { 1 - i } else { i };
                            let CategoryKey::Number(v) = values[j] else {
                                panic!()
                            };
                            close(*pixel, v);
                        }
                    }
                }
            }
            layout.scene.validate().unwrap();
            layout
                .interaction_binding()
                .validate(&layout.scene)
                .unwrap();
        }
    }
    assert_eq!(checked, 66); // Remaining ten actual-Nivo records prove negative-output rejection.
}
fn pick(layout: &ScatterLayout, point: [f64; 2], picking: Picking) -> Vec<String> {
    let mut model = SurfaceModel::new_with_interaction(
        layout.scene.clone(),
        SurfaceOptions {
            plot: Some(layout.plot),
            picking,
            ..Default::default()
        },
        Some(layout.interaction_binding()),
    )
    .unwrap();
    model
        .apply(SurfaceInput::Move {
            id: 1,
            point: (point[0], point[1]),
        })
        .unwrap();
    model.hovered().to_vec()
}
#[test]
fn typed_category_identity_stable_ids_permutation_and_owned_hidden_labels() {
    let (c, mut data, mut o) = fixture(6, 400., 240.);
    o.labels.enabled = true;
    let a = scatter_layout(&c, &data, &o).unwrap();
    assert_ne!(a.sites[0].raw_x, a.sites[1].raw_x);
    assert_ne!(a.sites[0].position[0], a.sites[1].position[0]);
    let ids = a
        .sites
        .iter()
        .map(|p| (p.node_id.clone(), p.mark_id.clone()))
        .collect::<BTreeMap<_, _>>();
    data[0].data.reverse();
    let b = scatter_layout(&c, &data, &o).unwrap();
    for p in &b.sites {
        assert_eq!(ids[&p.node_id], p.mark_id);
        assert!(p.label_index.is_some());
        assert_eq!(
            b.scene.label_series.get(&p.label_index.unwrap()),
            Some(&p.series_id)
        );
    }
    o.hidden.insert("Alpha".into());
    let hidden = scatter_layout(&c, &data, &o).unwrap();
    assert!(hidden.sites.is_empty());
    assert_eq!(hidden.metadata.len(), 3);
    assert!(hidden.metadata.iter().all(|p| p.hidden));
    assert!(hidden.scene.label_series.values().all(|s| s != "Alpha"));
}
#[test]
fn missing_size_clamp_zero_and_bad_output_are_explicit() {
    let c = frame(400., 240.);
    let data = [ScatterSeries::new(
        "A",
        vec![ScatterDatum::new("missing", 2., 3.)],
    )];
    let mut o = bare();
    o.node.shape = ScatterNodeShape::Circle {
        diameter: ScatterSize::Linear {
            values: [0., 1.],
            sizes: [2., 12.],
            clamp: false,
            missing: ScatterMissingSize::Error,
        },
    };
    assert!(scatter_layout(&c, &data, &o).is_err());
    if let ScatterNodeShape::Circle {
        diameter: ScatterSize::Linear { missing, .. },
    } = &mut o.node.shape
    {
        *missing = ScatterMissingSize::Skip;
    }
    let skip = scatter_layout(&c, &data, &o).unwrap();
    assert!(skip.sites.is_empty());
    assert_eq!(skip.metadata.len(), 1);
    if let ScatterNodeShape::Circle {
        diameter: ScatterSize::Linear { missing, .. },
    } = &mut o.node.shape
    {
        *missing = ScatterMissingSize::Fixed(0.);
    }
    let zero = scatter_layout(&c, &data, &o).unwrap();
    assert_eq!(zero.sites[0].node_size, 0.);
    assert!(pick(&zero, zero.sites[0].position, Picking::Geometry).is_empty());
    let mut supplied = data.to_vec();
    supplied[0].data[0].size_value = Some(2.);
    if let ScatterNodeShape::Circle {
        diameter: ScatterSize::Linear { clamp, .. },
    } = &mut o.node.shape
    {
        *clamp = true;
    }
    assert_eq!(
        scatter_layout(&c, &supplied, &o).unwrap().sites[0].node_size,
        12.
    );
    if let ScatterNodeShape::Circle {
        diameter: ScatterSize::Linear { clamp, .. },
    } = &mut o.node.shape
    {
        *clamp = false;
    }
    assert_eq!(
        scatter_layout(&c, &supplied, &o).unwrap().sites[0].node_size,
        22.
    );
    supplied[0].data[0].size_value = Some(-1.);
    assert!(scatter_layout(&c, &supplied, &o).is_err());
}
#[test]
fn true_glyph_units_origins_and_hollow_exact_differ_from_nearest() {
    let c = frame(400., 240.);
    let data = [ScatterSeries::new(
        "A",
        vec![ScatterDatum::new("center", 5., 5.)],
    )];
    let mut o = bare();
    for shape in [FilledGlyph::Star, FilledGlyph::Wye, FilledGlyph::Triangle] {
        o.node.shape = ScatterNodeShape::FilledGlyph {
            shape,
            area: ScatterSize::Fixed(1600.),
            rotation: 0.37,
        };
        let l = scatter_layout(&c, &data, &o).unwrap();
        let site = &l.sites[0];
        assert_eq!(site.size_unit, ScatterSizeUnit::FilledArea);
        assert_eq!(site.node_size, 1600.);
        let expected = glyph_geometry(
            site.position,
            &GlyphOptions {
                shape: GlyphShape::Filled(shape),
                size: 1600.,
                rotation: 0.37,
            },
        )
        .unwrap();
        assert_eq!(l.scene.marks[site.mark_index].geometry, expected.geometry);
        close(
            polygon_area(&l.scene.marks[site.mark_index].geometry, site.position),
            1600.,
        );
        assert!(l.anchors.contains_key(&site.mark_id));
        assert_eq!(
            pick(&l, site.position, Picking::Nearest { distance: 1. }),
            vec![site.mark_id.clone()]
        );
    }
    o.node.shape = ScatterNodeShape::StrokedGlyph {
        shape: StrokedGlyph::Triangle,
        size: ScatterSize::Fixed(1600.),
        rotation: 0.,
    };
    o.node.stroke = Some(GraphicStroke {
        color: Color(30, 70, 150, 255),
        width: 2.,
        cap: StrokeCap::Round,
    });
    let l = scatter_layout(&c, &data, &o).unwrap();
    let site = &l.sites[0];
    assert_eq!(site.size_unit, ScatterSizeUnit::StrokedNominal);
    assert!(pick(&l, site.position, Picking::Geometry).is_empty());
    assert_eq!(
        pick(&l, site.position, Picking::Nearest { distance: 1. }),
        vec![site.mark_id.clone()]
    );
    assert_ne!(
        [
            site.bounds.x + site.bounds.width / 2.,
            site.bounds.y + site.bounds.height / 2.
        ],
        site.position
    );
}
#[test]
fn clipped_center_outside_domain_keeps_visible_extent_and_rejects_outside_pointer() {
    let c = frame(400., 240.);
    let data = [ScatterSeries::new(
        "A",
        vec![ScatterDatum::new("edge", -0.2, 5.)],
    )];
    let mut o = bare();
    o.node.shape = ScatterNodeShape::Circle {
        diameter: ScatterSize::Fixed(40.),
    };
    let l = scatter_layout(&c, &data, &o).unwrap();
    assert_eq!(l.sites.len(), 1);
    assert!(l.sites[0].position[0] < l.plot.x);
    let inside = [l.plot.x + 3., l.sites[0].position[1]];
    assert_eq!(
        pick(&l, inside, Picking::Geometry),
        vec![l.sites[0].mark_id.clone()]
    );
    assert!(pick(&l, [l.plot.x - 1., inside[1]], Picking::Geometry).is_empty());
    assert_eq!(
        pick(&l, inside, Picking::Nearest { distance: 20. }),
        vec![l.sites[0].mark_id.clone()]
    );
}
#[test]
fn malformed_sources_styles_and_binding_updates_preserve_accepted_model() {
    let c = frame(400., 240.);
    let data = vec![ScatterSeries::new(
        "A",
        vec![ScatterDatum::new("p", 5., 5.)],
    )];
    let o = bare();
    let l = scatter_layout(&c, &data, &o).unwrap();
    let mut model = SurfaceModel::new_with_interaction(
        l.scene.clone(),
        SurfaceOptions {
            plot: Some(l.plot),
            ..Default::default()
        },
        Some(l.interaction_binding()),
    )
    .unwrap();
    model
        .apply(SurfaceInput::Move {
            id: 1,
            point: (230., 150.),
        })
        .unwrap();
    let before = model.clone();
    let mut bad = SceneInteraction {
        scene: l.scene.clone(),
        interaction: Some(l.interaction_binding()),
        plot: Some(l.plot),
    };
    bad.interaction.as_mut().unwrap().sites[0].position[0] += 1.;
    assert!(model.set_layout(bad).is_err());
    assert_eq!(model.scene(), before.scene());
    assert_eq!(model.hovered(), before.hovered());
    let mut invalid = data.clone();
    invalid[0].data[0].x = CategoryKey::Number(f64::NAN);
    assert!(scatter_layout(&c, &invalid, &o).is_err());
    invalid = data.clone();
    invalid[0].data[0].x = CategoryKey::String("5".into());
    assert!(scatter_layout(&c, &invalid, &o).is_err());
    invalid = data.clone();
    let repeated = invalid[0].data[0].clone();
    invalid[0].data.push(repeated);
    assert!(scatter_layout(&c, &invalid, &o).is_err());
    let mut invalid = o;
    invalid.node.shape = ScatterNodeShape::Circle {
        diameter: ScatterSize::Fixed(f64::INFINITY),
    };
    assert!(scatter_layout(&c, &data, &invalid).is_err());
}
#[cfg(feature = "raster")]
#[test]
fn cpu_zero_and_real_cap_ink_and_plot_clip_are_not_phantom_markers() {
    use kartis::render::raster::*;
    let c = frame(120., 100.);
    let data = [ScatterSeries::new(
        "A",
        vec![ScatterDatum::new("p", 5., 5.)],
    )];
    let mut o = bare();
    let ink = |layout: &ScatterLayout| {
        let out = render_raster(&layout.scene, &FontRegistry::new(), &Default::default()).unwrap();
        out.rgba
            .as_chunks::<4>()
            .0
            .iter()
            .filter(|px| px[..3].iter().any(|v| *v < 220))
            .count()
    };
    for shape in [
        ScatterNodeShape::Circle {
            diameter: ScatterSize::Fixed(0.),
        },
        ScatterNodeShape::FilledGlyph {
            shape: FilledGlyph::Star,
            area: ScatterSize::Fixed(0.),
            rotation: 0.,
        },
    ] {
        o.node.shape = shape;
        o.node.stroke = None;
        let l = scatter_layout(&c, &data, &o).unwrap();
        assert_eq!(l.sites.len(), 1);
        assert_eq!(ink(&l), 0);
    }
    o.node.shape = ScatterNodeShape::StrokedGlyph {
        shape: StrokedGlyph::Plus,
        size: ScatterSize::Fixed(0.),
        rotation: 0.,
    };
    for (cap, min, max) in [
        (StrokeCap::Butt, 0, 0),
        (StrokeCap::Round, 60, 95),
        (StrokeCap::Square, 90, 110),
    ] {
        o.node.stroke = Some(GraphicStroke {
            color: Color(25, 65, 150, 255),
            width: 10.,
            cap,
        });
        let l = scatter_layout(&c, &data, &o).unwrap();
        let count = ink(&l);
        assert!((min..=max).contains(&count), "{cap:?}: {count}");
        assert_eq!(
            !pick(&l, l.sites[0].position, Picking::Geometry).is_empty(),
            cap != StrokeCap::Butt
        );
    }
    o.node.shape = ScatterNodeShape::Circle {
        diameter: ScatterSize::Fixed(40.),
    };
    o.node.stroke = Some(GraphicStroke {
        color: Color(20, 65, 150, 255),
        width: 12.,
        cap: StrokeCap::Round,
    });
    let edge = [ScatterSeries::new(
        "A",
        vec![ScatterDatum::new("partial", -0.2, 5.)],
    )];
    let layout = scatter_layout(&c, &edge, &o).unwrap();
    assert!(layout.sites[0].position[0] < layout.plot.x);
    let out = render_raster(&layout.scene, &FontRegistry::new(), &Default::default()).unwrap();
    let at = |x: usize, y: usize| {
        &out.rgba[(y * c.width as usize + x) * 4..(y * c.width as usize + x) * 4 + 4]
    };
    assert_eq!(
        at(29, 80),
        &[255, 255, 255, 255],
        "thick circle paint escaped actual plot clip"
    );
    assert!(
        at(32, 80)[..3].iter().any(|v| *v < 220),
        "outside-center glyph lost its visible in-plot extent"
    );
}
#[test]
fn four_axes_and_rotated_owned_labels_share_actual_mappings() {
    let (c, data, mut o) = fixture(5, 400., 240.);
    o.axes.top = Some(AxisOptions::default());
    o.axes.right = Some(AxisOptions::default());
    o.labels.enabled = true;
    o.labels.rotation = 35.;
    o.labels.text = ScatterLabelText::Coordinates;
    let l = scatter_layout(&c, &data, &o).unwrap();
    for p in &l.sites {
        assert_eq!(l.x_scale.map(&p.raw_x).unwrap(), Some(p.position[0]));
        assert_eq!(l.y_scale.map(&p.raw_y).unwrap(), Some(p.position[1]));
        let i = p.label_index.unwrap();
        assert_eq!(l.scene.labels[i].rotation, 35.);
        assert_eq!(l.scene.label_series[&i], p.series_id);
        assert!(
            !l.scene.labels[i].text.contains(&p.position[0].to_string())
                || p.raw_x == CategoryKey::Number(p.position[0])
        );
    }
    assert!(l.scene.marks.iter().filter(|m| m.series.is_empty()).count() > 8);
    l.scene.validate().unwrap();
}
#[test]
fn source_and_real_path_command_budgets_reject_before_a_partial_result() {
    let c = frame(400., 240.);
    let data = [ScatterSeries::new(
        "A",
        (0..20_001)
            .map(|i| ScatterDatum::new(format!("point-{i}"), 5., 5.))
            .collect(),
    )];
    assert!(scatter_layout(&c, &data, &bare()).is_err());
    let mut o = bare();
    o.node.shape = ScatterNodeShape::FilledGlyph {
        shape: FilledGlyph::Star,
        area: ScatterSize::Fixed(100.),
        rotation: 0.,
    };
    let data = [ScatterSeries::new(
        "A",
        (0..20_000)
            .map(|i| ScatterDatum::new(format!("point-{i}"), 5., 5.))
            .collect(),
    )];
    // Actual Star geometry has 10 vertices + Close. It cannot be admitted as
    // 20k supposedly-simple circles to bypass Scene's 200k command budget.
    assert!(scatter_layout(&c, &data, &o).is_err());
}
