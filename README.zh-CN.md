# Kartis

[English](README.md)

Kartis 是面向 Glory 的 Rust 图表库。布局生成一份共享 `Scene`，可输出独立 SVG、浏览器 Canvas、Glory 控件和可选的 CPU 位图。布局和交互由 Rust 实现，无需 JavaScript 图表库。接口设计参考 D3 和 Nivo。[架构](docs/architecture.md) · [第三方许可](docs/third-party-notices.md)

## 快速开始

Kartis 要求 Rust 1.95 或更新版本。根据数据创建图表布局，再把场景传给 `chart`。示例使用 Glory 的内存宿主，应用可按平台选择对应的 Glory 宿主。

```rust
use kartis::{ChartConfig, LineOptions, Point, Series, chart, line};

# fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
let scene = line(
    &ChartConfig::default(),
    &[Series::new("Revenue", vec![Point::new(0., 20.), Point::new(1., 35.)])],
    &LineOptions::default(),
)?;
let app = glory::App::memory();
app.mount(chart(scene).on_select(|datum| println!("{}: {}", datum.label, datum.value)))?;
# Ok(())
# }
```

将 `Cage<Scene>`、`Bond<Scene>` 或 `Vary<Scene>` 传给控件即可响应式更新。Kartis 在发布前校验新场景，校验失败时保留上一帧。稳定的 mark ID 让控件在更新时维持原有身份。

```rust
# use kartis::*;
# fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
let app = glory::App::memory();
let scene = app.scope().cage(ChartConfig::default().scene()?);
app.mount(chart(scene).on_reject(|error| eprintln!("{error}")))?;
scene.revise(|frame| frame.title = "Updated chart".into());
app.flush()?;
# Ok(())
# }
```

## 图表类型

| 类别 | 布局与能力 |
| --- | --- |
| 笛卡尔 | 折线和面积、双轴、分组与堆叠柱状图、散点、热力图、Stream、Bump、AreaBump 和 Marimekko |
| 径向 | 饼图与环形图、雷达图、极坐标轴与柱状图、堆叠径向柱状图和弦图 |
| 统计 | 样本箱线图、子弹图、分布图、蜂群图、平面等高线和加权密度 |
| 层级与流向 | 树图、矩形树图、圆堆积、旭日图、冰柱图、漏斗图、桑基图和力导向网络 |
| 构成 | 支持圆角单元格或连续区域的华夫格图 |
| 空间与时间 | 地图、分级设色、地理路径和符号、Delaunay、Voronoi、日历和紧凑日期区间 |

### 散点、比例尺和符号

`scatter_layout` 接受稳定节点 ID，X/Y 均可使用数值、UTC 或类别 point 比例尺。返回值包含场景、原始值、格式化值、源索引、解析后的比例尺和交互锚点。默认点径为 9 px。原 `scatter` 接口的默认半径为 5 px。

`ScatterNodeShape` 区分圆点直径、填充符号面积和 D3 描边符号的标称尺寸。尺寸可固定，也可根据数据映射，并可配置钳制与缺失值策略。精确拾取使用实际绘制形状与裁剪区域。最近点拾取使用节点的逻辑原点，涵盖零尺寸节点。布局可提供类型化交互绑定和独立的 X/Y 视口域。

`scatter_layout_with_node_factory` 可根据原始数据、映射坐标、解析后的尺寸和继承样式绘制自定义节点。返回 `ScatterNodeGeometry` 即可提供自定义路径或居中的图元，并配置填充、渐变、描边或虚线。偏移路径仍以数据坐标作为交互原点。

`ContinuousScaleOptions` 支持反向、整化刻度域、像素取整、钳制和对数底数。`line_layout_with_scale_options` 让坐标轴、路径、标签和交互共享同一比例尺。`bar_layout_with_scale_options` 让坐标轴、分组或堆叠柱与合计共享值轴，包括正指数幂与平方根。`PointScale` 和 `point_line_layout` 使用零带宽类别点格。即使显示文字相同，数字键和字符串键也保留各自的身份。

使用 `ScaleKind::Pow { exponent: 2. }` 或 `ScaleKind::Sqrt` 可获得保留正负号的非线性数值坐标。刻度与整化仍使用原始值单位。共享图型布局和域视口使用同一变换，`scales::PowScale` 另提供独立映射与反算。负指数要求同号非零分支；零指数、变换溢出或塌缩明确拒绝。零基线柱和面积图要求正指数，倒数视口不能穿过原分支的零值奇点。

