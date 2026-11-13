# BoxPlot 逐摘要源颜色

[English](boxplot-color-factory.md)

使用 `BoxPlotFactories { color, legend_label }` 计算类型化源颜色，也可组合[逐摘要 ByLabel 图例工厂](boxplot-legend-factory.zh-CN.md)。两个字段都默认 `None`。均未提供回调时，构建器直接采用既有路径，默认 `Keys` 文字、布局、Debug、SVG 和 Canvas 输出保留原有行为。

下面的完整 Memory 程序按真实摘要返回 RGBA，将颜色共享给箱体、默认离群点和继承描边，并检查 ByLabel 的首代表。程序通过 canonical 目标激活真实 Glory 图例控件，受控悬停复用缓存颜色，源替换时显式重供两类工厂。两个闭包在每次构建结束后退出作用域。该程序验证 Memory 状态和语义控件，不声称绘制或比较了物理像素。

使用 `cargo run --example boxplot_color_factory_guide` 运行示例。

```rust
use kartis::*;
use glory::render::{Event, GraphicStroke, NodeId, Props, StrokeCap};
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
            .any(|s| s == "kartis-legend-item")
            && matches!(node.descriptor.view().props, Props::Button(button) if button.label == label)
        {
            return Ok(id);
        }
        pending.extend(node.children.iter().copied());
    }
    Err(ChartError("expected legend control is absent".into()))
}

fn expected_color(group: &str, subgroup: &str) -> Color {
    if group == "A" {
        Color(160, 80, 60, 128)
    } else if subgroup == "x" {
        Color(30, 110, 150, 200)
    } else {
        Color(60, 80, 100, 160)
    }
}

fn source_color(context: &BoxPlotColorContext<'_>) -> Result<Color> {
    assert_eq!(context.computed_box.count, 6);
    assert_eq!(context.quantile_probabilities, [0.1, 0.25, 0.5, 0.75, 0.9]);
    assert_eq!(
        context.cell_index,
        context.group_index * 3 + context.subgroup_index
    );
    assert!(context.observation_indices.iter().all(|&index| {
        let raw = &context.data[index];
        raw.group == context.computed_box.group && raw.subgroup == context.computed_box.subgroup
    }));
    // A nullable source remains available without becoming a zero sample.
    if context.computed_box.group == "B" && context.computed_box.subgroup == "y" {
        assert_eq!(context.observation_indices.len(), 7);
        assert!(
            context
                .observation_indices
                .iter()
                .any(|&i| context.data[i].value.is_none())
        );
    }
    Ok(if context.computed_box.mean >= 20. {
        Color(160, 80, 60, 128)
    } else if context.raw_color_key == "x" {
        Color(30, 110, 150, 200)
    } else {
        Color(60, 80, 100, 160)
    })
}

fn caption(context: &BoxPlotLegendContext<'_>) -> Result<String> {
    assert_eq!(
        context.color,
        expected_color(&context.computed_box.group, &context.computed_box.subgroup)
    );
    // B/y and A/y return the same text; B/y is the first representative.
    Ok(context.default_text.to_string())
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
            for (i, value) in [0., 2., 4., 6., 8., 10.].into_iter().enumerate() {
                let offset = if group == "A" { 20. } else { 0. };
                data.push(BoxObservation::new(
                    format!("{group}/{subgroup}/{i}"),
                    group,
                    subgroup,
                    Some(value + offset),
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
        fill_opacity: 0.5,
        border: Some(GraphicStroke {
            color: Color(9, 9, 9, 255),
            width: 2.,
            cap: StrokeCap::Butt,
        }),
        border_color_profile: Some(InheritedColor::FromContext(vec![])),
        median_color_profile: Some(InheritedColor::FromContext(vec![ColorModifier::Darker(1.)])),
        whisker_color_profile: Some(InheritedColor::FromContext(vec![ColorModifier::Opacity(
            0.5,
        )])),
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

    // No factories preserve the ordinary Keys layout and export paths.
    let mut keys = options.clone();
    keys.legend_mode = BoxPlotLegendMode::Keys;
    let legacy = boxplot_layout(&config, &data, &keys)?;
    let unchanged =
        boxplot_layout_with_factories(&config, &data, &keys, BoxPlotFactories::default())?;
    assert_eq!(legacy, unchanged);
    assert_eq!(
        render::to_svg(&legacy.scene)?,
        render::to_svg(&unchanged.scene)?
    );
    assert_eq!(
        render::to_canvas_script(&legacy.scene, "chart")?,
        render::to_canvas_script(&unchanged.scene, "chart")?
    );

    let color_calls = Rc::new(Cell::new(0));
    let label_calls = Rc::new(Cell::new(0));
    let mut controller = {
        let colors = color_calls.clone();
        let labels = label_calls.clone();
        let mut color = move |context: &BoxPlotColorContext<'_>| {
            colors.set(colors.get() + 1);
            source_color(context)
        };
        let mut label = move |context: &BoxPlotLegendContext<'_>| {
            labels.set(labels.get() + 1);
            caption(context)
        };
        BoxPlotInteraction::with_factories(
            &config,
            &data,
            &options,
            &BoxPlotInteractionOptions::default(),
            BoxPlotFactories {
                color: Some(&mut color),
                legend_label: Some(&mut label),
            },
        )?
    }; // Both closures have ended. The controller retains resolved results.
    assert_eq!((color_calls.get(), label_calls.get()), (4, 4));
    let scene = &controller.layout.scene;
    assert_eq!(
        scene
            .legend
            .iter()
            .map(|row| row.label.as_str())
            .collect::<Vec<_>>(),
        ["Plan", "Observed"]
    );
    assert_eq!(scene.legend[0].color, Color(60, 80, 100, 160));
    let row_id = scene.legend[0].id.clone();
    assert_eq!(
        scene.legend_target(&row_id),
        Some(LegendTarget::Series(boxplot_series_id("y")))
    );
    assert_ne!(row_id, boxplot_series_id("y"));
    for summary in &controller.layout.boxes {
        let color = expected_color(&summary.group, &summary.subgroup);
        let body = scene.marks.iter().find(|m| m.id == summary.id).unwrap();
        assert_eq!(body.fill, Some(color));
        assert_eq!(body.opacity, 0.5); // Source alpha is not premultiplied.
        let border = scene
            .marks
            .iter()
            .find(|m| m.id == format!("{}:border", summary.id))
            .unwrap();
        assert_eq!(border.stroke.unwrap().color, color);
        assert!(!summary.outliers.is_empty());
        for outlier in &summary.outliers {
            let id = format!(
                "{}:outlier:{}:{}",
                summary.id,
                outlier.source_id.len(),
                outlier.source_id
            );
            assert_eq!(
                scene.marks.iter().find(|m| m.id == id).unwrap().fill,
                Some(color)
            );
        }
    }
    let first = &controller.layout.boxes[0];
    let median = scene
        .marks
        .iter()
        .find(|m| m.id == format!("{}:median", first.id))
        .unwrap();
    assert_eq!(median.stroke.unwrap().color, Color(42, 56, 70, 160));
    let whisker = scene
        .marks
        .iter()
        .find(|m| m.id == format!("{}:whiskers", first.id))
        .unwrap();
    assert_eq!(whisker.stroke.unwrap().color, Color(60, 80, 100, 128));

    let cached = controller.layout.clone();
    controller.set_active(Some(BoxPlotKey::new("A", "x")))?;
    controller.set_active(None)?;
    assert_eq!(controller.layout, cached);
    assert_eq!((color_calls.get(), label_calls.get()), (4, 4));
    controller.set_active(Some(BoxPlotKey::new("A", "x")))?;

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
    app.dispatch(legend_node(&app, "Hide Plan")?, Event::Activate)?;
    assert_eq!(
        proposals.borrow()[0],
        BTreeSet::from([boxplot_series_id("y")])
    );
    app.dispatch(legend_node(&app, "Show Plan")?, Event::Activate)?;
    assert!(proposals.borrow()[1].is_empty());
    assert_eq!((color_calls.get(), label_calls.get()), (4, 4));

    let before = controller.layout.clone();
    assert!(controller.replace_source(&config, &data, &options).is_err());
    assert_eq!(controller.layout, before);
    data.retain(|datum| datum.group != "B" || datum.subgroup != "y");
    {
        let colors = color_calls.clone();
        let labels = label_calls.clone();
        let mut color = move |context: &BoxPlotColorContext<'_>| {
            colors.set(colors.get() + 1);
            source_color(context)
        };
        let mut label = move |context: &BoxPlotLegendContext<'_>| {
            labels.set(labels.get() + 1);
            caption(context)
        };
        controller.replace_source_with_factories(
            &config,
            &data,
            &options,
            BoxPlotFactories {
                color: Some(&mut color),
                legend_label: Some(&mut label),
            },
        )?;
    }
    assert_eq!((color_calls.get(), label_calls.get()), (7, 7));
    assert_eq!(controller.active(), Some(&BoxPlotKey::new("A", "x")));
    // A/y becomes the representative; its source color travels with its row.
    assert_eq!(
        controller.layout.scene.legend[0].color,
        Color(160, 80, 60, 128)
    );
    let next = controller.whole_parts_scene_interaction()?;
    source.revise(|old| *old = next.clone());
    app.flush()?;
    legend_node(&app, "Hide Plan")?;
    Ok(())
}
```

