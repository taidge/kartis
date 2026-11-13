# Inherited Bar and HeatMap border colors

Set `BarOptions.border_color_profile` or `HeatmapLayoutOptions.border_color_profile` to reuse the shared `InheritedColor` role for an existing border. `None` keeps the fixed `border.color`; `Some(InheritedColor::Theme)` resolves that same fixed color. The ordinary default remains `border: None`. Setting a profile alone does not create a border. Width and cap still come from the explicitly configured `GraphicStroke`.

The complete example covers classic Bar, styled Bar with text and geometry factories, and HeatMap with distinct null, zero and nonzero source values. The geometry factories change the visible fill to magenta; the inherited border still uses the numeric palette resolved before those callbacks.

```rust
use kartis::*;
use glory::render::{GraphicStroke, StrokeCap};

fn stroke(color: Color) -> GraphicStroke {
    GraphicStroke {
        color,
        width: 2.5,
        cap: StrokeCap::Round,
    }
}
fn mark<'a>(scene: &'a Scene, id: &str) -> &'a Mark {
    scene.marks.iter().find(|mark| mark.id == id).unwrap()
}

fn main() -> kartis::Result<()> {
    let mut config = ChartConfig::default();
    config.theme.colors = vec![Color(20, 40, 60, 160)];
    let fixed = stroke(Color(9, 8, 7, 137));
    let profile = InheritedColor::FromContext(vec![ColorModifier::Darker(1.)]);
    let categories = ["A", "B"].map(str::to_string);
    let data = [BarSeries::new("sales", vec![Some(4.), Some(8.)])];

    let fixed_options = BarOptions {
        border: Some(fixed),
        ..Default::default()
    };
    let classic_fixed = bar(&config, &categories, &data, &fixed_options)?;
    let classic_theme = bar(
        &config,
        &categories,
        &data,
        &BarOptions {
            border_color_profile: Some(InheritedColor::Theme),
            ..fixed_options.clone()
        },
    )?;
    assert_eq!(classic_theme, classic_fixed);
    let options = BarOptions {
        border_color_profile: Some(profile.clone()),
        ..fixed_options
    };
    let classic = bar(&config, &categories, &data, &options)?;
    for source in classic
        .marks
        .iter()
        .filter(|mark| mark.id.starts_with("bar-"))
    {
        assert_eq!(source.stroke, Some(stroke(Color(14, 28, 42, 160))));
    }
    let no_border = bar(
        &config,
        &categories,
        &data,
        &BarOptions {
            border: None,
            ..options.clone()
        },
    )?;
    assert!(
        no_border
            .marks
            .iter()
            .filter(|mark| mark.id.starts_with("bar-"))
            .all(|mark| mark.stroke.is_none())
    );

    let style = BarStyleOptions {
        colors: [("sales".into(), Color(20, 40, 60, 160))].into(),
        labels: BarLabels {
            enabled: true,
            color_profile: Some(InheritedColor::Fixed(Color(40, 50, 60, 101))),
            ..Default::default()
        },
        ..Default::default()
    };
    let mut styled = {
        let mut label = |context: &BarLabelContext<'_>| Ok(context.default_text.to_owned());
        let mut geometry = |context: &BarDatumContext<'_>| {
            assert_eq!(context.color, Color(20, 40, 60, 160));
            assert_eq!(context.stroke, Some(stroke(Color(14, 28, 42, 160))));
            let mut node = context.inherit(context.geometry.clone());
            node.fill = Some(Color(240, 30, 180, 255));
            node.opacity = 0.5;
            if context.category == "B" {
                node.stroke = Some(stroke(Color(1, 2, 3, 80)));
            }
            Ok(node)
        };
        bar_layout_with_factories(
            &config,
            &categories,
            &data,
            &options,
            &style,
            BarFactories {
                bar: Some(&mut geometry),
                label: Some(&mut label),
            },
        )?
    };
    let styled_base = styled.layout.scene.clone();
    let active_id = styled.layout.bars[0].id.clone();
    let second_id = styled.layout.bars[1].id.clone();
    assert_eq!(
        mark(&styled_base, &active_id).stroke,
        Some(stroke(Color(14, 28, 42, 160)))
    );
    assert_eq!(
        mark(&styled_base, &second_id).stroke,
        Some(stroke(Color(1, 2, 3, 80)))
    );
    let first_label = styled.layout.bars[0].label_index.unwrap();
    let second_label = styled.layout.bars[1].label_index.unwrap();
    assert_eq!(styled_base.labels[first_label].color, Color(40, 50, 60, 51));
    styled.set_active(Some(&active_id), 0.2)?;
    assert_eq!(mark(&styled.layout.scene, &second_id).opacity, 0.1);
    assert_eq!(
        mark(&styled.layout.scene, &second_id).stroke,
        mark(&styled_base, &second_id).stroke
    );
    assert_eq!(
        styled.layout.scene.labels[second_label].color,
        Color(40, 50, 60, 10)
    );
    styled.set_active(None, 0.2)?;
    assert_eq!(styled.layout.scene, styled_base);
    let _bar_binding = styled.interaction_binding()?;

    let heat_data = [
        HeatCell {
            row: "row".into(),
            column: "empty".into(),
            value: None,
        },
        HeatCell::new("row", "zero", 0.),
        HeatCell::new("row", "value", 10.),
    ];
    let heat_options = HeatmapLayoutOptions {
        border: Some(fixed),
        border_color_profile: Some(profile),
        axes: HeatmapAxes::none(),
        hover_target: HeatmapHoverTarget::Cell,
        colors: ColorScaleOptions {
            domain: Some([0., 10.]),
            colors: vec![Color(20, 40, 60, 160), Color(200, 180, 160, 220)],
            unknown: Color(90, 100, 110, 64),
            ..Default::default()
        },
        label_color_profile: Some(InheritedColor::Fixed(Color(40, 50, 60, 101))),
        empty_label: "NA".into(),
        opacity: 0.8,
        active_opacity: 0.9,
        inactive_opacity: 0.2,
        ..Default::default()
    };
    let heat = heatmap_layout_with_cell_factory(&config, &heat_data, &heat_options, |context| {
        let (palette, border_color) = match context.datum.value {
            None => (Color(90, 100, 110, 64), Color(63, 70, 77, 64)),
            Some(0.) => (Color(20, 40, 60, 160), Color(14, 28, 42, 160)),
            Some(_) => (Color(200, 180, 160, 220), Color(140, 126, 112, 220)),
        };
        assert_eq!(context.color, palette);
        assert_eq!(context.stroke, Some(stroke(border_color)));
        let mut node = context.inherit(context.geometry.clone());
        node.fill = Some(Color(240, 30, 180, 255));
        node.opacity = 0.5;
        Ok(node)
    })?;
    assert_eq!(
        heat.cells
            .iter()
            .filter(|cell| cell.value.is_none())
            .count(),
        1
    );
    let zero = heat
        .cells
        .iter()
        .find(|cell| cell.value == Some(0.))
        .unwrap();
    assert_eq!(zero.tooltip_datum().value, Some(0.));
    assert_eq!(
        mark(&heat.scene, &zero.mark_id).stroke,
        Some(stroke(Color(14, 28, 42, 160)))
    );
    assert_eq!(mark(&heat.scene, &zero.mark_id).opacity, 0.4);
    assert_eq!(
        heat.scene
            .labels
            .iter()
            .find(|label| label.text == "0")
            .unwrap()
            .color,
        Color(40, 50, 60, 40)
    );
    let active = heat.with_active(Some(HeatmapCellKey::new("row", "empty")))?;
    assert_eq!(mark(&active.scene, &zero.mark_id).opacity, 0.1);
    assert_eq!(
        mark(&active.scene, &zero.mark_id).stroke,
        mark(&heat.scene, &zero.mark_id).stroke
    );
    assert_eq!(
        active
            .scene
            .labels
            .iter()
            .find(|label| label.text == "0")
            .unwrap()
            .color,
        Color(40, 50, 60, 10)
    );
    assert_eq!(active.with_active(None)?.scene, heat.scene);
    let _heat_binding = heat.interaction_binding()?;
    let _classic_svg = render::to_svg(&classic)?;
    let _styled_svg = render::to_svg(&styled.layout.scene)?;
    let _heat_canvas = render::to_canvas_script(&heat.scene, "heatmap")?;
    Ok(())
}
```