`glyph_geometry` 提供七种填充符号和七种描边符号。填充尺寸表示面积，描边尺寸遵循 D3 符号约定。工厂返回几何、边界和原始锚点。折线、雷达、散点、蜂群、网络和地图布局共享这组符号。地图符号支持投影蒙版和同形大小、颜色图例。

`swarm_layout` 将节点固定在数据值映射的位置，并根据符号与描边的包络沿另一轴避碰。它支持数值与 UTC 比例尺、显式分组顺序、逐点尺寸和类型化交互锚点。

`contour_layout` 在 SVG、Canvas 和 CPU 共用的 Scene 中保留平面孤岛与孔洞。`estimate_density` 接受有符号权重；`scatter_density_layout` 保留原始 Scatter 身份，并将密度绘制在源标记后方。`recontour` 为阈值和样式复用已接受密度场；源输入或估算配置变化时需重新构建。[等高线与密度指南](docs/contour-density.zh-CN.md)

用 `combo_layout_with_scale_options` 为混合图配置独立的左右值轴。布局返回解析后的比例尺、分类窗口或连续 X 轴映射，以及可见图形的交互锚点。

`radar_layout_with_value_scale` 支持线性、对数和对称对数径向比例尺，返回布局、解析后的比例尺与网格刻度。可以按值轴生成刻度，也可以等距绘制网格环并通过比例尺反算数值。

`radar_layout_with_point_factory` 可根据原始值、映射坐标和继承样式绘制雷达点。偏移路径的最近点拾取仍使用原始映射坐标。`bullet_layout_with_scale_options` 让子弹图的轨道、标线和刻度共用解析后的值轴。

`line_layout_with_point_factory` 和 `point_line_layout_with_point_factory` 支持自定义折线点形状。工厂接收原始数值或类别键。`boxplot_layout_with_scale_options` 让箱线图的统计摘要、刻度和绘制图形共用数值或 UTC 值轴。

`AreaBumpLayout::place_endpoint_labels` 在绘图区两侧放置已测量的起止标签。应用传入实际文字边界，即可避开其他标签和指定障碍。

`layout_stacked_radial_bar_with_arc_style` 为径向堆叠柱及其轨道配置圆角半径和 D3 弧形间距。每个柱段保留原始累计角度和稳定的标记 ID，供交互使用。

Pie 和 RadialBar 标签可选择 `ArcLabelRadiusMeasure::RingThickness`，按 Nivo 的环厚度语义应用 `skipRadius`。`OuterRadius` 保留原有行为。两种类型化布局都提供经过校验的交互绑定，保留原始值和极坐标锚点，涵盖活动切片外扩与圆角弧形。

`funnel_layout` 返回各阶段的原始数据、投影轮廓、分隔线和交互目标。可配置方向、直线或平滑边缘、阶段间距和受控高亮。

`waffle_layout` 返回网格位置和各组的分配结果。支持四个填充方向、独立取整或最大余数分配，以及带间距的单元格或实心分组区域。

`sunburst_layout` 使用 D3 等面积分区，可配置弧形圆角和间距。节点同时保留自身数值与子树合计。子节点可继承父节点颜色，每个可见扇区都有稳定的交互目标。

Sunburst 另提供已测量标签的 `label_candidates` 和 `place_labels`。传入相对标签锚点、包含旋转的墨迹边界后，可选择绘图区放置、按实际圆角扇环裁剪，或保守的完整边界准入。它支持优先级、偏移、受控隐藏、恢复和蒙版工作预算，默认布局保持原行为。

`chord_layout` 在每条弦带中保留矩阵的双向数据，以数值较大的一侧作为来源。外环与弦带的半径可独立配置。`ChordLayout::tooltip_text` 从原始数值生成两个端点的提示行。

`bump_layout` 为类型化类别和离散排名分别建立点比例尺，保留转折肩部和缺失槽。`bump_layout_with_point_factory` 可绘制自定义点，同时保留原始交互锚点。

`stream_layout` 返回原始数值、堆叠边界、投影采样和图层，可配置堆叠顺序、偏移、曲线、坐标轴和点。`into_scene_interaction` 提供图层拾取，`into_slice_interaction` 提供采样切片，涵盖关闭点显示后的采样。

`StreamDotOptions::glyph` 可选择共享的填充或描边符号。`stream_layout_with_dot_factory` 接受自定义点几何与样式。起点、中点和终点位置保留各自的堆叠锚点，绘制偏移点时也一样。精确拾取使用实际墨迹与裁剪，最近点和切片保留原始数据坐标。

