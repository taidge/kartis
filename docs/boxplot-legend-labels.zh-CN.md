# BoxPlot 图例文字

[English](boxplot-legend-labels.md)

`BoxPlotLayoutOptions::legend_labels` 是可选的 `BTreeMap<String, String>`，按稳定原始身份指定显示文字。`color_by: Group` 使用原始 group 键；`SubGroup` 使用原始 subgroup 键，包括空字符串。SubGroup 映射键不是 `boxplot_series_id` 返回的规范 ID。这是 key-stable caption map，不是 Nivo 任意 `legendLabel` 工厂或属性访问器。

完整程序通过真实控制器更新 Group 文字，同时保留激活业务键、图例 ID、角色、颜色和标记；随后用原始键配置 SubGroup 文字，包括显式空 caption。

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

`None` 和 `Some(BTreeMap::new())` 保留原显示文字；未配置的键同样回退到原文字：Group 使用组名，SubGroup 使用子组名，空子组使用 `Samples`。已配置的空字符串就是空 caption，不会触发回退。

空 caption 或重复 caption 都保留原有每个图例 ID 与行。示例的两条 `Region` 仍是两个 Category 身份。Nivo 原 `getLegendData` 按 label 去重，相同文字可能合并不同身份；这里明确保留这一差异。本映射不增加任意回调、同一身份展开多个标签、新系列别名或图例动作。[原函数参考](../tests/fixtures/box-legend-labels/README.zh-CN.md) 记录真实 map 回调、去重及这些边界。

图例资格不变。Group 图例只包含子组隐藏过滤后仍有实际非空箱体的组；给预留组配置 caption 不会创建新行。悬停透明度为零不会移除其身份。SubGroup 保留完整配置域，包括预留和隐藏子组行。ID、Category／Series 角色、颜色、系列归属与隐藏行为，继续遵循[配色与受控悬停](boxplot-color-and-hover.zh-CN.md)。

每个映射键必须属于所选完整原始域，每条 caption 最多 1024 个 UTF-8 字节。即使 `legend: false`、没有观测值、来源已隐藏，或 caption 仅用于未使用的预留项，仍执行这两项检查。无效配置返回错误；控制器 `replace_source` 失败时保留已接纳帧与激活状态。仅更换 caption 时只改变图例文字，datum 提示、坐标轴标签、原始观测、统计量、几何、色盘和同组或同子组关联均保持不变。

更新受控 caption map 时，应把新布局选项传给 `replace_source`。直接修改公开的已接纳 Scene 不会刷新签名，反而会使后续受控操作拒绝。本指南说明已实现的 map 契约及聚焦检查，不表示全量原生或宿主验收已经完成。

Scene 保存图例元数据；启用图例的宿主提供可见图例控件。独立 SVG 将 caption 写入 `<desc>` 元数据，单独设置本选项不会增加绘制出来的图例。普通 CanvasScript 导出与 Scene CPU 图像只重放标记和文字，不内嵌可视宿主图例。程序中的 SVG 断言检查的是元数据文字，不是可见图例像素。宿主图例 Widget 可沿用既有展示 API 显示更新后的 caption。
