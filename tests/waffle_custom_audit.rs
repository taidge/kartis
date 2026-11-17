use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use kartis::surface::{Picking, SurfaceInput, SurfaceModel, SurfaceOptions};
use kartis::*;

fn one_cell() -> WaffleLayoutOptions {
    WaffleLayoutOptions {
        rows: 1,
        columns: 1,
        ..Default::default()
    }
}

fn surface(layout: &WaffleLayout, picking: Picking) -> SurfaceModel {
    SurfaceModel::new_with_interaction(
        layout.scene.clone(),
        SurfaceOptions {
            picking,
            ..Default::default()
        },
        Some(layout.interaction.clone()),
    )
    .unwrap()
}

fn hover(model: &mut SurfaceModel, p: [f64; 2]) -> Vec<String> {
    model
        .apply(SurfaceInput::Move {
            id: 1,
            point: (p[0], p[1]),
        })
        .unwrap();
    model.hovered().to_vec()
}

fn command_count(scene: &Scene) -> usize {
    scene
        .marks
        .iter()
        .map(|mark| match &mark.geometry {
            Geometry::Path(path) => path.len(),
            _ => 0,
        })
        .sum()
}

#[test]
fn area_union_commands_share_the_final_budget_with_replacement_empty_ink() {
    let c = ChartConfig::default();
    let data = [RadialDatum::new("occupied", 1.)];
    let o = WaffleLayoutOptions {
        rows: 1,
        columns: 2,
        total: Some(2.),
        mode: WaffleMode::Areas,
        corner_radius: 4.,
        ..Default::default()
    };
    let original = waffle_layout(&c, &data, &o).unwrap();
    let area_id = original.groups[0].area_mark_id.as_ref().unwrap();
    let area = original
        .scene
        .marks
        .iter()
        .find(|m| &m.id == area_id)
        .unwrap();
    let retained_commands = match &area.geometry {
        Geometry::Path(path) => path.len(),
        _ => panic!("occupied area must retain its union path"),
    };
    assert!(retained_commands > 0);
    assert!(command_count(&original.scene) > retained_commands);

    for (extra, accepted) in [(0, true), (1, false)] {
        let mut calls = 0;
        let result = waffle_layout_with_cell_factory(&c, &data, &o, &mut |ctx| {
            calls += 1;
            assert!(ctx.group.is_none());
            let point = P::new(ctx.position[0], ctx.position[1]);
            let count = 200_000 - retained_commands + extra;
            let mut commands = vec![C::Move(point)];
            commands.extend((1..count).map(|_| C::Line(point)));
            let mut custom = ctx.inherit(Geometry::Path(commands));
            custom.anchor_commands = Some(vec![0]);
            Ok(custom)
        });
        assert_eq!(calls, 1);
        assert_eq!(result.is_ok(), accepted);
        if let Ok(layout) = result {
            assert_eq!(command_count(&layout.scene), 200_000);
            assert_eq!(
                layout.scene.marks.iter().find(|m| &m.id == area_id),
                Some(area)
            );
            assert_eq!(layout.interaction, original.interaction);
        } else {
            assert!(result.unwrap_err().0.contains("geometry budget"));
        }
    }
}

#[test]
fn empty_painted_cell_is_geometrically_queryable_without_becoming_a_datum() {
    let layout =
        waffle_layout_with_cell_factory(&ChartConfig::default(), &[], &one_cell(), &mut |ctx| {
            let mut custom = ctx.inherit(Geometry::Circle {
                x: ctx.position[0],
                y: ctx.position[1],
                radius: 6.,
            });
            custom.fill = Some(Color(180, 20, 60, 255));
            Ok(custom)
        })
        .unwrap();
    let cell = &layout.cells[0];
    let p = [
        cell.bounds.x + cell.bounds.width / 2.,
        cell.bounds.y + cell.bounds.height / 2.,
    ];
    assert_eq!(
        layout.scene.hit_test(p[0], p[1]).unwrap().id,
        *cell.mark_id.as_ref().unwrap()
    );
    assert!(layout.interaction.sites.is_empty());
    assert!(layout.interaction.anchors.is_empty());
    assert!(layout.interaction.ink_owners.is_empty());
    for picking in [Picking::Geometry, Picking::Nearest { distance: 20. }] {
        assert!(hover(&mut surface(&layout, picking), p).is_empty());
    }
    assert_eq!(layout.cell_at(p[0], p[1]), Some(cell));
    assert!(layout.area_at(p[0], p[1]).is_none());
}

