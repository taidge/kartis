# 自定义 HeatMap 单元

`heatmap_layout_with_cell_factory` 将每个可见源单元替换为一个经过校验的 Scene mark。`heatmap_layout_with_color_profile_and_cell_factory` 还可选择共享插值配置或命名配色方案。两者都保留原有 HeatMap 布局、可空元数据和受控悬停行为。

```rust
use kartis::*;
use glory::render::{GraphicStroke, StrokeCap};

let config = ChartConfig {
    width: 320.,
    height: 180.,
    margin: Margin { left: 20., right: 20., top: 20., bottom: 20. },
    ..Default::default()
};
let cells = [
    HeatCell { row: "row".into(), column: "empty".into(), value: None },
    HeatCell::new("row", "zero", 0.),
    HeatCell::new("row", "value", 7.),
];
let options = HeatmapLayoutOptions {
    axes: HeatmapAxes::none(),
    labels: false,
    hover_target: HeatmapHoverTarget::Cell,
    ..Default::default()
};
let layout = heatmap_layout_with_color_profile_and_cell_factory(
    &config, &cells, &options, ColorScheme::Viridis, |cell| {
        let [x, y] = cell.position;
        let mut ink = cell.inherit(Geometry::Circle { x, y, radius: 14. });
        match cell.datum.value {
            None => ink.fill_paint = Some(FillPaint::Pattern(PatternFill::default())),
            Some(0.) => {
                ink.fill = None;
                ink.stroke = Some(GraphicStroke {
                    color: Color(20, 30, 40, 255), width: 2., cap: StrokeCap::Butt,
                });
            }
            Some(_) => {}
        }
        Ok(ink)
    },
)?;
let active = layout.with_active(Some(HeatmapCellKey::new("row", "empty")))?;
let binding = active.interaction_binding()?;
assert!(binding.sites.iter().any(|site| site.value.is_none()));
assert!(binding.sites.iter().any(|site| site.value == Some(0.)));
let svg = render::to_svg(&active.scene)?;
assert!(svg.contains("<pattern"));
# Ok::<(), kartis::ChartError>(())
```

回调接收 `HeatmapCellContext`：原始 datum、类型化行列 key、稳定 mark ID、原始输入 `source_index`、格式化数值、已解析 band、逻辑数据位置、绘图区、配置后的几何、颜色和边框。回调按配置的行列顺序执行。隐藏单元和稀疏数据中不存在的组合不会触发回调；显式 `None` 单元会触发。`ComputedHeatmapCell`、tooltip 和交互绑定始终区分 `None` 与 `Some(0.)`，通用 Scene mark 则以有限零标量兼容显式空值。

`HeatmapCellGeometry` 复用 Scatter/Line 的 `ScatterNodeGeometry` 契约。返回实际 `Geometry`，并可配置纯色、`FillPaint`、描边、dash 和透明度系数。矩形与圆形以 `position` 为中心。Path 可以让可见墨迹偏离逻辑原点，经校验的锚点会独立保留原始位置。`HeatmapLayout::hit_test` 使用实际绘制几何拾取。自定义 mark 按绘图区裁剪。

`cell.inherit(geometry)` 继承解析后的填充和边框，初始透明度系数为 1。该系数乘以配置的基础、活动或非活动透明度，也作用于所属标签的 alpha。既有标签保留配置的数值/空值文本、位置和行所有权。逐单元标签 accessor 与任意 Glory Widget 子组件属于独立能力；本 factory 为每个单元返回一个 Scene mark。

颜色域在回调前按既有 HeatMap 可见性策略解析。自定义渐变或图案不会改写返回的 `color_scale` 及其数值图例。自定义单元的 `color` 和绑定颜色使用实际画笔的代表色。当数据颜色图例适合所设计的编码时，可将返回的尺度传给 `append_color_legend`。

自定义布局私有保存已接受的元数据、mark 和所属标签。`set_active`、`with_active` 和 `interaction_binding` 会在返回新帧前拒绝被篡改的自定义几何、画笔、源数据或标签所有权。新的 binding API 也为默认布局校验原始身份、可空值、格式化数值、band 和逻辑原点。默认 Scene 的普通外观与装饰仍可编辑，原有默认悬停行为保持不变。数据、可见性或尺寸变化需要重新布局。回调错误、无效几何/画笔，以及既有 Scene、渐变、dash、锚点预算超限都会拒绝候选结果。

## 独立消费示例

运行 `cargo run --features raster,geojson --example heatmap_custom -- OUTPUT_DIRECTORY`，将 `OUTPUT_DIRECTORY` 替换为目标输出目录。示例通过 `FontRegistry::register` 注册仓库自带的 ABeeZee fixture，导出 `index.html`、两份 SVG、两份 CanvasScript、DPR 1/2 的四张 CPU PNG 和 `receipt.json`。

示例将显式空值绘制为条纹菱形，将零值绘制为空心圆，将正值绘制为渐变菱形。第二帧激活空值单元并淡化其余单元。同一批 Rust Scene 同时供 SVG、CanvasScript 和 CPU raster 使用。图表标签关闭，以单独验证几何与画笔；本示例不建立通用字体一致性结论。

[`scripts/heatmap-custom.js`](../scripts/heatmap-custom.js) 是面向生成页面的异步 Playwright page 校验函数。调用者提供并导航 page 后再执行该函数。它检查类型化源值、实际 SVG 几何、解码 SVG 与 Canvas 的整帧平均通道误差小于 0.5、源坐标处的图案/渐变/空心像素，以及篡改后恢复的 DOM/Canvas 负控。返回值包含几何、像素证据和用于保存产物的 PNG data URL。固定消费场景在 DPR 1/2 下验证；浏览器事件、任意 Widget factory 和物理设备行为不在此检查范围内。
