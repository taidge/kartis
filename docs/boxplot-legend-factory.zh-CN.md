# BoxPlot 逐摘要图例工厂

[English](boxplot-legend-factory.md)

`BoxPlotLayoutOptions::legend_mode` 默认为 `BoxPlotLegendMode::Keys`，保留原来的按键图例与 `legend_labels` 文字映射：重复或空 caption 保留各自原始行，SubGroup 包含完整配置域，空子组默认显示 `Samples`。Group 仅列出实际可见非空箱体所代表的组。参见[稳定键图例文字](boxplot-legend-labels.zh-CN.md)。

显式选择 `BoxPlotLegendMode::ByLabel`，即可使用 Nivo 的逐摘要文字拓扑。构建器对每个完全相同的返回 `String` 保留第一次出现的完整代表，再按所选完整颜色域索引稳定排序；行的数值色和类型化目标均来自这个代表。空文字也是实际保留的一行。因此同一子组可以产生多个不同文字行，指向同一个规范系列目标；不同键返回相同文字时只保留首代表的目标，不会变成联合动作。

完整程序生成四条摘要文字，激活真实 Glory 图例控件，验证不同文字行共享规范系列的隐藏位，再显式传入工厂替换源数据。条件编译的位图部分使用同一接受布局。内存宿主让程序自包含；浏览器应用可使用选定的 Glory 宿主挂载相同控件。

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

纯布局入口是 `boxplot_layout_with_legend_label_factory` 和 `boxplot_layout_with_scale_options_and_legend_label_factory`；对应控制器构造入口是 `BoxPlotInteraction::with_legend_label_factory` 与 `with_scale_options_and_legend_label_factory`。它们仅在本次调用中借用 `for<'a> FnMut(&BoxPlotLegendContext<'a>) -> Result<String>`，不保存闭包。工厂要求显式 `ByLabel`，与 `Keys` 同时传入会在首次回调前拒绝。`ByLabel` 不传工厂时，优先使用已有 caption 映射值，否则使用所选原始颜色键，包括空子组的空字符串。

上下文提供实际 `computed_box`、借用的原始 `data`，以及按调用者输入顺序排列、包含 `None` 的当前单元 `observation_indices`；统计、`computed_box.count` 和源 ID 仍省略缺失值。`summary_index` 表示实际可见摘要序列。`group_index`、`subgroup_index`、`cell_index` 和 `color_index` 使用完整配置域，保留预留与隐藏槽位；`cell_index = group_index * subgroup_count + subgroup_index`。`color_by`、`raw_color_key` 与数值 `color` 保留色盘身份；`default_text` 是进入工厂前的映射 caption 或原键。`quantile_probabilities` 是配置概率，`computed_box.quantiles` 是计算后的统计值，二者含义不同，UTC 小数统计也如此。

启用图例时，工厂按构建器顺序对每个实际可见非空摘要调用一次。真实零值、常值、透明或被值域裁掉的摘要仍有资格；预留及仅含 None 的单元，以及被 `BoxPlotLayoutOptions::hidden` 排除的摘要，不会凭空产生回调上下文。`legend: false` 不调用工厂。首次回调前完成源、比例尺、几何、画笔及其他场景资源检查。每次返回的文字立即检查 1024 UTF-8 字节限制。候选数量受已有源与单元预算约束，图例预算在去重后计算：2048 个候选可合并为一行，第 257 个不同文字则立即拒绝。即使禁用图例，caption 映射键与字节限制仍会提前校验。

控制器悬停与图例 UI 更新复用缓存文字和目标，不重新调用工厂。使用 `replace_source_with_legend_label_factory` 传入新借用工厂并保留配置值轴，或用 `replace_source_with_scale_options_and_legend_label_factory` 同时替换该比例尺。仍存在的激活业务键会保留，移除或隐藏后清除。经工厂构造的控制器即使最初没有图例行或设置了 `legend: false`，也明确拒绝旧的无工厂替换入口；该策略不会从空目标映射猜测。构造或替换失败会保留原接受帧与激活状态，但错误前已经发生的外部回调副作用不会回滚。篡改公开接受帧后，后续受控操作仍会拒绝。

图例行 ID 是唯一显示／传输身份，不是系列 ID。`Scene::legend_targets` 将行映射为 `LegendTarget::Series(canonical_id)` 或 `Category(raw_key)`；BoxPlot ByLabel 总是提供显式目标。`Scene::legend_target(row_id)` 解析已存在的行，没有映射条目时沿用原角色加行 ID 的回退规则。显式目标决定角色，与角色元数据冲突时拒绝。Category 目标允许原始空键，但 BoxPlot 自身仍要求非空组键。Category 行默认静态，只有显式启用动作时才可激活，并且始终不会隐藏系列。

`LegendActivation::target` 是激活意图中可选且不可变的目标快照；`None` 保留原角色／item ID 契约，`resolved_target()` 解析该快照。通知发生在系列切换之前，不代表隐藏状态已经提交；宿主会在回调或排队任务后重新核对接受行、角色与目标。自定义图例控件另外收到 live `LegendRenderContext::target` bond，与行 item 和显示文字分开。控件内容／符号渲染器与这个生成 String 的布局工厂是不同接口。

语义 Glory Chart 使用 `chart_layout` 和真实 Graphic 图形层。浏览器 SVG／Canvas 使用 `BrowserChart::mount` 配合 `BrowserRenderer::Svg` 或 `Canvas`，通过 `BrowserOptions::interaction_data` 提供 binding、通过 `BrowserOptions::interaction.plot` 提供 plot，再用 `set_layout` 一起替换 Scene／binding。CPU 图像发布与 Glory 控件使用 `RasterSurface::mount_layout` 和位图 handle 的 `set_layout`。由 BoxPlot 控制器负责高亮时，用 Chart `.highlight(false)`、`BrowserHighlight::Controlled` 或 `RasterSurfaceOptions { highlight: false, .. }` 关闭宿主额外淡化。这个受控整体部件路径应选择 `MotionPreference::Reduced`，墨迹拾取还须遵守已有 Geometry 准入，参见[整体部件指针](whole-part-pointer.zh-CN.md)。Category 动作通过 Chart `.on_legend_activate` 启用；Browser／Raster 需挂载时配置 `legend_actions: true` 并订阅对应回调，仅订阅不会启用其 Category 动作。

场景元数据与可见图例是两回事。独立 SVG 的描述可包含图例文字，普通 CanvasScript／CPU 场景导出重放场景 mark 和 label，不会自动加入宿主的可见图例控件。图例展示层文字覆盖发生在工厂拓扑之后，不重新去重。[原函数参考](../tests/fixtures/box-legend-factory/README.zh-CN.md)记录真实 Nivo hooks、属性访问器和 Lodash 首代表／完整索引稳定排序行为。Nivo 的 `Number(null)` 会产生零样本，而 Rust 省略 `None`；原生数字属性标签需要显式转换为 Rust `String`。此 API 不实现任意 JavaScript null／对象／属性访问，也不承诺 React memo、生命周期或多份 legend spec 的回调次数。GPU 与 worker 渲染继续后置。

新增公开字段需要调整完整结构体字面量：可适当使用 `..Default::default()`，为完整字面量补上 `legend_mode`／`legend_targets`；旧式 `LegendActivation` 字面量补 `target: None`，手工构造 `LegendRenderContext` 则需提供目标 bond。Debug 省略默认字段可保留旧文本输出，但不表示无需调整完整 Rust 结构体字面量就能保持源码兼容。
