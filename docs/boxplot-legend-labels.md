# BoxPlot legend captions

[简体中文](boxplot-legend-labels.zh-CN.md)

`BoxPlotLayoutOptions::legend_labels` is an optional `BTreeMap<String, String>` of display captions keyed by stable raw identities. `color_by: Group` selects raw group keys; `SubGroup` selects raw subgroup keys, including the empty string. SubGroup map keys are not the canonical IDs returned by `boxplot_series_id`. This is a key-stable caption map, not an arbitrary Nivo `legendLabel` factory or property accessor.

The complete program updates Group captions through the real controller while retaining its active business key, legend IDs, roles, colors and marks. It also configures SubGroup captions with raw keys, including an explicit empty caption.

```rust
use kartis::*;

fn main() -> Result<()> {
    let config = ChartConfig::default();
    let data: Vec<_> = ["North", "South"]
        .into_iter()
        .flat_map(|group| {
            ["Actual", ""].into_iter().flat_map(move |subgroup| {
                [2., 4., 6., 8.]
                    .into_iter()
                    .enumerate()
                    .map(move |(i, value)| {
                        BoxObservation::new(
                            format!("{group}/{subgroup}/{i}"),
                            group,
                            subgroup,
                            Some(value),
                        )
                    })
            })
        })
        .collect();
    let options = BoxPlotLayoutOptions {
        color_by: BoxPlotColorBy::Group,
        legend_labels: Some(
            [
                ("North".into(), "Northern region".into()),
                ("South".into(), "Southern region".into()),
            ]
            .into(),
        ),
        ..Default::default()
    };
    let mut controller = BoxPlotInteraction::new(
        &config,
        &data,
        &options,
        &BoxPlotInteractionOptions::default(),
    )?;
    let active = BoxPlotKey::new("North", "Actual");
    controller.set_active(Some(active.clone()))?;
    let before = controller.layout.clone();
    let renamed = BoxPlotLayoutOptions {
        legend_labels: Some(
            [
                ("North".into(), "Region".into()),
                ("South".into(), "Region".into()),
            ]
            .into(),
        ),
        ..options.clone()
    };
    controller.replace_source(&config, &data, &renamed)?;
    assert_eq!(controller.active(), Some(&active));
    assert_eq!(controller.layout.scene.marks, before.scene.marks);
    assert_eq!(controller.layout.scene.labels, before.scene.labels);
    assert_eq!(controller.layout.scene.legend.len(), 2);
    for (item, old) in controller
        .layout
        .scene
        .legend
        .iter()
        .zip(&before.scene.legend)
    {
        assert_eq!((&item.id, item.color), (&old.id, old.color));
        assert_eq!(item.label, "Region");
        assert_eq!(
            controller.layout.scene.legend_role(&item.id),
            Some(LegendRole::Category)
        );
    }
    let subgroups = boxplot_layout(
        &config,
        &data,
        &BoxPlotLayoutOptions {
            color_by: BoxPlotColorBy::SubGroup,
            legend_labels: Some(
                [("Actual".into(), "Observed".into()), ("".into(), "".into())].into(),
            ),
            ..options
        },
    )?;
    let empty_id = boxplot_series_id("");
    assert_eq!(
        subgroups
            .scene
            .legend
            .iter()
            .find(|item| item.id == empty_id)
            .unwrap()
            .label,
        ""
    );
    assert_eq!(
        subgroups.scene.legend_role(&empty_id),
        Some(LegendRole::Series)
    );
    let defaults = boxplot_layout(&config, &data, &BoxPlotLayoutOptions::default())?;
    assert_eq!(
        defaults
            .scene
            .legend
            .iter()
            .find(|item| item.id == empty_id)
            .unwrap()
            .label,
        "Samples"
    );
    assert!(render::to_svg(&controller.layout.scene)?.contains("<desc>Region; Region; </desc>"));
    let _script = render::to_canvas_script(&controller.layout.scene, "plot")?;
    Ok(())
}
```

`None` and `Some(BTreeMap::new())` preserve the original display text. An omitted key also keeps that text: Group uses the group name; SubGroup uses the subgroup name, or `Samples` for an empty subgroup. A configured empty string is a real empty caption and does not trigger fallback.

Empty or repeated captions retain every original legend ID and row. The example's two `Region` entries remain two Category identities. Nivo's original `getLegendData` deduplicates by label, so identical labels can collapse different identities; that behavior remains an explicit difference. The map does not add arbitrary callbacks, one-identity/multiple-label expansion, new series aliases or legend actions. The [original-function reference](../tests/fixtures/box-legend-labels/README.md) records actual map callbacks, deduplication and these boundaries.

Row eligibility stays unchanged. Group legends include only groups represented by actual nonempty boxes after subgroup hidden filtering; a caption for a reserved group does not create a row. Hover opacity zero does not remove those identities. SubGroup keeps the complete configured domain, including reserved and hidden subgroup rows. IDs, Category/Series roles, colors, series ownership and hidden behavior remain as described in [Colors and controlled hover](boxplot-color-and-hover.md).

Every configured map key must belong to the full selected raw domain, and every caption is limited to 1024 UTF-8 bytes. Both checks apply even with `legend: false`, no observations, hidden sources or an unused reserved caption. Invalid configuration returns an error. A failed controller `replace_source` preserves the accepted frame and active state. Caption-only updates change legend text, while datum tooltips, axis labels, original observations, statistics, geometry, palette and group-or-subgroup association stay unchanged.

Use `replace_source` with the new layout options to update a controlled caption map. Editing the public accepted Scene is not a signature-refresh operation and causes subsequent controlled operations to reject. This guide covers the implemented map contract and focused checks, not completion of full native or host acceptance.

The Scene stores legend metadata; visible legend controls are provided by hosts when enabled. Standalone SVG puts captions in `<desc>` metadata and does not gain a drawn legend merely from this option. The ordinary CanvasScript export and Scene CPU image replay marks/labels without embedding a visible host legend. This example's SVG assertion checks metadata text, not visible legend pixels. Host legend widgets can display the updated captions through their existing presentation APIs.