纯布局入口是 `boxplot_layout_with_factories` 和 `boxplot_layout_with_scale_options_and_factories`，对应 controller 构造器为 `BoxPlotInteraction::with_factories` 和 `with_scale_options_and_factories`。`BoxPlotColorFactory` 借用 `BoxPlotColorContext` 并返回 `Result<Color>`；可选文字工厂返回 `Result<String>`。回调只在这次构建期间借用，不存入布局或 controller。

`computed_box` 是真实的普通投影摘要，包含源统计量、矩形和离群点。`data` 借用原始观测；`observation_indices` 按调用者顺序选择原始单元，包含可空记录，而 `count`、统计值和源 ID 仍省略 `None`。`summary_index` 表示实际可见摘要的顺序。`group_index`、`subgroup_index`、`cell_index` 和 `color_index` 使用完整配置域，保留预留和隐藏槽位的索引；`cell_index = group_index * subgroup_count + subgroup_index`。`raw_color_key` 与 `color_by` 标识所选组或子组，`default_color` 是它的固定或数字色盘颜色。`quantile_probabilities` 是配置概率，`computed_box.quantiles` 是实际统计值，两者不可混用。

颜色工厂按构建器顺序，对每个实际可见且非空的摘要调用一次，真实零值、常量、透明和被值域裁剪的摘要仍符合资格。隐藏、预留和只有空值的单元不造摘要，也不造回调。`legend: false` 仍调用颜色工厂。所有颜色调用先完成，随后才调用图例文字工厂。文字工厂沿用现有资格：显式 `ByLabel`、启用图例，并对每个实际摘要调用一次。返回空文字仍是合法图例行，遵循既有 1024 UTF-8 字节限制和去重后的行数预算。

