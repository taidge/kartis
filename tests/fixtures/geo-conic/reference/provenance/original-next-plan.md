# 下一批普通 Conic / Albers：只读准备

状态：**尚未实现**。本文件及 `geo-conic-next-probe/` 只保存下一批调查，不计入当前验收；正式源码、README、docs、fixtures、gates 和历史验收产物未修改。没有启动构建、浏览器、GPU 或 worker。

## 已实际核对的依据

- 当前入口：`src/geo.rs`（10 种投影、GeoMapOptions、拟合/相机/球面裁剪）、`src/geo/inverse.rs`、`src/geo/path.rs`、`src/geo/stream.rs`。
- 原函数：已冻结 `tests/fixtures/geo-inverse-d3-oracle.js`，包含官方 d3-geo **3.1.1** / d3-array **3.2.5** 原 UMD。Bundle SHA256：`99a3c922957a209b6ca551a7a463f31bb30589509459b197ba1bf4d80da0eb5b`。完整许可证 SHA256：`6c28d42e4014c126c953198f9bde4dba985d4215e5d0d33b06c7b4a1568417ce`。
- 本地官方 ESM：正常 metadata target 下 `d3-geo-reference/node_modules/d3-geo/src/projection/{conic,conicEqualArea,conicConformal,conicEquidistant,cylindricalEqualArea,albers,albersUsa}.js`，另读 projection/index、resample、math 和 antimeridian。已核该本地包版本及完整 dist 与冻结 UMD 分段 SHA 一致；每个 ESM 文件 SHA 保存在 probe JSON。ESM用于解释；所有实际数值均执行冻结 UMD 原函数产生。
- 实际 probe：81 个参数 profile，7,290 条 raw 正/反算，4,128 条任意平面反算，72 条锥面横轴临界反算，5 个完整工厂默认值，24 个可变 parallels setter，20 个默认 stream，4 个独立 Data fit，18 个 World-frame 探测。
- 选取合法常用标准纬线、`|lat| <= 85`、`|lon| < 180` 的 1,260 条普通子集，原版自身最大 raw 往返误差 `3.1086244689504383e-15`，最大球面向量误差 `1.1329188834853814e-14`。这不是 Rust 对照成绩。
- 630 条 raw 非有限输出包括故意传入 NaN/Infinity/越界纬线，不能称为普通投影失败。任意平面反算有 2,415 条虽返回有限值却超出地理分支或无法回投，说明不能仅以 `is_finite()` 判定 intrinsic 有效。

原始数据：[original-d3-conic-probe.json](geo-conic-next-probe/original-d3-conic-probe.json)。摘要：[summary.json](geo-conic-next-probe/summary.json)、[findings.json](geo-conic-next-probe/findings.json)。可重跑的只读 producer：[probe.mjs](geo-conic-next-probe/probe.mjs)，最新日志：[probe-final.log](geo-conic-next-probe/probe-final.log)。初轮数据/源码/log另存，未覆盖正式历史。

## 官方默认参数与 Albers 的实际含义

以下是原 getter 的名义值；JSON保留角度往返产生的 `59.99999999999999` 等实际 double。

| 原工厂 | parallels | center | rotate | scale | translate |
|---|---|---|---|---|---|
| geoConicEqualArea | [0,60] | [0,33.6442] | [0,0,0] | 155.424 | [480,250] |
| geoConicConformal | [30,30] | [0,0] | [0,0,0] | 109.5 | [480,250] |
| geoConicEquidistant | [0,60] | [0,13.9389] | [0,0,0] | 131.154 | [480,250] |
| geoAlbers | [29.5,45.5] | [-0.6,38.7] | [96,0,0] | 1070 | [480,250] |

这四个工厂的 precision 默认均为 `sqrt(0.5)`；原 getter `clipAngle()` 为 0（antimeridian preclip），`clipExtent()` 为 null。0 不表示一个半径为零的球面 cap。

`geoAlbers()` **没有新增 raw 方程**，就是 Conic Equal Area 的上表参数预设。9 个原函数查询与手工配置相同参数的 geoConicEqualArea 逐 double 完全一致。

当前 Kartis 的 `GeoMapOptions.center` 是球面相机中心；D3 `.center()` 是投影平面重定心，和 `.rotate()` 不同。不能将 Albers 的 `[-0.6,38.7]` 填进现有相机 center。把相机改成 `[−96.6,38.7]` 来替代 D3 的 rotate+center，虽让中心点重合，旧金山探测点仍偏移 **55.080339229558206 px**，纽约点偏移 **40.12634656226987 px**。

