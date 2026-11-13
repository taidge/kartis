use std::collections::BTreeSet;

use kartis::surface::{SurfaceModel, SurfaceOptions};
use kartis::*;

fn source() -> Scene {
    let mut scene = ChartConfig::default().scene().unwrap();
    let mut mark = Mark::new(
        "original",
        Geometry::Rect {
            x: 80.,
            y: 60.,
            width: 100.,
            height: 80.,
        },
        Color(20, 40, 60, 255),
    );
    mark.series = "same".into();
    scene.marks.push(mark);
    scene.legend = vec![
        LegendItem {
            id: "same".into(),
            label: "Category same".into(),
            color: Color(20, 40, 60, 255),
        },
        LegendItem {
            id: "other".into(),
            label: "Other".into(),
            color: Color(100, 120, 140, 255),
        },
    ];
    scene
}

#[test]
fn category_identity_does_not_reinterpret_explicit_series_visibility() -> Result<()> {
    let mut scene = source();
    let hidden = BTreeSet::from(["same".into(), "other".into(), "unknown".into()]);
    assert_eq!(scene.legend_role("same"), Some(LegendRole::Series));
    assert!(scene.legend_hidden("same", &hidden));
    scene
        .legend_roles
        .insert("same".into(), LegendRole::Category);
    scene.validate()?;
    assert_eq!(scene.legend_role("same"), Some(LegendRole::Category));
    assert!(!scene.legend_hidden("same", &hidden));
    assert!(scene.legend_hidden("other", &hidden));
    assert_eq!(scene.legend_role("unknown"), None);
    assert!(!scene.legend_hidden("unknown", &hidden));
    let filtered = scene.without_series(&hidden);
    filtered.validate()?;
    assert!(filtered.marks.is_empty());
    assert_eq!(filtered.legend, scene.legend);
    assert_eq!(filtered.legend_roles, scene.legend_roles);
    Ok(())
}

#[test]
fn role_metadata_participates_in_equality_and_target_frame_propagation() -> Result<()> {
    let old = source();
    let mut next = old.clone();
    next.legend_roles
        .insert("same".into(), LegendRole::Category);
    assert_ne!(old, next);
    assert!(!format!("{old:?}").contains("legend_roles"));
    assert!(!format!("{old:#?}").contains("legend_roles"));
    assert!(format!("{next:?}").contains("legend_roles"));
    for progress in [0., 0.5, 1.] {
        let frame = old.interpolate(&next, progress)?;
        assert_eq!(frame.legend_role("same"), Some(LegendRole::Category));
        assert_eq!(frame.marks, old.marks);
        assert_eq!(frame.legend, old.legend);
    }
    Ok(())
}

#[test]
fn unknown_roles_reject_before_surface_replacement_and_leave_the_source_intact() -> Result<()> {
    let old = source();
    let mut model = SurfaceModel::new(old.clone(), SurfaceOptions::default())?;
    let before = model.controls_scene();
    let mut invalid = old.clone();
    invalid
        .legend_roles
        .insert("unknown".into(), LegendRole::Category);
    assert_eq!(invalid.legend_role("unknown"), None);
    assert!(invalid.validate().is_err());
    assert!(model.set_scene(invalid).is_err());
    assert_eq!(model.controls_scene(), before);
    let mut category = old.clone();
    category
        .legend_roles
        .insert("same".into(), LegendRole::Category);
    model.set_scene(category.clone())?;
    assert_eq!(model.controls_scene(), category);
    Ok(())
}

#[test]
fn role_budget_and_dangling_keys_are_checked_without_mutating_inputs() {
    let old = source();
    let mut invalid = old.clone();
    invalid.legend_roles = (0..257)
        .map(|i| (format!("unknown-{i}"), LegendRole::Category))
        .collect();
    let before = invalid.clone();
    assert!(invalid.validate().is_err());
    assert_eq!(invalid, before);
    let mut explicit = old.clone();
    explicit
        .legend_roles
        .insert("same".into(), LegendRole::Series);
    assert_ne!(explicit, old);
    assert_eq!(explicit.legend_role("same"), old.legend_role("same"));
    explicit.validate().unwrap();
}

#[cfg(feature = "raster")]
#[test]
fn painted_label_reconstruction_keeps_category_roles_and_the_original_source() -> Result<()> {
    let mut scene = source();
    scene
        .legend_roles
        .insert("same".into(), LegendRole::Category);
    scene.labels.push(Label {
        x: 120.,
        y: 100.,
        text: "source".into(),
        color: Color(1, 2, 3, 255),
        size: 12.,
        rotation: 0.,
        anchor: TextAnchor::Middle,
    });
    let old = scene.clone();
    let mut fonts = render::raster::FontRegistry::new();
    fonts.register(include_bytes!("fonts/ABeeZee-Regular.ttf").as_slice(), 0)?;
    let painted = scene_with_painted_labels(&scene, &fonts, &Default::default(), "category-role")?;
    painted.validate()?;
    assert!(painted.labels.is_empty());
    assert!(painted.marks.len() > scene.marks.len());
    assert_eq!(painted.legend, scene.legend);
    assert_eq!(painted.legend_roles, scene.legend_roles);
    assert_eq!(scene, old);
    Ok(())
}
