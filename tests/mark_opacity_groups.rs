use glory::render::GraphicStroke;
use kartis::interaction::Interaction;
use kartis::surface::{SurfaceModel, SurfaceOptions};
use kartis::{render, *};
use std::collections::BTreeSet;

#[derive(Default)]
struct DestinationProbe {
    supported: bool,
    fail: bool,
    begun: bool,
    paints: Vec<f64>,
    composites: Vec<(f64, bool)>,
    finished: bool,
}
impl render::Canvas for DestinationProbe {
    fn supports_group_opacity(&self) -> bool {
        self.supported
    }
    fn prepare_mark_blends(&mut self, _: &Scene, _: &MarkBlends) -> Result<()> {
        Ok(())
    }
    fn begin(&mut self, _: f64, _: f64, _: Color) -> Result<()> {
        self.begun = true;
        Ok(())
    }
    fn begin_mark_blend(&mut self, _: usize) -> Result<bool> {
        Ok(true)
    }
    fn end_mark_blend(&mut self, _: BlendMode, opacity: f64, commit: bool) -> Result<()> {
        self.composites.push((opacity, commit));
        Ok(())
    }
    fn begin_path(&mut self) -> Result<()> {
        Ok(())
    }
    fn move_to(&mut self, _: f64, _: f64) -> Result<()> {
        Ok(())
    }
    fn line_to(&mut self, _: f64, _: f64) -> Result<()> {
        Ok(())
    }
    fn cubic_to(&mut self, _: f64, _: f64, _: f64, _: f64, _: f64, _: f64) -> Result<()> {
        Ok(())
    }
    fn close_path(&mut self) -> Result<()> {
        Ok(())
    }
    fn fill(&mut self, _: Color, o: f64) -> Result<()> {
        self.paints.push(o);
        if self.fail {
            Err(ChartError("member rejected".into()))
        } else {
            Ok(())
        }
    }
    fn stroke(&mut self, _: GraphicStroke, _: f64) -> Result<()> {
        Ok(())
    }
    fn text(&mut self, _: &Label) -> Result<()> {
        Ok(())
    }
    fn finish(&mut self) -> Result<()> {
        self.finished = true;
        Ok(())
    }
}

#[test]
fn legacy_canvas_rejects_before_begin_and_failed_members_discard_the_group() -> Result<()> {
    let s = source(0.25);
    let mut legacy = DestinationProbe::default();
    assert!(render::draw(&s, &mut legacy).is_err());
    assert!(!legacy.begun && legacy.paints.is_empty());
    let mut successful = DestinationProbe {
        supported: true,
        ..Default::default()
    };
    render::draw(&s, &mut successful)?;
    assert_eq!(successful.paints, vec![1., 1.]);
    assert_eq!(successful.composites, vec![(0.25, true)]);
    let mut rejected = DestinationProbe {
        supported: true,
        fail: true,
        ..Default::default()
    };
    assert!(render::draw(&s, &mut rejected).is_err());
    assert_eq!(rejected.composites, vec![(0.25, false)]);
    assert!(rejected.finished);
    Ok(())
}

fn source(opacity: f64) -> Scene {
    let mut s = ChartConfig {
        width: 100.,
        height: 80.,
        margin: Margin {
            top: 0.,
            right: 0.,
            bottom: 0.,
            left: 0.,
        },
        ..Default::default()
    }
    .scene()
    .unwrap();
    s.background = Color(0, 0, 0, 0);
    for (id, y, color) in [
        ("body", 10., Color(0, 0, 255, 255)),
        ("median", 25., Color(0, 0, 0, 255)),
    ] {
        let mut m = Mark::new(
            id,
            Geometry::Rect {
                x: 10.,
                y,
                width: 60.,
                height: 20.,
            },
            color,
        );
        m.series = id.into();
        s.marks.push(m);
    }
    s.opacity_groups.push(MarkOpacityGroup {
        id: "box".into(),
        members: vec!["body".into(), "median".into()],
        opacity,
    });
    s
}