因此首批应延续 Kartis 的 World/Data fit 与相机语义：普通 Albers 作为 `[29.5,45.5]` 的数学投影别名，提供明确的美国地图 view 示例（相机经度 −96、数据拟合）。不要宣称此拟合布局等同 D3 默认固定 960×500 视口。若要公开“逐像素复制 D3 Albers 默认工厂”的 preset，必须另有明确的 projected-origin/affine 配置，不能暗改现有 center 含义。

## 最小推荐公开 API

优先采用可附带受检参数的新 enum 分支，避免给 `GeoMapOptions` 增添一个所有非 Conic 调用都要填写的字段：

```rust
// 名称为设计建议；尚未添加到正式代码。
pub struct GeoConicParallels { /* 私有、有界、有限的纬度对 */ }
impl GeoConicParallels {
    pub fn new(degrees: [f64; 2]) -> Result<Self>;
    pub fn degrees(self) -> [f64; 2];
}
// GeoProjection 新分支示意：
// ConicEqualArea(GeoConicParallels)
// ConicConformal(GeoConicParallels)
// ConicEquidistant(GeoConicParallels)
// GeoProjection::albers() => EqualArea([29.5,45.5])
```

首批参数策略建议：有限、`abs(latitude) < 90`，保留输入顺序，支持相等、反序、南半球和对称纬线；不把两个标准纬线限制成必须递增。±90 标准纬线另列未支持奇点配置。私有构造器保证无 NaN，并规范化零，从而继续保持当前 `GeoProjection` 的 Copy/Eq 性质，不引入运行时依赖。不要把任意 f64 公共字段直接塞进原本 Eq 的 enum。

24 个 setter 实测还确认：D3 会原地修改并返回同一个 projection。Conformal 的 `.parallels([NaN,30])` 不抛错，getter保留NaN，却因 `!n` 进入 Mercator fallback，正反算仍返回有限值。因此受检 API 必须先拒绝非法参数，不能通过“输出仍有限”来放行；应用变更应走新候选/context的原子替换。

默认构造器分别使用上表标准纬线。Albers 复用 Equal Area 内核，不能与 AlbersUSA 混为一个分支。实际 Scene、choropleth、point overlay、route/graticule、inverse 都必须使用同一个解析后的参数内核；不能只交付一个独立 raw mapper。

私有 `ResolvedProjection`/`ConicKernel` 在 options/context 构造时缓存 n、c/r0 或 f/g 及 fallback 类型。当前 `raw`/`raw_vector`、Sampler 和 stream 仅保存无参数 enum，新增参数后需要一起接上；避免每个 adaptive sample 重新计算 sin/log/pow 参数常数。

## 实测退化、奇点与受检策略

1. **Equal Area**：`n=(sin(phi0)+sin(phi1))/2`；原 D3 `abs(n)<1e-6` 时调用 cylindricalEqualAreaRaw(phi0)，不是 Equirectangular。现有库没有 cylindrical equal-area raw，需新增这两个短公式及其受检反算。`[-30,30]`、raw 输入 `[1,0.5]` 实测结果 `[0.8660254037844387,0.5535929275390358]`。
2. **Conformal**：相等纬线用 `sin(phi0)`，否则用 log 比值；原 D3仅在 n 为零/假值时回退 mercatorRaw。可复用现有 Mercator 的正反公式，但要分离“公式复用”与既有 85.051128…° cap 政策。非零 n 的另一侧极点被原 D3钳到 `pi/2 - 1e-6` 的距离，形成多对一平台；这些极点不能宣称严格 source 往返。
3. **Equidistant**：原 D3 `abs(n)<1e-6` 返回原 Equirectangular raw（不带 cos(phi0) 的水平缩放）。当前 Equirectangular raw/受检反算可复用。`[-30,30]` 的 `[1,0.5]` 实测恰为 `[1,0.5]`，而 `[-30,30.001]` 为 `[0.8660168737978546,0.5000036083947634]`；因此不能擅自用平滑极限取代官方 fallback。
4. **近零非零 n**：存在大常数相减。Conformal `[-30,30.00001]` 的原版自身 raw 往返误差已达 `1.862645149230957e-9`。初版可明确拒绝非零 `abs(n)<1e-6` 的 Conformal 参数，作为待下一批 Rust/JS 实测验证的受检范围建议；它不是本轮已经加入的合同。不要通过放宽全部普通点阈值掩盖这类条件数问题。
5. **锥面横轴反算**：72 个临界原查询中22个发生 >1e-7 的 raw 往返误差。实证：Equal Area `[29.5,45.5]`，source `[149.29408664190134°,30°]` 投到 `[1.4350348127226495,1.9282500093701398]`，原 inverse 却返回 longitude 0。`sign(r0-y)==0` 丢失方向。应使用符号一致的 atan2 等价式并做原 forward→正确 authored source 的独立安全 profile；不要求复制原版错误，也不能把这些记录算进原 inverse 数值等价通过。
6. **任意平面输入**：Equal Area 的原 asin 会钳 ±1，可能把锥顶/环外任意点伪装成极点。需要检查径向纬度表达式落在 [-1,1]、canonical lambda 落在 [-pi,pi]，Conformal/EQD 亦需检查对应径向/纬度域，并检查可表示分支的 forward 重投残差。非法 intrinsic 点返回 None；非有限或溢出仍是 Err，遵现有 inverse 合同。

