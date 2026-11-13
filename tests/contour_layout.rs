#[cfg(feature = "raster")]
use kartis::render::raster::*;
use kartis::surface::*;
use kartis::*;
#[path = "contour_common/layout.rs"]
mod common;
use common::*;
#[cfg(feature = "raster")]
#[path = "contour_common/raster.rs"]
mod pixels;
#[cfg(feature = "raster")]
use pixels::*;
#[test]
fn threshold_identity_hidden_empty_and_rejected_frames_are_transactional() {
    let c = frame(340., 340.);
    let values = nested();
    let base = contour_layout(&c, grid(&values), &compute(vec![1., 1., 4.]), &teal()).unwrap();
    let more = contour_layout(&c, grid(&values), &compute(vec![-2., 1., 1., 4.]), &teal()).unwrap();
    assert_eq!(base.levels[0].id, more.levels[1].id);
    assert_eq!(base.levels[1].id, more.levels[2].id);
    assert_ne!(base.levels[0].id, base.levels[1].id);
    assert_eq!(base.levels[1].duplicate_rank, 1);
    assert!(base.levels[2].mark_id.is_none());
    assert_eq!(base.binding.sites.len(), 2);
    let mut hidden = teal();
    hidden.hidden.insert(base.levels[0].id.clone());
    let visible = contour_layout(&c, grid(&values), &compute(vec![1., 1., 4.]), &hidden).unwrap();
    assert!(visible.levels[0].hidden && visible.levels[0].mark_id.is_none());
    assert_eq!(visible.binding.sites.len(), 1);
    assert_eq!(base.colors.map(Some(1.)), visible.colors.map(Some(1.)));
    let empty = contour_layout(
        &c,
        grid(&vec![None; 225]),
        &ContourOptions::default(),
        &teal(),
    )
    .unwrap();
    assert!(
        empty.levels.is_empty() && empty.scene.marks.is_empty() && empty.binding.sites.is_empty()
    );
    let mut surface = SurfaceModel::new_with_interaction(
        base.scene.clone(),
        SurfaceOptions {
            plot: Some(base.plot),
            ..Default::default()
        },
        Some(base.binding.clone()),
    )
    .unwrap();
    let mut proposal = more.scene_interaction();
    proposal.interaction.as_mut().unwrap().sites[0].value = Some(f64::NAN);
    assert!(surface.set_layout(proposal).is_err());
    assert_eq!(surface.scene(), &base.scene);
    assert_eq!(surface.interaction_data(), Some(&base.binding));
    surface.set_layout(visible.scene_interaction()).unwrap();
    assert_eq!(surface.interaction_data(), Some(&visible.binding));
}

#[test]
fn lower_scene_budgets_and_bad_geometry_reject_even_if_hidden() {
    let c = frame(340., 340.);
    let values = nested();
    let options = compute(vec![1.]);
    let layout = contour_layout(&c, grid(&values), &options, &teal()).unwrap();
    for limits in [
        ContourSceneLimits {
            commands: 4,
            ..Default::default()
        },
        ContourSceneLimits {
            rings: 1,
            ..Default::default()
        },
        ContourSceneLimits {
            polygons: 1,
            ..Default::default()
        },
        ContourSceneLimits {
            levels: 0,
            ..Default::default()
        },
        ContourSceneLimits {
            commands: 200001,
            ..Default::default()
        },
    ] {
        let mut style = teal();
        style.limits = limits;
        style.hidden.insert(layout.levels[0].id.clone());
        assert!(contour_layout(&c, grid(&values), &options, &style).is_err());
    }
    let mut raw = contours(grid(&values), &options).unwrap();
    raw[0].polygons[0].outer[1][0] = f64::NAN;
    assert!(
        contour_levels_layout(
            &c,
            &raw,
            ClipRect {
                x: 0.,
                y: 0.,
                width: 15.,
                height: 15.
            },
            &teal()
        )
        .is_err()
    );
    let mut invalid = teal();
    invalid.opacity = f64::NAN;
    assert!(contour_layout(&c, grid(&values), &options, &invalid).is_err());
    assert_eq!(values, nested());
}