### 多断点尺度和地理反算

`PiecewiseScaleOptions` 保留完整的数值或 UTC 域与输出断点。`line_layout_with_piecewise_scale_options` 和 `ScatterAxisScale::Piecewise` 让实际坐标轴、几何和类型化拾取共用该映射；图型中的 range 使用首项为 0、末项为 1 的绘图区比例。域视口逐个重算断点，并在布局回调中传入 `piecewise_domains`。转向的输出域支持绘制及显式 `invert_segment`；全局反算和数据视口要求输出域严格单调。[尺度与反算指南](docs/advanced-scales.zh-CN.md)

```rust
# use kartis::*;
let profile = PiecewiseScaleOptions {
    domain: vec![0., 2., 10.], range: vec![0., 0.75, 1.],
    ..Default::default()
};
let layout = line_layout_with_piecewise_scale_options(
    &ChartConfig::default(),
    &[Series::new("Observed", vec![Point::new(1., 2.), Point::new(6., 8.)])],
    &LineOptions::default(), &LineStyleOptions::default(),
    &PiecewiseLineScaleOptions { x: Some(profile), ..Default::default() },
)?;
let controller = layout.viewport_controller(ViewportLimits::default())?;
assert_eq!(controller.piecewise_domains()?.x, Some(vec![0., 2., 10.]));
# Ok::<(), ChartError>(())
```

`GeoProjectionContext::invert` 使用已接受的拟合、相机、缩放和平移反算经纬度，`invert_in_plot` 额外检查绘图区矩形。浏览器和位图 handle 提供客户端或宿主坐标转换，已接受 Scene 的相机只逆变换一次；应用应将投影上下文与对应帧共同保留。

```rust
# use kartis::*;
let map = geographic_map_layout(
    &ChartConfig::default(), &[],
    &GeoMapOptions { projection: GeoProjection::Orthographic, ..Default::default() },
)?;
let pixel = map.projection.project([10., 20.])?.unwrap();
let geographic = map.projection.invert(pixel)?.unwrap();
assert!((geographic[0] - 10.).abs() < 1e-8);
assert!((geographic[1] - 20.).abs() < 1e-8);
# Ok::<(), ChartError>(())
```

`GeoProjection` 还支持带参数检查的等积、等距、保角圆锥投影，以及固定的 Albers 预设。地图、路线和符号共用拟合后的投影上下文及其有限轮廓。数据拟合使用曲边的保守包络；保角投影采用有限纬度上限。[圆锥投影与 Albers 指南](docs/geo-conic.zh-CN.md)

```rust
# use kartis::*;
# fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
let projection = GeoProjection::conic_equal_area([29.5, 45.5])?;
let map = geographic_map_layout(
    &ChartConfig::default(), &[],
    &GeoMapOptions { projection, ..Default::default() },
)?;
let pixel = map.projection.project([-95., 35.])?.unwrap();
let geographic = map.projection.invert(pixel)?.unwrap();
assert!((geographic[0] + 95.).abs() < 1e-8);
assert!((geographic[1] - 35.).abs() < 1e-8);
# Ok(())
# }
```

### 绘制与格式化

`Scene` 支持有序图层、系列所属标签、绘图区和路径裁剪、渐变、图案与虚线描边。SVG 和 Canvas 共享绘制语义，包括将单点的 Round 或 Square 描边解析成实际绘制轮廓。

`MarkBlends` 按标记 ID 分配艺术混色模式。SVG、Canvas 和 CPU 位图可将标记的填充与描边合成后，再与图表背景及先前绘制内容混色。可用 `*_with_blends` 渲染函数，或通过 `BrowserChart::mount_with_blends` 创建保留式图表。为 `animate_to_with_blends` 传入目标混色配置即可启动动画。

`ColorScaleOptions::resolve_with_profile` 支持为自定义色板选择颜色插值空间，也可使用 38 条 Nivo/D3 命名色带。热力图、日历和分级设色布局接受相同配置。将解析后的色标传给 `append_color_legend`，即可共用数据域、缺失值颜色和查表边界。

`ColorScaleTransform` 为连续与发散颜色提供 Linear、Log、Pow、Sqrt 和 SymLog 输入变换。图表和图例共用变换，包括非对称发散中点的真实位置。Log 域使用同号非零分支，负指数幂也保留同号非零分支；不可表示的反算查询返回错误。

