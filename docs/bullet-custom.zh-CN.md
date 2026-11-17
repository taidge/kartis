# 自定义 Bullet 部件

[English](bullet-custom.md)

此 API 自 Kartis 0.2.0 提供。
它为 Bullet 的范围、度量和标记提供借用式单 mark 几何与绘制工厂。

`bullet_layout_with_part_factory(config, data, options, &mut factory)` 返回
`CustomBulletLayout`，包含 `layout: BulletLayout`、
`interaction: InteractionBinding` 和 `plot: ClipRect`。
已有的 `BulletLayout` 保留原字段。所有部件都返回
`ctx.inherit(ctx.geometry.clone())` 时，内部布局与普通布局完整相等；
包装类型额外提供 typed 交互绑定。

```rust
# use kartis::*;
let data = [BulletDatum {
    id: "Orders".into(),
    ranges: vec![40., 75., 100.],
    measures: vec![62.],
    markers: vec![80.],
}];
let custom = bullet_layout_with_part_factory(
    &ChartConfig::default(),
    &data,
    &BulletLayoutOptions::default(),
    &mut |ctx| {
        let geometry = if ctx.part.kind == BulletPartKind::Measure {
            Geometry::Circle {
                x: ctx.position[0],
                y: ctx.position[1],
                radius: 10.,
            }
        } else {
            ctx.geometry.clone()
        };
        Ok(ctx.inherit(geometry))
    },
)?;
let svg = render::to_svg(&custom.layout.scene)?;
# let _ = svg;
# Ok::<(), ChartError>(())
```

`bullet_layout_with_scale_options_and_part_factory(config, data, options, scale,
&mut factory)` 将工厂与已有连续刻度选项组合。它返回
`CustomScaledBulletLayout`：`layout` 是自定义包装类型，
`shared_value_scale` 和 `row_value_scales` 保留已解析的查询刻度。
自定义墨迹不改变定义域、刻度投影或配置的反向设置。

## 回调数据与顺序

回调借用的 `BulletPartContext` 包含：

- `datum`：原始 `BulletDatum`；`source_row_index` 是它在输入中的索引；
- `row`：普通布局计算出的行；`row_index` 遵循配置后的可见行顺序；
- `part`：部件类型、稳定 ID、原始 `source_index`、原值和区间；
- `rank`：部件在同类中的序号，包括没有绘制 mark 的部件；
- 原始 `geometry`、位于原始可见部件中心的 `position`、图表 `plot`，以及继承的
  `fill`、`fill_paint`、`stroke`、`stroke_dash`、`opacity`。

在每个可见行内，范围和度量分别按端点排序，然后按原始顺序处理标记。
`source_index` 指向对应种类输入向量中的原始元素，与排序后的 `rank` 不同。
结束于定义域最大值的合成末尾范围使用 `source_index: None`；
只要它有实际 mark，也会调用工厂。

工厂只处理实际存在的部件 mark。隐藏行、零长度或完全裁去的区间，以及因尺寸
选项而不绘制的标记或部件，不会获得 mark 或回调，但保留已有元数据。
每次回调看到的 `row` 都是原始行快照，不包含先前回调返回的替换几何。
全部回调结束后，结果统一更新被替换部件的 `part.geometry`。

## 几何、绘制与交互

回调返回 `ScatterNodeGeometry`。`ctx.inherit(geometry)` 保留原始绘制设置，
包括标记描边和部件透明度。可使用共享的纯色、渐变、图案填充，以及描边、虚线
和透明度。矩形或圆必须以 `position` 为中心；偏移墨迹应使用 Path。
Path 可以通过受检的 `anchor_commands` 跟随拓扑兼容的动画帧。

部件 ID、所属行、原始值、原始及可见区间、定义域、坐标轴和标题保持原有含义。
即使替换形状超出行轨道，仍保留原来的行轨道裁剪。
`part.geometry` 表示已接受的替换几何；原始区间继续表示源数据，
不会从替换形状反推。

可将包装类型的 `interaction` 与 `layout.scene` 交给已有 typed Surface API。
精确几何拾取遵循替换 mark 及其裁剪，并保留已有几何拾取器的描边容差。
最近点拾取使用原始可见部件中心；动画期间，Path 锚点跟随实际交付的几何。
因此，偏移 Path 的可见墨迹可以与其逻辑原点分离。
交互 site 的颜色取自自定义有效绘制的代表色；行图例颜色仍遵循原来的行调色板
策略，用作行可见性控制键，不会重新汇总为自定义部件颜色图例。

## 验证与 Nivo 对应范围

构造期间借用源数据与回调，结果不保留它们。函数返回完整验证的候选结果或错误，
不发布部分布局。非法几何、绘制、透明度、虚线或锚点，以及超出共享 Scene 累计
预算的候选结果都会被拒绝。如果回调修改了外部状态，后续回调或验证失败时无法
撤回这些副作用；应用状态应在构造成功后发布。

Nivo 的 `rangeComponent` 和 `measureComponent` 经由 `BulletRects.tsx`
接收矩形数据、尺寸、边框、动画属性和鼠标处理器；`markerComponent` 经由
`BulletMarkers.tsx` 接收位置、尺寸、旋转、动画属性和处理器，接口定义在
`types.ts`。Kartis 用一个按部件种类分支的回调提供其中静态几何与绘制的子集，
接入共享 Scene 的 SVG、Canvas、CPU 和 Glory 路径。
它不复现任意 React 组件、动画属性对象或组件生命周期。

数据契约也有区别：Nivo 的 `compute.ts::stackValues` 会在排序范围和度量之前
删除值为零的端点，其 `index` 是处理后的序号。Kartis 保留零值、负值及原始源
索引。定义域从负数开始时，零端点也可能形成非空区间；零长度区间仍没有绘制
部件。因此，迁移自定义绘制时需要分别处理排序序号与原始源身份。

`custom.into_scene_interaction()` 保留 Surface/浏览器的旧 Geometry 与最近点协议。接入 Glory `chart_layout` 时，使用 `custom.into_painted_scene_interaction()?` 显式预检所需的实色 PaintedGeometry 协议；它拒绝合格标记中的渐变/图案填充，只支持 Geometry 拾取，不支持 Nearest 或切片。普通语义 Graphic 的每标记 256 命令及普通 Text 不支持旋转等限制仍适用。两种包装均提供这两个转换。
