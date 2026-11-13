use std::collections::{BTreeMap, BTreeSet, HashSet};

use kartis::surface::{SurfaceModel, SurfaceOptions};
use kartis::*;

fn item(id: &str) -> LegendItem {
    LegendItem {
        id: id.into(),
        label: "same caption".into(),
        color: Color(20, 40, 60, 160),
    }
}
fn source() -> Scene {
    let mut scene = ChartConfig::default().scene().unwrap();
    let mut mark = Mark::new(
        "source-body",
        Geometry::Rect {
            x: 80.,
            y: 60.,
            width: 100.,
            height: 80.,
        },
        Color(20, 40, 60, 160),
    );
    mark.series = "canonical-series".into();
    scene.marks.push(mark);
    scene.legend = vec![item("row-a"), item("row-b"), item("row-c")];
    scene
}

#[test]
fn defaults_resolve_only_actual_rows_and_keep_legacy_role_identity() -> Result<()> {
    let mut scene = source();
    assert!(scene.legend_targets.is_empty());
    assert_eq!(
        scene.legend_target("row-a"),
        Some(LegendTarget::Series("row-a".into()))
    );
    scene
        .legend_roles
        .insert("row-b".into(), LegendRole::Category);
    scene.validate()?;
    assert_eq!(
        scene.legend_target("row-b"),
        Some(LegendTarget::Category("row-b".into()))
    );
    scene
        .legend_targets
        .insert("not-a-row".into(), LegendTarget::Series("reserved".into()));
    assert_eq!(scene.legend_target("not-a-row"), None);
    assert_eq!(scene.legend_role("not-a-row"), None);
    assert!(!scene.legend_hidden("not-a-row", &BTreeSet::from(["reserved".into()])));
    assert!(scene.validate().is_err());
    Ok(())
}

#[test]
fn repeated_rows_share_canonical_visibility_while_categories_remain_independent() -> Result<()> {
    let mut scene = source();
    for row in ["row-a", "row-b"] {
        scene
            .legend_targets
            .insert(row.into(), LegendTarget::Series("canonical-series".into()));
    }
    scene.legend_targets.insert(
        "row-c".into(),
        LegendTarget::Category("canonical-series".into()),
    );
    scene.validate()?;
    let hidden = BTreeSet::from(["canonical-series".into()]);
    assert!(scene.legend_hidden("row-a", &hidden));
    assert!(scene.legend_hidden("row-b", &hidden));
    assert!(!scene.legend_hidden("row-c", &hidden));
    assert!(!scene.legend_hidden("row-a", &BTreeSet::from(["row-a".into()])));
    assert_eq!(scene.legend_role("row-c"), Some(LegendRole::Category));
    let filtered = scene.without_series(&hidden);
    filtered.validate()?;
    assert!(filtered.marks.is_empty());
    assert_eq!(filtered.legend, scene.legend);
    assert_eq!(filtered.legend_targets, scene.legend_targets);
    Ok(())
}

#[test]
fn reserved_targets_and_row_named_targets_are_scalar_not_alias_chains() -> Result<()> {
    let mut scene = source();
    scene
        .legend_targets
        .insert("row-a".into(), LegendTarget::Series("row-b".into()));
    scene
        .legend_targets
        .insert("row-b".into(), LegendTarget::Category(String::new()));
    scene.legend_targets.insert(
        "row-c".into(),
        LegendTarget::Series("reserved-with-no-mark".into()),
    );
    scene.validate()?;
    assert_eq!(
        scene.legend_target("row-a"),
        Some(LegendTarget::Series("row-b".into()))
    );
    assert_eq!(scene.legend_role("row-a"), Some(LegendRole::Series));
    assert_eq!(
        scene.legend_target("row-b"),
        Some(LegendTarget::Category(String::new()))
    );
    assert!(scene.legend_hidden("row-a", &BTreeSet::from(["row-b".into()])));
    assert!(!scene.legend_hidden("row-b", &BTreeSet::from([String::new()])));
    assert_eq!(scene.legend_target("reserved-with-no-mark"), None);
    Ok(())
}