```rust
# use kartis::*;
let scale = ColorScaleOptions {
    transform: ColorScaleTransform::Log, domain: Some([1., 1_000.]),
    ..Default::default()
}.resolve([Some(1.), Some(10.), Some(1_000.)])?;
assert!((scale.position(10.).unwrap() - 1. / 3.).abs() < 1e-12);
assert!((scale.value_at(2. / 3.)? - 100.).abs() < 1e-10);
# Ok::<(), ChartError>(())
```

离散配色可使用 `ColorScaleKind::Quantile`，按非缺失样本计算 R7 分位阈值；也可使用 `ColorScaleKind::Threshold`，在 `ColorScaleOptions::thresholds` 指定升序阈值。两者共用图表和图例路径，保留重复阈值，等值归入后一档。`bin_extent` 保留阈值首尾档的无界语义，可选的有限 `domain` 控制图例显示窗口。空分位样本使用缺失颜色。源值与阈值须为有限数值，不进行隐式转换。

`ColorScaleKind::SequentialQuantile` 将完整样本排名映射到连续色带，并保留重复观测。排名统计、R7 分位值和按序号采样的 `rank_colors` 各有独立用途；精确图例在现有 paint 预算内保留数据轴跳变。[连续分位色带指南](docs/sequential-quantile.zh-CN.md)

```rust
# use kartis::*;
let scale = ColorScaleOptions {
    kind: ColorScaleKind::SequentialQuantile,
    ..Default::default()
}.resolve([Some(0.), Some(0.), Some(5.), Some(10.), Some(10.), None])?;
assert_eq!(scale.rank(0.), Some(0.25));
assert_eq!(scale.quantiles(4)?, Some(vec![0., 0., 5., 10., 10.]));
# Ok::<(), ChartError>(())
```

用 `OrdinalColorOptions` 解析一次类别色盘，再传给柱状图、折线图和饼图的类别颜色入口。显式键域让数据重排或隐藏后仍保留颜色。`PieColorStyles` 可继承并修改切片颜色，也可根据实际背景的对比度选择标签颜色。

```rust
use kartis::*;
let mut scene = ChartConfig::default().scene()?;
let mut mark = Mark::new("volume", Geometry::Rect {
    x: 60., y: 40., width: 200., height: 180.,
}, Color(30, 120, 180, 255));
mark.fill_paint = Some(FillPaint::Linear {
    start: [60., 40.], end: [60., 220.],
    stops: vec![GradientStop::new(0., Color(30, 170, 170, 255)),
                GradientStop::new(1., Color(45, 75, 170, 255))],
});
scene.marks.push(mark);
let svg = render::to_svg(&scene)?;
# Ok::<(), ChartError>(())
```

用 `ValueFormat` 和 `FormatLocale` 设置数值标签与提示框的格式。

```rust
# use kartis::*;
# fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
let locale = FormatLocale::default();
let thousands = format_prefix(".1", 1_000., &locale)?;
assert_eq!(thousands.format(2_500.)?, "2.5k");
assert_eq!(format_text(">8c", "Rust", &locale)?, "    Rust");
# Ok(())
# }
```

UTC 坐标使用整数毫秒时间戳。日期时间可指定区域格式和时区偏移。`timezone` 功能支持 IANA 时区、民用日历区间与日历聚合。

```rust
# use kartis::*;
# fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
let format = ValueFormat::date_time(
    "%Y年%-m月%-d日 %H:%M %Z", &TimeLocale::zh_cn(), TimeZone::fixed_offset(480)?,
)?;
assert_eq!(format.format(0.)?, "1970年1月1日 08:00 +0800");
# Ok(())
# }
```

### 树图、平行坐标和网络

树图可配置连接线曲线、节点形状与标签。层级图可聚焦子树。树图、网络图、弦图和桑基图可通过显式图关系实现高亮。

`HierarchyFocusMotion` 为矩形树图、圆堆积和冰柱图的子树切换提供动画。已接受的目标布局与当前采样几何分别可读。`frame_interaction` 将当前帧和拾取所需的类型化数据、锚点一起返回。

`HierarchyLayout::label_candidates` 提供身份稳定的标签供应用测量。将旋转后的实际墨迹边界传给 `place_labels`，即可配置优先级、备选偏移、障碍和受控隐藏。标签保留原有节点或绘图区裁剪，无法放置的标签可在后续重新布局时恢复。启用 `raster` 后，`FontRegistry::measure_label` 可用已注册字体提供边界。

