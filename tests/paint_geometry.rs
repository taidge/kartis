use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as Cmd, StrokeCap};
use kartis::mesh::PointMesh;
use kartis::surface::*;
use kartis::*;

fn scene(geometry: Geometry, clip: Option<ClipRect>, stroke: bool) -> Scene {
    let mut scene = ChartConfig::default().scene().unwrap();
    let mut mark = Mark::new("datum", geometry, Color(40, 110, 190, 255));
    mark.series = "series".into();
    mark.clip = clip;
    if stroke {
        mark.fill = None;
        mark.stroke = Some(GraphicStroke {
            color: Color(30, 50, 70, 255),
            width: 2.,
            cap: StrokeCap::Round,
        });
    }
    scene.marks.push(mark);
    scene.validate().unwrap();
    scene
}

#[test]
fn zero_area_clips_are_empty_for_hit_testing_mesh_and_every_pointer_picker() {
    for clip in [
        ClipRect {
            x: 100.,
            y: 0.,
            width: 0.,
            height: 200.,
        },
        ClipRect {
            x: 0.,
            y: 100.,
            width: 200.,
            height: 0.,
        },
    ] {
        clip.validate().unwrap();
        assert!(!clip.contains(100., 100.));
        let scene = scene(
            Geometry::Circle {
                x: 100.,
                y: 100.,
                radius: 10.,
            },
            Some(clip),
            false,
        );
        assert!(scene.hit_test(100., 100.).is_none());
        assert!(PointMesh::from_scene(&scene).unwrap().points().is_empty());
        for picking in [
            Picking::Geometry,
            Picking::Nearest { distance: 50. },
            Picking::SliceX { distance: 50. },
            Picking::SliceY { distance: 50. },
        ] {
            let mut surface = SurfaceModel::new(
                scene.clone(),
                SurfaceOptions {
                    picking,
                    ..Default::default()
                },
            )
            .unwrap();
            surface
                .apply(SurfaceInput::Move {
                    id: 1,
                    point: (100., 100.),
                })
                .unwrap();
            assert!(surface.hovered().is_empty());
            surface
                .apply(SurfaceInput::Down {
                    id: 1,
                    point: (100., 100.),
                    brush: false,
                })
                .unwrap();
            let events = surface
                .apply(SurfaceInput::Up {
                    id: 1,
                    point: (100., 100.),
                })
                .unwrap();
            assert!(surface.selected().is_none());
            assert!(
                !events
                    .iter()
                    .any(|event| matches!(event, SurfaceEvent::Activated(_)))
            );
        }
    }
}

#[test]
fn zero_extent_fills_do_not_produce_geometry_hits() {
    for geometry in [
        Geometry::Rect {
            x: 100.,
            y: 90.,
            width: 0.,
            height: 20.,
        },
        Geometry::Rect {
            x: 90.,
            y: 100.,
            width: 20.,
            height: 0.,
        },
        Geometry::Circle {
            x: 100.,
            y: 100.,
            radius: 0.,
        },
        Geometry::Path(vec![
            Cmd::Move(P::new(100., 90.)),
            Cmd::Line(P::new(100., 110.)),
            Cmd::Close,
        ]),
    ] {
        let scene = scene(geometry, None, false);
        assert!(scene.hit_test(100., 100.).is_none());
        let mut surface = SurfaceModel::new(scene, SurfaceOptions::default()).unwrap();
        surface
            .apply(SurfaceInput::Move {
                id: 1,
                point: (100., 100.),
            })
            .unwrap();
        assert!(surface.hovered().is_empty());
    }
}

#[test]
fn stroked_zero_width_or_height_segments_remain_pickable_and_exportable() {
    for geometry in [
        Geometry::Rect {
            x: 100.,
            y: 90.,
            width: 0.,
            height: 20.,
        },
        Geometry::Rect {
            x: 90.,
            y: 100.,
            width: 20.,
            height: 0.,
        },
        Geometry::Path(vec![
            Cmd::Move(P::new(100., 100.)),
            Cmd::Line(P::new(100., 100.)),
        ]),
    ] {
        let single_point_path = matches!(&geometry, Geometry::Path(_));
        let scene = scene(geometry, None, true);
        let authored = scene.clone();
        assert_eq!(scene.hit_test(100., 100.).unwrap().id, "datum");
        assert!(scene.hit_test(120., 120.).is_none());
        let svg = render::to_svg(&scene).unwrap();
        if single_point_path {
            // The stroke's real ink is a radius-one circle, rendered as one
            // filled footprint so native engines cannot drop a zero path.
            let mut expected = scene.clone();
            expected.marks[0].geometry = Geometry::Circle {
                x: 100.,
                y: 100.,
                radius: 1.,
            };
            expected.marks[0].fill = Some(Color(30, 50, 70, 255));
            expected.marks[0].stroke = None;
            assert_eq!(svg, render::to_svg(&expected).unwrap());
            assert!(svg.contains("data-id=\"datum\""));
        } else {
            assert!(svg.contains("fill=\"none\"") && svg.contains("stroke-linecap=\"round\""));
        }
        assert_eq!(
            scene, authored,
            "paint normalization must not rewrite business geometry"
        );
        glory::ssr::render(chart(scene)).unwrap();
    }
}