#[test]
fn contradictory_explicit_roles_reject_and_matching_roles_are_allowed() -> Result<()> {
    for (target, opposite) in [
        (LegendTarget::Series("source".into()), LegendRole::Category),
        (LegendTarget::Category("source".into()), LegendRole::Series),
    ] {
        let mut scene = source();
        scene.legend_targets.insert("row-a".into(), target.clone());
        scene.legend_roles.insert("row-a".into(), opposite);
        let before = scene.clone();
        assert!(scene.validate().is_err());
        assert_eq!(scene, before);
        scene.legend_roles.insert("row-a".into(), target.role());
        scene.validate()?;
        assert_eq!(scene.legend_role("row-a"), Some(target.role()));
    }
    Ok(())
}

#[test]
fn utf8_identity_bounds_preserve_empty_categories_but_reject_empty_series() -> Result<()> {
    let mut scene = source();
    scene
        .legend_targets
        .insert("row-a".into(), LegendTarget::Category(String::new()));
    scene
        .legend_targets
        .insert("row-b".into(), LegendTarget::Series("é".repeat(512)));
    scene.validate()?;
    let before = scene.clone();
    scene
        .legend_targets
        .insert("row-b".into(), LegendTarget::Series(String::new()));
    assert!(scene.validate().is_err());
    scene = before.clone();
    scene
        .legend_targets
        .insert("row-b".into(), LegendTarget::Series("é".repeat(513)));
    assert!(scene.validate().is_err());
    scene = before;
    scene
        .legend_targets
        .insert("row-a".into(), LegendTarget::Category("é".repeat(513)));
    assert!(scene.validate().is_err());
    Ok(())
}

#[test]
fn complete_256_row_identity_budget_is_valid_and_257_rows_reject() -> Result<()> {
    let mut scene = ChartConfig::default().scene()?;
    for i in 0..256 {
        let row = format!("{i:03}{}", "x".repeat(MAX_LABEL_BYTES - 3));
        scene.legend.push(item(&row));
        scene
            .legend_targets
            .insert(row, LegendTarget::Series("é".repeat(512)));
    }
    scene.validate()?;
    let before = scene.clone();
    scene.legend.push(item("extra"));
    scene
        .legend_targets
        .insert("extra".into(), LegendTarget::Category(String::new()));
    assert!(scene.validate().is_err());
    assert_eq!(before.legend.len(), 256);
    assert_eq!(before.legend_targets.len(), 256);
    Ok(())
}

#[test]
fn explicit_targets_affect_equality_and_debug_without_changing_default_output() -> Result<()> {
    let old = source();
    assert!(!format!("{old:?}").contains("legend_targets"));
    assert!(!format!("{old:#?}").contains("legend_targets"));
    let mut explicit = old.clone();
    explicit
        .legend_targets
        .insert("row-a".into(), LegendTarget::Series("row-a".into()));
    explicit.validate()?;
    assert_ne!(explicit, old);
    assert!(format!("{explicit:?}").contains("legend_targets"));
    explicit.legend_targets.clear();
    assert_eq!(explicit, old);
    assert_eq!(format!("{explicit:?}"), format!("{old:?}"));
    assert_eq!(format!("{explicit:#?}"), format!("{old:#?}"));
    Ok(())
}

#[test]
fn target_metadata_follows_target_frames_through_interpolation_and_motion() -> Result<()> {
    let old = source();
    let mut next = old.clone();
    next.legend_targets.insert(
        "row-a".into(),
        LegendTarget::Category("raw-category".into()),
    );
    for progress in [0., 0.5, 1.] {
        let frame = old.interpolate(&next, progress)?;
        assert_eq!(frame.legend_targets, next.legend_targets);
        assert_eq!(frame.legend, old.legend);
        assert_eq!(frame.marks, old.marks);
    }
    let mut spring = motion::SpringMotion::new(old, motion::SpringConfig::default())?;
    spring.retarget(next.clone())?;
    spring.tick(0.1)?;
    assert_eq!(spring.scene().legend_targets, next.legend_targets);
    spring.finish();
    assert_eq!(spring.scene(), &next);
    Ok(())
}