#[test]
fn svg_composites_one_real_parent_and_keeps_member_opacity() -> Result<()> {
    let s = source(0.25);
    let svg = render::to_svg(&s)?;
    assert!(svg.contains("<g data-opacity-group=\"box\" opacity=\"0.25\"><path data-id=\"body\""));
    assert_eq!(svg.matches(" opacity=\"1\"").count(), 2);
    assert!(svg.ends_with("</path></g></svg>"));
    let js = render::to_canvas_script(&s, "plot")?;
    assert!(js.contains("source-over"));
    assert!(js.contains("blendRegions[2]"));
    Ok(())
}

#[test]
fn groups_validate_actual_mixed_order_and_disjoint_membership() {
    let base = source(0.25);
    base.validate().unwrap();
    let mut variants = Vec::new();
    let mut s = base.clone();
    s.opacity_groups[0].members.reverse();
    variants.push(s);
    let mut s = base.clone();
    s.opacity_groups[0].members.push("absent".into());
    variants.push(s);
    let mut s = base.clone();
    s.opacity_groups.push(s.opacity_groups[0].clone());
    variants.push(s);
    let mut s = base.clone();
    s.opacity_groups[0].opacity = f64::NAN;
    variants.push(s);
    let mut s = base.clone();
    s.opacity_groups[0].members.clear();
    variants.push(s);
    let mut s = base.clone();
    s.labels.push(Label {
        text: "between".into(),
        x: 5.,
        y: 5.,
        size: 12.,
        color: Color(0, 0, 0, 255),
        anchor: TextAnchor::Start,
        rotation: 0.,
    });
    s.draw_order = vec![DrawItem::Mark(0), DrawItem::Label(0), DrawItem::Mark(1)];
    variants.push(s);
    for s in variants {
        assert!(s.validate().is_err());
    }
}

#[test]
fn transparent_group_is_not_pointer_ink_or_default_keyboard_data() -> Result<()> {
    let s = source(0.);
    assert!(s.hit_test(20., 20.).is_none());
    let mut interaction = Interaction::default();
    assert!(interaction.pointer(&s, 20., 20.).is_none());
    assert!(interaction.step(&s, true).is_none());
    let surface = SurfaceModel::new(s, SurfaceOptions::default())?;
    assert!(surface.datum("body").is_none());
    Ok(())
}

#[test]
fn picking_and_navigation_accept_the_full_disjoint_group_budget() -> Result<()> {
    let mut s = source(0.5);
    s.marks.clear();
    s.opacity_groups.clear();
    for i in 0..MAX_MARKS {
        let id = format!("node-{i}");
        let mut m = Mark::new(
            &id,
            Geometry::Rect {
                x: 10.,
                y: 10.,
                width: 20.,
                height: 20.,
            },
            Color(0, 0, 0, 255),
        );
        m.series = "source".into();
        s.marks.push(m);
        s.opacity_groups.push(MarkOpacityGroup {
            id: format!("group-{i}"),
            members: vec![id],
            opacity: if i % 2 == 0 { 0. } else { 0.5 },
        });
    }
    s.validate()?;
    assert_eq!(
        s.hit_test(15., 15.).unwrap().id,
        format!("node-{}", MAX_MARKS - 1)
    );
    let mut interaction = Interaction::default();
    assert_eq!(interaction.step(&s, true), Some("node-1"));
    assert_eq!(interaction.step(&s, true), Some("node-3"));
    let mut surface = SurfaceModel::new(s.clone(), SurfaceOptions::default())?;
    assert!(surface.datum("node-0").is_none());
    assert!(surface.datum("node-1").is_some());
    s.opacity_groups[0].opacity = 0.5;
    s.opacity_groups[1].opacity = 0.;
    surface.set_scene(s)?;
    assert!(surface.datum("node-0").is_some());
    assert!(surface.datum("node-1").is_none());
    Ok(())
}

