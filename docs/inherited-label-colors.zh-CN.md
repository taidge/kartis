# Bar 与 HeatMap 标签继承颜色

配置 `BarLabels.color_profile` 或 `HeatmapLayoutOptions.label_color_profile`，即可复用共享 `InheritedColor` 角色。`None` 保留原固定标签样式或主题；`Some(Theme)` 也使用原样式/主题回退。`Fixed`、`FromContext` 和 `Contrast` 则显式覆盖固定颜色。

```rust
use kartis::*;

fn main() -> kartis::Result<()> {
    let config = ChartConfig::default();
    let profile = InheritedColor::FromContext(vec![ColorModifier::Darker(1.)]);
    let categories = ["A", "B"].map(str::to_string);
    let data = [BarSeries::new("sales", vec![Some(4.), Some(8.)])];
    let mut bar = bar_layout_with_bar_factory(
        &config,
        &categories,
        &data,
        &BarOptions::default(),
        &BarStyleOptions {
            colors: [("sales".into(), Color(20, 40, 60, 160))].into(),
            labels: BarLabels {
                enabled: true,
                color_profile: Some(profile.clone()),
                ..Default::default()
            },
            ..Default::default()
        },
        |context| {
            let mut mark = context.inherit(context.geometry.clone());
            mark.fill = Some(Color(240, 30, 180, 255));
            mark.opacity = 0.5;
            Ok(mark)
        },
    )?;
    let label = bar.layout.bars[0].label_index.unwrap();
    assert_eq!(bar.layout.scene.labels[label].color, Color(14, 28, 42, 80));
    let base = bar.layout.scene.clone();
    let active_id = bar.layout.bars[0].id.clone();
    bar.set_active(Some(&active_id), 0.2)?;
    bar.set_active(None, 0.2)?;
    assert_eq!(bar.layout.scene, base);

    let heat = heatmap_layout_with_cell_factory(
        &config,
        &[
            HeatCell { row: "row".into(), column: "empty".into(), value: None },
            HeatCell::new("row", "zero", 0.),
            HeatCell::new("row", "value", 10.),
        ],
        &HeatmapLayoutOptions {
            axes: HeatmapAxes::none(),
            hover_target: HeatmapHoverTarget::Cell,
            label_color_profile: Some(profile),
            colors: ColorScaleOptions {
                domain: Some([0., 10.]),
                colors: vec![Color(20, 40, 60, 160), Color(200, 180, 160, 220)],
                unknown: Color(90, 100, 110, 64),
                ..Default::default()
            },
            opacity: 0.8,
            active_opacity: 0.9,
            inactive_opacity: 0.2,
            ..Default::default()
        },
        |context| {
            let mut mark = context.inherit(context.geometry.clone());
            mark.fill = Some(Color(240, 30, 180, 255));
            mark.opacity = 0.5;
            Ok(mark)
        },
    )?;
    let zero = |frame: &HeatmapLayout| {
        frame.scene.labels.iter().find(|label| label.text == "0").unwrap().color
    };
    assert_eq!(zero(&heat), Color(14, 28, 42, 64));
    let active = heat.with_active(Some(HeatmapCellKey::new("row", "empty")))?;
    assert_eq!(zero(&active), Color(14, 28, 42, 16));
    assert_eq!(active.with_active(None)?.scene, heat.scene);
    let _binding = heat.interaction_binding()?;
    let _bar_svg = render::to_svg(&bar.layout.scene)?;
    let _heatmap_canvas = render::to_canvas_script(&heat.scene, "heatmap")?;
    Ok(())
}
```

示例的 palette RGBA 为 (20,40,60,160)。Darker(1) 得到 (14,28,42,160)。Bar 工厂透明度 0.5 使标签 alpha 为 80；HeatMap 基础透明度 0.8 再乘同一工厂透明度得到 alpha 64。激活缺失单元格后，零值单元格的非活动标签 alpha 为 16；清除 active key 后恢复原帧。

两种角色都在自定义几何和画笔工厂之前计算。继承色和对比度表面均来自数值布局的原 palette；HeatMap 的 NULL 使用 unknown 颜色。自定义品红填充、渐变或图案不会用交互画笔代表色替换该 palette。Contrast 在原 palette 表面和图表背景上比较候选颜色，不保证任意自定义画笔每个像素的对比度。

`ColorModifier::Opacity` 替换颜色 alpha，布局、工厂和受控高亮系数随后应用。每次已接受的高亮帧均从原解析 RGBA 出发，最终标签 alpha 只取整一次。Bar 原标签的显示资格、位置和数值格式不变；HeatMap 保留 NULL/空文字及标签来源归属校验。这一配置不增加逐项文字 accessor 或零尺寸柱标签。数据或 profile 变化应重建布局。

工厂调用前会验证 profile，包括隐藏源和关闭标签的配置。依赖上下文的校验使用实际源 palette；真正空源不会伪造一个颜色。现有 Contrast 规则要求有效背景不透明。非法 profile 会在受控帧被接受之前返回错误。

类型化 RGBA 在 alpha 为零时仍保留隐藏 RGB 通道，Opacity(1) 可以恢复它们。原 Nivo 部分 modifier 分支先通过 d3-color 解析 CSS alpha-zero 字符串，会丢失这些通道并可能得到黑色。参考数据同时保留两种真实结果；此 API 继续采用已有 Rust 类型化颜色契约。普通不透明、半透明 modifier profile 已对照原版 D3/Nivo 结果。

独立 SVG 对最终仅有一次实色填充、无描边和 paint server、采用普通混合的部分透明标记使用 `fill-opacity`，保留原透明度数值，避免 WebKit 的对象透明度舍入差异；其他标记继续使用对象透明度。这项导出修正不改 CanvasScript 或浏览器保留式 SVG。后处理 SVG 时应读取对应属性，不要把两种因子重复相乘。

新增了公开配置字段，构造时建议使用 `..Default::default()`。[Bar 几何与画笔](bar-custom-geometry.zh-CN.md) · [HeatMap 单元格](heatmap-custom.zh-CN.md)