#[test]
fn immutable_density_field_reuses_exact_storage_and_threshold_units() {
    let field = estimate_density(
        &[DensityPoint {
            position: [16., 12.],
            weight: 1.,
        }],
        DensityOptions {
            size: [32., 24.],
            bandwidth: 2.,
            cell_size: 3.,
            ..Default::default()
        },
    )
    .unwrap();
    let pointer = field.values().as_ptr();
    let bits = field
        .values()
        .iter()
        .map(|v| v.to_bits())
        .collect::<Vec<_>>();
    let copy = field.clone();
    assert_eq!(pointer, copy.values().as_ptr());
    assert_eq!(field.cell_size(), 2);
    assert_eq!(field.requested_cell_size(), 3.);
    assert_eq!(field.sample_count(), 1);
    assert!(field.origin().iter().all(|v| *v < 0.));
    let c = frame(72., 64.);
    let first = density_layout(
        &c,
        &field,
        &DensityThresholds::Values(vec![0.001]),
        ContourLimits::default(),
        &teal(),
    )
    .unwrap();
    let second = density_layout(
        &c,
        &copy,
        &DensityThresholds::Values(vec![0.001, 0.002]),
        ContourLimits::default(),
        &teal(),
    )
    .unwrap();
    assert_eq!(first.levels[0].threshold, 0.001);
    assert_eq!(first.levels[0].id, second.levels[0].id);
    assert_eq!(pointer, field.values().as_ptr());
    assert_eq!(
        bits,
        field
            .values()
            .iter()
            .map(|v| v.to_bits())
            .collect::<Vec<_>>()
    );
    #[cfg(feature = "raster")]
    {
        let image = render_raster(&first.scene, &FontRegistry::new(), &Default::default()).unwrap();
        assert_eq!(pixel(&image, 36, 32), &[10, 130, 140, 255]);
        assert_eq!(pixel(&image, 21, 21), &[255, 255, 255, 255]);
        export("density-one", &first.scene, &image);
    }
    assert!(
        density_layout(
            &frame(73., 64.),
            &field,
            &DensityThresholds::Count(4),
            ContourLimits::default(),
            &teal()
        )
        .is_err()
    );
}

#[test]
fn real_scatter_projection_weighted_overlay_matches_original_d3_float32_field() {
    let row = include_str!("fixtures/density-candidate.csv")
        .lines()
        .find(|r| {
            let c = r.split('|').collect::<Vec<_>>();
            c.len() > 10 && c[1] == "fractional" && c[4] == "2" && c[5] == "3"
        })
        .unwrap();
    let cells = row.split('|').collect::<Vec<_>>();
    let mut weights = DensityWeights::new();
    let points = cells[7]
        .split(';')
        .enumerate()
        .map(|(i, p)| {
            let v = p
                .split(',')
                .map(|v| v.parse::<f64>().unwrap())
                .collect::<Vec<_>>();
            let id = format!("p{i}");
            weights.insert(("observations".into(), id.clone()), v[2]);
            let mut p = ScatterDatum::new(id, v[0], 24. - v[1]);
            p.size_value = Some(999.);
            p
        })
        .collect();
    let c = frame(72., 64.);
    let data = [ScatterSeries::new("observations", points)];
    let scatter = exact_scatter(&c, &data);
    let before = scatter.scene.clone();
    let output = scatter_density_layout(
        &c,
        &scatter,
        &weights,
        ScatterDensityOptions {
            bandwidth: 2.,
            cell_size: 3.,
            ..Default::default()
        },
        &DensityThresholds::Values(vec![0.001, 0.01]),
        &teal(),
    )
    .unwrap();
    let expected = cells[8]
        .split(',')
        .map(|v| u32::from_str_radix(v, 16).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        output
            .field
            .values()
            .iter()
            .map(|v| v.to_bits())
            .collect::<Vec<_>>(),
        expected
    );
    assert_eq!(output.field.max(), cells[9].parse::<f64>().unwrap());
    assert_eq!(output.observations.len(), 3);
    assert_eq!(scatter.scene, before);
    assert!(output.scene.draw_order.len() > scatter.scene.draw_order.len());
    for site in &scatter.sites {
        assert!(
            output
                .binding
                .sites
                .iter()
                .any(|s| s.mark_id == site.mark_id && s.position == site.position)
        );
    }
    let site = &scatter.sites[0];
    assert_eq!(
        output
            .scene
            .hit_test(site.position[0], site.position[1])
            .unwrap()
            .id,
        site.mark_id
    );
    let orders = output.scene.draw_items();
    let first_source=orders.iter().position(|d|matches!(d,DrawItem::Mark(i) if output.scene.marks[*i].id==scatter.sites[0].mark_id)).unwrap();
    let first_density = orders
        .iter()
        .position(
            |d| matches!(d,DrawItem::Mark(i) if output.scene.marks[*i].id.starts_with("contour:")),
        )
        .unwrap();
    assert!(first_density < first_source);
    #[cfg(feature = "raster")]
    {
        let fonts = FontRegistry::new();
        let image = render_raster(&output.scene, &fonts, &Default::default()).unwrap();
        export("weighted-scatter", &output.scene, &image);
    }
    let mut invalid = weights.clone();
    invalid.insert(("missing".into(), "p0".into()), 1.);
    assert!(
        scatter_density_layout(
            &c,
            &scatter,
            &invalid,
            ScatterDensityOptions::default(),
            &DensityThresholds::default(),
            &teal()
        )
        .is_err()
    );
    assert_eq!(scatter.scene, before);
}

