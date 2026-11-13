# 球面度量与采样小圆

这组公共 API 在单位球面上计算经纬度几何。输入坐标和圆参数使用度；`geo_spherical_distance`、`geo_spherical_length` 返回弧度，`geo_spherical_area` 返回球面度（sr）。若选定球半径 R，弧长乘以 R，面积乘以 R²。这是球面计算，不是椭球测地线计算。

```rust
use kartis::*;

let quarter_turn = geo_spherical_distance([0., 0.], [90., 0.])?;
let ring = geo_circle(&GeoCircleOptions {
    center: [15., 25.],
    radius_degrees: 25.,
    precision_degrees: 3.,
    ..Default::default()
})?;
let area_sr = geo_spherical_area(
    GeoSphericalGeometry::Polygon(std::slice::from_ref(&ring)))?;
let perimeter_rad = geo_spherical_length(
    GeoSphericalGeometry::LineString(&ring))?;
# Ok::<(), kartis::ChartError>(())
```

示例生成采样小圆。面积和周长属于相邻样本以大圆弧连接的多边形，不是解析圆的值。`examples/shared/geo_spherical.rs` 将这些真实输出用于地图面、边界路线、距离符号数值以及注明单位的文字。第二个场景度量八分之一球面三角形：面积 π/2 sr，反向面积 7π/2 sr，周长 3π/2 rad。两个场景都是普通 `Scene`，可供 SVG、Canvas 脚本和 CPU 光栅渲染器使用。`examples/geo_spherical_export.rs` 向显式指定的目录导出；运行时启用 `raster`、`geojson`，并复用仓库已有 ABeeZee 测试字体。

## 几何与检查

`GeoSphericalGeometry` 借用 Point、MultiPoint、LineString、MultiLineString、Polygon、MultiPolygon、GeometryCollection 的坐标，也能表达 Empty 和 Sphere。坐标必须有限，经度范围 [-180,180]，纬度范围 [-90,90]。一次计算最多接收合计 100,000 个坐标（包含闭合坐标）、4,096 个几何/线/环/多边形部件和 32 层嵌套。非法输入返回 `Err`。

每个非空多边形环至少有四个坐标，末坐标必须在数值上等于首坐标。空 Polygon `[]` 合法，但含空环的 Polygon 被拒绝。`GeoSphericalGeometry::from(&feature)` 只借用 feature 中的多边形。地图渲染可以自行闭合 `GeoFeature` 的开放环；对同一开放环调用球面度量会明确报错。计算前应显式闭合并检查作者数据。面积计算拒绝对跖或近对跖边（角长 ≥ π − 1e-8），因为它们不能唯一确定大圆弧。

面积遵循已固定 D3 3.1.1 的有向积分：顺时针小外环选择小区域，反向选择补区域，孔洞使用相反绕向。它不修复自交，也不计算地理并集。各多边形和集合成员进行代数累加；重叠多边形或重复 Sphere 可以让结果超过 4π。单个 Sphere 面积为 4π，流式长度为零。

## Polygon 长度与闭合周长

为保持 D3 `geoLength` 行为，Polygon 流会省略重复的末坐标，而且不会补最后一条闭合边。因此其长度不等于周长。需要闭合周长时，将完整闭合环传给 `GeoSphericalGeometry::LineString`，如上例。对八分之一球面环 `[[0,0],[0,90],[90,0],[0,0]]`，Polygon 长度为 π rad，完整闭合 LineString 长度为 3π/2 rad。没有隐式周长辅助函数改变这个区别。

## 圆的精度与闭合

`GeoCircleOptions` 要求有限且符合范围的圆心、[0,180] 度半径、严格正的 (0,90] 度精度，以及 [4,100000] 范围的 `max_points`。精度表示绕圆采样的角步长。实现先检查估计工作量，再逐个检查实际输出点（包含闭合点）；步长相对预算过细时返回错误。