For Bar, palette RGBA `(20,40,60,160)` with `Darker(1)` produces border RGBA `(14,28,42,160)`. The geometry factory factor `0.5` remains in `Mark.opacity`; it does not change the stored border alpha to 80. An inactive factor of `0.2` changes that mark opacity to `0.1`, while its border RGBA remains unchanged. The owned label has its own fixed RGBA `(40,50,60,101)`: its final alpha is 51 at base and 10 when inactive. Clearing the active ID restores the accepted base frame.

The label-plus-geometry factory path receives the same resolved stroke as ordinary and geometry-only Bar layouts. `BarLabelContext` supplies original source and text; the accompanying `BarDatumContext` supplies `stroke`, `color` and `inherit`. Returning an explicit custom stroke takes precedence, as shown for category B. Neither a custom solid fill nor a gradient or pattern representative feeds back into the border role. Source values, tooltip text, label ownership, logical interaction positions, series/category identity and existing crop/zero-extent policies remain independent of the color override. Resize, hide, data and profile changes require rebuilding the layout; hover does not rerun factories.

For HeatMap, an explicit null source cell uses the numeric scale's `unknown` color; zero uses the real zero endpoint. Absent sparse pairs are not synthesized. The three border colors in the example keep alpha 64, 160 and 220 even though the factory factor `0.5` combines with base opacity `0.8` to give mark opacity `0.4`. Activating the null cell gives the zero cell inactive opacity `0.1`; the zero label's alpha changes from 40 to 10. The null cell instead uses active opacity `0.9 × 0.5 = 0.45`. The role does not give an existing zero-size HeatMap primitive a stroke where the layout previously suppressed it.

