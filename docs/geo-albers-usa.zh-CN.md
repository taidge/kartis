# Albers USA：拟合地图与分区覆盖层

[English](geo-albers-usa.md)

**状态：自 Kartis 0.2.0 提供。** 已提供专用拟合地图、分区线路／经纬网、点／逆算内核、平面矩形
裁剪、所选分区的符号／glyph／地点标签 mask，以及宿主逆算适配器。以下输入和数值
限制仍适用，不承诺任意球面拓扑或逐位相同的 D3 细分。已发布 Kartis 0.1.0 不含这些 API。

```rust
use kartis::*;

let projection = GeoAlbersUsaProjection::default();
let projected = projection.project([-149.9, 61.2])?.expect("visible Anchorage");
assert_eq!(projected.region, GeoUsaRegion::Alaska);
let geographic = projection
    .invert_in_region(projected.region, projected.position)?
    .expect("same-region inverse");
# Ok::<(), ChartError>(())
```

`new(scale, translate)` 接收每复合 raw 单位的像素比例和屏幕原点，Default 采用 D3 的
scale 1070、原点 `[480, 250]`。三个子投影保留原标准纬线、固有经度框架与投影后的原点；
Alaska 使用基准比例的 0.35，Hawaii 使用基准比例。不提供作者 camera、标准纬线或任意
clip-angle 控制。

`project` 按 Lower48／Alaska／Hawaii 依次选择可见子投影，返回含 `position` 和 `region`
的 `GeoUsaProjectedPoint`。`project_in_region` 指定单一分区。各区矩形含边界，Alaska／
Hawaii 使用原版 **最终像素单位的 1e-6 内缩**，不随 scale 放大；`region_clip` 返回对应
矩形。无效输入、算术或塌缩／不可表示的窗口明确拒绝，不静默缺掉部分分区。投影窗口不是
政治国界 mask。

`invert` 按 D3 半开 inset 窗口选择分区并检查该分区，失败不尝试其他分区，返回含经纬度及
分区的 `GeoUsaInverse`。已知来源分区可使用 `invert_in_region`。复合图像存在重叠，
全局 `invert(project(p))` 不保证找回原地理点。选中可见 footprint 外返回 None，区别于
D3 对任意远方像素仍做代数逆算的行为。准入先检查请求像素在精确分区矩形内，再做 checked
inverse 与未裁剪的同区回投残差验证。结果经纬度的浮点舍入可能让严格前向结果落在边界外
一个 ULP；这不会移动或扩大前向窗口。固定内核采用原表达式顺序，同时保留已有圆锥的有限域
与回投残差准入。

## 平面矩形裁剪

```rust
use kartis::*;

let rings = vec![vec![[-5., -5.], [15., -5.], [15., 15.], [-5., 15.]]];
let result = clip_geo_planar_polygon(
    &rings,
    ClipRect { x: 0., y: 0., width: 10., height: 10. },
    &GeoRectClipOptions::default(),
)?;
assert!(!result.rings.is_empty());
# Ok::<(), ChartError>(())
```

`clip_geo_planar_polygon` 对 nonzero winding 多边形执行分裂与边界重连；
`clip_geo_planar_lines` 独立裁剪开放线，不连接不同片段。这是投影后平面裁剪，不能代替
地理纬度或反经线裁剪。凹轮廓分裂、洞穿边界和整窗覆盖均需要这一拓扑步骤。输出保留原生
winding、重连闭合重复点及退化接触，不能按列表位置把第一个当外环、其余当洞。每个输入
多边形环至少三点，允许显式重复首点闭合。

`GeoRectClipOptions` 限制最终点数与工作量，包含空输入项头、扫描、排序和图遍历；中间
buffer 点数最多 `max_points` 的两倍。Stats 分别记录输入／buffer／输出点数和工作量。
坐标须有限，穿边界线段按原版使用 +/-1e9 clamp；不可表示的 winding 算术明确拒绝。
错误不返回部分结果。最终消费者仍须在三个分区和拟合轮次间共享预算。

## 拟合地图