#[test]
fn effective_gradient_and_dashed_stroke_colors_do_not_recolor_source_or_legend() {
    let c = ChartConfig::default();
    let data = [RadialDatum::new("occupied", 1.)];
    let o = WaffleLayoutOptions {
        rows: 1,
        columns: 2,
        ..Default::default()
    };
    let base = waffle_layout(&c, &data, &o).unwrap();
    let gradient_color = Color(170, 20, 90, 255);
    let stroke_color = Color(20, 180, 40, 255);
    let custom = waffle_layout_with_cell_factory(&c, &data, &o, &mut |ctx| {
        let [x, y] = ctx.position;
        let mut value = ctx.inherit(Geometry::Path(vec![
            C::Move(P::new(x - 20., y)),
            C::Line(P::new(x + 20., y)),
        ]));
        value.fill = Some(Color(255, 0, 0, 0));
        if ctx.cell.index == 0 {
            value.fill_paint = Some(FillPaint::Linear {
                start: [x - 20., y],
                end: [x + 20., y],
                stops: vec![
                    GradientStop::new(0., Color(0, 0, 0, 0)),
                    GradientStop::new(1., gradient_color),
                ],
            });
        } else {
            value.stroke = Some(GraphicStroke {
                color: stroke_color,
                width: 4.,
                cap: StrokeCap::Butt,
            });
            value.stroke_dash = Some(StrokeDash {
                segments: vec![10., 10.],
                offset: 0.,
            });
        }
        Ok(value)
    })
    .unwrap();
    assert_eq!(custom.groups, base.groups);
    assert_eq!(custom.scene.legend, base.scene.legend);
    assert_eq!(custom.cells, base.cells);
    assert_eq!(custom.interaction.sites[0].color, gradient_color);
    assert_eq!(custom.interaction.sites[1].color, stroke_color);
    assert_eq!(
        custom.interaction.sites[0].position,
        base.interaction.sites[0].position
    );
    let p = custom.interaction.sites[1].position;
    let id = &custom.interaction.sites[1].mark_id;
    let mut exact = surface(&custom, Picking::Geometry);
    assert_eq!(
        hover(&mut exact, [p[0] - 15., p[1]]).as_slice(),
        std::slice::from_ref(id)
    );
    assert!(hover(&mut exact, [p[0] - 5., p[1]]).is_empty());
    // Legacy geometry picking includes two scene units of stroke tolerance.
    assert!(hover(&mut exact, [p[0] - 15., p[1] + 5.]).is_empty());
    let svg = render::to_svg(&custom.scene).unwrap();
    assert!(svg.contains("stroke-dasharray=\"10 10\""));
    assert!(svg.contains("<linearGradient"));
}