```rust
use kartis::*;
let data = [HierarchyDatum::new("root", None, 0.),
            HierarchyDatum::new("leaf", Some("root".into()), 1.)];
let mut options = TreeOptions::default();
options.style.links.curve = TreeLinkCurve::Step;
options.style.leaves = Some(TreeNodeStyle {
    shape: TreeNodeShape::Square, radius: Some(6.), ..Default::default()
});
let scene = tree_with_options(&ChartConfig::default(), &data, &options)?;
assert!(matches!(&scene.marks.iter().find(|m| m.id == "node:leaf").unwrap().geometry,
                 Geometry::Rect { .. }));
# Ok::<(), ChartError>(())
```

平行坐标的每个维度拥有独立比例尺，缺失值保留为断点。

`parallel_layout_with_scale_options` 为每个维度独立配置边界和连续比例尺。它返回解析后的比例尺、原始行值、投影顶点和交互绑定。隐藏分组时保留推断出的数据域。

```rust
use kartis::*;
let mut cost = ParallelDimension::new("Cost");
cost.domain = Some([0., 100.]);
let mut rate = ParallelDimension::new("Rate");
rate.scale = ScaleKind::Log;
rate.domain = Some([1., 1000.]);
let rows = [ParallelDatum::new("first", vec![Some(25.), Some(8.)])];
let scene = parallel_coordinates_with_options(
    &ChartConfig::default(), &[cost, rate], &rows, &Default::default(),
)?;
assert!(scene.marks.iter().any(|m| m.series == "first"));
# Ok::<(), ChartError>(())
```

`ForceSimulation` 和 `NetworkDragController` 使用显式的世界坐标到屏幕坐标映射来支持节点拖动。

```rust
use kartis::*;
let config = ChartConfig::default();
let mut simulation = ForceSimulation::new(&[NetworkNode::new("A")], &[], Default::default())?;
simulation.set_position("A", [0., 0.])?;
let mapping = NetworkDragMapping::from_config(&config,
    ClipRect { x: 0., y: 0., width: config.width, height: config.height },
    ZoomTransform::default())?;
let mut drag = NetworkDragController::new(simulation, Default::default())?;
drag.begin(1, "A", mapping.world_to_client([0., 0.])?, &mapping)?;
drag.end(1, mapping.world_to_client([40., -20.])?, &mapping)?;
assert!(drag.active().is_none());
assert!((drag.simulation().node("A").unwrap().position[0] - 40.).abs() < 1e-9);
# Ok::<(), ChartError>(())
```

用 `ForceSimulation::scene_with_glyphs` 按节点 ID 配置符号，用 `NetworkDragController::begin_with_glyphs` 从实际绘制的轮廓启动拖动。符号尺寸独立于模拟半径。启用 `web` 功能后，`BrowserNetwork::mount_with_glyphs` 可将同一几何接入实时模拟与指针输入。

## 渲染与交互

| Cargo 功能 | 提供能力 |
| --- | --- |
| 默认 | Rust 布局、独立 SVG、Canvas 脚本导出和语义 Glory 图表 |
| `canvas` | 用 Rust/Wasm 绘制到浏览器 Canvas |
| `web` | `BrowserChart`、可保留节点的 SVG 或 Canvas、输入和动画 |
| `raster` | CPU RGBA 和 PNG、注册字体、字形轮廓和位图控件 |
| `raster-bidi` | CPU 位图和字形轮廓可选的单行双向排版 |
| `geojson` | GeoJSON 输入 |
| `timezone` | IANA 时区数据和带时区的日历操作 |

`render::to_svg` 生成独立 SVG 文档。`render::to_svg_with_namespace` 可在同一文档嵌入多个图表。`render::to_canvas_script` 为指定 canvas 导出绘制指令。`render::draw_canvas` 将场景绘制到浏览器 Canvas。实现 `render::Canvas` 可接入自定义绘制后端。

```rust
# use kartis::*;
# fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
let mut options = PieOptions { inner_radius: 0.6, ..Default::default() };
options.style.corner_radius = 8.;
options.style.arc_labels.legacy_fit = false;
options.style.arc_labels.text = PieLabelText::Percent(ValueFormat::Percent(0));
options.style.link_labels.enabled = true;
let scene = pie(&ChartConfig::default(), &[RadialDatum::new("A", 40.), RadialDatum::new("B", 60.)],
                &options)?;
let svg = render::to_svg(&scene)?;
assert!(svg.contains("<svg"));
# Ok(())
# }
```

