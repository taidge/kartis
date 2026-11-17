//! Private host composition state; business identity remains authored.
use super::*;

impl SurfaceModel {
    pub(crate) fn widget_replacements(&self) -> &BTreeSet<String> {
        &self.widget_replacements
    }

    /// Admit a physical ink mask without changing the authored Scene, logical
    /// controls or selection. Invalid masks leave every accepted field intact.
    pub(crate) fn set_widget_replacements(
        &mut self,
        ids: BTreeSet<String>,
    ) -> Result<Vec<SurfaceEvent>> {
        ensure(
            ids.len() <= crate::MAX_MARKS && ids.iter().all(|id| self.lookup.contains_key(id)),
            "Widget replacement requires current physical mark IDs",
        )?;
        if ids == self.widget_replacements {
            return Ok(Vec::new());
        }
        let mut next = self.clone();
        next.widget_replacements = ids;
        next.reindex()?;
        let mut events = next.reconcile();
        // A replacement changes actual pointer eligibility even during a live
        // click/drag session. Its eventual Up also uses the new projection.
        if let Some(point) = next.hover_point {
            let picked = next.pick(point)?;
            next.hover(picked, &mut events);
        }
        next.physical_events(self, &mut events);
        *self = next;
        Ok(events)
    }

    pub(super) fn pointer_data(&self, mark: &Mark) -> bool {
        !self.widget_replacements.contains(&mark.id) && self.data(mark)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ChartConfig, Color, DrawItem, Geometry, InteractionBinding, InteractionPartDatum,
        InteractionSite, Label, LabelWhitespace, LegendItem, MarkOpacityGroup, PickingProfile,
        RadarInteractionOptions, RadarLayoutOptions, RadarSeries, TextAnchor,
    };

    fn ids(values: &[&str]) -> BTreeSet<String> {
        values.iter().map(|id| (*id).into()).collect()
    }
    fn scene() -> Scene {
        let mut scene = ChartConfig {
            width: 240.,
            height: 180.,
            ..Default::default()
        }
        .scene()
        .unwrap();
        for (id, x, y) in [
            ("lower", 50., 50.),
            ("upper", 50., 50.),
            ("distant", 150., 100.),
        ] {
            let mut mark = Mark::new(
                id,
                Geometry::Circle { x, y, radius: 12. },
                Color(30, 90, 180, 255),
            );
            mark.series = id.into();
            mark.label = id.into();
            mark.value = if id == "upper" { 42. } else { 7. };
            scene.marks.push(mark);
        }
        scene
    }
    fn binding(scene: &Scene, strict: bool) -> InteractionBinding {
        InteractionBinding {
            sites: scene
                .marks
                .iter()
                .filter(|m| !m.label.is_empty())
                .map(|m| InteractionSite {
                    mark_id: m.id.clone(),
                    series: m.series.clone(),
                    label: m.label.clone(),
                    value: Some(m.value),
                    formatted_value: None,
                    color: Color(30, 90, 180, 255),
                    position: crate::crosshair::mark_center(m).unwrap(),
                })
                .collect(),
            picking_profile: if strict {
                PickingProfile::PaintedGeometry
            } else {
                PickingProfile::LegacyGeometry
            },
            ..Default::default()
        }
    }
    fn move_to(model: &mut SurfaceModel, p: (f64, f64)) {
        model.apply(SurfaceInput::Move { id: 1, point: p }).unwrap();
    }

