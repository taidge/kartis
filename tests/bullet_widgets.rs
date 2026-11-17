use kartis::*;
#[path = "../examples/shared/bullet_widgets.rs"]
mod model;

#[test]
fn marker_widgets_accept_inferred_and_explicit_constant_domains() {
    for explicit in [false, true] {
        let source = [BulletDatum {
            id: "constant".into(),
            ranges: vec![],
            measures: vec![],
            markers: vec![5.],
        }];
        let layout = bullet_layout(
            &ChartConfig::default(),
            &source,
            &BulletLayoutOptions {
                min: explicit.then_some(5.),
                max: explicit.then_some(5.),
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(layout.rows[0].domain, [5., 5.]);
        let packet = bullet_part_widgets(&layout, 0, |c| {
            assert_eq!(c.part.kind, BulletPartKind::Marker);
            Ok(Some(PlotWidgetPlacement::Rect(LayoutRect {
                x: c.row.track.x,
                y: c.row.track.y,
                width: 40.,
                height: 20.,
            })))
        })
        .unwrap();
        assert_eq!(packet.data.len(), 1);
        assert_eq!(packet.data.values().next().unwrap().domain, [5., 5.]);
    }
}

#[test]
fn typed_bullet_slots_follow_source_identity_rank_and_current_metadata() {
    let (scene, initial) = model::source(0, 600., 4).unwrap();
    assert_eq!(initial.layout.slots.len(), 6);
    let (_, next) = model::source(1, 520., 4).unwrap();
    for slot in &initial.layout.slots {
        assert_eq!(slot.revision, 4);
        let a = &initial.data[&slot.mark_id];
        let b = &next.data[&slot.mark_id];
        let mark = scene.marks.iter().find(|m| m.id == slot.mark_id).unwrap();
        assert_eq!(a.part.geometry, mark.geometry);
        assert_eq!(a.part.raw_value, mark.value);
        assert_eq!(a.part.row, mark.series);
        assert_eq!(a.row_index, 1 - b.row_index);
        assert_ne!(a.part.raw_value, b.part.raw_value);
        assert_ne!(a.track.width, b.track.width);
        // Retained zero endpoints count toward sorted rank without fake marks.
        assert_eq!(
            a.rank,
            if a.part.kind == BulletPartKind::Marker {
                0
            } else {
                1
            }
        );
    }
}

#[test]
fn placement_callbacks_skip_missing_marks_and_reject_stale_metadata() {
    let data = [BulletDatum {
        id: "part".into(),
        ranges: vec![0., 20.],
        measures: vec![0., 30.],
        markers: vec![40.],
    }];
    let mut layout = bullet_layout(
        &ChartConfig::default(),
        &data,
        &BulletLayoutOptions::default(),
    )
    .unwrap();
    let mut kinds = vec![];
    let packet = bullet_part_widgets(&layout, 0, |c| {
        kinds.push((c.part.kind, c.rank, c.part.source_index));
        Ok(None)
    })
    .unwrap();
    assert!(packet.data.is_empty() && packet.layout.slots.is_empty());
    assert!(kinds.contains(&(BulletPartKind::Range, 1, Some(1))));
    assert!(kinds.contains(&(BulletPartKind::Range, 2, None)));
    assert!(
        !kinds
            .iter()
            .any(|(kind, _, source)| *source == Some(0) && *kind != BulletPartKind::Marker)
    );
    let part = layout.rows[0]
        .parts
        .iter_mut()
        .find(|p| p.source_index == Some(1))
        .unwrap();
    part.raw_value += 1.;
    assert!(bullet_part_widgets(&layout, 0, |_| Ok(None)).is_err());
}

#[test]
fn placement_failure_returns_no_partial_packet_or_changed_scene() {
    let data = [BulletDatum {
        id: "a".into(),
        ranges: vec![20., 80.],
        measures: vec![40.],
        markers: vec![60.],
    }];
    let layout = bullet_layout(
        &ChartConfig::default(),
        &data,
        &BulletLayoutOptions::default(),
    )
    .unwrap();
    let before = layout.clone();
    let mut calls = 0;
    let result = bullet_part_widgets(&layout, 0, |_| {
        calls += 1;
        if calls == 2 {
            return Err(ChartError("placement rejected".into()));
        }
        Ok(Some(PlotWidgetPlacement::Rect(LayoutRect {
            x: 20.,
            y: 20.,
            width: 40.,
            height: 20.,
        })))
    });
    assert!(result.is_err());
    assert_eq!(calls, 2);
    assert_eq!(layout, before);
}