`FromContext` uses each path's numeric pre-factory palette as both `inherited` and contrast `surface`. Bar retains its classic or configured series/category palette selection; HeatMap retains its numeric scale and unknown color. With a configured stroke, `Theme` uses that stroke's fixed color, not the text theme or an arbitrary Nivo theme-property path. The chart background is the contrast backdrop. `Fixed` and ordered `Brighter`, `Darker` and `Opacity` modifiers reuse the existing typed color implementation. `ColorModifier::Opacity` replaces the role color's alpha; it does not replace mark opacity. Border and label profiles are separate roles.

Intrinsic profile validation runs even with no border, hidden data or disabled labels, before factory callbacks. Context-dependent validation also preflights actual source values, including hidden, cropped and zero-valued sources. A Bar `None` value has no bar surface; an explicit HeatMap null cell does have an unknown-color surface. A genuinely empty source validates configuration without inventing a palette context. Invalid profiles or actual contexts return an error before callbacks and do not yield an accepted frame; callback side effects from other later failures are not rolled back.

`Contrast` is a Rust extension. It chooses between its two candidates over the numeric palette surface and chart background and requires an opaque effective background; this is not a promise about every pixel of arbitrary custom paint, or a matching Nivo configuration. Typed RGBA preserves hidden RGB channels at alpha zero. Nivo's CSS alpha-zero parsing through d3-color can discard those channels before modifiers run. A later nonzero `Opacity` modifier can therefore make the difference visible: the original CSS path may restore alpha on black while typed Rust restores it on the retained RGB. The reference values retain that difference; this API keeps the existing Rust typed-color behavior rather than changing the shared parser or algorithms.

For Normal blending, `Mark.opacity` applies to the mark as a whole. Canvas and CPU raster render fractional-opacity marks with both potential fill and stroke into a bounded transparent source, retaining each paint color's alpha, then composite that source once at the mark opacity. This follows SVG and Glory's object-opacity semantics. Existing scratch-pixel and per-frame work budgets still apply; destination clips apply once at composition. Single-paint inputs, normalized point strokes and zero-area primitives retain direct paths. General Path fill is classified conservatively and may require isolation, so visually single-paint paths do not carry a blanket promise of unchanged Canvas output bytes.

Custom `Canvas` adapters must opt into `supports_group_opacity`, which defaults to `false`, and implement `prepare_mark_blends`, `begin_mark_blend` and `end_mark_blend` together. Their contract is bounded resource preflight before `begin`, transparent-source drawing at unit mark opacity, then one composite under the destination clips or source discard on error. A Normal frame requiring this capability rejects an unsupported adapter before preparation or `begin` can touch its destination.

With no border profile, fixed-border layouts, business metadata and SVG remain compatible. The object-opacity renderer correction can change Canvas code and CPU pixels for affected existing marks; earlier role-only comparisons do not establish final Canvas/CPU byte identity. Matching Nivo's inherited-color calculation does not imply that its original Canvas and SVG opacity behavior agrees: the original HeatMap Canvas path also has opacity differences. The color role and object compositing are separate contracts.

Glory `Chart` now accepts `Scene.label_clips` with `Geometry::Rect` through real overflow-hidden containers. It keeps the original label text, source coordinates, anchors, fonts, RGBA and business identities, shifting only the containing origin. Clip-only updates and transitions between clipped and unclipped labels remain reactive; unclipped labels keep their prior structure. The clip does not create an interaction hit region. Circle and Path label masks remain unsupported, and a rejected reactive proposal retains the accepted frame. Ordinary text still requires zero rotation; `outline_labels` and `painted_labels` retain their existing font, rotation and glyph-budget rules while sharing the rectangular container.

Use `..Default::default()` for the options. Construct `GraphicStroke` with its explicit `color`, `width` and `cap`; it has no `Default` implementation. [Inherited label colors](inherited-label-colors.md) · [Typed labels](typed-labels.md) · [Custom Bar geometry](bar-custom-geometry.md) · [Custom HeatMap cells](heatmap-custom.md)