    #[test]
    fn paint_projection_remaps_only_marks_and_preserves_all_label_and_business_metadata() {
        let mut source = scene();
        for (i, text) in ["axis", "row title"].into_iter().enumerate() {
            source.labels.push(Label {
                x: 20.,
                y: 20. + i as f64 * 20.,
                text: text.into(),
                color: Color(0, 0, 0, 255),
                size: 12.,
                anchor: TextAnchor::Start,
                rotation: 0.,
            });
            source.label_series.insert(i, "upper".into());
        }
        source
            .typography
            .label_whitespace
            .insert(1, LabelWhitespace::Preserve);
        source.typography.labels.insert(
            0,
            crate::FontOptions {
                weight: Some(650.),
                ..Default::default()
            },
        );
        source.label_clips.insert(
            0,
            crate::ClipPath {
                geometry: crate::Geometry::Rect {
                    x: 0.,
                    y: 0.,
                    width: 100.,
                    height: 50.,
                },
                rule: crate::ClipRule::NonZero,
            },
        );
        source.legend.push(LegendItem {
            id: "upper".into(),
            label: "Source legend".into(),
            color: Color(30, 90, 180, 255),
        });
        source.draw_order = vec![
            DrawItem::Label(0),
            DrawItem::Mark(1),
            DrawItem::Mark(0),
            DrawItem::Label(1),
            DrawItem::Mark(2),
        ];
        source.opacity_groups = vec![
            MarkOpacityGroup {
                id: "overlap".into(),
                members: vec!["upper".into(), "lower".into()],
                opacity: 0.5,
            },
            MarkOpacityGroup {
                id: "last".into(),
                members: vec!["distant".into()],
                opacity: 0.7,
            },
        ];
        source.validate().unwrap();
        let before = source.clone();
        let paint = source.without_mark_ink(&ids(&["upper"]));
        paint.validate().unwrap();
        assert_eq!(
            paint
                .marks
                .iter()
                .map(|m| m.id.as_str())
                .collect::<Vec<_>>(),
            ["lower", "distant"]
        );
        assert_eq!(
            paint.draw_order,
            [
                DrawItem::Label(0),
                DrawItem::Mark(0),
                DrawItem::Label(1),
                DrawItem::Mark(1)
            ]
        );
        assert_eq!(paint.opacity_groups[0].members, ["lower"]);
        assert_eq!(paint.opacity_groups[0].opacity, 0.5);
        assert_eq!(paint.labels, source.labels);
        assert_eq!(paint.label_series, source.label_series);
        assert_eq!(paint.label_clips, source.label_clips);
        assert_eq!(paint.typography, source.typography);
        assert_eq!(paint.legend, source.legend);
        let all = source.without_mark_ink(&ids(&["upper", "lower", "distant"]));
        all.validate().unwrap();
        assert!(all.marks.is_empty() && all.opacity_groups.is_empty());
        assert_eq!(all.draw_order, [DrawItem::Label(0), DrawItem::Label(1)]);
        assert_eq!(all.labels, source.labels);
        assert_eq!(source, before);
        assert_eq!(source.without_mark_ink(&BTreeSet::new()), source);
    }

