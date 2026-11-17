# 自定义 Swarm 节点图形

[English](swarm-custom.md)

**Unreleased。** 已发布 Kartis 0.1.0 不包含此扩展。

`swarm_layout_with_node_factory` 在原有布局完成后替换 Swarm 节点的 Scene 图形与绘制样式。
同一 Scene 可用于 SVG、Canvas、CPU 栅格、语义 Chart 控件与浏览器宿主。
原有 `swarm_layout` 入口及 `SwarmLayoutOptions` 保持兼容。

```rust
use kartis::*;

let data = [SwarmPoint::new("sample", "Group", 5.)];
let layout = swarm_layout_with_node_factory(
    &ChartConfig::default(),
    &data,
    &SwarmLayoutOptions::default(),
    |ctx| {
        let [x, y] = ctx.position;
        Ok(ctx.inherit(Geometry::Rect {
            x: x - 6., y: y - 4., width: 12., height: 8.,
        }))
    },
)?;
let frame = layout.into_painted_scene_interaction()?;
# Ok::<(), ChartError>(())
```

回调收到 `SwarmNodeContext`：借用的原始 `datum`、已计算的 `group` 与 `site`，
源数据、分组和站点索引，原始排布 `position`、`size`、保守碰撞半径 `ink_radius`，
已解析的 `value_scale`、`plot`，以及原始 `geometry` 和样式。
`site_index` 遵循排布后的绘制顺序；`source_index` 仍对应原输入切片。
主坐标相同时沿用原来的 ID 排序。回调可以借用应用状态，构建结束后不会被保存。

`SwarmNodeGeometry` 是 `ScatterNodeGeometry` 的别名，支持一个 Circle、Rect 或包含多个子路径的 Path，
以及纯色、渐变、图案填充、描边、虚线和透明度。使用 `ctx.inherit(geometry)` 保留全部继承样式。
返回 `ctx.inherit(ctx.geometry.clone())` 会完整保留默认帧，包括原有字形的动画锚点。
Circle 与 Rect 必须保留逻辑中心；需要偏移绘制内容时应使用 Path。
可通过 `anchor_commands` 选择跟随后续图形变化的 Path 端点，同时保留已接受帧的原始逻辑坐标。

## 排布、可见性和交互

只有全部源数据验证与排布成功后才调用工厂，每个原有站点调用一次。
隐藏分组，以及在 `SwarmValueFit::Clip` 下原碰撞包络与绘图区完全分离的节点不会调用回调。
零尺寸、透明色和零透明度节点仍保留原有逻辑站点，因此会调用回调。
自定义绘制不会恢复已经剔除的源节点，也不会新增业务数据。

自定义图形和样式不会重新计算碰撞圆、分组区域、比例尺、源值、源元数据或图例颜色。
因此 `ink_radius` 描述原排布所用的包络，并不表示替换后图形的可见范围。
替换图形更大时可能与邻居重叠，超出原绘图区的内容会被裁剪。
站点的交互颜色会更新为自定义样式的代表色。

Exact 几何命中使用替换后真实的填充与描边，包括路径偏移和虚线间隙。
Nearest 使用原排布的数据中心，同时遵循宿主现有的透明度与绘制样式资格。
例如，仅包含 Move 且保留填充样式的 Path 可以保留 Nearest 站点，又不会产生 Exact 可见图形。
移除全部填充与描边，或将透明度设为零，不会凭空产生额外绘制内容或业务站点。
键盘控件与可见性继续遵循宿主自身规则；已有零尺寸源节点可以保留语义控件，同时 Exact 没有可命中的图形。

向宿主传递类型化帧，而不只是 Scene，才能保留显式 Path 锚点与原始源值。
`layout.into()` 为 Surface 和浏览器宿主保留原有 Geometry/Nearest 配置。
`layout.into_painted_scene_interaction()?` 验证真实绘制内容，并选择 Glory `chart_layout` 要求的 Geometry 专用配置；
该配置不支持 Nearest 和切片拾取。
键盘控件保留节点规范 ID、源分组和标签。图形偏移不会改变提示信息中的原业务坐标。

Glory 语义 Graphic 支持纯色填充、矩形裁剪，每个标记最多 256 条命令，
会明确拒绝渐变、图案填充和任意形状裁剪遮罩。
SVG、Canvas 和 RasterSurface 支持更丰富的共享绘制样式；
需要这些填充并保留真实 Glory 控件时可以使用 RasterSurface。

## 验证与失败

最终帧遵循原有 Scene 限制，包括轴与装饰共同占用的 200,000 条 Path 命令预算，以及全帧渐变色标预算。
回调结果跨节点累计，每个渐变在接受前单独验证。
无效图形、绘制样式、虚线、锚点支持或透明度都会拒绝候选帧。
回调返回 `Err` 后不再调用后续节点，也不会返回部分布局。
回调产生的外部副作用无法回滚；应先成功构建候选布局，再调用 `set_layout`。

`swarm_custom` 示例通过真实 SVG、Canvas 与 CPU 栅格导出横向、纵向、更新和调整尺寸后的帧。
共享浏览器夹具覆盖可信鼠标与键盘事件、原中心 Nearest 和偏移图形 Exact 的区别、失败工厂、源数据更新与清理。
拒绝验收在失败构建前后同步捕获源数据、修订号、排队输入、绘制内容与控件。
随后到来的 RAF 或显式空闲 `flush` 可以独立推进绘制修订号，并不表示候选布局已接受；
浏览器回执将这类遥测和未改变的已接受帧分别记录。

## 与 Nivo 的关系

Nivo 0.99 的 SwarmPlot SVG `circleComponent` 与 Canvas `renderCircle` 接收已计算的节点，
对应源码为 `types.ts`、`hooks.ts`、`Circles.tsx` 和 `SwarmPlotCanvas.tsx`。
本 Rust API 在共享渲染器中覆盖其类型化图形与样式替换职责。
它不执行 React 组件、任意命令式 Canvas 回调或真实 Glory Widget 子控件。
Kartis 继续采用已说明的固定数值坐标和确定性排布，不承诺与 Nivo 双轴力模拟或动画轨迹一致。