```rust
use kartis::*;

let mut layout = geographic_usa_map_layout(
    &ChartConfig::default(),
    &[],
    &GeoUsaMapOptions::default(),
)?;
layout.add_symbols(
    &[GeoSymbol::new("Anchorage", [-149.9, 61.2], Some(7.))],
    &GeoSymbolOptions { clip_to_projection: true, ..Default::default() },
)?;
let projected = layout.projection.project_usa([-149.9, 61.2])?.unwrap();
assert_eq!(projected.region, GeoUsaRegion::Alaska);
# Ok::<(), ChartError>(())
```

`GeoUsaMapOptions` 提供 `fit`、`padding`、`scale`、`translate`、`fill`、`border`、
`border_width` 与自适应 `path`，不扩旧 `GeoProjection` 枚举或 `GeoMapOptions` 字段。
`World` 拟合固定的复合矩形；`Data` 按临时 scale 150／原点零测量三路真正裁剪后的
多边形流，再重建最终像素窗口。`translate` 表示拟合边界中心的位置；Data fit 的原生
D3 原点还须计入 `fitted_raw_center()`。覆盖层复用保留的 context，不单独重新拟合。

每个源多边形均经三个连续圆锥子投影再矩形 postclip。一个源 feature 保留一个复合
`map:{id}` mark、series、label 和 value，包含断开的片段与洞，保留原生轮廓 winding。
`choropleth_usa_layout` 使用默认色阶；`choropleth_usa_with_color_scale` 与
`_with_color_profile` 从全部源值解析共同颜色域，包含不可见 feature 与缺失值。

采样输出、postclip 输出与工作量在三个分区及 Data 预拟合／最终两轮间共享 `path`
预算。`usa_map_stats()` 返回累计点数、工作量、深度和轮数。准备／拓扑工作保守预检，
最终还检查 Scene 总命令限制。这比每个分区独立享用一份预算更严格。自适应采样算法与
D3 的细分算法不同，不承诺相同路径点数或逐浮点完全相同的拟合边界。

## 分区覆盖层与拾取

保留的 context 上原有 `add_symbols`／`add_glyphs` 使用复合前向优先级。
`clip_to_projection` 将完整原始填充／描边 mask 到所选分区矩形；
`clip_labels_to_projection` 独立裁剪新添地点标签，图例保持完整。plot 裁剪另行控制。
glyph 几何、源索引、nullable 值和类型化锚点保留，不使用全局 union mask 代替分区隔离。
mark 使用原生 `Mark.clip`，需要 plot 裁剪时取精确矩形交集，因此默认配置也能挂载为
Glory Graphic。标签矩形沿用共享的可编辑文字裁剪管线。

拟合 context 的 `project_usa`、`invert_usa`、`invert_usa_in_region` 提供分区信息，
普通 context 调用明确报错。浏览器宿主提供 `invert_usa_at_client`／
`invert_usa_in_region_at_client`；原生位图宿主提供对应 `_at_host` 方法。
它们复用已接受坐标链，Scene pan／zoom 只移除一次。context 须与已接受布局一起保留，
尺寸相同无法识别过期的同尺寸地图。

通用 `project`、`invert`、`invert_in_plot` 也分派到复合内核。`projection_disk`
返回 None；单一 `projection_clip_path` 无法保证插图隔离，因此拒绝。
旧 `add_lines`／`add_graticule` 签名明确拒绝 USA context；使用下面的专用方法保留
分区几何和交互归属。

## 线路、经纬网与 canonical 交互

```rust
use kartis::*;

let mut layout = geographic_usa_map_layout(
    &ChartConfig::default(), &[], &GeoUsaMapOptions::default(),
)?;
layout.add_usa_graticule(&GeoGraticuleOptions::default(), &GeoStreamOptions::default())?;
let mut route = GeoLineFeature::new("route", vec![]);
route.lines = vec![
    vec![[-124., 38.], [-119., 38.]],
    vec![[-152., 61.], [-146., 61.]],
    vec![[-159., 21.], [-156., 21.]],
];
let metadata = layout.add_usa_lines(&[route], &GeoStreamOptions::default())?;
let frame = metadata.into_scene_interaction(layout.scene)?;
let widget = chart_layout(frame);
# Ok::<(), ChartError>(())
```