#[test]
fn filtering_and_compatible_interpolation_preserve_group_semantics() -> Result<()> {
    let a = source(0.25);
    let filtered = a.without_series(&BTreeSet::from(["median".into()]));
    filtered.validate()?;
    assert_eq!(filtered.opacity_groups[0].members, vec!["body"]);
    let mut b = a.clone();
    b.opacity_groups.clear();
    let mid = a.interpolate(&b, 0.5)?;
    assert_eq!(mid.opacity_groups[0].opacity, 0.625);
    assert_eq!(a.interpolate(&b, 1.)?, b);
    let mut renamed = a.clone();
    renamed.opacity_groups[0].id = "other".into();
    assert!(a.interpolate(&renamed, 0.5).is_err());
    Ok(())
}

#[cfg(feature = "raster")]
#[test]
fn overlapping_members_composite_once_on_transparent_and_opaque_backdrops() -> Result<()> {
    use render::raster::*;
    for background in [Color(0, 0, 0, 0), Color(255, 255, 255, 255)] {
        let mut s = source(0.25);
        s.background = background;
        let image = render_raster(&s, &FontRegistry::new(), &Default::default())?;
        let pixel = |x: u32, y: u32| -> [u8; 4] {
            image.rgba
                [((y * image.width + x) * 4) as usize..((y * image.width + x) * 4 + 4) as usize]
                .try_into()
                .unwrap()
        };
        let expected = if background.3 == 0 {
            [0, 0, 0, 64]
        } else {
            [191, 191, 191, 255]
        };
        assert_eq!(
            pixel(20, 27),
            expected,
            "median completely covers the body before group alpha"
        );
        let expected = if background.3 == 0 {
            [0, 0, 255, 64]
        } else {
            [191, 191, 255, 255]
        };
        assert_eq!(pixel(20, 15), expected);
    }
    Ok(())
}

#[cfg(feature = "raster")]
#[test]
fn member_clips_apply_inside_group_and_outside_labels_stay_independent() -> Result<()> {
    use render::raster::*;
    let mut s = source(0.5);
    s.marks[0].clip = Some(ClipRect {
        x: 20.,
        y: 10.,
        width: 20.,
        height: 40.,
    });
    s.marks[1].clip_path = Some(Box::new(ClipPath {
        geometry: Geometry::Rect {
            x: 30.,
            y: 25.,
            width: 20.,
            height: 20.,
        },
        rule: ClipRule::NonZero,
    }));
    let image = render_raster(&s, &FontRegistry::new(), &Default::default())?;
    let pixel = |x: u32, y: u32| -> [u8; 4] {
        image.rgba[((y * image.width + x) * 4) as usize..((y * image.width + x) * 4 + 4) as usize]
            .try_into()
            .unwrap()
    };
    assert_eq!(pixel(15, 20), [0, 0, 0, 0]);
    assert_eq!(pixel(25, 27), [0, 0, 255, 128]);
    assert_eq!(pixel(35, 27), [0, 0, 0, 128]);
    assert_eq!(pixel(45, 27), [0, 0, 0, 128]);
    assert_eq!(pixel(55, 27), [0, 0, 0, 0]);
    Ok(())
}

#[cfg(feature = "raster")]
#[test]
fn unsupported_nested_layers_reject_and_raster_remains_reusable() -> Result<()> {
    use render::raster::*;
    let s = source(0.5);
    let mut target = RasterCanvas::new(FontRegistry::new(), RasterOptions::default())?;
    let before = target.render(&s)?.clone();
    let blends = MarkBlends::new(&s, [("body", BlendMode::Multiply)])?;
    assert!(render::draw_with_blends(&s, &mut target, &blends).is_err());
    assert_eq!(target.render(&s)?, &before);
    Ok(())
}