## World/Data fit 是必要集成点

不能直接把新分支塞进当前 `world()` 的默认两点对称 bbox：它只采 raw(pi,0).x 与 raw(0,pi/2).y，对普通锥面不保守。

原 Equal Area `[0,60]` 的实际完整 Sphere stream 在 unit raw frame 得到边界约 `[-3.0851005144,-0.8452994616,3.0851005144,2.1328206677]`，旧两点公式却给 `[-2.2584503230,-1.4641016151,2.2584503230,1.4641016151]`。这是不覆盖真实原函数点的直接证据，不能用于 World fit。

- EA/EQD 用圆锥扇区/环的径向端点与 theta 的端点、0、±pi/2、±pi 等落在有效区间内的临界值求保守边界；南半球符号一并处理。圆柱 fallback 使用其真实矩形域。
- 非零 n 的 Conformal 需要按所声明的极点 clamp/有限域求边界。原 `.precision(1e-5)` Sphere 探测会超过本探测器 20,000 点预算；相关 JSON `boundsComplete=false`，**部分 bounds 不是准确全局 bbox**。不要直接照抄为 golden bounds，也不需要提高产品预算来迁就原算法细分。
- Conformal 零 n 退化为通用 mercatorRaw 时，原 generic Conic Sphere stream 实际发出3个非有限点，不能当有限 World-fit 参考。最小建议将这一退化明确规范到当前已支持的 bounded Mercator 内核/纬度 cap（保持既有 Mercator 合同），并单列与原 generic Conic 极点流的差异；若不接受该政策，应拒绝此配置的无界 World fit，不能静默制造 bbox。
- Data fit 必须使用可见裁剪后 polygon 边界（包括曲边 extrema），overlay 点不参与重新拟合。保留现有 fitted_raw_center/scale/translate；相机旋转与拟合平移各只应用一次。固定 scale/translate 测试和独立原 D3 fitExtent 测试分开，不能把 Rust 拟合中心直接喂回 oracle 来“验证”拟合本身。

## 正算、反算、流与预算的落点

- `src/geo.rs`：options 参数检查、rotate/spherical、raw/raw_vector、world/fitted_context、planar_polygon。Conic 仍走 antimeridian 分割，不走 hemisphere clipper；不能把 clip_angle 当标准纬线或纬度 cap。
- `src/geo/path.rs`：Sampler 持有解析后 kernel；保持 quarter/mid/three-quarter 的逻辑像素误差规则。声明的是 sampled 误差，不是任意曲线 Hausdorff 保证。
- `src/geo/stream.rs`：Sample.raw、source 的 intrinsic 可见范围、projected/sampler、routes/graticules 共同使用同一 kernel。`clip_projection` 当前仅会添加 azimuthal 圆形 clip；Cone不是圆盘，不应伪造 projection_disk。
- `src/geo/inverse.rs`：先撤拟合 affine，再受检 raw inverse，再撤球面相机；添加 annular-sector/纬度/分支检查。`invert_in_plot` 继续先检查包含边界的 plot 矩形。feature polygon/hole 覆盖仍由 Scene typed picking 处理，inverse 本身不推断 feature 覆盖。
- `src/browser/geo_coordinates.rs` 和 native `src/raster_surface/geo_coordinates.rs` 已共享 host/client→Scene 的坐标链；新投影直接复用，Scene camera只撤一次，保留 intrinsic inverse 与 visible plot inverse 两种选择。应用要与 accepted Scene 一起替换 context，不能仅凭同尺寸判断旧 context 仍有效。
- 不提高现有上限：features≤4096、source points≤100,000、generated points≤180,000；adaptive 默认 precision0.5px / depth20 / points180,000 / work4,000,000，受检允许 depth≤24、precision0.0001..64；overlay 总 command 预算200,000、max_marks现有上限，未解叶节点/非有限采样必须失败。相机/scale/translate/Scene ±1,000,000 坐标预算照旧。所有失败先构造候选，不能留下半个 Scene/binding/context。

