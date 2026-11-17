//! Geometry and paint projection checks use actual Label source items.
use super::*;
use crate::{
    ChartConfig, Color, DrawItem, FontOptions, Label, LabelWhitespace, SceneWidgetsData, TextAnchor,
};

fn fixture(rotation: f64) -> (Scene, PlotWidgetsPacket) {
    let mut scene = ChartConfig {
        width: 400.,
        height: 240.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    scene.labels = vec![
        Label {
            x: 100.,
            y: 80.,
            text: "  custom\tlabel ".into(),
            color: Color(0, 0, 0, 0),
            size: 12.,
            anchor: TextAnchor::End,
            rotation,
        },
        Label {
            x: 30.,
            y: 40.,
            text: "remaining".into(),
            color: Color(20, 30, 40, 255),
            size: 15.,
            anchor: TextAnchor::Start,
            rotation: 0.,
        },
    ];
    let mut mark = Mark::new(
        "node",
        Geometry::Circle {
            x: 100.,
            y: 80.,
            radius: 8.,
        },
        Color(30, 60, 90, 255),
    );
    mark.label = "business node".into();
    mark.series = "owner".into();
    scene.marks.push(mark);
    scene.label_series.insert(0, "owner".into());
    scene.label_series.insert(1, "other".into());
    scene.typography.labels.insert(
        1,
        FontOptions {
            weight: Some(650.),
            ..Default::default()
        },
    );
    scene
        .typography
        .label_whitespace
        .insert(1, LabelWhitespace::Preserve);
    scene.label_clips.insert(1, ClipPath::circle(30., 40., 20.));
    scene.draw_order = vec![DrawItem::Label(1), DrawItem::Mark(0), DrawItem::Label(0)];
    scene.validate().unwrap();
    let packet = PlotWidgetsPacket::scene_composition(SceneWidgetsComposition {
        widgets: SceneWidgetsData {
            layout: SceneWidgetsLayout {
                plot: None,
                slots: vec![SceneWidgetSlot {
                    key: "label-key".into(),
                    target: SceneWidgetTarget::Label(0),
                    revision: 0,
                    placement: PlotWidgetPlacement::AtDatum {
                        offset: [-10., -5.],
                        measured: TextSize {
                            width: 20.,
                            height: 10.,
                        },
                    },
                }],
            },
            data: BTreeMap::from([("label-key".into(), 7u32)]),
        },
        replace_ink: BTreeSet::from(["label-key".into()]),
    });
    (scene, packet)
}

#[test]
fn source_label_projection_remaps_every_metadata_pool_and_preserves_business_node() {
    let (scene, packet) = fixture(30.);
    let before = scene.clone();
    let paint = packet.project_scene(&scene);
    paint.validate().unwrap();
    assert_eq!(paint.labels, vec![scene.labels[1].clone()]);
    assert_eq!(paint.label_series, BTreeMap::from([(0, "other".into())]));
    assert_eq!(paint.label_clips.get(&0), scene.label_clips.get(&1));
    assert_eq!(
        paint.typography.labels.get(&0),
        scene.typography.labels.get(&1)
    );
    assert_eq!(
        paint.typography.label_whitespace.get(&0),
        Some(&LabelWhitespace::Preserve)
    );
    assert_eq!(
        paint.draw_order,
        vec![DrawItem::Label(0), DrawItem::Mark(0)]
    );
    assert_eq!(paint.marks, scene.marks);
    assert!(packet.replace_ink().is_empty()); // Labels never enter the physical mark-hit mask.
    assert_eq!(scene, before);
}

#[test]
fn label_affine_rotates_local_offset_and_whole_measured_box_before_camera() {
    let (scene, packet) = fixture(90.);
    let view = ZoomTransform::new(2., 7., 11.).unwrap();
    let rows = prepare_in_space_packet(
        &scene,
        &packet,
        Profile::HTML,
        SlotSpace {
            plot: None,
            transform: view,
            responsive: true,
            require_current_data: true,
        },
        |_| panic!("Label placement must not request a node anchor"),
        |_| None,
    )
    .unwrap();
    let frame = rows[0].frame(&BTreeSet::new());
    let SceneWidgetFrame::Label(frame) = frame else {
        panic!("real Label frame required")
    };
    assert_eq!(frame.label, scene.labels[0]);
    assert_eq!(frame.label_index, 0);
    let actual = frame.transform.unwrap();
    // Source (100,80)+R90*(-10,-5)=(105,70), camera=(217,151).
    let expected = [0., 2., -2., 0., 217., 151.];
    for (a, b) in actual.matrix.into_iter().zip(expected) {
        assert!((a - b).abs() < 1e-10);
    }
    assert!((frame.rect.x - 197.).abs() < 1e-10);
    assert!((frame.rect.y - 151.).abs() < 1e-10);
    assert!((frame.rect.width - 20.).abs() < 1e-10);
    assert!((frame.rect.height - 40.).abs() < 1e-10);
    assert!(frame.interactive); // Transparent source text is still a real custom-component role.
    assert_eq!(
        content_style(&SceneWidgetFrame::Label(frame.clone()), true).pointer_events,
        Some(PointerEvents::None)
    );
    assert!(prepare_packet(&scene, &packet, Profile::HTML, view, None).is_err()); // Native cannot fake a rotated subtree.
    let hidden = rows[0].frame(&BTreeSet::from(["owner".into()]));
    assert!(hidden.visible().is_none());
    assert!(!hidden.interactive());
}

#[test]
fn explicit_label_rect_is_axis_aligned_and_empty_measurement_remains_a_passive_role() {
    let (scene, mut packet) = fixture(41.);
    packet.layout.slots[0].placement = PlotWidgetPlacement::Rect(LayoutRect {
        x: 5.,
        y: 6.,
        width: 20.,
        height: 10.,
    });
    let rows = prepare_packet(
        &scene,
        &packet,
        Profile::HTML,
        ZoomTransform::default(),
        None,
    )
    .unwrap();
    assert!(rows[0].frame(&BTreeSet::new()).transform().is_none());
    packet.layout.slots[0].placement = PlotWidgetPlacement::AtDatum {
        offset: [0., 0.],
        measured: TextSize::default(),
    };
    let rows = prepare_in_space_packet(
        &scene,
        &packet,
        Profile::HTML,
        SlotSpace {
            plot: None,
            transform: ZoomTransform::default(),
            responsive: true,
            require_current_data: true,
        },
        |_| Ok(None),
        |_| None,
    );
    let rows = rows.unwrap();
    assert_eq!(rows.len(), 1);
    assert!(!rows[0].frame(&BTreeSet::new()).interactive());
}