#[test]
fn scatter_density_recomputes_hidden_observations_and_never_uses_marker_size_as_weight() {
    let c = frame(72., 64.);
    let mut a = ScatterDatum::new("p", 8., 12.);
    a.size_value = Some(0.);
    let mut b = ScatterDatum::new("p", 24., 12.);
    b.size_value = Some(999.);
    let data = [
        ScatterSeries::new("a", vec![a]),
        ScatterSeries::new("b", vec![b]),
    ];
    let scatter = exact_scatter(&c, &data);
    let options = ScatterDensityOptions {
        bandwidth: 2.,
        cell_size: 2.,
        ..Default::default()
    };
    let output = scatter_density_layout(
        &c,
        &scatter,
        &Default::default(),
        options,
        &DensityThresholds::Values(vec![0.001]),
        &teal(),
    )
    .unwrap();
    assert_eq!(output.field.sample_count(), 2);
    assert!(output.observations.iter().all(|p| p.weight == 1.));
    let mut changed = data.clone();
    changed[0].data[0].size_value = Some(777.);
    changed[1].data[0].size_value = None;
    let other = exact_scatter(&c, &changed);
    let other = scatter_density_layout(
        &c,
        &other,
        &Default::default(),
        options,
        &DensityThresholds::Values(vec![0.001]),
        &teal(),
    )
    .unwrap();
    assert_eq!(output.field.values(), other.field.values());
    let mut hidden = scatter_layout(
        &c,
        &data,
        &ScatterLayoutOptions {
            scales: scatter.scale_options.clone(),
            axes: ScatterAxes {
                top: None,
                right: None,
                bottom: None,
                left: None,
            },
            hidden: ["a".into()].into(),
            ..Default::default()
        },
    )
    .unwrap();
    let hidden_out = scatter_density_layout(
        &c,
        &hidden,
        &Default::default(),
        options,
        &DensityThresholds::Values(vec![0.001]),
        &teal(),
    )
    .unwrap();
    assert_eq!(hidden_out.observations.len(), 1);
    assert_eq!(hidden_out.observations[0].series_id, "b");
    assert_eq!(hidden_out.field.sample_count(), 1);
    assert_ne!(hidden_out.field.values(), output.field.values());
    hidden.metadata[1].position.as_mut().unwrap()[0] += 1.;
    assert!(
        scatter_density_layout(
            &c,
            &hidden,
            &Default::default(),
            options,
            &DensityThresholds::Values(vec![0.001]),
            &teal()
        )
        .is_err()
    );
    let mut oversized = teal();
    oversized.limits.commands = 200001;
    assert!(
        scatter_density_layout(
            &c,
            &scatter,
            &Default::default(),
            options,
            &DensityThresholds::Count(4),
            &oversized
        )
        .is_err()
    );
}

#[test]
fn nested_holes_keep_typed_exact_picking_and_glory_cleanup() {
    let c = frame(340., 340.);
    let values = nested();
    let layout = contour_layout(&c, grid(&values), &compute(vec![1.]), &teal()).unwrap();
    assert_eq!((layout.levels[0].polygons, layout.levels[0].holes), (2, 2));
    assert_eq!(layout.scene.marks.len(), 1);
    assert_eq!(layout.binding.sites[0].value, Some(1.));
    assert!(layout.binding.sites[0].label.contains("Threshold"));
    #[cfg(feature = "raster")]
    let image = render_raster(&layout.scene, &FontRegistry::new(), &Default::default()).unwrap();
    let mut surface = SurfaceModel::new_with_interaction(
        layout.scene.clone(),
        SurfaceOptions {
            plot: Some(layout.plot),
            pan: false,
            zoom: false,
            brush: false,
            pinch: false,
            ..Default::default()
        },
        Some(layout.binding.clone()),
    )
    .unwrap();
    for (x, filled) in [
        (10, false),
        (30, true),
        (70, false),
        (110, true),
        (170, false),
    ] {
        #[cfg(feature = "raster")]
        assert_eq!(
            pixel(&image, x, 170),
            if filled {
                &[10, 130, 140, 255]
            } else {
                &[255, 255, 255, 255]
            },
            "independent nested pixel x{x}"
        );
        assert_eq!(layout.scene.hit_test(x as f64, 170.).is_some(), filled);
        surface
            .apply(SurfaceInput::Move {
                id: 1,
                point: (x as f64, 170.),
            })
            .unwrap();
        assert_eq!(!surface.hovered().is_empty(), filled, "typed hole hit x{x}");
        if filled {
            assert_eq!(
                surface.active_interaction(surface.hovered()).unwrap().rows[0].value,
                Some(1.)
            );
        }
    }
    #[cfg(feature = "raster")]
    export("nested-holes", &layout.scene, &image);
    let app = glory::App::memory();
    app.mount(chart(layout.scene)).unwrap();
    app.flush().unwrap();
    app.clear().unwrap();
    assert_eq!(app.stats().nodes, 0);
}

