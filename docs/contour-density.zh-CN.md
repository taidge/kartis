# 平面等高线与加权密度

[English](contour-density.md) · [库概览](../README.zh-CN.md)

Kartis 将平面等值区域提取为保留孤岛与孔洞的填充多边形，并按逻辑像素估算二维加权密度。
二者生成 SVG、Canvas 和可选 CPU 位图共用的 `Scene`。接口从 `kartis::contour`
和 crate 根导出。地理投影、多边形修复和拓扑变化的自动动画属于其他功能范围。

## 按行排列的网格

`ContourGrid` 借用 `width * height` 个按行排列的采样：
`values[y * width + x]`。宽高必须为正，非缺失采样必须为有限值。
`None` 始终低于任意阈值，非缺失采样按 `value >= threshold` 分类。
输出坐标覆盖平面网格范围 `[0, width] × [0, height]`，保留原有半单元边界约定。

`contour` 提取一个有限阈值；`contours` 通过 `ContourOptions` 接受
`ContourThresholds::Values`、`Count` 或默认的 `Sturges` 规则。
显式阈值会排序，重复值保留。自动数量表示刻度密度请求，并非精确的输出层数；
常值或全部缺失的网格不生成自动层。平滑默认开启，也可关闭。

```rust
use kartis::*;

fn main() -> Result<()> {
    let samples = [
        Some(0.), Some(1.), Some(0.),
        Some(1.), Some(4.), Some(1.),
        Some(0.), Some(1.), None,
    ];
    let layout = contour_layout(
        &ChartConfig::default(),
        ContourGrid { width: 3, height: 3, values: &samples },
        &ContourOptions {
            thresholds: ContourThresholds::Values(vec![0.5, 2.]),
            ..Default::default()
        },
        &ContourLayoutOptions::default(),
    )?;
    assert_eq!(layout.levels.len(), 2);
    let svg = render::to_svg(&layout.scene)?;
    assert!(svg.contains("<svg"));
    layout.scene_interaction().validate()?;
    Ok(())
}
```

`contour_layout` 将该网格范围一次映射到配置的绘图区。
`contour_levels_layout` 接受已计算的 `ContourLevel` 和宽高为正的显式
`ClipRect` 源范围。每个 `ContourPolygon` 包含显式闭合的外环和孔洞环。
调用方提供的环必须保留拓扑与绕向，此适配器不会修复多边形。
每个层的全部孤岛和孔洞保存在同一个使用 nonzero 规则填充的路径中。

## 可复用的密度场

`DensityPoint { position, weight }` 使用估算器局部逻辑像素。
权重可以为正、零或负，但必须有限。有符号密度是加权场，并非归一化概率分布。
负权重不会被丢弃或取绝对值。非有限坐标、非有限权重，以及 Float32 累加或模糊溢出会拒绝。

`estimate_density` 返回不可变且可低成本克隆的 `DensityField`。
它保留聚合结果，不保留输入业务点。`values()` 暴露含填充区域的原始 Float32 网格；
`max()` 和等高线阈值以每平方逻辑像素的权重为单位，与渲染器 DPR 无关。
`dimensions()`、`origin()`、`size()`、`bandwidth()`、`cell_size()` 和
`requested_cell_size()` 描述已接受的估算器。

默认尺寸为 `[960., 500.]`、带宽为 `sqrt(420)`、请求单元大小为 `4`。
实际单元大小向下取二的整数次幂。尺寸各分量必须大于零且不超过 1,000,000；
带宽范围为 0 到 100,000；请求单元大小范围为 1 到 1,048,576。
最终含填充区域的网格还须满足单元和工作预算，因此数值范围合法的配置也可能超预算。

