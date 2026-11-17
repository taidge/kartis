# 自定义 Funnel 阶段主体

[English](funnel-custom.md)

本 API 自 Kartis 0.2.0 提供。
它为现有 Nivo 风格阶段布局提供借用的 Rust 几何工厂，不等同于任意 React 自定义
图层或实时 Widget。

`funnel_layout_with_stage_factory` 接收原 `funnel_layout` 的配置、数据和选项，
再接收 `&mut` 回调。上下文包含原始 `datum`、`source_index`、计算后的 `part`、
原始主体 `geometry`、继承的 `fill`、`fill_paint`、`opacity` 和绘图区矩形。
数据按借用方式访问，返回的布局不保留回调。数字和字符串类别身份仍各自独立。
回调按源顺序仅作用于原本存在主体的阶段；零宽度且没有主体的阶段保留标签和元数据，
不制造拾取代理。

```rust
# use kartis::*;
let data = [FunnelDatum::new("Visitors", 80.), FunnelDatum::new("Orders", 20.)];
let layout = funnel_layout_with_stage_factory(
    &ChartConfig::default(), &data, &FunnelLayoutOptions::default(),
    &mut |ctx| {
        let [x, y] = ctx.part.center;
        let mut stage = ctx.inherit(Geometry::Circle { x, y, radius: 12. });
        stage.retain_profile_border = false;
        Ok(stage)
    },
)?;
# Ok::<(), ChartError>(())
```

返回的 `FunnelStageGeometry.body` 通过 `ScatterNodeGeometry` 支持纯色、渐变、
图案、透明度、描边和虚线。圆和矩形必须以 `part.center` 为中心；偏移 Path 可以
保留原始数据锚点，并显式指定动画支撑命令索引。精确几何拾取使用新轮廓，最近点
拾取仍使用原始数据中心。自定义主体不会改变阶段带、源宽度、轮廓采样、标签位置、
分隔线位置、数值、格式或系列归属。`part.bounds` 仍是原始起始宽度矩形，
不是自定义主体的墨迹边界。

`ctx.inherit(ctx.geometry.clone())` 完整保留原始帧，包括渐变源颜色和旧锚点。
实际变化的主体将可见画笔的代表色接入阶段元数据、交互、图例及继承标签颜色。
`retain_profile_border = true` 保留原开放侧边轮廓的独立描边和透明度，不会沿新主体
重新描边。更换轮廓时设为 `false`，需要描边可使用 `body.stroke`。
自定义描边与保留的原描边属于两个独立绘制层，可能重叠。

所有回调及验证完成后才返回候选帧。非法几何、非有限透明度、矛盾的锚点、非法画笔
或虚线、累计预算超限都会拒绝候选帧。全帧路径命令上限为 200,000，共享渐变、
虚线和裁剪预算继续适用。若后续回调或验证失败，应用自身的回调副作用不会被回滚。

结果复用共享 Scene 导出与 Glory 交互通道。Rust 回归测试覆盖继承等价、偏移拾取和
拒绝行为；真实浏览器、设备验收另行记录，不将这些 API 检查视作物理输入或任意
Widget 支持已完成。
