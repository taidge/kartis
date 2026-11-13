# BoxPlot per-summary legend factories

[简体中文](boxplot-legend-factory.zh-CN.md)

`BoxPlotLayoutOptions::legend_mode` defaults to `BoxPlotLegendMode::Keys`. This keeps the existing key-based legend and `legend_labels` caption map: repeated or empty captions keep separate original rows, SubGroup includes the full configured domain, and an empty subgroup falls back to `Samples`. Group lists groups represented by actual visible nonempty boxes. See [key-stable captions](boxplot-legend-labels.md).

Choose `BoxPlotLegendMode::ByLabel` for Nivo's per-summary label topology. The builder keeps the first complete representative of each exact returned `String`, then stably sorts the retained rows by the complete selected color-domain index. It keeps the representative's numeric color and typed target. Empty text is a real retained row. One subgroup can therefore produce several differently named rows sharing one canonical series target. Different keys returning the same text retain only the first representative's target; they do not form a union action.

The complete program builds four summary labels, activates real Glory legend controls, verifies that differently named rows share the canonical hidden-series bit, and replaces the source with an explicitly supplied factory. Its conditional raster section uses the same accepted layout. The memory host makes the example self-contained; a browser application can mount the same widgets with its chosen Glory host.

```rust
use kartis::*;
use glory::render::{Event, NodeId, Props};
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

fn legend_node(app: &glory::App, label: &str) -> Result<NodeId> {
    let tree = app.tree();
    let mut pending = tree.roots().to_vec();
    while let Some(id) = pending.pop() {
        let node = tree.node(id).map_err(|e| ChartError(e.to_string()))?;
        if node
            .descriptor
            .view()
            .classes
            .iter()
            .any(|class| class == "kartis-legend-item")
            && matches!(node.descriptor.view().props, Props::Button(button) if button.label == label)
        {
            return Ok(id);
        }
        pending.extend(node.children.iter().copied());
    }
    Err(ChartError("expected legend control is absent".into()))
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let config = ChartConfig {
        width: 480.,
        height: 300.,
        ..Default::default()
    };
    let mut data = Vec::new();
    for group in ["B", "A"] {
        for subgroup in ["y", "x"] {
            for (i, value) in [1., 2., 3., 4.].into_iter().enumerate() {
                data.push(BoxObservation::new(
                    format!("{group}/{subgroup}/{i}"),
                    group,
                    subgroup,
                    Some(value),
                ));
            }
        }
    }
    data.push(BoxObservation::new("B/y/null", "B", "y", None));
    let options = BoxPlotLayoutOptions {
        groups: Some(vec!["B".into(), "A".into(), "spare".into()]),
        subgroups: Some(vec!["unused".into(), "y".into(), "x".into()]),
        legend_mode: BoxPlotLegendMode::ByLabel,
        legend_labels: Some(BTreeMap::from([
            ("y".into(), "Plan".into()),
            ("x".into(), "Observed".into()),
        ])),
        value_axis: AxisOptions {
            visible: false,
            grid: false,
            ..Default::default()
        },
        group_axis: CategoryAxisOptions {
            visible: false,
            grid: false,
            ..Default::default()
        },
        ..Default::default()
    };
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let mut label = move |context: &BoxPlotLegendContext<'_>| -> Result<String> {
        count.set(count.get() + 1);
        assert_eq!(context.computed_box.count, 4);
        assert_eq!(context.quantile_probabilities, [0.1, 0.25, 0.5, 0.75, 0.9]);
        assert_eq!(context.computed_box.quantiles[2], 2.5);
        assert_eq!(
            context.cell_index,
            context.group_index * 3 + context.subgroup_index
        );
        assert!(context.observation_indices.iter().all(|&i| {
            context.data[i].group == context.computed_box.group
                && context.data[i].subgroup == context.computed_box.subgroup
        }));
        Ok(format!(
            "{} / {}",
            context.computed_box.group, context.default_text
        ))
    };
    let mut controller = BoxPlotInteraction::with_legend_label_factory(
        &config,
        &data,
        &options,
        &BoxPlotInteractionOptions::default(),
        &mut label,
    )?;
    assert_eq!(calls.get(), 4);
    assert_eq!(
        controller
            .layout
            .scene
            .legend
            .iter()
            .map(|item| item.label.as_str())
            .collect::<Vec<_>>(),
        ["B / Plan", "A / Plan", "B / Observed", "A / Observed"]
    );
    let row = &controller.layout.scene.legend[0];
    let target = controller.layout.scene.legend_target(&row.id).unwrap();
    assert_eq!(target, LegendTarget::Series(boxplot_series_id("y")));
    assert_ne!(row.id, target.id()); // Display-row identity is not a hidden-series key.
    controller.set_active(Some(BoxPlotKey::new("B", "y")))?;
    assert_eq!(calls.get(), 4); // Controlled hover uses cached legend rows.

    let app = glory::App::memory();
    let source = app
        .scope()
        .cage(controller.whole_parts_scene_interaction()?);
    let proposals = Rc::new(RefCell::new(Vec::<BTreeSet<String>>::new()));
    let output = proposals.clone();
    app.mount(
        chart_layout(source)
            .highlight(false)
            .on_hidden_change(move |hidden| output.borrow_mut().push(hidden)),
    )?;
    app.flush()?;
    app.dispatch(legend_node(&app, "Hide B / Plan")?, Event::Activate)?;
    assert_eq!(
        proposals.borrow()[0],
        BTreeSet::from([boxplot_series_id("y")])
    );
    // Both differently labelled y rows share the same canonical visibility bit.
    legend_node(&app, "Show A / Plan")?;
    app.dispatch(legend_node(&app, "Show B / Plan")?, Event::Activate)?;
    assert!(proposals.borrow()[1].is_empty());
    assert_eq!(calls.get(), 4);

    let before = controller.layout.clone();
    assert!(controller.replace_source(&config, &data, &options).is_err());
    assert_eq!(controller.layout, before);
    data.retain(|datum| datum.group != "A" || datum.subgroup != "y");
    controller.replace_source_with_legend_label_factory(&config, &data, &options, &mut label)?;
    assert_eq!(calls.get(), 7);
    let next = controller.whole_parts_scene_interaction()?;
    source.revise(|old| *old = next.clone());
    app.flush()?;
    assert_eq!(controller.layout.scene.legend.len(), 3);

    #[cfg(feature = "raster")]
    {
        let raster_app = glory::App::memory();
        let raster = RasterSurface::mount_layout(
            raster_app.scope(),
            next,
            render::raster::FontRegistry::new(),
            RasterSurfaceOptions {
                highlight: false,
                ..Default::default()
            },
        )?;
        raster_app.mount(raster.widget())?;
        raster_app.flush()?;
        assert_eq!(raster.handle().frame()?.width, 480);
    }
    Ok(())
}
```

