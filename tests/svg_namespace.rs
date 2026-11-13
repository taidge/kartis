use kartis::*;
use std::collections::BTreeSet;
fn scene() -> Scene {
    let mut scene = ChartConfig::default().scene().unwrap();
    let clip = ClipPath {
        geometry: Geometry::Circle {
            x: 100.,
            y: 100.,
            radius: 80.,
        },
        rule: ClipRule::EvenOdd,
    };
    let mut first = Mark::new(
        "url(#kartis-business)",
        Geometry::Rect {
            x: 20.,
            y: 20.,
            width: 160.,
            height: 160.,
        },
        Color(0, 0, 0, 255),
    );
    first.clip_path = Some(Box::new(clip.clone()));
    first.fill_paint = Some(FillPaint::Linear {
        start: [20., 20.],
        end: [180., 20.],
        stops: vec![
            GradientStop::new(0., Color(255, 0, 0, 255)),
            GradientStop::new(1., Color(0, 0, 255, 255)),
        ],
    });
    scene.marks.push(first);
    let mut second = Mark::new(
        "second",
        Geometry::Circle {
            x: 250.,
            y: 100.,
            radius: 40.,
        },
        Color(0, 0, 0, 255),
    );
    second.fill_paint = Some(FillPaint::Radial {
        center: [250., 100.],
        radius: 40.,
        stops: vec![
            GradientStop::new(0., Color(255, 255, 255, 255)),
            GradientStop::new(1., Color(0, 0, 0, 255)),
        ],
    });
    scene.marks.push(second);
    for i in 0..2 {
        let mut mark = Mark::new(
            format!("pattern-{i}"),
            Geometry::Rect {
                x: 350. + i as f64 * 60.,
                y: 20.,
                width: 40.,
                height: 100.,
            },
            Color(0, 0, 0, 255),
        );
        mark.fill_paint = Some(FillPaint::Pattern(PatternFill::default()));
        scene.marks.push(mark);
    }
    scene.labels.push(Label {
        x: 100.,
        y: 100.,
        text: "literal url(#kartis-paint-user) id=\"kartis-user\"".into(),
        color: Color(0, 0, 0, 255),
        size: 12.,
        anchor: TextAnchor::Middle,
        rotation: 0.,
    });
    scene.label_clips.insert(0, clip);
    scene
}
fn ids(svg: &str) -> BTreeSet<&str> {
    svg.split(" id=\"")
        .skip(1)
        .map(|s| s.split('"').next().unwrap())
        .collect()
}
fn refs(svg: &str) -> Vec<&str> {
    ["fill=\"url(#", "clip-path=\"url(#"]
        .into_iter()
        .flat_map(|prefix| {
            svg.split(prefix)
                .skip(1)
                .map(|s| s.split(')').next().unwrap())
        })
        .collect()
}
#[test]
fn scoped_export_isolates_all_definition_types_without_rewriting_data_or_user_text() {
    let scene = scene();
    let before = scene.clone();
    let plain = render::to_svg(&scene).unwrap();
    let a = render::to_svg_with_namespace(&scene, "chart-a").unwrap();
    let b = render::to_svg_with_namespace(&scene, "chart-b").unwrap();
    let a_ids = ids(&a);
    let b_ids = ids(&b);
    assert_eq!(a_ids.len(), 4);
    assert!(a_ids.is_disjoint(&b_ids));
    assert!(a_ids.iter().all(|id| id.starts_with("kartis-ns-chart-a-")));
    assert!(refs(&a).iter().all(|id| a_ids.contains(id)));
    assert!(refs(&b).iter().all(|id| b_ids.contains(id)));
    assert_eq!(refs(&a).len(), 6);
    for svg in [&plain, &a, &b] {
        assert!(svg.contains("data-id=\"url(#kartis-business)\""));
        assert!(svg.contains("literal url(#kartis-paint-user) id=&quot;kartis-user&quot;"));
        assert_eq!(svg.matches("<pattern ").count(), 1);
    }
    assert_eq!(scene, before);
    assert_eq!(plain, render::to_svg(&scene).unwrap());
    assert_eq!(a, render::to_svg_with_namespace(&scene, "chart-a").unwrap());
}
#[test]
fn namespaces_reject_markup_and_invalid_tokens_before_producing_markup() {
    let scene = scene();
    for namespace in [
        "",
        "a\" onclick=\"evil",
        "<script>",
        "space here",
        "1bad",
        "bad/url",
        "bad#fragment",
        "日本語",
    ] {
        assert!(
            render::to_svg_with_namespace(&scene, namespace).is_err(),
            "{namespace:?}"
        );
    }
    assert!(render::to_svg_with_namespace(&scene, &"a".repeat(129)).is_err());
    assert!(render::to_svg_with_namespace(&scene, "_valid-01.x").is_ok());
}