Glory 语义图表提供图例、数据控件、键盘导航和提示框。`LegendOptions` 与 `TooltipOptions` 可调整展示，应用也可提供自定义图例和提示框控件。图例过滤会同步处理所属标签，应用可在可见性变更后重新计算布局。

`Chart::data_control_labels` 按稳定标记 ID 设置按钮文本。`BrowserChart` 用它在数据控件中显示类型化缺失值。

用 `Chart::plot_widgets` 将 Glory 控件放入与数据标记关联的已测量绘图矩形。插槽在数据更新和重排时保留控件身份。局部裁剪的控件仍在裁剪范围内显示，同时暂停交互。

位图图表也可在 PNG 图像上放置同类实时控件。插槽随 `RasterOptions::view` 调整，独立于像素比。场景、插槽布局、图像选项和图片发布共同进行准入校验。支持矩形蒙版，以及完整位于路径内或外的插槽；局部任意路径蒙版仍需浏览器蒙版宿主。实时控件与导出的 Scene 像素分别保留。

用 `BrowserChart::mount_with_plot_widgets` 在保留式 SVG 或 Canvas 上放置同类控件。插槽会随相机变换和响应式重排调整位置，并采用数据标记的矩形或路径裁剪。`set_layout_with_plot_widgets` 一次提交几何、类型化交互数据和插槽。

`BrowserChart` 支持指针和键盘输入、悬停与选择、最近点和切片拾取、缩放、平移及框选。数据域视口回调可更新图表几何，同时固定坐标轴。提交 `SceneInteraction` 可同时更新场景、绘图区和类型化交互数据。动画支持图形属性和几何变化，轮廓过渡可显式配置。

`EditableBrush` 提供持续的 X、Y、XY 选区，支持移动、边角调整、居中调整、方向锁定和按住 Space 临时移动。`SurfaceModel` 以视口逻辑像素保存选区，报告选中 ID 时只换算一次当前类型化数据锚点。配置明确的范围后，SVG、Canvas 和位图宿主共用同一受检控制器。程序更新会退休旧手势会话，失败更新保留已接受帧。[可编辑框选](docs/editable-brush.zh-CN.md)

```rust
use kartis::{surface::{SurfaceModel, SurfaceOptions}, *};
let scene = ChartConfig::default().scene()?;
let extent = Brush::from_points((0., 0.), (scene.width, scene.height))?;
let mut surface = SurfaceModel::new(scene, SurfaceOptions::default())?;
surface.configure_editable_brush(
    Some(EditableBrushOptions { extent, ..Default::default() }),
    BrushReconfigure::Preserve,
)?;
surface.set_brush_selection(Some(Brush::from_points((40., 30.), (160., 100.))?))?;
surface.apply_editable_brush(EditableBrushInput::Nudge {
    target: BrushTarget::Selection, delta: (10., 0.),
})?;
assert_eq!(surface.editable_brush().unwrap().selection().unwrap().x0, 50.);
# Ok::<(), ChartError>(())
```

`Theme::font` 设置图表的字体族列表和字重。`Scene::typography` 携带默认值、逐标签覆盖，以及图例和提示框角色配置，贯穿绘制、过滤和动画。`PresentationText::font` 提供展示文字的局部覆盖。SVG、Canvas 和语义控件使用同一解析字体；`Chart::font_text_measurer` 将该字体传给应用的测量函数。Pie、Hierarchy、Sunburst 和 AreaBump 的测量候选包含字体，拒绝来自其他字体的旧测量；使用其 `with_label_font` 方法在测量前修改受管文字。

普通 Text 需要精确定位时，使用 `Chart::semantic_text_measurer(profile, measure)`。宿主返回绑定请求的 `SemanticTextMeasurement`，包含真实 advance、行盒高度、alphabetic 基线和墨迹边界。绘制与避碰共用这些度量，保留 bearing、descender 和原 Text 节点。`SemanticTextProfile::line_height` 按字号倍率请求行高；字体资源或测量上下文变化后递增 `revision`。旧测量或测量失败会与场景、布局更新一起拒绝。未提供适配器时，普通 Text 保留兼容定位方式。

启用 `web` 后，可用 `BrowserSemanticTextMeasurer::new_in` 在图表的布局上下文中测量真实 HTML 行。字体就绪后挂载，将测量器移入图表的回调；Drop 会移除探针。自定义宿主可提供自己的实测基线，原生示例展示了 Apace 的真实行指标。旋转文字继续使用显式单色轮廓或彩色路径 API。

