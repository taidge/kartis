# Bump 两端独立文字

[English](bump-endpoint-labels.md)

此 API 自 Kartis 0.2.0 提供。
Start 与 End 可以使用不同文字，不修改系列原 label、图例、点 tooltip、几何、画笔或交互身份。
旧入口与 `BumpLayoutOptions` 的既有字段保持原行为。

```rust
use kartis::*;

let data = [BumpSeries::new("Orders", vec![
    BumpDatum::new("Q1", 3.), BumpDatum::new("Q2", 1.),
])];
let options = BumpLayoutOptions {
    labels: BumpEndpointLabels { start: true, end: true, ..Default::default() },
    ..Default::default()
};
let layout = bump_layout_with_label_factory(
    &ChartConfig::default(), &data, &options,
    |ctx| Ok(match ctx.endpoint {
        BumpEndpoint::Start => format!("Opening {}", ctx.series.id),
        BumpEndpoint::End => format!("Closing {}", ctx.series.id),
    }),
)?;
# Ok::<(), ChartError>(())
```

`BumpEndpointLabelContext` 借用原始 `series`、已完成的 `computed` 行及原系列文字
`default_text`。`series_index` 是原输入切片索引，计入隐藏系列。`line_position` 是既有
首／末肩部扩展线点，`position` 是应用该端有符号 padding 后的最终文字锚点。
Start／End 遵循既有源遍历；反向 X 时也不会把任意源点重排为几何最左／最右端。

只有实际产生的 owned endpoint Label 才调用文字工厂。隐藏系列、禁用端以及缺失的首／末
端不调用；空字符串保留所属 Label 槽。零 opacity、透明文字色或零点尺寸本身不会删除
已有端点槽，也不会抑制该槽的回调。返回文字最多 `MAX_LABEL_BYTES`，即 1,024 个 UTF-8
字节，与原系列 label 的长度限制分开。返回布局不持有源或回调。

## 与点几何组合

`bump_layout_with_factories` 接受 `BumpFactories`，可分别借用可选的 `point` 和
`endpoint_label` 回调。公开别名为 `BumpPointFactory` 与 `BumpEndpointLabelFactory`。

```rust
# use kartis::*;
# fn example(data: &[BumpSeries], options: &BumpLayoutOptions) -> Result<BumpLayout> {
let mut point = |ctx: &BumpPointContext<'_>| {
    Ok(ctx.inherit(Geometry::Circle {
        x: ctx.position[0], y: ctx.position[1], radius: 4.,
    }))
};
let mut text = |ctx: &BumpEndpointLabelContext<'_>| {
    Ok(format!("{:?}: {}", ctx.endpoint, ctx.default_text))
};
bump_layout_with_factories(
    &ChartConfig::default(), data, options,
    BumpFactories { point: Some(&mut point), endpoint_label: Some(&mut text) },
)
# }
```

每个源系列先按源顺序执行已有点回调，再构建线，最后依次执行 Start、End 文字回调。
错误停止构建，不返回半帧；应用回调已做的外部修改不能撤销，应在构建成功后再发布业务状态。
旧点工厂入口保持行为，文字工厂返回 `ctx.default_text.to_owned()` 保留原文字。

## 参考与渲染边界

Nivo commit `0dae2c32052a573f9f1d66ec1b453b572119b265` 的
`packages/bump/src/bump/types.ts` 公开独立的 `startLabel`／`endLabel` 系列 accessor。
`hooks.ts` 的 `useBumpSeriesLabels` 按位置分别调用 hook，并在排除缺失端点之前执行 accessor。
Kartis 只调用实际标签，并采用上文明确的逐系列组合顺序，因此不声称副作用或调用次序等价于
React hooks。Nivo 泛型额外属性不会自动反射进 Rust 结构；闭包可按源身份捕获应用数据。

返回的普通 Label 继续经过既有 SVG、Canvas、CPU 和 Glory Text。字体选择、空白协议与宿主
测量仍显式控制。所提供字体的路径导出提供固定 shaping profile，不能证明可编辑原生文字
像素完全相同，也不包含自动避碰、任意组件或真实设备验收。
可执行示例 `bump_endpoint_labels` 展示组合与导出，具体结果见
[验收记录](verification.md#development-bump-endpoint-text)。
