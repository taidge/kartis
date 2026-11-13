use kartis::*;
fn rect(x: f64, y: f64, w: f64, h: f64) -> LayoutRect {
    LayoutRect {
        x,
        y,
        width: w,
        height: h,
    }
}
fn size(w: f64, h: f64) -> TextSize {
    TextSize {
        width: w,
        height: h,
    }
}

#[test]
fn horizontal_legend_wraps_rows_and_vertical_legend_packs_columns_with_stable_ids() {
    let items = (0..5)
        .map(|i| LegendMeasuredItem {
            id: i.to_string(),
            size: size(30., 20.),
        })
        .collect::<Vec<_>>();
    let layout = layout_legend(
        &items,
        &LegendLayoutOptions {
            bounds: rect(10., 20., 100., 100.),
            max_columns: 2,
            row_gap: 5.,
            column_gap: 10.,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(layout.items[0].rect, rect(10., 20., 30., 20.));
    assert_eq!(layout.items[1].rect, rect(50., 20., 30., 20.));
    assert_eq!(layout.items[2].rect, rect(10., 45., 30., 20.));
    assert_eq!(layout.items[4].rect, rect(10., 70., 30., 20.));
    let vertical = layout_legend(
        &items,
        &LegendLayoutOptions {
            bounds: rect(0., 0., 100., 70.),
            direction: LegendDirection::Vertical,
            max_columns: 2,
            row_gap: 5.,
            column_gap: 10.,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(vertical.items[2].rect, rect(0., 50., 30., 20.));
    assert_eq!(vertical.items[3].rect, rect(40., 0., 30., 20.));
    assert_eq!(
        vertical
            .items
            .iter()
            .map(|i| i.id.as_str())
            .collect::<Vec<_>>(),
        ["0", "1", "2", "3", "4"]
    );
}
#[test]
fn legend_alignment_uses_measured_variable_extents_and_rejects_overflow() {
    let items = [
        LegendMeasuredItem {
            id: "a".into(),
            size: size(20., 10.),
        },
        LegendMeasuredItem {
            id: "b".into(),
            size: size(40., 20.),
        },
    ];
    let options = LegendLayoutOptions {
        bounds: rect(0., 0., 100., 40.),
        column_gap: 10.,
        align: PresentationAlign::End,
        ..Default::default()
    };
    let layout = layout_legend(&items, &options).unwrap();
    assert_eq!(layout.items[0].rect.x, 30.);
    assert_eq!(layout.items[1].rect.x, 60.);
    assert!(
        layout_legend(
            &items,
            &LegendLayoutOptions {
                bounds: rect(0., 0., 50., 10.),
                wrap: false,
                ..options.clone()
            }
        )
        .is_err()
    );
    assert!(layout_legend(&[items[0].clone(), items[0].clone()], &options).is_err());
}
#[test]
fn tooltip_chooses_clear_anchor_and_keeps_all_content_inside_viewport() {
    let options = TooltipLayoutOptions {
        viewport: rect(0., 0., 200., 120.),
        preferred: vec![TooltipAnchor::BottomRight, TooltipAnchor::TopLeft],
        gap: 5.,
        viewport_padding: 4.,
        forbidden: vec![rect(100., 60., 100., 60.)],
    };
    let placed = place_tooltip((100., 60.), size(60., 30.), &options).unwrap();
    assert_eq!(placed.anchor, TooltipAnchor::TopLeft);
    assert_eq!(placed.rect, rect(35., 25., 60., 30.));
    assert_eq!(placed.overlap_area, 0.);
    assert!(!placed.clamped);
    for point in [(0., 0.), (200., 120.), (-100., 500.)] {
        let p = place_tooltip(point, size(60., 30.), &options).unwrap();
        assert!(rect(4., 4., 192., 112.).contains(p.rect));
    }
}
#[test]
fn tooltip_tries_obstacle_edges_and_reports_impossible_overlap_or_oversize() {
    let options = TooltipLayoutOptions {
        viewport: rect(0., 0., 100., 100.),
        preferred: vec![TooltipAnchor::BottomRight],
        gap: 0.,
        viewport_padding: 0.,
        forbidden: vec![rect(20., 20., 60., 60.)],
    };
    let p = place_tooltip((40., 40.), size(15., 15.), &options).unwrap();
    assert_eq!(p.overlap_area, 0.);
    assert!(!options.forbidden[0].contains(p.rect));
    let blocked = TooltipLayoutOptions {
        forbidden: vec![rect(0., 0., 100., 100.)],
        ..options
    };
    let p = place_tooltip((50., 50.), size(200., 200.), &blocked).unwrap();
    assert_eq!(p.rect, rect(0., 0., 100., 100.));
    assert_eq!(p.overlap_area, 10_000.);
    assert!(p.clamped);
}
#[test]
fn collision_priority_is_stable_and_lower_labels_try_offsets_then_hide() {
    let input = vec![
        LabelCandidate {
            id: "low".into(),
            rect: rect(10., 10., 30., 10.),
            priority: 0,
            offsets: vec![(0., 20.)],
        },
        LabelCandidate {
            id: "high".into(),
            rect: rect(10., 10., 30., 10.),
            priority: 5,
            offsets: vec![],
        },
        LabelCandidate {
            id: "hidden".into(),
            rect: rect(10., 10., 30., 10.),
            priority: -1,
            offsets: vec![],
        },
    ];
    let placed = layout_labels(
        &input,
        &LabelCollisionOptions {
            viewport: rect(0., 0., 100., 100.),
            padding: 2.,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(placed[0].rect, Some(rect(10., 30., 30., 10.)));
    assert_eq!(placed[1].rect, Some(input[1].rect));
    assert_eq!(placed[2].rect, None);
    let same = input
        .iter()
        .take(2)
        .cloned()
        .map(|mut l| {
            l.priority = 0;
            l.offsets.clear();
            l
        })
        .collect::<Vec<_>>();
    let placed = layout_labels(&same, &LabelCollisionOptions::default()).unwrap();
    assert!(placed[0].rect.is_some() && placed[1].rect.is_none());
}
#[test]
fn collision_obstacles_and_work_budget_reject_transactionally() {
    let labels = (0..3)
        .map(|i| LabelCandidate {
            id: i.to_string(),
            rect: rect(i as f64 * 20., 10., 10., 10.),
            priority: 0,
            offsets: vec![],
        })
        .collect::<Vec<_>>();
    let original = labels.clone();
    assert!(
        layout_labels(
            &labels,
            &LabelCollisionOptions {
                max_checks: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert_eq!(labels, original);
    let placed = layout_labels(
        &labels,
        &LabelCollisionOptions {
            forbidden: vec![rect(0., 0., 15., 30.)],
            ..Default::default()
        },
    )
    .unwrap();
    assert!(placed[0].rect.is_none());
    assert!(placed[1].rect.is_some());
}
#[test]
fn finite_bounds_and_explicit_resource_profiles_reject_without_panics() {
    assert!(size(f64::NAN, 10.).validate().is_err());
    assert!(rect(1e6, 0., 10., 10.).validate().is_err());
    assert!(
        place_tooltip(
            (f64::INFINITY, 0.),
            size(1., 1.),
            &TooltipLayoutOptions::default()
        )
        .is_err()
    );
    assert!(
        place_tooltip(
            (0., 0.),
            size(1., 1.),
            &TooltipLayoutOptions {
                preferred: vec![],
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        TooltipLayoutOptions {
            viewport_padding: 1000.,
            ..Default::default()
        }
        .validate()
        .is_err()
    );
    assert!(
        LegendLayoutOptions {
            max_columns: 0,
            ..Default::default()
        }
        .validate()
        .is_err()
    );
    let bad = LabelCandidate {
        id: "bad".into(),
        rect: rect(1e6, 0., 0., 10.),
        priority: 0,
        offsets: vec![(1., 0.)],
    };
    assert!(
        layout_labels(
            &[bad],
            &LabelCollisionOptions {
                viewport: rect(0., 0., 10., 10.),
                ..Default::default()
            }
        )
        .is_err()
    );
}