    #[test]
    fn legacy_exact_nearest_and_both_slices_exclude_ink_without_retiring_logical_data() {
        for typed in [false, true] {
            for zoom in [false, true] {
                for picking in [
                    Picking::Geometry,
                    Picking::Nearest { distance: 2. },
                    Picking::SliceX { distance: 2. },
                    Picking::SliceY { distance: 2. },
                ] {
                    let source = scene();
                    let data = typed.then(|| binding(&source, false));
                    let mut model = SurfaceModel::new_with_interaction(
                        source.clone(),
                        SurfaceOptions {
                            picking,
                            ..Default::default()
                        },
                        data,
                    )
                    .unwrap();
                    if zoom {
                        model
                            .apply(SurfaceInput::Wheel {
                                point: (0., 0.),
                                delta: -2f64.ln() / 0.002,
                            })
                            .unwrap();
                    }
                    let p = model.paint_transform().project((50., 50.)).unwrap();
                    move_to(&mut model, p);
                    assert!(!model.hovered().is_empty());
                    let controls = model.controls_scene();
                    model.set_selected(Some("upper".into())).unwrap();
                    model.set_widget_replacements(ids(&["upper"])).unwrap();
                    assert_eq!(
                        model.hovered(),
                        ["lower"],
                        "{picking:?}, typed={typed}, zoom={zoom}"
                    );
                    assert_eq!(model.scene(), &source);
                    assert_eq!(model.controls_scene(), controls);
                    assert!(model.logical_datum("upper").is_some());
                    assert!(model.datum("upper").is_some());
                    assert_eq!(model.selected(), Some("upper"));
                    if matches!(picking, Picking::SliceX { .. } | Picking::SliceY { .. }) {
                        assert_eq!(
                            model
                                .interaction_ids(&["upper".into(), "lower".into()])
                                .into_iter()
                                .collect::<BTreeSet<_>>(),
                            ids(&["upper", "lower"])
                        );
                    }
                    model
                        .set_widget_replacements(ids(&["upper", "lower"]))
                        .unwrap();
                    assert!(model.hovered().is_empty());
                    assert_eq!(model.interaction_ids(&["upper".into()]), ["upper"]);
                    assert!(
                        model
                            .activate("upper")
                            .unwrap()
                            .iter()
                            .any(|e| matches!(e,SurfaceEvent::Activated(id) if id=="upper"))
                    );
                    model.set_widget_replacements(BTreeSet::new()).unwrap();
                    assert!(!model.hovered().is_empty());
                    assert!(model.widget_replacements().is_empty());
                }
            }
        }
    }

    #[test]
    fn strict_physical_aliases_and_tokens_are_independent_of_replaced_logical_body() {
        let mut source = scene();
        let mut alias = source.marks[1].clone();
        alias.id = "upper:alias".into();
        alias.label.clear();
        alias.geometry = Geometry::Circle {
            x: 95.,
            y: 50.,
            radius: 12.,
        };
        source.marks.push(alias);
        let mut data = binding(&source, true);
        data.ink_owners.insert("upper:alias".into(), "upper".into());
        data.part_data.insert(
            "upper:alias".into(),
            InteractionPartDatum {
                source_id: "original alias".into(),
                value: 17.,
            },
        );
        let mut model = SurfaceModel::new_with_interaction(
            source.clone(),
            SurfaceOptions::default(),
            Some(data),
        )
        .unwrap();
        move_to(&mut model, (50., 50.));
        let old_hit = model.hovered_physical_hit().unwrap().clone();
        let controls = model.controls_scene();
        assert_eq!(old_hit.physical_mark_id, "upper");
        let events = model.set_widget_replacements(ids(&["upper"])).unwrap();
        assert_eq!(model.hovered(), ["lower"]);
        assert_eq!(
            model.hovered_physical_hit().unwrap().physical_mark_id,
            "lower"
        );
        assert!(!model.is_current_hit(&old_hit));
        assert!(
            matches!(events.last(),Some(SurfaceEvent::PhysicalHover(Some(hit))) if hit.physical_mark_id=="lower")
        );
        move_to(&mut model, (95., 50.));
        let alias_hit = model.hovered_physical_hit().unwrap().clone();
        assert_eq!(alias_hit.physical_mark_id, "upper:alias");
        assert_eq!(alias_hit.logical_site_id, "upper");
        assert_eq!(
            alias_hit.source_data.as_ref().unwrap().source_id,
            "original alias"
        );
        let token = model.frame_token();
        for invalid in [ids(&["missing"]), ids(&[""]), ids(&["upper", "missing"])] {
            assert!(model.set_widget_replacements(invalid).is_err());
            assert_eq!(model.frame_token(), token);
            assert_eq!(model.hovered_physical_hit(), Some(&alias_hit));
            assert!(model.is_current_hit(&alias_hit));
            assert_eq!(model.widget_replacements(), &ids(&["upper"]));
        }
        assert!(
            model
                .set_widget_replacements(ids(&["upper"]))
                .unwrap()
                .is_empty()
        );
        assert_eq!(model.frame_token(), token);
        model
            .set_widget_replacements(ids(&["upper", "upper:alias"]))
            .unwrap();
        assert!(model.hovered().is_empty());
        assert!(model.physical_hit_at((95., 50.)).unwrap().is_none());
        assert_eq!(model.controls_scene(), controls);
        assert!(model.datum("upper").is_some());
        model.set_selected(Some("upper".into())).unwrap();
        assert!(model.activate("upper").is_ok());
        model
            .set_widget_replacements(ids(&["upper:alias"]))
            .unwrap();
        assert_eq!(
            model
                .physical_hit_at((50., 50.))
                .unwrap()
                .unwrap()
                .physical_mark_id,
            "upper"
        );
        model.set_widget_replacements(BTreeSet::new()).unwrap();
        assert_eq!(
            model.hovered_physical_hit().unwrap().physical_mark_id,
            "upper:alias"
        );
    }