一个私有源颜色缓存供箱体填充、默认离群点填充、箱边／中位线／须线继承颜色和交互站点代表色共同使用。显式 `outlier_fill` 仍可独立覆盖。继承角色必须显式选择；固定描边或 `InheritedColor::Theme` 保留该描边的配置颜色。线宽、端帽、几何与原始数值独立于回调。返回 RGBA 保持 straight alpha，不预乘 `fill_opacity` 或受控整箱透明度，这些仍是独立合成输入。

组合工厂中，`BoxPlotLegendContext::color` 是解析后的源颜色；原有纯文字工厂入口仍提供数字色盘颜色。ByLabel 按返回文字精确去重，保留第一个完整源代表，再按完整颜色域索引稳定排序。代表同时提供行颜色和类型化目标；不同键返回同一文字不会产生联合动作。颜色工厂配合 `Keys` 时，每个键的首个实际摘要提供行颜色。没有摘要的预留 SubGroup 行保留色盘回退，不伪造颜色调用。Group／Category 与 SubGroup／Series 的可见性语义继续分开。

行 ID 和 caption 都不是隐藏系列的键。使用 `Scene::legend_target(row_id)` 或宿主提供的类型化激活目标。示例中，`Plan` 行在首代表改变后仍指向 `boxplot_series_id("y")`。Category 目标默认静态，始终不切换系列；动作须由宿主显式启用。Browser SVG／Canvas 与 Raster 的布局发布方式见[图例目标与宿主 API 指南](boxplot-legend-factory.zh-CN.md)。controller 管理高亮时，应像示例的 `.highlight(false)` 一样关闭宿主额外 dimming。

悬停、图例展示及宿主普通隐藏／恢复复用缓存，不重调任何工厂。通过 `replace_source_with_factories` 重建来源，或使用 `replace_source_with_scale_options_and_factories` 同时替换配置数值刻度。仍存在的激活业务键会保留；删除、隐藏或变为空单元则清除。使用颜色工厂构造的 controller 会记录该策略，即使源为空或 `legend: false`。文字工厂有独立构造策略。显式源替换必须重供此前使用过的每类回调，即使它此前调用了零次。原无工厂替换入口和纯文字替换入口不能丢弃颜色工厂策略；缺少必要回调时，在任何回调执行前拒绝。

已知源、刻度、继承 profile 内在参数、几何、画笔和资源检查先于首个用户回调。部分资格依赖返回颜色，例如 Contrast 的有效背景仍半透明时，可以在颜色回调完成后拒绝。文字错误及字节／行数预算错误也可能发生在更早的回调之后。构建或替换失败会保留输入、此前已接受帧和激活状态；回调已造成的外部副作用不能回滚。编辑公开已接受布局不会刷新其来源凭证，之后的受控悬停、binding 和源替换将拒绝该帧。

此 API 提供完整摘要调用，不复刻 Nivo 函数色标的整个生命周期。Nivo 会先用只有 `{ group }` 或 `{ subGroup }` 的部分对象预热配置颜色域，悬停重新计算箱体时也可能重调颜色函数。Rust 不执行这两种调用；次数描述的是有界构建合同，不是 React render、hook 或动画帧次数。Nivo 的 `Number(null)` 可将缺失样本转成零，Rust 则省略 `None`。不会伪造任意 JavaScript datum 属性 getter，也不合成 `BoxObservation` 没有的字段。类型化 `Color(r, g, b, 0)` 保留 RGB，而 CSS／D3 解析路径可能在 alpha 为零时丢弃 RGB。[原函数参考资料](../tests/fixtures/box-colors-function/README.zh-CN.md)记录了这些差异，不需要为此改动共享继承颜色解析器。GPU 与 worker 渲染继续后置。