`raster` 功能提供 `render_raster`、`RasterCanvas` 和 `chart(scene).raster(fonts, options)`。通过 `FontRegistry` 注册字体字节以绘制文字。显式选择字体时，用 `bind_face(FontId, FaceDescriptor)` 将静态字体的真实字重绑定到命名或通用字体族。字体族回退和 CSS 字重匹配从这些绑定中选择，不合成粗体，也不查询系统字体。测量、CPU 绘制和单色或彩色轮廓共用所选字节。未绑定的字体族条目会被跳过，所有条目都无绑定时拒绝该帧。未声明字体时仍按注册顺序回退。位图控件通过宿主图像源发布 PNG。[CPU 导出示例](examples/raster_export.rs) · [位图控件示例](examples/browser-raster/README.md)

`RasterSurface::mount_layout` 和 `RasterSurfaceHandle::set_layout` 将布局的类型化交互绑定与图像一起保留。控件区分缺失值和零，提示框使用原始数据行，显式锚点支持自定义形状。校验与图像发布成功后，图像、控件、绘图区和绑定共同更新。应用通过 `dispatch_host` 提供宿主指针边界。

启用 `raster-bidi` 后，可用 `FontRegistry::with_bidi` 为单行标签选择自动、从左到右或从右到左的段落方向。阿拉伯文连写等依赖上下文的文本，每个文字段需要一个可完整覆盖的注册字体。位图和字形轮廓共用排版后的字形位置。

用 `FontRegistry::with_writing_mode(RasterWritingMode::Vertical)` 绘制一列字形直立、从上到下的文字。测量、位图和字形轮廓共用竖排后的字形位置。要为零旋转标签生成轮廓，请将 `OutlineTextOptions::rotated_only` 设为 `false`。

`FontRegistry::with_vertical_layout` 支持按显式换行向左或向右分列，以及 Unicode 混合字形方向。中日韩字形保持直立，侧向文字段使用横向塑形后旋转，并在可用时采用竖排替换字形。`measure_layout` 返回各列进位和二维边界。自动换行和完整 CSS 文字布局仍不在这一支持范围内。

用 `FontRegistry::with_color_glyphs(ColorGlyphMode::ColrV0 { palette })` 启用 COLR v0/CPAL 实心彩层。CPU 文字和 `Chart::painted_labels` 共用彩色路径，支持调色板透明度、孔洞、竖排塑形与旋转。透明前景色只影响前景层，其他调色板层仍可显示。默认字体配置拒绝不支持的彩色字形；COLR v1 绘制图和位图字形仍是后续任务。

`scene_with_painted_labels` 为独立 SVG、Canvas 或 CPU 导出生成文字转路径的副本，保留业务标记、绘制顺序、裁剪和系列归属，并校验生成 ID 与资源预算。需要保留原始文本或编辑能力时，应同时保存原 Scene。这一转换导出字体路径，实时控件内容另行处理。

