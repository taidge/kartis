//! Regression checks for inconsistent external Scatter metadata.
use kartis::*;

fn source() -> (ChartConfig, ScatterLayout) {
    let config = ChartConfig {
        width: 400.,
        height: 300.,
        ..Default::default()
    };
    let layout = scatter_layout(
        &config,
        &[ScatterSeries::new(
            "a",
            vec![ScatterDatum::new("p", 16., 12.)],
        )],
        &ScatterLayoutOptions {
            scales: ScatterScaleOptions {
                x: ScatterAxisScale::Continuous {
                    options: ContinuousScaleOptions::default(),
                    min: ScatterBound::Fixed(0.),
                    max: ScatterBound::Fixed(32.),
                },
                y: ScatterAxisScale::Continuous {
                    options: ContinuousScaleOptions::default(),
                    min: ScatterBound::Fixed(0.),
                    max: ScatterBound::Fixed(24.),
                },
            },
            axes: ScatterAxes {
                top: None,
                right: None,
                bottom: None,
                left: None,
            },
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(layout.sites.len(), 1);
    (config, layout)
}
fn overlay(config: &ChartConfig, scatter: &ScatterLayout) -> Result<ScatterDensityLayout> {
    scatter_density_layout(
        config,
        scatter,
        &DensityWeights::new(),
        ScatterDensityOptions {
            bandwidth: 0.,
            cell_size: 4.,
            ..Default::default()
        },
        &DensityThresholds::Values(vec![0.001]),
        &ContourLayoutOptions::default(),
    )
}

#[test]
fn visible_source_index_must_agree_with_original_typed_scatter_site() {
    let (config, mut scatter) = source();
    let base = overlay(&config, &scatter).unwrap();
    assert_eq!(base.observations[0].source_index, 0);
    scatter.metadata[0].source_index = 999;
    let result = overlay(&config, &scatter);
    if let Ok(output) = &result {
        eprintln!(
            "accepted metadata source_index={} but actual typed site source_index={}",
            output.observations[0].source_index, scatter.sites[0].source_index
        );
    }
    assert!(result.is_err(), "inconsistent source index must reject");
}
#[test]
fn painted_visible_mark_cannot_be_silently_removed_by_metadata_hidden_flag() {
    let (config, mut scatter) = source();
    scatter.metadata[0].hidden = true;
    let result = overlay(&config, &scatter);
    if let Ok(output) = &result {
        eprintln!(
            "accepted observations={} sample_count={} while {} source marks remain",
            output.observations.len(),
            output.field.sample_count(),
            scatter.scene.marks.len()
        );
    }
    assert!(
        result.is_err(),
        "visible source and hidden metadata must not disagree"
    );
}
#[test]
fn painted_visible_mark_cannot_be_silently_removed_by_erasing_metadata_position() {
    let (config, mut scatter) = source();
    scatter.metadata[0].position = None;
    let result = overlay(&config, &scatter);
    if let Ok(output) = &result {
        eprintln!(
            "accepted observations={} sample_count={} with still-visible typed datum {}",
            output.observations.len(),
            output.field.sample_count(),
            scatter.sites[0].mark_id
        );
    }
    assert!(
        result.is_err(),
        "a painted mapped source cannot lose its metadata position"
    );
}

#[test]
fn mapped_observation_without_glyph_cannot_lose_density_coordinates() {
    let config = ChartConfig {
        width: 400.,
        height: 300.,
        ..Default::default()
    };
    let mut scatter = scatter_layout(
        &config,
        &[ScatterSeries::new(
            "a",
            vec![ScatterDatum::new("p", 16., 12.)],
        )],
        &ScatterLayoutOptions {
            scales: ScatterScaleOptions {
                x: ScatterAxisScale::Continuous {
                    options: ContinuousScaleOptions::default(),
                    min: ScatterBound::Fixed(0.),
                    max: ScatterBound::Fixed(32.),
                },
                y: ScatterAxisScale::Continuous {
                    options: ContinuousScaleOptions::default(),
                    min: ScatterBound::Fixed(0.),
                    max: ScatterBound::Fixed(24.),
                },
            },
            axes: ScatterAxes {
                top: None,
                right: None,
                bottom: None,
                left: None,
            },
            node: ScatterNodeStyle {
                shape: ScatterNodeShape::Circle {
                    diameter: ScatterSize::Linear {
                        values: [0., 1.],
                        sizes: [0., 10.],
                        clamp: true,
                        missing: ScatterMissingSize::Skip,
                    },
                },
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    assert!(scatter.sites.is_empty());
    assert!(scatter.metadata[0].position.is_some());
    assert_eq!(overlay(&config, &scatter).unwrap().field.sample_count(), 1);
    scatter.metadata[0].position = None;
    let result = overlay(&config, &scatter);
    if let Ok(output) = &result {
        eprintln!(
            "accepted mapped no-glyph datum but dropped its density observation: sample_count={}",
            output.field.sample_count()
        );
    }
    assert!(
        result.is_err(),
        "a mapped no-glyph source cannot lose its density position"
    );
}

#[test]
fn painted_source_cannot_be_declared_glyphless_by_erasing_sites_and_indices() {
    let (config, mut scatter) = source();
    let id = scatter.sites[0].mark_id.clone();
    assert!(overlay(&config, &scatter).is_ok());
    scatter.sites.clear();
    scatter.anchors.clear();
    scatter.metadata[0].mark_index = None;
    scatter.metadata[0].node_size = None;
    assert!(scatter.scene.marks.iter().any(|mark| mark.id == id));
    assert!(
        overlay(&config, &scatter).is_err(),
        "a painted mark sharing a source metadata ID must retain its typed site"
    );
}

#[test]
fn unrelated_scene_decoration_does_not_require_a_scatter_site() {
    let (config, mut scatter) = source();
    let decoration = Mark::new(
        "application-decoration",
        Geometry::Rect {
            x: 5.,
            y: 5.,
            width: 10.,
            height: 10.,
        },
        Color(70, 80, 90, 255),
    );
    scatter.scene.marks.push(decoration.clone());
    let output = overlay(&config, &scatter).unwrap();
    assert!(output.scene.marks.contains(&decoration));
    assert_eq!(output.observations.len(), 1);
    assert!(
        output
            .binding
            .sites
            .iter()
            .all(|site| site.mark_id != decoration.id)
    );
}

#[test]
fn consistently_renamed_public_source_identity_remains_valid() {
    let (config, mut scatter) = source();
    let old_id = scatter.sites[0].mark_id.clone();
    let new_id = "application-owned-datum".to_owned();
    scatter.scene.marks[scatter.sites[0].mark_index].id = new_id.clone();
    scatter.sites[0].mark_id = new_id.clone();
    scatter.metadata[0].mark_id = new_id.clone();
    if let Some(anchor) = scatter.anchors.remove(&old_id) {
        scatter.anchors.insert(new_id.clone(), anchor);
    }
    let output = overlay(&config, &scatter).unwrap();
    assert_eq!(output.observations[0].mark_id, new_id);
    assert!(
        output
            .binding
            .sites
            .iter()
            .any(|site| site.mark_id == new_id)
    );
}