    #[test]
    fn excluded_ink_still_has_full_strict_source_validation_and_failed_layout_is_atomic() {
        let source = scene();
        let data = binding(&source, true);
        let mut model = SurfaceModel::new_with_interaction(
            source.clone(),
            SurfaceOptions::default(),
            Some(data.clone()),
        )
        .unwrap();
        model.set_widget_replacements(ids(&["upper"])).unwrap();
        move_to(&mut model, (50., 50.));
        let hit = model.hovered_physical_hit().cloned();
        let token = model.frame_token();
        let mut bad = source.clone();
        bad.marks[1].fill_paint = Some(crate::FillPaint::Linear {
            start: [0., 0.],
            end: [100., 0.],
            stops: vec![
                crate::GradientStop::new(0., Color(0, 0, 0, 255)),
                crate::GradientStop::new(1., Color(255, 0, 0, 255)),
            ],
        });
        bad.validate().unwrap();
        assert!(model.set_scene_with_interaction(bad, Some(data)).is_err());
        assert_eq!(model.scene(), &source);
        assert_eq!(model.widget_replacements(), &ids(&["upper"]));
        assert_eq!(model.frame_token(), token);
        assert_eq!(model.hovered_physical_hit(), hit.as_ref());
    }

    #[test]
    fn legacy_and_editable_brushes_and_touch_exclude_only_the_replaced_ink() {
        let mut model = SurfaceModel::new(
            scene(),
            SurfaceOptions {
                pan: false,
                ..Default::default()
            },
        )
        .unwrap();
        model.set_widget_replacements(ids(&["upper"])).unwrap();
        model
            .apply(SurfaceInput::Down {
                id: 1,
                point: (20., 20.),
                brush: true,
            })
            .unwrap();
        let events = model
            .apply(SurfaceInput::Up {
                id: 1,
                point: (80., 80.),
            })
            .unwrap();
        assert!(events.iter().any(
            |event| matches!(event,SurfaceEvent::Brush{ids,complete:true,..} if ids==&["lower"])
        ));
        model
            .configure_editable_brush(
                Some(crate::EditableBrushOptions {
                    extent: Brush {
                        x0: 0.,
                        y0: 0.,
                        x1: 240.,
                        y1: 180.,
                    },
                    ..Default::default()
                }),
                crate::BrushReconfigure::Preserve,
            )
            .unwrap();
        model
            .set_brush_selection(Some(Brush {
                x0: 20.,
                y0: 20.,
                x1: 80.,
                y1: 80.,
            }))
            .unwrap();
        assert_eq!(model.editable_brush_ids().unwrap(), ["lower"]);
        model
            .set_widget_replacements(ids(&["lower", "upper"]))
            .unwrap();
        assert!(model.editable_brush_ids().unwrap().is_empty());
        model
            .configure_editable_brush(None, crate::BrushReconfigure::Preserve)
            .unwrap();
        model.set_selected(None).unwrap();
        model
            .apply(SurfaceInput::TouchStart {
                id: 7,
                point: (50., 50.),
            })
            .unwrap();
        assert!(
            !model
                .apply(SurfaceInput::TouchEnd {
                    id: 7,
                    point: (50., 50.)
                })
                .unwrap()
                .iter()
                .any(|e| matches!(e, SurfaceEvent::Activated(_)))
        );
        model.set_widget_replacements(ids(&["upper"])).unwrap();
        model
            .apply(SurfaceInput::TouchStart {
                id: 8,
                point: (50., 50.),
            })
            .unwrap();
        assert!(
            model
                .apply(SurfaceInput::TouchEnd {
                    id: 8,
                    point: (50., 50.)
                })
                .unwrap()
                .iter()
                .any(|e| matches!(e,SurfaceEvent::Activated(id) if id=="lower"))
        );
    }