```rust
use kartis::*;

fn main() -> Result<()> {
    let config = ChartConfig { width: 360., height: 240., ..Default::default() };
    let plot = ClipRect::plot(&config);
    let field = estimate_density(
        &[
            DensityPoint { position: [plot.width * 0.3, plot.height * 0.5], weight: 1. },
            DensityPoint { position: [plot.width * 0.7, plot.height * 0.5], weight: -0.25 },
        ],
        DensityOptions { size: [plot.width, plot.height], bandwidth: 12., ..Default::default() },
    )?;
    assert_eq!(field.sample_count(), 2);
    let layout = density_layout(
        &config, &field, &DensityThresholds::Values(vec![0.0001, 0.001]),
        ContourLimits::default(), &ContourLayoutOptions::default(),
    )?;
    layout.scene_interaction().validate()?;
    Ok(())
}
```

`field.contour(value, limits)` 提取单层，
`field.contours(&thresholds, limits)` 提取多层。
`DensityThresholds::Count` 默认为 20，`Values` 接受有限的显式阈值。
`density_layout` 可为阈值、颜色、图例、格式和等高线层可见性复用聚合结果。
密度场尺寸必须与当前绘图区尺寸完全一致。它将绘图区局部坐标平移到场景中；
不要再次给估算器输入加上绘图区偏移。

采样位置、权重、带宽、单元大小或估算器尺寸变化时，需重新计算密度场。
数据图表的数据、源系列可见性、坐标轴或绘图区尺寸变化也可能改变这些位置。
可通过 `contour_levels_layout` 将结果显式仿射显示到其他单位，
但这不会重新估算新单位下的密度。

## 类型化 Scatter 布局的密度底层

`scatter_density_layout` 接受实际的 `ScatterLayout`，
以其绘图区作为估算器范围，以解析后的 Scatter 坐标轴提供位置。
全部已映射且未隐藏的观测参与估算，包括零尺寸节点，以及缺失尺寸策略跳过图形的已映射节点。
处于估算器填充区域的点即使中心位于绘图区外，仍可贡献密度。
标记的 `size_value` 始终不会被用作密度权重。

`DensityWeights` 以原始 `(系列 ID, 数据 ID)` 为键；缺少的键使用权重一，
不存在于源数据的键和非有限权重会拒绝。源元数据、实际绘制的交互点、索引和解析位置必须一致。
应通过新的 Scatter 布局更新数据或可见性，不要修改已有元数据来过滤观测。
`DensityObservation` 保留原始坐标、源索引、原始 ID、映射位置和实际权重。
源标记保留在前景，其类型化拾取、字体、裁剪和绘制顺序保持有效。
`ScatterLayout` 的公开字段可修改：校验检查元数据、交互点和同 ID 标记之间的一致性，
但无法证明调用方提供、且没有元数据的装饰标记的原始业务来源。
调用者仍须负责正确构造所需输入。

```rust
use kartis::*;

fn main() -> Result<()> {
    let config = ChartConfig::default();
    let scatter = scatter_layout(
        &config,
        &[ScatterSeries::new("observed", vec![
            ScatterDatum::new("a", 1., 2.),
            ScatterDatum::new("b", 3., 4.),
        ])],
        &ScatterLayoutOptions::default(),
    )?;
    let weights = DensityWeights::from([
        (("observed".into(), "b".into()), -0.25),
    ]);
    let layout = scatter_density_layout(
        &config, &scatter, &weights, ScatterDensityOptions::default(),
        &DensityThresholds::Values(vec![0.0001]), &ContourLayoutOptions::default(),
    )?;
    assert_eq!(layout.observations[0].node_id, "a");
    let restyled = layout.recontour(
        &DensityThresholds::Values(vec![0.0002, 0.001]),
        ContourLimits::default(),
        &ContourLayoutOptions { opacity: 0.6, ..Default::default() },
    )?;
    assert_eq!(layout.field.values(), restyled.field.values());
    restyled.scene_interaction().validate()?;
    Ok(())
}
```

`ScatterDensityLayout::recontour` 使用私有保留的已接受配置、Scatter 场景、观测和密度场。
公开输出字段是快照，编辑它们不会替换 `recontour` 的输入。
它重新提取等高线，不重新估算密度，也不重新执行 Scatter 投影。
数据、源可见性、权重、带宽、单元大小、坐标轴或绘图区范围变化后，
应以当前输入重新调用 `scatter_density_layout`。
只在宿主中隐藏源标记，不会移除其在现有密度场中的贡献。