#[test]
fn retained_weighted_recontour_reuses_field_and_keeps_accepted_source_private() {
    let c = frame(72., 64.);
    let scatter = exact_scatter(
        &c,
        &[ScatterSeries::new(
            "observations",
            vec![
                ScatterDatum::new("a", 16., 12.),
                ScatterDatum::new("b", 20., 10.),
            ],
        )],
    );
    let weights = [(("observations".into(), "a".into()), 2.)].into();
    let old = scatter_density_layout(
        &c,
        &scatter,
        &weights,
        ScatterDensityOptions {
            bandwidth: 2.,
            cell_size: 3.,
            ..Default::default()
        },
        &DensityThresholds::Values(vec![0.001]),
        &teal(),
    )
    .unwrap();
    let pointer = old.field.values().as_ptr();
    let old_scene = old.scene.clone();
    let old_binding = old.binding.clone();
    let old_values = old.field.values().to_vec();
    let old_id = old.levels[0].id.clone();
    let mut paint = teal();
    paint.opacity = 0.7;
    let next = old
        .recontour(
            &DensityThresholds::Values(vec![0.0005, 0.001]),
            ContourLimits::default(),
            &paint,
        )
        .unwrap();
    assert_eq!(next.field.values().as_ptr(), pointer);
    assert_eq!(next.field.values(), old_values);
    assert_eq!(next.levels[1].id, old_id);
    assert_eq!(next.observations, old.observations);
    assert_eq!(
        next.binding
            .sites
            .iter()
            .filter(|s| s.mark_id == scatter.sites[0].mark_id)
            .count(),
        1
    );
    assert!(
        next.scene
            .marks
            .iter()
            .filter(|m| m.id.starts_with("contour:"))
            .all(|m| m.opacity == 0.7)
    );
    paint.hidden = next.levels.iter().map(|l| l.id.clone()).collect();
    let hidden = next
        .recontour(
            &DensityThresholds::Values(vec![0.0005, 0.001]),
            ContourLimits::default(),
            &paint,
        )
        .unwrap();
    assert_eq!(hidden.field.values().as_ptr(), pointer);
    assert!(hidden.levels.iter().all(|l| l.hidden));
    assert_eq!(hidden.binding, scatter.interaction_binding());
    assert!(
        old.recontour(
            &DensityThresholds::Values(vec![f64::NAN]),
            ContourLimits::default(),
            &teal()
        )
        .is_err()
    );
    paint.opacity = f64::NAN;
    assert!(
        old.recontour(
            &DensityThresholds::Count(4),
            ContourLimits::default(),
            &paint
        )
        .is_err()
    );
    let mut too_small = teal();
    too_small.limits.commands = 1;
    assert!(
        old.recontour(
            &DensityThresholds::Values(vec![0.001]),
            ContourLimits::default(),
            &too_small
        )
        .is_err()
    );
    assert!(
        density_layout(
            &frame(73., 64.),
            &old.field,
            &DensityThresholds::Values(vec![0.001]),
            ContourLimits::default(),
            &teal()
        )
        .is_err()
    );
    // Public output mutation cannot covertly supply a new extent or aggregate.
    let mut edited_output = old.clone();
    edited_output.plot.width += 1.;
    edited_output.scene.width += 1.;
    edited_output.observations.clear();
    edited_output.field = estimate_density(&[], DensityOptions::default()).unwrap();
    let restored = edited_output
        .recontour(
            &DensityThresholds::Values(vec![0.001]),
            ContourLimits::default(),
            &teal(),
        )
        .unwrap();
    assert_eq!(restored.plot, old.plot);
    assert_eq!(restored.scene, old_scene);
    assert_eq!(restored.binding, old_binding);
    assert_eq!(restored.observations, old.observations);
    assert_eq!(restored.field.values().as_ptr(), pointer);
    assert_eq!(old.scene, old_scene);
    assert_eq!(old.binding, old_binding);
    assert_eq!(old.field.values(), old_values);
}