#[test]
fn very_wide_centered_rect_preserves_fractional_slot_anchor_exactly() {
    let c = ChartConfig {
        width: 100.74,
        height: 100.,
        margin: Margin {
            top: 0.,
            right: 0.,
            bottom: 0.,
            left: 0.,
        },
        ..Default::default()
    };
    let data = [RadialDatum::new("occupied", 1.)];
    let original = waffle_layout(&c, &data, &one_cell()).unwrap();
    let origin = original.interaction.sites[0].position;
    let mut rect = None;
    let custom = waffle_layout_with_cell_factory(&c, &data, &one_cell(), &mut |ctx| {
        let [x, y] = ctx.position;
        let width = [128., 512., 65_536., 1_000_000.]
            .into_iter()
            .find(|width| (x - width / 2.) + width / 2. != x)
            .expect("fractional center must exercise bounded f64 reconstruction loss");
        let geometry = Geometry::Rect {
            x: x - width / 2.,
            y: y - 3.,
            width,
            height: 6.,
        };
        rect = Some(geometry.clone());
        Ok(ctx.inherit(geometry))
    })
    .unwrap();
    let site = &custom.interaction.sites[0];
    let geometry = &custom
        .scene
        .marks
        .iter()
        .find(|m| m.id == site.mark_id)
        .unwrap()
        .geometry;
    assert_eq!(Some(geometry), rect.as_ref());
    assert_eq!(site.position, origin);
    assert_eq!(
        custom.interaction.anchors[&site.mark_id]
            .resolve(geometry)
            .unwrap(),
        origin
    );
    assert_eq!(
        hover(
            &mut surface(&custom, Picking::Nearest { distance: 0. }),
            origin
        )
        .as_slice(),
        std::slice::from_ref(&site.mark_id)
    );
}

#[test]
fn offset_path_anchor_follows_delivered_animation_and_rejected_surface_update_keeps_state() {
    let c = ChartConfig::default();
    let data = [RadialDatum::new("occupied", 7.)];
    let layout = waffle_layout_with_cell_factory(&c, &data, &one_cell(), &mut |ctx| {
        let [x, y] = ctx.position;
        let mut custom = ctx.inherit(Geometry::Path(vec![
            C::Move(P::new(x + 20., y - 4.)),
            C::Line(P::new(x + 30., y - 4.)),
            C::Line(P::new(x + 30., y + 4.)),
            C::Line(P::new(x + 20., y + 4.)),
            C::Close,
        ]));
        custom.anchor_commands = Some(vec![0, 1, 2, 3]);
        Ok(custom)
    })
    .unwrap();
    let id = layout.interaction.sites[0].mark_id.clone();
    let origin = layout.interaction.sites[0].position;
    let mut target = layout.scene.clone();
    for command in match &mut target.marks[0].geometry {
        Geometry::Path(path) => path,
        _ => unreachable!(),
    } {
        if let C::Move(p) | C::Line(p) = command {
            p.x += 40.;
            p.y -= 10.;
        }
    }
    let frame = layout.scene.interpolate(&target, 0.375).unwrap();
    let expected = [origin[0] + 15., origin[1] - 3.75];
    let mut model = surface(&layout, Picking::Nearest { distance: 0.001 });
    model
        .set_scene_preserving_interaction(frame.clone())
        .unwrap();
    assert_eq!(
        hover(&mut model, expected).as_slice(),
        std::slice::from_ref(&id)
    );
    let active = model.active_interaction(std::slice::from_ref(&id)).unwrap();
    assert_eq!(active.anchor, expected);
    assert_eq!(active.rows[0].value, Some(7.));
    assert!(hover(&mut model, origin).is_empty());
    assert_eq!(
        hover(&mut model, expected).as_slice(),
        std::slice::from_ref(&id)
    );
    model.apply(SurfaceInput::First).unwrap();
    let before = model.scene().clone();
    let binding = model.interaction_data().cloned();
    let token = model.frame_token();
    let hovered = model.hovered().to_vec();
    let selected = model.selected().map(str::to_owned);
    let mut incompatible = frame.clone();
    incompatible.marks[0].geometry = Geometry::Circle {
        x: expected[0],
        y: expected[1],
        radius: 8.,
    };
    assert!(
        model
            .set_scene_preserving_interaction(incompatible)
            .is_err()
    );
    let mut invalid = frame;
    invalid.marks[0].opacity = f64::NAN;
    assert!(
        model
            .set_scene_with_interaction(invalid, binding.clone())
            .is_err()
    );
    assert_eq!(model.scene(), &before);
    assert_eq!(model.interaction_data(), binding.as_ref());
    assert_eq!(model.frame_token(), token);
    assert_eq!(model.hovered(), hovered);
    assert_eq!(model.selected(), selected.as_deref());
    assert_eq!(model.active_interaction(&[id]).unwrap().anchor, expected);
}
