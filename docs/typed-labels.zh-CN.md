# HeatMap 与 Bar 的类型化标签文字

使用返回 `Result` 的类型化文字回调，替换图表拥有的数值标签，同时保留原始数据、几何布局与交互元数据。文字回调可以与几何回调组合使用。已有入口和默认选项保持原来的行为。

| 图表 | 借用的回调 | 构建入口 |
| --- | --- | --- |
| HeatMap | `HeatmapFactories { cell, label }` | `heatmap_layout_with_factories`；`heatmap_layout_with_color_profile_and_factories` |
| Bar | `BarFactories { bar, label }` | `bar_layout_with_factories`；`bar_layout_with_scale_options_and_factories` |

各字段都可省略。HeatMap 的 profile 入口接受共享 `ColorScaleProfile` 支持的转换；Bar 的 scale 入口接受 `ContinuousScaleOptions`。文字回调返回 `Result<String>`，几何回调返回已有的受检几何类型。闭包仅在构建期间被借用，返回的布局不会保存闭包。下方示例在开始 hover 更新前，就结束了两种闭包的作用域。

`HeatmapLabelContext` 借用原始 `HeatCell`、类型化行列键、mark ID 和原始输入下标 `source_index`，并提供可空的 `formatted_value`、`default_text`、单元格 band、逻辑原点 `position`、普通标签位置 `label_position` 和 plot。`BarLabelContext` 提供原始系列与分类及其输入下标、mark ID、原始值和显示值、堆叠区间、默认文字、逻辑原点、标签位置与 plot。因此，即使数值相同，不同行、系列或分类也能显示不同文字，而不改变原来的身份。

完整示例包含 HeatMap 的显式空值与零值、相同数值按行区分的文字、一个空的自有标签、零长度 Bar 标签、继承标签颜色配置、自定义几何透明度，以及受控 hover 的恢复。

```rust
use kartis::*;
use std::cell::RefCell;

fn main() -> kartis::Result<()> {
    let config = ChartConfig::default();
    let heat_data = [
        HeatCell { row: "north".into(), column: "same".into(), value: None },
        HeatCell::new("south", "same", 0.),
        HeatCell::new("north", "other", 0.),
        HeatCell::new("east", "empty", 1.),
    ];
    let heat_events = RefCell::new(Vec::new());
    let mut heat = {
        let mut text = |context: &HeatmapLabelContext<'_>| {
            heat_events.borrow_mut().push(("label", context.source_index));
            Ok(if context.datum.row == "east" {
                String::new()
            } else {
                format!("{} / {}: {}", context.datum.row, context.datum.column, context.default_text)
            })
        };
        let mut cell = |context: &HeatmapCellContext<'_>| {
            heat_events.borrow_mut().push(("cell", context.source_index));
            let mut mark = context.inherit(context.geometry.clone());
            mark.opacity = 0.5;
            Ok(mark)
        };
        heatmap_layout_with_factories(
            &config,
            &heat_data,
            &HeatmapLayoutOptions {
                axes: HeatmapAxes::none(),
                hover_target: HeatmapHoverTarget::Cell,
                empty_label: "NA".into(),
                label_color_profile: Some(InheritedColor::Fixed(Color(40, 50, 60, 101))),
                opacity: 0.8,
                ..Default::default()
            },
            HeatmapFactories { cell: Some(&mut cell), label: Some(&mut text) },
        )
    }?;
    assert_eq!(*heat_events.borrow(), vec![
        ("label", 3), ("cell", 3), ("label", 2), ("cell", 2),
        ("label", 0), ("cell", 0), ("label", 1), ("cell", 1),
    ]);
    assert_eq!(heat.scene.labels.iter().map(|label| label.text.as_str()).collect::<Vec<_>>(),
        vec!["", "north / other: 0", "north / same: NA", "south / same: 0"]);
    assert!(heat.scene.labels.iter().all(|label| label.color == Color(40, 50, 60, 40)));
    let binding = heat.interaction_binding()?;
    assert!(binding.sites.iter().any(|site| site.value.is_none()));
    assert!(binding.sites.iter().any(|site| site.value == Some(0.)));
    let base = heat.scene.clone();
    heat.set_active(Some(HeatmapCellKey::new("north", "same")))?;
    heat.set_active(None)?;
    assert_eq!(heat.scene, base);
    assert_eq!(heat_events.borrow().len(), 8);

    let categories = ["A", "B", "C"].map(str::to_string);
    let data = [BarSeries::new("values", vec![Some(0.), None, Some(5.)])];
    let bar_events = RefCell::new(Vec::new());
    let mut bar = {
        let mut text = |context: &BarLabelContext<'_>| {
            bar_events.borrow_mut().push(("label", context.source_category));
            Ok(format!("{}={}", context.category, context.default_text))
        };
        let mut geometry = |context: &BarDatumContext<'_>| {
            bar_events.borrow_mut().push(("bar", context.source_category));
            let mut mark = context.inherit(context.geometry.clone());
            mark.opacity = 0.5;
            Ok(mark)
        };
        bar_layout_with_factories(
            &config,
            &categories,
            &data,
            &BarOptions { value_domain: Some([0., 10.]), ..Default::default() },
            &BarStyleOptions {
                labels: BarLabels {
                    enabled: true,
                    zero_extent: BarZeroExtentLabels::ValueAxis,
                    color_profile: Some(InheritedColor::Fixed(Color(40, 50, 60, 101))),
                    ..Default::default()
                },
                ..Default::default()
            },
            BarFactories { bar: Some(&mut geometry), label: Some(&mut text) },
        )
    }?;
    assert_eq!(*bar_events.borrow(), vec![("label", 0), ("bar", 0), ("label", 2), ("bar", 2)]);
    let zero = bar.layout.bars.iter().find(|datum| datum.raw_value == 0.).unwrap();
    let label = zero.label_index.unwrap();
    assert_eq!(bar.layout.scene.labels[label].text, "A=0");
    assert_eq!(bar.layout.scene.labels[label].color, Color(40, 50, 60, 51));
    assert_eq!(bar.interaction_binding()?.sites.len(), 2);
    let base = bar.layout.scene.clone();
    let active = bar.layout.bars.iter().find(|datum| datum.raw_value == 5.).unwrap().id.clone();
    bar.set_active(Some(&active), 0.2)?;
    assert_eq!(bar.layout.scene.labels[label].color, Color(40, 50, 60, 10));
    bar.set_active(None, 0.2)?;
    assert_eq!(bar.layout.scene, base);
    assert_eq!(bar_events.borrow().len(), 4);
    let _svg = render::to_svg(&bar.layout.scene)?;
    let _script = render::to_canvas_script(&heat.scene, "heatmap")?;
    Ok(())
}
```