原版 D3 在精度为零时返回空环，负精度则可能不终止；受检 API 拒绝这两种输入，也拒绝非有限参数。原始 oracle 结果另行保留。为输出精确闭合的环，若原始末样本距首点不超过 1e-12 弧度，则将末坐标替换成精确首坐标；否则追加精确首坐标。其余采样数值和顺序遵循已固定原函数。

示例证明 25° 半径小圆能够生成实际可绘几何。圆生成成功不表示所有半径、精度或投影都能通过现有受检渲染器：零/180° 圆退化，极小图形涉及渲染容差，过细或过宽的环也可能触发其拓扑和工作量限制。Scene 平面命中测试不能充当球面 contains。

## 独立检查

可迁移的 `scripts/geo-spherical-gallery-geometry.js` 接收 Playwright page 并返回报告。它调用已固定原版 D3 重新生成圆样本、独立投影作者输入，检查距离/面积/长度元数据和明确单位，检查真实填充/符号像素，并以既有平均通道误差 <0.5 门槛比较解码 SVG 图像与真实 Canvas 导出。大圆边界采样误差门槛为 0.5 像素，实际顶点归属另用密集 oracle 检查。错源坐标、错误面积、错误颜色和 Canvas 内部像素均有真实变异反例。这些检查仅覆盖两个展示场景，不声称证明任意拓扑或交互式 BrowserChart 行为。

## 重心与包含判断

数值敏感的多边形计算采用数学等价的稳定公式，不复现参考实现中不稳定的浮点错值。小环面积一阶矩在局部坐标中累计，近直角边的系数避开 `asin` 的导数奇点；舍入敏感度估计决定是否使用稳定结果，原面积、线、点回退仍保留。多边形经界比较数值敏感时采用稳定边叉积，精确作者顶点仍保留原边界取舍。这些工程估计不是认证区间界。普通固定参考容差保持不变，独立 80/120 位见证另行保留原错值。

`geo_spherical_centroid` 返回角度制的 `Result<Option<[f64; 2]>>`，遵循原 D3 面积向量、长度加权、点均值的回退顺序。Empty、单独 Sphere 和无方向的抵消返回 `None`。Polygon 的线回退保留原 asin(cross) 权重，包括超过 90° 的边。集合按作者贡献累计，不计算地理并集。

`geo_spherical_contains` 使用原球面谓词：Point 要求角距离恰为零，线采用 D3 epsilon 判据，多边形使用有向绕数与南极成员关系。各几何的边界并非统一包含。相同极点采用不同经度时，原角距离可能非零，因此 Point 包含判断可能为 false。

两个入口均先完整验证几何，包含已命中 Sphere 之后的集合成员。上限为累计 100000 坐标、4096 部件、深度 32，要求有限规范坐标及精确闭环。线与环中距离对跖不足 1e-8 弧度的边返回 `Err`；无重心方向的 `None` 与非法来源不同。这些计算不修复拓扑，也不替代 Scene 拾取。

```rust
use kartis::*;

let region = vec![vec![[0., 0.], [0., 90.], [90., 0.], [0., 0.]]];
let geometry = GeoSphericalGeometry::Polygon(&region);
let center_degrees = geo_spherical_centroid(geometry)?;
let inside = geo_spherical_contains(geometry, [30., 30.])?;
assert!(center_degrees.is_some());
assert!(inside);
# Ok::<(), kartis::ChartError>(())
```

两个共享展示场景也实际调用多边形重心和包含判断，并在第四行显示角度制重心、受检查询点及布尔结果；原圆心零距离符号保持其真实来源值。

导出器支持 `--surface direct|offscreen`、Offscreen 的 `--fallback error|direct`，以及 `--font sans|system`。原生 CPU 导出将选中的通用字体别名显式绑定到提供的 ABeeZee 字体；浏览器仍采用各自的字体解析。例如：

```sh
cargo run --features raster,geojson --example geo_spherical_export -- output-directory --surface offscreen --fallback direct --font system
```

能力、预算与发布行为见 [CanvasScript 表面选项](canvas-script.zh-CN.md)。