## 最小“完成”范围与验收

推荐下一批完成：三种 Conic 的受检标准纬线/默认构造器、普通 Albers 数学预设；EA/EQD 的 World/Data fit、上述明确政策下的 Conformal World/Data fit；解析后的同一 context 覆盖 polygon+hole、choropleth、GeoSymbol、routes/graticule、正反算。参数/尺寸/数据更新时重新布局、保原 feature/raw IDs、拒绝时保旧 Scene/typed binding/context。公开受检范围及两个数值边界 profile 必须随代码一起交付。

原参考测试方案：

1. 不新增 npm/runtime依赖，复用现有完整许可 UMD。下一批 producer 可读新 profile 配置，冻结独立 conic golden 与每个新官方 ESM SHA；本轮 target probe 不直接冒充正式 fixture。
2. 普通 profile：三类 ×默认/反序/同纬线/南半球/对称/Albers参数，camera `[0,0]`、`[35,20]`、`[-80,30]` 与 roll0/25/−30，scale75/150及非对称 translate；显式 raw正/反、单位affine、原 D3 fitExtent 分表。保持现有 Geo 普通点误差标准，不为新增家族放宽旧 gates。
3. singular profiles：±90标准纬线拒绝、NaN/Inf/非法scale/非法纬线；近零 n、canonical seam、极点、原 D3 横轴错误、annulus/cone之外点、Conformal clamp和Mercator退化流单列。普通 inverse 同值验证与“原 forward+authored source”的安全改正验证分开计数。
4. Stream oracle：读取原 stream events，检查 seam components、独立 polygon+hole 拓扑、可见端点、同球面 minor great-circle 上的点、实际路径 sampled deviation及预算失败。两种 adaptive 算法不要求相同细分点数；不能仅比 bbox，也不能为通过删除 closure/hole/segments。
5. 真实消费者：一个 Albers 区域 choropleth+source symbols，一个 Conformal/EQD route+graticule；gallery SVG/Canvas导出及CPU像素；BrowserChart SVG/Canvas的可信pointer→inverse→marker回投（DPR1/2、正CSS非均匀缩放、Scene pan/zoom只逆一次）；原生 Apace typed pointer显式Bounds adapter与intrinsic Enter分开标注，含非法提案/resize/stop清理。最终正常工作区新产物重跑全部保留旧 gates，本轮不启动。

## AlbersUSA 必须单独排期

它不是普通 Albers 的一个 parallels 预设。原实现复合 lower48、Alaska、Hawaii 三套独立 conic 参数/旋转/中心；Alaska scale为主图0.35，translate随主图 scale联动，三套互斥 clip rectangles带 epsilon。

原 inverse 按标准化 inset 矩形选择 region（部分边界半开），否则落到lower48；比如 probe `[1000,1000]` inverse仍返回经纬度，但再 forward 为null。受检实现要验证 region+forward，而不能把这种返回值直接当可见地图坐标。

现有 GeoProjectionContext 是单一 raw/affine，不能靠新增一个unit enum分支完成USA。完整复合支持还需要：每region上下文/clip/所属关系、multiplex线和多边形重组、inset-aware符号和拾取、复合fit/resize、边界优先级/空隙、跨region原始ID保留；三路累计预算不得各自放宽成三倍。建议作为独立后一批，普通 Albers 的完成状态不能算作 AlbersUSA 已支持。

本轮只完成调查与原函数数据准备；上述API、数学修正和集成均尚未写入正式库。