The pure layout entries are `boxplot_layout_with_legend_label_factory` and `boxplot_layout_with_scale_options_and_legend_label_factory`. The corresponding controller constructors are `BoxPlotInteraction::with_legend_label_factory` and `with_scale_options_and_legend_label_factory`. They borrow a `for<'a> FnMut(&BoxPlotLegendContext<'a>) -> Result<String>` for that call and never store it. A factory requires explicit `ByLabel`; supplying one with `Keys` rejects before any callback. `ByLabel` without a factory uses the existing caption-map value when present, otherwise the selected raw color key, including an empty subgroup's empty string.

The context exposes the actual `computed_box`, borrowed original `data`, and `observation_indices` for that cell in caller order, including `None`. Statistics, `computed_box.count` and source IDs continue to omit missing values. `summary_index` describes the actual visible-summary sequence. `group_index`, `subgroup_index`, `cell_index` and `color_index` refer to the complete configured domains, preserving reserved and hidden slots; `cell_index = group_index * subgroup_count + subgroup_index`. `color_by`, `raw_color_key` and numeric `color` retain the palette identity. `default_text` is the map caption or raw key before the factory. `quantile_probabilities` contains the configured probabilities; `computed_box.quantiles` contains the calculated statistical values. They are distinct, including for fractional UTC statistics.

