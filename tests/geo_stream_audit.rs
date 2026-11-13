use kartis::*;

#[test]
fn representable_short_arc_must_not_be_silently_removed() {
    let line = GeoLineFeature::new("short", vec![[0., 0.], [5e-11, 0.]]);
    let layout = geographic_lines(
        &ChartConfig::default(),
        &[line],
        &GeoMapOptions {
            fit: GeoFit::Data,
            projection: GeoProjection::Equirectangular,
            ..Default::default()
        },
        &GeoStreamOptions::default(),
    )
    .unwrap();
    assert!(
        layout.metadata.lines[0].geometry.is_some(),
        "a distinct finite short great-circle arc was silently removed"
    );
    let geometry = layout.metadata.lines[0].geometry.as_ref().unwrap();
    let commands = geometry.commands();
    let [glory::render::PathCommand::Move(a), ..] = commands.as_slice() else {
        panic!()
    };
    let glory::render::PathCommand::Line(b) = commands.last().unwrap() else {
        panic!()
    };
    assert!(
        b.x - a.x > 500.,
        "Data-fit short arc must be actually drawable, not collapsed to a fake endpoint"
    );
    let expected = layout.projection.project([5e-11, 0.]).unwrap().unwrap();
    assert!((b.x - expected[0]).abs() < 1e-8 && (b.y - expected[1]).abs() < 1e-8);
}

#[test]
fn caller_two_point_budget_rejects_long_arc_transactionally() {
    let view = GeoMapOptions {
        projection: GeoProjection::Equirectangular,
        ..Default::default()
    };
    let mut layout = geographic_lines(
        &ChartConfig::default(),
        &[],
        &view,
        &GeoStreamOptions::default(),
    )
    .unwrap();
    let before = layout.scene.clone();
    assert!(
        layout
            .projection
            .add_lines(
                &mut layout.scene,
                &[GeoLineFeature::new("long", vec![[-80., 10.], [80., 10.]])],
                &GeoStreamOptions {
                    path: Some(GeoPathOptions {
                        max_points: 2,
                        ..Default::default()
                    }),
                    ..Default::default()
                }
            )
            .is_err()
    );
    assert_eq!(layout.scene, before);
}

#[test]
fn horizon_boundary_arc_is_consistent_with_projected_visible_endpoints() {
    let view = GeoMapOptions {
        projection: GeoProjection::Orthographic,
        ..Default::default()
    };
    let positions = vec![[90., 10.], [90., 50.]];
    assert!(project_geo_point(positions[0], &view).unwrap().is_some());
    assert!(project_geo_point(positions[1], &view).unwrap().is_some());
    let layout = geographic_lines(
        &ChartConfig::default(),
        &[GeoLineFeature::new("horizon", positions)],
        &view,
        &GeoStreamOptions::default(),
    )
    .unwrap();
    assert!(
        layout.metadata.lines[0].geometry.is_some(),
        "visible horizon boundary arc was removed"
    );
}

#[test]
fn sub_epsilon_antimeridian_crossing_does_not_draw_across_world() {
    let view = GeoMapOptions {
        projection: GeoProjection::Equirectangular,
        ..Default::default()
    };
    let layout = geographic_lines(
        &ChartConfig::default(),
        &[GeoLineFeature::new(
            "tiny seam",
            vec![[180. - 2e-11, 0.], [-180. + 2e-11, 0.]],
        )],
        &view,
        &GeoStreamOptions::default(),
    )
    .unwrap();
    let geometry = layout.metadata.lines[0].geometry.as_ref().unwrap();
    let mut previous = None;
    let mut lengths = Vec::new();
    let mut endpoints = Vec::new();
    for command in geometry.commands() {
        match command {
            glory::render::PathCommand::Move(p) => {
                previous = Some(p);
                endpoints.push(p);
            }
            glory::render::PathCommand::Line(p) => {
                let a = previous.unwrap();
                assert!(
                    (p.x - a.x).abs() < 100.,
                    "tiny antimeridian arc drew a world-spanning chord: {a:?} -> {p:?}"
                );
                lengths.push((p.x - a.x).hypot(p.y - a.y));
                endpoints.push(p);
                previous = Some(p);
            }
            _ => panic!(),
        }
    }
    assert_eq!(layout.metadata.lines[0].components, 2);
    assert_eq!(lengths.len(), 2);
    assert!(
        lengths.iter().all(|&length| length > 0.),
        "real tiny source endpoints were snapped to the seam"
    );
    for (actual, source) in [
        (*endpoints.first().unwrap(), [180. - 2e-11, 0.]),
        (*endpoints.last().unwrap(), [-180. + 2e-11, 0.]),
    ] {
        let expected = layout.projection.project(source).unwrap().unwrap();
        assert!((actual.x - expected[0]).abs() < 1e-9 && (actual.y - expected[1]).abs() < 1e-9);
    }
}

#[test]
fn same_side_tiny_seam_and_pole_arcs_preserve_datafit_original_endpoints() {
    for positions in [
        vec![[180. - 4e-11, 0.], [180. - 2e-11, 0.]],
        vec![[20., 90. - 2e-11], [20., 90. - 1e-11]],
    ] {
        let view = GeoMapOptions {
            projection: GeoProjection::Equirectangular,
            fit: GeoFit::Data,
            ..Default::default()
        };
        let layout = geographic_lines(
            &ChartConfig::default(),
            &[GeoLineFeature::new("tiny", positions.clone())],
            &view,
            &GeoStreamOptions::default(),
        )
        .unwrap();
        let commands = layout.metadata.lines[0]
            .geometry
            .as_ref()
            .unwrap()
            .commands();
        let glory::render::PathCommand::Move(a) = commands[0] else {
            panic!()
        };
        let glory::render::PathCommand::Line(b) = *commands.last().unwrap() else {
            panic!()
        };
        assert!(
            (b.x - a.x).hypot(b.y - a.y) > 300.,
            "same-side finite arc disappeared under maximal Data fit"
        );
        for (actual, source) in [(a, positions[0]), (b, positions[1])] {
            let expected = layout.projection.project(source).unwrap().unwrap();
            assert!(
                (actual.x - expected[0]).abs() < 1e-8 && (actual.y - expected[1]).abs() < 1e-8,
                "Data-fit source endpoint changed {actual:?} vs {expected:?}, {positions:?}"
            );
        }
    }
}
