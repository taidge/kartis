# Waffle 自定义单元格

[English](waffle-custom.md)

这是**尚未发布的开发版 API**，不包含在 crates.io Kartis 0.1.0 中。
它为已有 Waffle 布局增加借用的单标记几何与画笔工厂，不实现任意 React 组件或实时 Widget。

`waffle_layout_with_cell_factory(config, data, options, &mut factory)` 返回
`WaffleLayout`。回调借用 `WaffleCellContext`，包含：

- `cell`：原网格槽、行列、归属及标记身份；
- `group`：已占用单元格为 `Some(&WaffleGroup)`，空格为 `None`；
- `geometry`、`position`、`plot`、`grid`、`padding` 和 `corner_radius`；
- 继承的 `fill`、`fill_paint`、`stroke`、`stroke_dash` 和 `opacity`。

返回 `ScatterNodeGeometry`，或从 `ctx.inherit(geometry)` 开始保留原画笔。
源数据和回调只在构建时借用，返回布局不保留它们。

```rust
# use kartis::*;
let data = [RadialDatum::new("Orders", 40.), RadialDatum::new("Other", 20.)];
let layout = waffle_layout_with_cell_factory(
    &ChartConfig::default(),
    &data,
    &WaffleLayoutOptions { rows: 5, columns: 10, total: Some(100.), ..Default::default() },
    &mut |ctx| {
        let [x, y] = ctx.position;
        Ok(ctx.inherit(Geometry::Circle {
            x, y, radius: ctx.cell.bounds.width * 0.25,
        }))
    },
)?;
# Ok::<(), ChartError>(())
```

工厂按布局的单元格遍历顺序处理具有真实 cell mark 的槽，包括空单元格。
`WaffleMode::Areas` 的已占用组是实心区域路径，没有逐格 mark，所以该模式下只对
空格 mark 调用工厂，已占用区域的几何和画笔不变。即使工厂给空格返回可见几何，
空格也不会因此获得表示已占用数据的类型化交互站点。

## 几何、身份和交互

`ctx.inherit(ctx.geometry.clone())` 保持整份原布局相等，包括圆角、画笔和交互绑定。
Circle、Rect 必须以 `position` 为中心；偏移墨迹使用 Path。自定义主体通过
`ScatterNodeGeometry` 使用共享纯色/渐变/图案、透明度、描边和 dash 配置。

槽边界、行列、owner、组源值、请求/实际占用区间、polygons、组颜色及图例颜色保持
原始语义，不从替换后的墨迹重新推算。`cell_at(x, y)` 仍查询包含 padding 的原完整网格槽，
不是实际墨迹命中查询。

Exact 几何拾取跟随新标记及其 clip，Nearest 保留原槽中心。已占用交互站点的颜色取
自定义画笔的实际代表色，但组和图例仍保留源颜色。因此 Path 可以把墨迹绘制到逻辑数据
站点之外，而不移动原数据站点或重新分配网格。

## 校验与范围

所有回调和候选校验完成后才返回布局。非法几何、画笔、透明度、dash、锚点，以及超出
整份 Scene 累计预算的结果都会拒绝候选；工厂不会修改已有接受布局。若后续回调失败，
回调自身已产生的外部副作用无法撤销，应用应在回调外统一提交接受后的结果。

输出复用共享 Scene 的 SVG、Canvas、CPU 和 Glory 绘制路径，类型化 binding 可交给已有
Scene 交互 API。实际后端验收另行记录；布局校验通过不等于物理设备输入或任意 Widget
生命周期已经验收。

本次原 Nivo 参考在 `packages/waffle/src/types.ts` 和 `WaffleCells.tsx` 中提供
`cellComponent`，包括空格、padding 和边框参数。Rust 入口提供有界几何工厂，不复制该
React 组件协议。Nivo 的 areas 是覆盖在 cells 上的透明交互层，而 Kartis 的
`WaffleMode::Areas` 绘制实心组区域；两者契约不同，Areas 模式中的空格工厂不能称为
完整移植 Nivo areas 或自定义图层。

需要覆盖单元格间隙的占用组事件时，可显式使用 [Waffle 区域绑定](waffle-regions.zh-CN.md)，保留工厂实际画笔并独立报告物理墨迹。