With legends enabled, the factory runs once per actual visible nonempty summary in builder order. Real zero, constant, transparent and domain-cropped summaries remain eligible. Reserved and None-only cells, and summaries excluded by `BoxPlotLayoutOptions::hidden`, do not invent callback contexts. `legend: false` invokes no factory. All source, scale, geometry, paint and other scene-resource checks finish before the first callback. Each returned label is checked immediately against 1024 UTF-8 bytes. Existing source and cell budgets bound candidates; the legend budget applies after deduplication: 2048 candidates can produce one row, while the 257th distinct label rejects immediately. Caption-map keys and byte limits retain their eager validation even when legends are disabled.

Controller hover and legend UI updates reuse cached text and targets; they do not run the factory again. Use `replace_source_with_legend_label_factory` to rebuild with a new borrowed factory while keeping the configured value scale, or `replace_source_with_scale_options_and_legend_label_factory` to replace that scale too. A still-present active business key survives; removing or hiding it clears it. A factory-built controller rejects the old no-factory replacement entries even when construction had no rows or `legend: false`; the policy is not guessed from an empty target map. Failed construction/replacement preserves the previously accepted frame and active state. External callback side effects already performed before an error are not rolled back. Public frame tampering still causes subsequent controlled operations to reject.

A legend row ID is a unique display/transport identity, not a series ID. `Scene::legend_targets` maps rows to `LegendTarget::Series(canonical_id)` or `Category(raw_key)`. BoxPlot ByLabel always supplies an explicit target. `Scene::legend_target(row_id)` resolves an existing row; absent map entries keep the old role plus row-ID fallback. An explicit target determines the role, and conflicting role metadata rejects. A Category target may contain an empty raw key; BoxPlot's own group-key validation still requires nonempty group keys. Category rows are static unless actions are explicitly enabled, and never hide series.

`LegendActivation::target` is the optional immutable target snapshot of an activation intent; `None` preserves the old role/item-ID contract, and `resolved_target()` resolves that snapshot. Notification precedes a series toggle, so it is not confirmation of a hidden-state commit. Hosts recheck the accepted row, role and target after callbacks or queued work. Custom legend widgets receive a live `LegendRenderContext::target` bond separately from the row item and displayed label. Their content/symbol widget renderers are separate from this String-producing layout factory.

Use `chart_layout` for the semantic Glory Chart with its actual Graphic plot. For browser SVG or Canvas, use `BrowserChart::mount` with `BrowserRenderer::Svg` or `Canvas`, supply the binding via `BrowserOptions::interaction_data` and the plot via `BrowserOptions::interaction.plot`, and replace Scene/binding together with `set_layout`. For CPU publication and Glory controls, use `RasterSurface::mount_layout` and the raster handle's `set_layout`. When the BoxPlot controller owns highlighting, disable the host's additional dimming with Chart `.highlight(false)`, `BrowserHighlight::Controlled`, or `RasterSurfaceOptions { highlight: false, .. }`. For this controlled whole-part path choose `MotionPreference::Reduced`; painted picking also requires its existing Geometry admission; see [whole-part pointer input](whole-part-pointer.md). Enable category actions with Chart `.on_legend_activate`, or mount-time `legend_actions: true` plus the corresponding Browser/Raster subscriber. Subscribing alone does not enable Browser/Raster category actions.

Scene metadata is distinct from a visible legend. Standalone SVG descriptions can expose legend text, and ordinary CanvasScript/CPU scene exports replay scene marks and labels; these exports do not add the hosts' visible legend controls. Legend presentation text overrides happen after factory topology and do not rerun deduplication. The [original-function reference](../tests/fixtures/box-legend-factory/README.md) records real Nivo hooks, its property accessor and Lodash's first-representative/stable-index behavior. Nivo's `Number(null)` creates zero samples where Rust omits `None`; native numeric property labels require explicit conversion to Rust `String`. This API does not implement arbitrary JavaScript null/object/property access or promise React memo, lifecycle or multiple-legend-spec callback counts. GPU and worker rendering remain deferred.

New public struct fields need source-literal updates: use `..Default::default()` where appropriate, add `legend_mode`/`legend_targets` to exhaustive literals, and use `target: None` for a legacy `LegendActivation` literal. A manually constructed `LegendRenderContext` needs its target bond. Omitting default fields from Debug preserves legacy text output; it does not make exhaustive Rust struct literals source-compatible without these updates.