`add_usa_lines` 复用已拟合地图 context。每个源的开放组件独立经过三个连续子投影，
再按最终像素矩形 postclip，不连接插图之间的复合点采样。每个可见分区使用真实路径和
自己的完整墨迹矩形。`clip_projection` 控制描边／端帽 mask，中心线 postclip 始终生效。
`clip_plot` 独立控制；两者启用时取精确矩形交集，包括空交集。虚线、透明度和逐源样式
保留为实际绘制，`behind` 与现有显式绘制顺序按事务保持。`add_usa_graticule` 复用
major／minor 源生成器，始终不交互。

`GeoUsaStreamMetadata.lines` 保留源索引、ID、原始 nullable 值和各区 `parts`。
`source_series` 保留输入分组串；有效 `series` 在输入为空时回退到源 ID。空标签也在真实
mark／控件上回退到 ID，metadata `label` 仍为原空串。数值格式独立存储，只应用一次。
不可见源不制造 mark 或控件；第一个可见真实 part 使用 `geo-line:{id}`，其余分区 alias
到该 canonical site。类型化锚点是首个可见 Move 端点，不是地理重心。

`interaction_binding(scene)` 核对已创作层的几何、样式、裁剪、元数据和绘制顺序。
`into_scene_interaction(scene)` 将此层绑定到 Glory／browser／raster，显式采用
`PaintedGeometry` 与 Geometry picking。每源只有一个键盘控件，各分区墨迹都归到它。
无关地图／glyph 仍绘制，但不会被偷偷加入此 binding；Nearest／slice 不属于该 profile。
替换已创作几何或样式时重新计算 metadata；宿主管理的动画遵循自己的已接受帧规则。

源、coarse、自适应和 postclip 点数共享 `path.max_points`：
`source_points + coarse_points + sampling.points + clip_stats.output_points`。
总工作量为 `sampling.work + clip_stats.work`，输入项和空行头也计费，并继续检查全场景
命令／mark 限制。失败不修改调用者 Scene。绑定 Glory 的默认 tooltip 使用 nullable／
formatted 源行，不会读回变化中的 live mark 标量；静止指针下仅元数据更新也会刷新。
现有接收 `Mark` 的自定义 formatter 保留其有限标量兼容契约。

## 验收边界

原版 child 流和 composite 事件记录均保留。六个跨插图参考案例中，原版 D3 composite
会交错 child `lineStart` 事件；Kartis 保留独立分区路径，不声称直接压平这些交错事件
也能得到等价的无桥几何。有效轮廓和双向线段距离单独核对。三个约 `1e-13` 像素的角接触
存在采样差异，作为诊断保留，没有重标成完全相同。歧义对跖弧、不可表示坐标和预算耗尽
明确拒绝。

原生／浏览器宿主输入、分区信息、实际 SVG／Canvas／CPU 墨迹、可编辑浏览器标签及
canonical 控件事务均有专项验收。像素探针避开抗锯齿边缘，不代表整图逐像素相等或物理
设备验收。详见[验收记录](verification.md#development-albers-usa-streams-and-host-integration)。
更广地理拓扑、其他功能差距和 GPU／worker 渲染仍属独立工作。

参考为 d3-geo 3.1.1 `projection/albersUsa.js`、圆锥子投影及
`clip/{rectangle,line,buffer,rejoin}.js`（ISC）。冻结参考清单和生成器位于
`tests/fixtures/geo-usa-kernel`、`tests/fixtures/geo-rect-clip`、
`scripts/geo-usa-reference.mjs` 与 `scripts/geo-rect-clip-reference.mjs`。
独立地图流／fit 参考由 `scripts/geo-usa-map-reference.mjs` 生成，保存在
`tests/fixtures/geo-usa-map`。
线路／经纬网参考为 `scripts/geo-usa-stream-reference.mjs` 与
`tests/fixtures/geo-usa-stream`。