#[test]
fn invalid_target_frames_do_not_replace_accepted_surface_state_or_token() -> Result<()> {
    let old = source();
    let mut model = SurfaceModel::new(old.clone(), SurfaceOptions::default())?;
    let token = model.frame_token();
    for (key, target) in [
        ("unknown", LegendTarget::Series("canonical-series".into())),
        ("row-a", LegendTarget::Series(String::new())),
    ] {
        let mut bad = old.clone();
        bad.legend_targets.insert(key.into(), target);
        assert!(model.set_scene(bad).is_err());
        assert_eq!(model.scene(), &old);
        assert_eq!(model.frame_token(), token);
    }
    Ok(())
}

#[test]
fn legacy_activation_debug_and_explicit_target_snapshot_are_independent() {
    #[derive(Debug)]
    struct OldActivation {
        item: LegendItem,
        role: LegendRole,
    }
    for role in [LegendRole::Series, LegendRole::Category] {
        let old = OldActivation {
            item: item("row-a"),
            role,
        };
        let activation = LegendActivation {
            item: old.item.clone(),
            role: old.role,
            target: None,
        };
        assert_eq!(
            format!("{activation:?}"),
            format!("{old:?}").replacen("OldActivation", "LegendActivation", 1)
        );
        assert_eq!(
            format!("{activation:#?}"),
            format!("{old:#?}").replacen("OldActivation", "LegendActivation", 1)
        );
        assert_eq!(activation.resolved_target().role(), role);
        assert_eq!(activation.resolved_target().id(), "row-a");
    }
    let mut scene = source();
    scene.legend_targets.insert(
        "row-a".into(),
        LegendTarget::Series("original-series".into()),
    );
    let activation = LegendActivation {
        item: scene.legend[0].clone(),
        role: scene.legend_role("row-a").unwrap(),
        target: scene.legend_targets.get("row-a").cloned(),
    };
    scene.legend_targets.insert(
        "row-a".into(),
        LegendTarget::Series("replacement-series".into()),
    );
    assert_eq!(
        activation.resolved_target(),
        LegendTarget::Series("original-series".into())
    );
    assert_ne!(
        Some(activation.resolved_target()),
        scene.legend_target("row-a")
    );
    assert!(format!("{activation:?}").contains("target: Some(Series"));
}

#[test]
fn typed_targets_form_distinct_row_generation_keys_for_same_raw_id() {
    let series = LegendTarget::Series("same".into());
    let category = LegendTarget::Category("same".into());
    let keys = BTreeSet::from([
        ("row".to_string(), true, series.clone()),
        ("row".to_string(), false, category.clone()),
    ]);
    assert_eq!(keys.len(), 2);
    let hashed = HashSet::from([series.clone(), category.clone()]);
    assert_eq!(hashed.len(), 2);
    let roles = BTreeMap::from([
        (series, LegendRole::Series),
        (category, LegendRole::Category),
    ]);
    assert_eq!(roles.len(), 2);
}

#[cfg(feature = "raster")]
#[test]
fn painted_label_reconstruction_keeps_row_targets_and_original_scene() -> Result<()> {
    let mut scene = source();
    scene.legend_targets.insert(
        "row-a".into(),
        LegendTarget::Series("canonical-series".into()),
    );
    scene
        .legend_targets
        .insert("row-c".into(), LegendTarget::Category(String::new()));
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
    let painted = scene_with_painted_labels(&scene, &fonts, &Default::default(), "legend-target")?;
    painted.validate()?;
    assert!(painted.labels.is_empty());
    assert_eq!(painted.legend, old.legend);
    assert_eq!(painted.legend_roles, old.legend_roles);
    assert_eq!(painted.legend_targets, old.legend_targets);
    assert_eq!(scene, old);
    Ok(())
}