    #[test]
    fn polar_pointer_and_expanded_tooltip_do_not_resurrect_replaced_members() {
        let layout = crate::radar_layout(
            &ChartConfig {
                width: 300.,
                height: 300.,
                ..Default::default()
            },
            &["x".into(), "y".into(), "z".into()],
            &[
                RadarSeries::new("A", vec![2., 3., 4.]),
                RadarSeries::new("B", vec![7., 8., 9.]),
            ],
            &RadarLayoutOptions {
                domain: Some([0., 10.]),
                ..Default::default()
            },
            &RadarInteractionOptions::default(),
        )
        .unwrap();
        let members = layout.slices[0]
            .rows
            .iter()
            .map(|r| r.point_mark_id.clone())
            .collect::<Vec<_>>();
        let anchor = layout.polar.slices()[0].anchor;
        let point = (
            (anchor[0] + layout.center[0]) / 2.,
            (anchor[1] + layout.center[1]) / 2.,
        );
        let mut model = SurfaceModel::new_with_interaction(
            layout.scene.clone(),
            SurfaceOptions::default(),
            Some(layout.interaction_binding()),
        )
        .unwrap();
        move_to(&mut model, point);
        assert_eq!(model.hovered().len(), 2);
        model
            .set_widget_replacements([members[0].clone()].into_iter().collect())
            .unwrap();
        assert_eq!(model.hovered(), &members[1..]);
        assert_eq!(
            model.active_interaction(model.hovered()).unwrap().ids,
            &members[1..]
        );
        model
            .set_widget_replacements(members.iter().cloned().collect())
            .unwrap();
        assert!(model.hovered().is_empty());
        model.set_selected(Some(members[0].clone())).unwrap();
        assert_eq!(
            model
                .active_interaction(std::slice::from_ref(&members[0]))
                .unwrap()
                .ids,
            [members[0].clone()]
        );
        model.set_widget_replacements(BTreeSet::new()).unwrap();
        assert_eq!(model.hovered().len(), 2);
    }

    #[test]
    fn source_removal_retires_old_physical_masks_but_rejects_invalid_new_ids() {
        let mut model = SurfaceModel::new(scene(), SurfaceOptions::default()).unwrap();
        model.set_widget_replacements(ids(&["upper"])).unwrap();
        let mut next = scene();
        next.marks.retain(|m| m.id != "upper");
        model.set_scene(next.clone()).unwrap();
        assert!(model.widget_replacements().is_empty());
        assert_eq!(model.scene(), &next);
        assert!(model.set_widget_replacements(ids(&["upper"])).is_err());
        model.set_widget_replacements(ids(&["lower"])).unwrap();
        next.marks.retain(|m| m.id != "lower");
        model
            .relayout(
                next.clone(),
                ClipRect {
                    x: 0.,
                    y: 0.,
                    width: 240.,
                    height: 180.,
                },
                ZoomTransform::default(),
                [1., 20.],
                false,
            )
            .unwrap();
        assert!(model.widget_replacements().is_empty());
        assert_eq!(model.scene(), &next);
    }
}