桌面示例在 Apace 使用 Glory 矢量图形，在 Wry 使用 CPU 位图控件。`Chart::basic_data_controls` 让支持标准按钮的宿主保留焦点与激活交互。[桌面示例](examples/native-raster/README.md#desktop-window)

## 功能范围

公开接口覆盖 D3 和 Nivo 的部分布局与交互能力，具体选项因图型而异。语义 Graphic、SVG、Canvas 与 CPU 后端各有支持范围。GPU 和 worker 渲染暂缓。[差距清单](docs/feature-gaps.md) · [验收记录](docs/verification.md) · [跨浏览器验收](docs/cross-browser-verification.md)

采用 Apache License 2.0 许可。[许可证](./LICENSE)

球面几何提供借用类型化输入的受检距离、流式长度、有向面积、采样圆、重心和包含判断。[球面指南](docs/geo-spherical.zh-CN.md)

独立 CanvasScript 导出还提供显式主线程 Offscreen2D、累计缓冲区准入和不可用时的处理策略。[CanvasScript 指南](docs/canvas-script.zh-CN.md)

Bar 类别带支持显式共享舍入、对齐、间距与方向配置。[类别尺度指南](docs/bar-category-scale.zh-CN.md)

类型化工厂可逐个定制 Bar 和 HeatMap 标记的几何、渐变、图案、描边及透明度，保留原始值与交互站点。受控悬停会校验已接受的来源及绘制帧。[自定义 Bar 指南](docs/bar-custom-geometry.zh-CN.md) · [自定义 HeatMap 指南](docs/heatmap-custom.zh-CN.md)

Bar 和 HeatMap 数值标签可显式启用共享继承颜色、有序 modifier 和对比度选择。标签在自定义画笔之前根据布局 palette 计算颜色，并保留受控高亮透明度。[标签继承颜色](docs/inherited-label-colors.zh-CN.md)

Bar 和 HeatMap 普通边框可显式启用相同的继承颜色角色，保留配置的描边宽度与端帽。颜色在自定义工厂前根据数值色盘解析；单独配置 profile 不会启用边框。Normal Canvas/CPU 将填充与描边作为整体只应用一次标记透明度。Glory Chart 支持矩形标签裁剪及仅修改裁剪框的更新，仍不支持 Circle 和 Path 标签遮罩。[边框颜色角色](docs/border-color-roles.zh-CN.md)

Bar 和 HeatMap 的类型化标签工厂可根据原始来源定制文字，保留 raw 值和 tooltip，与几何工厂组合并在受控高亮时维持标签归属。Bar 还可显式启用零值轴长度标签，默认继续跳过。[类型化标签指南](docs/typed-labels.zh-CN.md)

RadialBar 标签可继承分类颜色；Sunburst 可在测量前使用类型化文字工厂；BoxPlot 提供显式 D3 分类带圆整、对齐和间距配置。[标签与带刻度配置](docs/chart-labels-and-bands.zh-CN.md)

Bump 两端标签可分别覆盖样式，Radar 点画笔可使用继承颜色角色，BoxPlot 值域上下界可分别选择自动或固定值。[端点、点颜色与上下界配置](docs/endpoint-point-bounds.zh-CN.md)

BoxPlot 可按摘要分位数推导自动值域；Bar 支持来源分类图例与显式系列顺序，配合类型化图例角色及显式分类动作。[摘要值域与图例动作](docs/summary-index-legend.zh-CN.md)

RadialBar 扇段边框，以及 BoxPlot 箱边、中位线和须，可显式启用继承颜色角色，保留固定默认值和描边几何。角色 alpha 与扇段或填充透明度保持独立。[描边颜色指南](docs/radial-box-strokes.zh-CN.md)

BoxPlot 支持按组或子组取色，并受控高亮与激活项同组或同子组的所有箱体。整组透明度在 SVG、Canvas、CPU 位图和 Glory 语义图表中只合成一次。[BoxPlot 配色与悬停指南](docs/boxplot-color-and-hover.zh-CN.md)

BoxPlot 图例可按稳定的原始组或子组键覆盖显示文字，保留身份、角色、色盘与受控关联。空 caption 和重复 caption 仍保留独立条目。[图例文字指南](docs/boxplot-legend-labels.zh-CN.md)

通过 `BoxPlotInteraction::whole_parts_scene_interaction()` 显式选择整体部件墨迹拾取：边框、中位线、须线、端帽和离群点共享每箱唯一的逻辑站点，同时保留物理 mark ID 与已接受离群点源快照。`chart_layout` 及 Browser／Raster layout API 一起接受 Scene 和 binding；原主体站点入口继续保留。[整体部件指针指南](docs/whole-part-pointer.zh-CN.md)

BoxPlot 可显式使用逐摘要 String 图例工厂，配合 `ByLabel` 按文字去重、完整域稳定排序和类型化的行到系列／分类目标。默认 `Keys` 保留按键 caption；受控悬停复用缓存行，显式工厂源替换重新构建图例。[逐摘要图例工厂指南](docs/boxplot-legend-factory.zh-CN.md)

BoxPlot 通过 `BoxPlotFactories` 接受借用的逐摘要 RGBA 与 ByLabel 文字工厂。箱体、默认离群点、继承描边及首代表图例共享解析后的源颜色；受控悬停复用缓存，源替换显式重供必要工厂。[源颜色工厂指南](docs/boxplot-color-factory.zh-CN.md)

Calendar 的 FullYears 布局支持横纵方向的显式月份间距，日期单元格、月轮廓和标签同步移动；零值保留原布局。[日历月份间距](docs/calendar-month-spacing.zh-CN.md)

Radar 点标签接受借用的文字工厂，可使用轴与系列身份、原始值和格式化值，并与几何工厂及共享值尺度组合。[Radar 点标签](docs/radar-point-labels.zh-CN.md)