## 准入、身份和交互

预算可降低，不可超过固定上限。一次多层调用累计计入所有层的等高线工作量；
每次新调用拥有独立预算。隐藏的几何也计入 Scene 准入预算。

| 资源 | 上限 |
| --- | ---: |
| 网格单元 | 1,048,576 |
| 等高线层 | 256 |
| 算法工作量 | 32,000,000 |
| 边 / 点 | 各 1,000,000 |
| 算法环 | 20,000 |
| 密度输入采样 | 20,000 |
| 等高线 Scene 命令 | 200,000 |
| Scene 多边形 / 环 | 各 20,000 |

`ContourLimits` 控制计算，`ContourSceneLimits` 控制布局准入。
Scatter 合成先预留源场景已使用的命令预算，再准入等高线。
不会为规避渲染器限制而简化或拆开完整环。
较大路径可能需要 SVG、Canvas 或 CPU 位图，无法交由路径容量较小的语义 `Graphic` 宿主直接绘制。

`ContourLayoutOptions` 配置命名空间 `id`、共享 `colors` 与 `color_profile`、
透明度、描边、`value_format`、`color_legend`、`level_legend` 和隐藏层 ID。
`ContourLevelMetadata` 提供阈值、原始输入索引、重复排名、几何数量和可选的实际标记 ID。
层 ID 由命名空间、阈值位模式和当前重复排名组成，正负零共用身份，不保留历史阈值缓存。
空层和隐藏层有元数据，但没有数据控件。配置可见性时应读取 `layout.levels` 中已接受的 ID。

`scene_interaction()` 将场景、绘图区和类型化绑定打包供宿主更新。
精确拾取使用实际路径墨迹、孔洞和裁剪；键盘锚点是真实路径端点。
应共同保留布局和交互绑定。先计算并验证候选布局，再发布到宿主；
这些函数返回错误时不会修改调用方先前保留的布局。

## 渲染与参考范围

通过 `render::to_svg` 导出独立 SVG，将共享 Scene 交给 Canvas 渲染器，
或启用 `raster` 功能生成 CPU 输出。`web` 宿主可保留 SVG 或 Canvas 节点并发布类型化布局。
等高线提取不依赖 JavaScript 运行时。此功能未实现 GPU 计算、worker、多边形修复和拓扑变化的自动变形。

[原函数检查器](../scripts/contour-density-reference.mjs) 执行仓库中保留的
**d3-contour 4.0.2** 与 **d3-array 3.2.4**，无需 npm 或其他源码检出：
`node scripts/contour-density-reference.mjs --check`。
固定参考包含 546 个网格记录、500 个密度记录和 844,260 个精确 Float32 单元。
这些数字表示原函数参考检查，不表示浏览器验收已经完成。

两项差异显式保留：`None` 统一使用 NaN-low 语义，原生 D3 `null`
在阈值 -1 和 0 的部分网格边界上有不同表现；当最大值为负时，
原生批量密度输出可能将几何附到另一个阈值标签上，Kartis 将几何绑定到自身阈值，
并保留独立单阈值参考。两项都不宣称与原批量或 null 行为等价。
参见[源清单](../tests/fixtures/contour-density-d3-source.json)与[第三方声明](third-party-notices.md)。

正式保留式浏览器功能已在 Chrome、Firefox、Playwright WebKit、Edge 的 SVG/Canvas 与 DPR1/2 场景通过，覆盖独立孔洞/源点像素、类型化输入、拒绝、SVG 发布恢复及清理；CPU 与完整 headless Apace 回归也通过。范围回归的成绩及保留失败见[验收记录](verification.md)。这些检查不包含真实触摸设备或操作系统窗口。独立的严格合成器屏幕基准仍未解决，本项功能验收不代表全部 Nivo 功能已对齐。