对每个符合条件的标签，文字回调在该来源的几何回调之前执行一次。HeatMap 使用配置的行列顺序，Bar 使用普通布局的来源顺序。隐藏来源、稀疏数据中不存在的组合、关闭的标签都不会触发文字回调。Bar 还会跳过 `None`、完全裁剪掉的柱子，以及不满足原有可见性、位置或尺寸阈值的标签。可见 mark 的几何回调仍可能执行，即使其标签被关闭或跳过。这些回调次数以 Rust 实际接受的标签槽位为准，不承诺复刻 Nivo 对未渲染标签调用 accessor 的次数。

HeatMap 的显式 `None` 与 `Some(0.)` 保持区分，两者都可生成文字。空值单元格的 `default_text` 来自 `empty_label`，即使默认文字为空也允许回调替换。返回空字符串会保留图表拥有的 `Label`、行关联与槽位，只是不绘制字形。Bar 文字也采用相同的空结果规则。

`BarLabels.zero_extent` 默认为 `BarZeroExtentLabels::Skip`。选择 `ValueAxis` 后，如果可见分类宽度为正，数值轴方向的可见长度为零，且原始、未钳制的两个端点映射到同一个有限像素，就允许生成标签。这包括真实的零区间和比例尺取整导致的映射重合。非零区间被裁剪到 plot 边界而缩成零，不满足该条件。原有的 `skip_width`、`skip_height`、普通标签锚点位置、plot 包含判断及裁剪规则仍然生效；自定义几何回调不能使原本被跳过的标签获得资格。

自定义文字仅改变自有标签的 `text`。原始数值与空值、数值 `formatted_value`、tooltip、来源下标、ID 和交互元数据都保留原来的含义。`default_text` 仍由现有数值或空值格式规则生成。文字回调不会替换 tooltip 格式，也不会解析动态属性路径。

标签颜色继续使用配置的[继承标签颜色角色](inherited-label-colors.zh-CN.md)，在自定义几何之前根据原始调色板与主题解析。文字不会改变这个上下文，自定义填充的代表色也不会悄悄替代继承标签颜色。透明度仍按已有规则相乘：角色颜色的 alpha 乘以几何 factory 的系数，再乘以图表原有的基础、active 或 inactive 系数。示例中的 HeatMap 标签 alpha 为 `round(101 × 0.5 × 0.8) = 40`；Bar 标签为 `round(101 × 0.5) = 51`，在 inactive 系数 `0.2` 下变为 `10`。

`set_active` 与 `interaction_binding` 使用已经接受的来源与自有标签帧。Hover 只更新透明度，不会再次调用任何 factory；清除 hover 会恢复已接受的帧。使用 factory 的布局被接受后，篡改自有文字、行或系列归属、来源数据或已接受的几何与 paint，会使受检 binding 或 hover 操作失败。受控 hover 的拒绝是原子的。没有使用 factory 的默认布局保留原有的 Scene 装饰编辑行为。

提供文字 factory 时，配置与计划中的 Scene 预算会在回调前检查：按实际要生成的标签计数，包括已有坐标轴标签和 Bar totals，不按稠密的行列乘积计算。Scene 仍最多允许 20,000 个标签，每个回调结果都会立即检查已有的 1,024 字节 UTF-8 上限；标签总存储也受这些边界约束。回调报错或返回超限文字会拒绝构建，不改变借用的输入或已经接受的帧。调用者闭包自行产生的外部副作用，例如日志或计数器递增，不会被回滚。`label: None` 保留已有纯几何回调及其错误检查顺序。

这些 API 为 SVG、CanvasScript 与 CPU 渲染生成普通 Scene 标签和跨后端 mark，不提供任意 Glory Widget 插槽，本指南也不宣称已经通过跨浏览器像素验收。几何契约详见[自定义 HeatMap 单元格](heatmap-custom.zh-CN.md)与[自定义 Bar 几何](bar-custom-geometry.zh-CN.md)。
