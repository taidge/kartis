# Nivo / D3 差距审计与下一轮任务

原始审计日期：2026-11-10；共享字体与可编辑 Brush 更新：2026-11-11。范围是本地快照的公开图表数据契约、配置与实际计算；这是可实施清单，不是逐个 React prop 的移植承诺。已有通用 `Scene`、图层、格式器或调用者手改 mark 能实现某种效果，不等于对应图型已经提供稳定的 typed API。

## 当前差距摘要（2026-11-16）

BoxPlot整部件指针本轮已正式验收：显式PaintedGeometry及原始outlier快照、Glory有限坐标通道与四宿主接线完成；完整第三轮8上下文／128宿主／512阶段、独立1888来源像素与6480逻辑PNG通过。旧body-only入口保留；Geometry solid-paint、有界曲线／clip、平台transform与真实设备边界见[整部件指南](whole-part-pointer.zh-CN.md)及[验收记录](verification.md)。逐摘要String图例工厂／显式ByLabel也已完成正式受检范围：类型化行到目标、缓存／显式替换、完整第四轮8上下文／224逻辑宿主／288实际mount／896阶段及独立审计通过。借用的逐摘要源颜色工厂已落实，实际受检范围见[颜色工厂指南](boxplot-color-factory.zh-CN.md)。FullYears 月份间距与 Radar 逐点标签工厂也已补齐，详见下方配置说明。GPU／worker继续暂缓。

Conic 中点进度盲点已修复，修复前基线 featureAccepted=false 已归档；新版本已按同源终结证据完成受检范围验收并归档，未沿用修复前成绩。全新完整 cohort 2863059d-9ad6-4490-a2be-a0f1caf966bf 通过 183/183，Firefox、WebKit、Edge 各完成明确选取的 61 套，整个 piecewiseScales 仍排除。四组系统 Chrome CLI 独立通过，host 严格 compositor CLI 未计入。新版首轮 182/183 因 WebKit scaleProfiles 等待滚动稳定超时而保留为失败；原脚本在新旧宿主的四次独立对照未复现，原因仍未确定。最终仅采用三份完整新子报告，不复用部分通过项，也未更改断言、超时或像素门槛。详细新版本状态见[验收记录](verification.md)。

普通 Nivo 图族已有对应布局，主要差距已转为配置完整度、D3 基建和宿主能力。
以下按当前实现区分，完成证据与历史检查点见后文和 verification。

Waffle 分组颜色／格式／边框函数与 owned 原始应用数据已落实受检范围：同包默认 2003、
全特性 2339、文档 18，以及五浏览器配置／两 DPR 的 40 场景、240 帧、160 次候选拒绝
通过。普通 Tree motion 的真实额外注释重复也已修复并通过 60 步专项及 450 步 Spring
回归。组区域实时组件与 Waffle Spring 仍单列；字面 SVG 层及保留式整层已实现，完整浏览器矩阵及独立读回通过，见后节。Funnel Spring 实现与验收进度见后节。Funnel 字面
SVG builtin occurrences 与真实 Glory 整层组件已在独立 `BrowserFunnel` 落地并通过专项
验收；四个原生 part hook、标准与保留式 Glory tooltip 也已通过后续 10 配置／40 profile、
2,000 帧、160 次拒绝及 20 个真实 DOM 负控，见[事件与提示验收](verification.md#development-funnel-native-events-and-tooltips)。
GPU／worker 后置。见[Waffle 指南](waffle-functions.zh-CN.md)与
[受检边界](verification.md#development-waffle-functions-and-owned-data)。

本次重新对照原版 Nivo 后，已把 Bar 分类 BandScale 配置、Bar 单柱和 HeatMap 单元格的类型化几何/画笔工厂落实到正式源码。使用方式见 [分类尺度](bar-category-scale.zh-CN.md)、[自定义 Bar](bar-custom-geometry.zh-CN.md) 和 [自定义 HeatMap](heatmap-custom.zh-CN.md)。这三项只覆盖受检的尺度与单标记契约，不替代任意组件、更多角色配置或宿主验收。

| 范围 | 当前状态 | 继续补齐的重点 |
| --- | --- | --- |
| Nivo 图族 | 已有普通图族对应入口，保留不同数据模型 | 逐图型共享配置、自定义几何/Widget、标签与图例的角色接线 |
| 等值线与二维密度 | 正式功能验收通过：网格拓扑/孔洞、加权 Float32 场、稳定身份与阈值复用、SVG/Canvas/CPU、真实拾取及发布恢复 | 更广的数据与宿主 profile；新图型专属配置继续按预算扩展 |
| Brush | X/Y/XY 持续选区、移动/边角调整、修饰键、会话生命周期已实现；四引擎专项、165 套矩阵及原生 headless 接线统一验收通过 | 原生原始修饰键事件需应用显式转发；更多真实触摸设备与辅助技术验收 |
| 数值尺度 | Linear/Log/SymLog/UTC/Point/Pow/Sqrt；多断点含重复／全常量域已接 Line、Scatter 和完整断点视口，原生及四浏览器专项通过；本轮完整 Chrome 宿主 CLI 的原严格跨帧门禁通过；更广跨引擎整目录尚未同版本重跑 | 更多图型接线、更宽的 D3 输入契约及平台数学边界 |
| 颜色尺度 | 连续/发散、Quantize/Quantile/Threshold、非线性输入；SequentialQuantile 样本右排名及精确数据轴图例、三消费者和四引擎功能验收已完成；较早整体严格跨帧基准仍未通过 | 更多消费者角色；空样本非零自动域策略 |
| 地理 | Nivo 十种投影、有界裁剪/线流/经纬网；Conic 三家族与 Albers、曲边拟合、反经线重连和真实符号轮廓；六项球面工具已有原版 D3、高精度数值及真实导出专项；开发版 Albers USA 地图、分区线网、符号及真实宿主交互已通过专项验收 | 更广球面拓扑、退化角接触的逐位兼容和可见范围；默认 Firefox Sans 绘制差异与较早严格跨帧基准仍未通过，显式 System/Offscreen 专项不替代默认字体结果 |
| 文字 | 真实测量、字体/字重、普通 Text 基线、字形轮廓、显式 bidi/竖排、COLR v0；开发版真实测量折行及单行空白协议 | 更完整 CSS/基线、COLR v1、bitmap glyph |
| 平台与性能 | SVG/Canvas/CPU、浏览器矩阵及部分原生窗口已有证据 | 更多真实输入设备/读屏/系统验收，完整分配归因；GPU/worker 按用户指示最后处理 |

保留式 SVG 的五种已复现包装节点事务故障已完成正常四引擎专项与 180 项范围验收；任意持续 DOM 故障、Canvas 及 legacy phase/mode 应用缓存原子性不在此结论内。

等值线/密度已完成正式功能交付，见 [中文指南](contour-density.zh-CN.md) 与 [English guide](contour-density.md)；[原计划](d3-contour-density-plan-2026-11-11.md) 保留准备阶段证据。

计划与参考数据不单独计入已交付功能。较早高级尺度第五轮矩阵 173/174、Chrome 严格跨帧像素失败仍保留；本轮新包的完整 Chrome 宿主 CLI 已通过，包括 DPR1/2 原始严格尺度流程，96 对回滚的 paint/screen raw 差均为零。未推断旧差异的唯一根因，也未把旧三引擎整目录改绿；更广同版本全目录仍待重跑。详见 [当前验收检查点](verification.md#development-cell-widgets-and-complete-chrome-host-cli)。可编辑 Brush 的实现入口与边界见
[使用说明](editable-brush.zh-CN.md)，原要求保留在 [Brush 计划](d3-editable-brush-plan-2026-11-11.md)。

## 新增独立图型与逐轴布局

| 图型 | 已实现的入口与行为 | 支持边界 |
| --- | --- | --- |
| Funnel | `funnel_layout` 提供横纵布局、阶段前后宽度、线性与平滑边缘、真实间距与分隔线、受控高亮、原始阶段和拾取绑定。开发版提供函数样式、绑定注解、字面 SVG 层、保留式 Glory 整层组件、原生 hook、tooltip、泛型原始业务数据及逐属性 Spring；见后节。 | 专用浏览器 owner 为 SVG；纯采样 Scene 可用于 SVG／Canvas／raster。任意可变 D3 generator、Source Widgets 内层混排与任意 UI 导出仍单列。上游无 partComponent 属性，普通 keyed map 也没有退出过渡。 |
| Waffle | `waffle_layout` 提供四向网格、两种分配、隐藏后重排、圆角单元格和有色分组 union；函数及 `WaffleDataFrame<D>` 保留完整来源。开发版专属 SVG owner 增加可重复 Cells／透明 Areas／Legends／Glory 整层及 typed 四 hook／共享提示，实现与当前验收状态见后节。 | 新 SVG IndependentRound 支持超额请求并裁剪占用、合并角接触轮廓；legacy 总量／轮廓规则不变。新 owner 完整浏览器矩阵及独立读回通过；Canvas owner、Spring、内层 item／组区域 Widget 与 UI 导出仍单列。 |
| Sunburst | `sunburst_layout` 提供 D3 等面积分区、内部节点权重、父色继承、弧形圆角和间距、节点路径及类型化拾取。类型化文字工厂在测量签名前构建所属文字，保留原datum和数值。测量标签支持优先级、偏移、隐藏恢复及 Plot/ClipToArc/RequireFullInkBounds 三种策略。 | 隐藏节点保留原角度区间。真实文字墨迹须相对原锚点测量；严格 AABB 可能额外省略旋转文字。扇环蒙版保留实际 pad、圆角与内孔，路径查询另设工作预算。 |
| 平行坐标 | `parallel_layout_with_scale_options` 提供独立端点推断、Linear、Log、SymLog、UTC、刻度与行几何共享比例尺，并返回原始值、投影顶点和拾取绑定。 | 显式或推断域在隐藏行时保持稳定。刻度预算包含实际日历区间刻度。开发版已提供逐轴数据/像素/Brush 区间筛选控制器，保留全源尺度与身份；实时宿主 Brush 事件桥仍需应用连接。分类轴仍是未实现的 Rust 扩展。 |

四个新增入口保留旧入口的行为。它们与共享 `Scene`、Glory 控件及保留式 SVG、Canvas 共用原有渲染路径。

## 近期实施：独立 Scatter

本批 `scatter_layout` 已直接布局节点，保留旧Line删路径的 `scatter()`。新入口有稳定series/node业务ID、raw `CategoryKey` XY/格式/源索引，Continuous numeric/UTC和X/Y Point、四侧轴、动态尺寸、真实glyph及typed交互/导航。默认linear min0/maxauto/niceDefault、Circle直径9与旧半径5分开；Point Y遵循实际Nivo的top→bottom。Filled面积、Stroked D3 nominal与直径分别建模，不混用单位。

基础8all/8no、独立9all/8no（CPU1）、Ycap3all/3no以及actual Nivo9profiles/76points/18domains已通过；182静态Chrome/Firefox已通过。独立Circle0审计确认的共享描边/拾取缺陷已修复，正式pipeline实测841项Rust+9doctest=850、no-default733及四consumer通过，较前轮813项Rust新增28。统一包的72入口、六case×两phase动态和12组端帽通过。旧矩阵为113/114，唯一失败是WebKit的平行坐标悬停事件顺序。所有权修复先在四浏览器通过隔离专项，再随共享尺度阶段通过完整 120/120 复验。前轮822/707、176/66/111另行归档。

保留明确profile：series/node ID为有界String；UTC为有界整数epoch millis，非JS Date对象；连续轴拒绝String，Point保留数字/字符串身份。显式Point domain遗漏的raw key只留metadata、不造像素代理；尺寸missing可Error/Skip/Fixed，负外推/不可表示结果Err。logical sites保留size0和域外中心，Nearest按真实origin且query限plot，Exact按真实墨迹/clip；不把不可见中心当假填充。公共 `Scene` 仍是有限f64兼容标量，类别Y的真实值保存在metadata/typed binding而非伪造数值。任意React nodeComponent、blendMode、自动文字避碰和无界规模并未因此完成。

零长度stroke的Chrome/Firefox原生行为各经228组观测，结果存在差异。Kartis对零Circle、双零Rect、所有支持点重合且含Line/Cubic或Close的Path（含`M Z`，仅Move除外）按SVG2规则生成paint footprint，SVG/retained/Canvas/CPU/typed Exact一次fill实际cap。零长度 `[3,3]` offset3保留端点cap。原Scene及业务metadata/anchor/clip保留，SVG `d`可为规范化墨迹。修复专项7all/6no、Widget1all/1no、Chrome/Firefox每家60组×3端帽零像素差，以及真实host 12组端帽均已通过。[验收记录](verification.md)

## 参考边界

| 来源 | 本次核对的实际版本与路径 |
| --- | --- |
| Nivo | `E:/Repos/nivo`，commit `0dae2c32052a573f9f1d66ec1b453b572119b265`；已核对的 28 个图表包均为 `0.99.0`。下文 `N/` 表示其 `packages/`。主要依据各包 `src/types.ts`、`src/index.ts`、计算/hooks，而非网站展示截图。Geo 使用 `N/geo/index.d.ts` 与 `src/*.js`；Bump 的类型在子目录。 |
| D3 聚合仓库 | `E:/Repos/d3`，commit `db118c93a429e82b83f091dbaa18fd0a82c06130`，`package.json` 标示 `7.9.2`；`src/` 只有导出文件 `index.js`，不能把它当作所有子包算法源码。另核对其 `docs/d3-{shape,scale,geo,hierarchy}/`。 |
| 实际 D3 子包源码 | 本项目先前验收解包的 `target/d3-reference/node_modules/d3-shape` **3.2.0**（`src/arc.js`、`symbol.js`、`curve/*`）；`target/d3-geo-reference/node_modules/d3-geo` **3.1.1**（`src/projection/transverseMercator.js`、`graticule.js`、`resample.js`、`clip/*`）。这些是参考产物，不是 Kartis 运行依赖。 |
| P0 颜色参考 | 所属workspace的metadata `target_directory` 下 `d3-color-reference`：`d3-scale` **4.0.2**、`d3-interpolate` **3.0.1**、`d3-color` **3.1.0**，300条独立冻结oracle。再生成命令见 [verification](verification.md)，脚本按metadata动态发现实际参考路径。 |
| Kartis | 当前 `0.1.0` 工作树，含P0、P1配置及本轮独立 `marimekko_layout`、`delaunay`、`geo/path` / `geo/stream`。默认Rust依赖增加Spade2.15.1与robust1.2.0；它们与开发oracle分开，见[third-party notices](third-party-notices.md)。实现profile与正式验收状态分开记录。 |

**不再列作缺口：**闭合/open 曲线与 tension、Wiggle/InsideOut、循环 Sankey、FrontChain/最小包围圆、Resquarify、分类与独立 X/Y 域导航、完整图案/渐变/dash/mask、显式多轮廓 morph、日期解析/locale/IANA、真实网络拖动与动态换图、原生 CPU raster、可测量标签避碰基础设施、可自定义 legend/tooltip Widget。这些已存在；P0共享颜色/HeatMap/Bar/公共polar与角堆叠RadialBar，以及P1 AreaBump、Hierarchy样式/focus、compact Calendar、样本BoxPlot/Bullet配置均已有typed实现与专项证据。本轮独立模型已落实；下文区分其可用profile与仍需完成的共享协议、配置及平台能力。

分类：**A**＝可用现有 Scene/宿主完成；**B**＝需要新增布局算法、数据模型、控制器或跨后端协议；**C**＝需要 Glory/平台能力或实际设备验收。GPU/worker 单独放在最后。

## 模型差异的已落实入口

- **Marimekko 两个独立模型均保留**：旧 `marimekko` 从维度总和决定宽度并固定expand；新 `marimekko_layout` 接独立category thickness + dimensions，默认none/nice10，支持五种offset。None/zero/隐藏、raw/interval/displayed和padding计数均有明确metadata/profile；不是旧入口改名。来源为`N/marimekko/src/{types,hooks,defaults}.ts`，实际hooks328cell oracle已核对。

此前缺失的AreaBump和compact TimeRange已分别由`layout_area_bump`与`calendar_layout(Compact)`落实；旧`bump`/`stream`和完整年份`calendar_range`继续保留，没有用它们改名代替新模型。

其余已核对的普通图族均有对应布局入口，包括 BoxPlot/Bullet/Chord/Funnel/Waffle、Tree/Treemap/Sunburst/Icicle/CirclePacking、Swarm/Parallel/Network、GeoMap/Choropleth。**有入口不代表全部 props 等价**。Voronoi已有KD-tree/cells，本轮Delaunay已新增真实triangles/halfedges/hull/links/neighbors；地理已有graticule、TransverseMercator和有界自适应路径，不再列作缺失算法。`grid/arcs/rects/polar-axes`是共享基建，React `express/static`不是缺失图族；Candlestick、violin、contour不作为本地Nivo图族缺口。

## 前轮完成：共享尺度与真实 glyph

本轮已落地ContinuousScaleOptions/ResolvedContinuousScale、PointScale/point_line_layout，以及通用Filled/Stroked glyph和Line/Radar/Geo接线。正式813项Rust+9doctest=822、no-default707及四consumer流水线已通过，实际252组/1440记录D3 glyph oracle与1132条scale查询独立核对；176画廊正式Chrome及Scale/Glyph/Geo新专项、66导航/32host专项/Chrome五CLI已同源实测通过。37×3=111矩阵已同一次fresh完整运行全部通过（每引擎37/37、176场景/66导航），初轮incomplete诊断不拼入最终报告，不将上一检查点778/no-default668/164/102沿用为本轮成绩。

| 新typed入口 | 实际实现 | 保留边界 |
| --- | --- | --- |
| ContinuousScaleOptions / ResolvedContinuousScale | reverse/nice/round/clamp/log base，source/display domain分开；Line轴、几何、site共用映射 | additive入口；不把全部图族旧scale自动切换；viewport用canonical域且不重复nice；UTC整数instant和有界tick输出 |
| PointScale / point_line_layout | 零bandwidth类别点格，数字/字符串身份分开，padding/align/reverse/round/窗口与真实类别轴 | 非Band改名；显式遗漏类别成为gap；无插值数值x；旧line入口保持 |
| glyph_geometry / glyph_mark | Filled七种、Stroked七种、rotation、真实Geometry/origin/bounds/paint_mode、checked绘制与不可表示拒绝 | filled size是面积，stroke size是D3感知参数；size0依真实cap有无墨迹；有限坐标和Scene预算保留，不自动自定义Widget |
| Path typed anchors / Geo add_glyphs | 当前几何支持点驱动anchor；Line/Radar真实Path Exact、Geo共享projection/同形图例/nullable与projectionmask | bareScene不自动获得typedanchor；Geo zero_radius0省略marker；图例过滤不重算域/fit；任意球面拓扑与标签避碰仍有限 |

跨shape默认Fade已使用目标业务Scene/binding与当前物理帧的原子接入，enter0保留逻辑focus/selection/controls，ghost与opacity0无物理hover/Exact；这不表示任意采样morph都能推断anchor支持点。normalized/canonical morph签名与目标anchor不兼容仍明确Err。旧无binding拾取profile保留。

## 上轮完成：几何、定位与关系协议

本轮补齐此前P1三项，不再把Pie asin/partial sector fit、Line crosshair、Radar polar slices和显式图关系集合作为未实现任务。旧入口和默认输出保留，新模型与旧Scene-only setter的差别明确。最新正式原生769+9=778、no-default668，独立native/oracle已通过；动画union+overlay（含累计dash）发布前准入已补，完整pipeline/四consumer/native Apace、严格WASM/rustdoc与Chrome五CLI已过，四家最新pkg专项已复验（164场景、56导航）；最终34×3=102/102也已同一次fresh完整通过（各164场景、56导航），原阈值保留；这证明本轮验收范围通过，不代表整个Nivo等价目标完成。

| 新typed入口 | 已实现范围 | 仍保留的profile边界 |
| --- | --- | --- |
| `pie_layout` / `PieLayoutOptions` | D3Applied或NivoRequested pad、独立内外asin、圆角、zero/padding与显式border、NivoSector/Centered fit，候选签名与`place_link_labels`真实测量放置 | sector fit包含基础扇区中心，非tight annulus；与Nivo整数角overshoot明确不同。公开Scene已自定义后再次place明确Err而非覆盖；无字体测量不伪造避碰，贪心非全局最优、leader不保证全无交叉；旧角度gap默认保持。 |
| `LineLayout` / `CrosshairOptions` | raw/format/source index/site/slice metadata、11种真实crosshair；Scene/plot/binding一起提交；typed resize/viewport、动画与当前数据 | 不自动重建应用数据；普通Scene setter清binding；单次world inverse，Relayout不重复camera。预排队拒绝保旧motion/clock，末段帧拒绝回旧数据并取消animation/gesture/captures。点/文字避碰与全props仍非自动。 |
| `RadarLayout` / `PolarSliceIndex` | 顺逆时针极角分区、稳定轴 ID、排序多数据提示与键盘导航。共享值轴支持线性、Log 和 SymLog。点几何工厂支持偏移路径和继承样式。 | 最近点使用原始数据锚点，精确拾取使用实际绘制。自动全局标签避碰与 Radar 专属控件工厂待补。 |
| `RelationGraph` / `RelationLayout` | raw节点/边/mark/label归属、Tree祖先后代、Network邻居、Chord incident、Sankey同向parallel、hidden traversal和事务replace | 需显式绑定拓扑，应用接事件提交当前scene；Network edge raw-source系列保证Surface可达，双端hidden仍由Graph处理；隐藏只移ink不重排几何；不推断自动业务语义，style重新从authored Scene合成。 |

## 本轮 P0 已落实

| 原任务 | 已落地的 typed API / 行为 | 仍保留的明确边界 |
| --- | --- | --- |
| 共享颜色尺度 | 连续尺度提供 RGB gamma、Lab、HCL、HSL、Cubehelix 与 38 条命名色带。类别尺度提供 11 种 Nivo 色盘和官方 ColorBrewer 色板，已接入 Bar、Line 和 Pie。 | 命名色带用于钳制的连续尺度。类别颜色工厂与自动对比度已接入 Pie，其他图族的颜色角色仍可继续接入。 |
| HeatMap 扩展 | `heatmap_layout` / `HeatmapLayoutOptions`：NULL、稳定row/column ID、shape/size、两轴padding/四侧axis、边框/圆角/labels；`related`与事务`set_active`精确Cell/Row/Column/RowColumn/All；新增 `heatmap_layout_with_cell_factory` 与显式颜色 profile 工厂、真实墨迹和原始 nullable 拾取绑定 | 稀疏未供给pair不补NULL；明确 NULL 会调用工厂，零值保持零。工厂返回单个受检 Scene 标记，隐藏/resize 通过重建；不等于任意 Widget 插槽。受管来源和标签归属必须保持；另有开发版 `heatmap_cell_widgets` 为实际单元格提供类型化实时组件，使用既有组合/裁剪协议。完整整合验收见 verification。 |
| 极坐标轴与径向堆叠柱 | `PolarFrame`、数值和类别刻度、`polar_axes` / `append_polar_axes`、`polar_bar_with_axes`。`stacked_radial_bar` 按系列建环，按类别累计角度。`layout_stacked_radial_bar_with_arc_style` 为柱段和轨道接入 D3 圆角与半径感知间距。 | 角轴支持非负线性值，隐藏类别保留完整域和栈槽位，正值角投影失真时返回错误。图型专属自定义组件待补。 |
| Bar 展示配置 | `bar_with_style` / `bar_layout_with_style`：group pixel gap、系列/分类色、值标签与 Net/PositiveNegative totals；`BandScaleOptions` / `category_scale` / `category_reverse` 使柱、刻度、合计共享 round/align/inner/outer/step/bandwidth 与分类窗口；`series_order` 统一源系列顺序，`legend_source` 选择Keys/Indexes | 显式类别 profile 先完整范围取整，再投影窗口且不二次取整；`None` 保留旧行为。Keys图例控制series；Indexes分类只独立展示/激活，隐藏系列需重布局才能重排slot/totals。选中的聚合必须可表示，不能把域外 total 挪到 crop 边。 |
| Bar 自定义标记 | `bar_layout_with_bar_factory` / `bar_layout_with_scale_options_and_bar_factory` 提供原始 series/category/index、区间、可见与未裁剪信息、单个 Geometry/FillPaint、私有来源和受控高亮 | `None` 不造标记；零值保留原值及逻辑站点。自定义墨迹不移动普通柱的固定逻辑站点。真实墨迹拾取与逻辑最近点分开，受管整帧变更需工厂重建；开发版 `bar_widgets` 已从类型化工厂布局接入实时角色，原源/几何/标签均受检。 |

这些入口均产生共享Scene/metadata，原实现保留。已有数值/有限值/守恒、CPU像素、当前输入/隐藏/原子拒绝等专项证据，正式统一结果见 [verification.md](verification.md)；不以本表宣称所有Nivo props等价。

P0还修复inline SVG实例共享defs ID的真实渲染缺陷：`to_svg_with_namespace`与retained owner作用域覆盖gradient/pattern/clip及typed引用，原业务ID/文字不改写；live双SVG+Canvas在首owner隐藏/销毁/重挂后保持独立。该轮649项Rust/doctest、126场景和三引擎69套历史结果见 [verification.md](verification.md)，不把历史数量当作本轮新实测，也不据此抹去后续差距。

## 本轮 P1 已落实

| 原任务 | 已落地的 typed API / 行为 | 仍保留的明确边界 |
| --- | --- | --- |
| AreaBump | `layout_area_bump` 提供逐列排名、厚度、曲线、缺失区间和受控焦点。`place_endpoint_labels` 接收实际文字边界，自动在原侧避开其他标签和障碍。 | 非负值，数字和字符串类别键保持独立。隐藏后的排名和厚度由应用重布局。端标签沿原侧竖直调整，可附引线。 |
| 层级样式与焦点 | `treemap_layout` 和 `TreemapLayoutState` 支持内外边距、四侧标题和节点样式。圆堆积投影子树，冰柱图支持四向重排。`HierarchyFocusMotion` 以稳定 ID 过渡，并提供当前几何和业务交互绑定。`HierarchyLayout::place_labels` 接收实际旋转墨迹测量，提供优先级、位移、障碍、受控隐藏和恢复。 | 目标只绘所选子树，同时保留全树元数据。隐藏节点保持原有子节点分配。测量须覆盖当前全部原生标签，旧签名与损坏的受管槽拒绝；圆盘裁剪按完整 AABB 四角保守准入。绘图区变化时立即应用新布局。 |
| Calendar / TimeRange | `calendar_layout`的Compact/FullYears、firstWeekday、weekday ticks、横纵/方长方日格、nullable/zero、文字/圆角/边框、真实月union边界；`calendar_layout_zoned`复用当地日聚合 | Gregorian年1..9999、日期≤12000/32年；空源自动推域需端点；outline pad是边界扩张而非月组间距；NULL需metadata tooltip，不等同Scene标量0。 |
| BoxPlot / Bullet 配置 | 两者均接入 Linear、Log、SymLog 与 UTC 值轴。BoxPlot 支持原始样本、分位数、须和离群点，Bullet 支持共享或逐行域。 | Box 样本≤10000、每分类≤256、组合≤2048。Bullet ≤256 行、每类分段≤64、总源分段≤20000。Log 要求同号非零域，UTC 源值使用有界整数毫秒，BoxPlot 分位数可保留小数。 |

此前轮次已补齐最初队列的Marimekko模型、地理线流和Delaunay，但最初10项只是优先队列，**不代表整个目标完成**。此前轮次最终原生726项Rust+9doctest=735、no-default629已通过，新增37项Rust；native/WASM严格检查及WASM rustdoc通过，完整流水线、四consumer及Chrome五CLI通过（151场景、41项导航、三家新专项）；三引擎各30套、共90/90在同一次fresh完整重跑全部通过。首轮88/90的Geo指针probe歧义仅修fixture测量，新宿主严格构建与Chrome重验后完整重跑，不拼接旧成绩或修改阈值。新入口继续受下列profile与共享协议/图型/平台差距约束，不能由原生或专项推导全部通过。

| 本轮入口 | 当前可用profile | 仍明确保留的边界 |
| --- | --- | --- |
| 独立Marimekko | 独立厚度/维度、None默认、nice、五offset、四轴/标签/样式；零厚度仍进padding/domain而不造ink | 厚度非负，Expand/Silhouette/Wiggle非负；Zero/Diverging signed；数值厚度轴为未加gap的压缩尺度，非所有callback/custom组件 |
| Delaunay | Spade/robust真实剖分、重复ownership、共线链、拓扑索引/links/邻居与均匀fit Scene | ≤20000raw sites；必须可逆归一化到谓词范围，共圆不承诺D3相同对角；Scene预算更低，不jitter |
| 地理线流/精度 | 第10种TM、minor-arc line stream、major/minor graticule、共享context overlay与显式adaptive polygon | 屏幕误差是有界采样判据；近对跖/真实塌缩、polygon开放半球与hole歧义仍拒绝；旧两度入口保留 |

## 本轮 P2 接线

| 图型 | 已实现的入口与契约 | 支持范围 |
| --- | --- | --- |
| Bar | `bar_layout_with_scale_options` 返回 `ScaledBarLayout`。值轴、分组与堆叠几何、文字和合计共享 `ResolvedContinuousScale`，保留原始区间与真实可见区间。 | 支持 Linear/SymLog、反向、整化域和取整。柱的零基线要求有零值定义的数值尺度。渲染裁剪保留真实区间，即使调用者对返回尺度启用了 clamp。 |
| Swarm | `swarm_layout` 返回场景、原域/显示域、组槽、原始数据与交互锚点。支持横纵方向、四种连续尺度及独立的直径、填充面积、描边标称尺寸。 | 共享尺度锁定数据轴坐标，沿另一轴以保守墨迹包络确定性避碰。隐组保留槽与域。贪心放置不可行时返回错误。Nivo 的双轴力模拟可能同时移动两个坐标。 |
| Network | `ForceSimulation::scene_with_glyphs` 按节点 ID 定义符号与样式。`NetworkDragController::begin_with_glyphs` 按可见轮廓拾取，沿用原始物理节点和抓取偏移。`BrowserNetwork::mount_with_glyphs` 以已接受的场景处理实时力模拟和拖动。 | 物理碰撞半径、力和连线端点独立于画出的符号。未指定样式的节点保持圆形。空心和透明形状使用实际绘制语义。浏览器驱动持有类型化交互并保留自定义描边。 |
| Combo | `combo_layout_with_scale_options` 提供独立连续 X、左右 Y 配置与分类窗口。返回原域/显示域、可见几何锚点、语义值位置、原始区间与业务 ID。 | 柱和面积的值轴支持 Linear/SymLog。数值柱与箱体的 X 支持 Linear/UTC。可见区间、最近锚点与原始数据保留各自含义。 |
| Scatter 自定义节点 | `scatter_layout_with_node_factory` 以原始数据、映射坐标、尺寸和继承样式生成单个自定义标记。精确拾取使用真实绘制与裁剪，最近点保留原始数据锚点。 | 可用路径或居中的图元。兼容路径动画跟随指定端点的平均位移，提交帧使用精确映射坐标。开发版 `scatter_node_widgets` 已接入测量和实时生命周期，显式原系列提供完整 datum；自动 spring、animated props 和任意组件导出仍保留。 |
| Line 自定义点 | 连续 Line 与 PointLine 支持同一自定义几何工厂、原始数值或类别键、配置值轴和类别配色。 | 精确拾取按实际裁剪后的墨迹，最近点与切片保留数据锚点。工厂返回单个图形。 |
| 双向、竖排与彩色文字 | `FontRegistry::with_bidi`、`with_vertical_layout` 提供双向、显式多列和 Unicode mixed 方向。COLR v0/CPAL solid layers 可显式选择 palette；CPU 与 `Chart::painted_labels` 共用实际彩层。`scene_with_painted_labels` 为独立 SVG/Canvas/CPU 导出生成路径副本，保留业务标记、归属、绘制顺序及裁剪。 | 默认横排、彩色 glyph 拒绝。透明前景不抹去其他 palette 层。路径副本不保留可编辑原文字，原 Scene 由应用保留。自动折行、完整 CSS bidi/BASE baseline、COLR v1 绘制图及 bitmap glyph 待补。 |
| CPU 图像交互宿主 | `RasterSurface::mount_layout` 与 `RasterSurfaceHandle::set_layout` 原子接纳绘图区、Scene 和类型化绑定，支持自定义路径原始锚点、nullable 控件与原始提示行。metadata-only 更新刷新提示且保留 PNG。 | 应用提供真实宿主坐标边界与图像源发布。图片发布或绑定校验失败保留原始图像、控件和模型。这里的宿主坐标接线不等于已验收硬件输入设备或屏幕阅读器。 |
| Windows 桌面宿主 | Apace 显示语义矢量图，Wry 显示 CPU 位图并使用 `Chart::basic_data_controls`。两者的窗口绘制、键盘、图例、更新和拒绝由实际窗口与 Rust 事件记录验证。 | Windows 辅助功能树可读出图表、图例和数据按钮。Wry 增强导航和语义矢量图取决于宿主支持。硬件鼠标、触摸、屏幕阅读器播报和其他桌面系统待验收。 |
| 艺术混色 | `MarkBlends` 为标记指定 Normal 或 15 种艺术混色模式。SVG、Canvas 和 CPU 将填充、描边与透明度作为一层合成。BrowserChart 提供显式混色动画入口，过渡实例保留各自模式。 | 默认语义 Graphic 使用普通绘制。旧动画入口要求普通绘制配置，图型专属便利配置待补。 |
| 图内 Widget | `Chart::plot_widgets` 和 `BrowserChart::mount_with_plot_widgets` 提供测量矩形、稳定身份和当前帧。开发版另有 `PlotWidgetsData<T>`，将拥有所有权的类型化数据与插槽帧共同接受，覆盖语义 Chart、`raster_chart` 及 SVG/Canvas 浏览器宿主；Bullet、HeatMap/Waffle 单元格、Stream 点、Scatter 节点、Bar、Bump 点、Swarm 节点及三种层次图节点提供类型化角色适配。CPU 图像插槽随 RasterOptions.view 调整且独立于 DPR，并与 PNG 发布共同准入。浏览器覆盖层支持带孔路径裁剪。 | 旧接口为原墨迹上方的覆盖层；开发版 `plot_widgets_composed` 先隐藏预挂载候选，严格共同准入 Scene、元数据、组件与指定原墨迹替换及拾取排除。它仍位于顶部 Widget 层，保留所有 Scene 标签和逻辑控件，不具备任意原图层交错语义。部分裁剪控件暂停输入。图像宿主支持矩形 mask 与路径 Full/Outside，任意 partial mask 仍需 browser mask host。Scene 导出不含实时控件，控件快照导出和更多图型角色仍待补。 |

## 余下任务与完成条件

### 标签配置接线

- Bar/HeatMap 的 owned-value-label 已正式验收 `BarLabels.color_profile` / `HeatmapLayoutOptions.label_color_profile`，复用共享 `InheritedColor`，保留原 palette、工厂画笔、标签 alpha 和受控高亮的独立契约。配套单实色 SVG 导出透明度修复也通过四浏览器。见[标签继承颜色指南](inherited-label-colors.zh-CN.md)。
- `HeatmapFactories` / `BarFactories` 已提供类型化逐源文字和几何回调组合，各有普通与颜色 profile/数值 scale 变体两个入口。相同数值可按原 row/category/index 显示不同文字，原 raw/tooltip/formatted source 不变。仅实际可见/合格标签回调，空串保留 owned slot，hover 不保留或重调闭包；完整规则见[类型化标签指南](typed-labels.zh-CN.md)。
- `BarLabels.zero_extent` 默认 `Skip`；显式 `ValueAxis` 允许正类别宽度下真实映射为零长度的值轴标签，包括零值和像素取整坍缩。继续尊重 skip 阈值、原未裁剪锚点、plot、颜色和工厂透明度。缺失源、完全域外裁剪、零类别带不借此造标签，也不制造有填充面积的拾取代理。

### D3 分位数与显式阈值配色

`ColorScaleKind::Quantile` 按非缺失有限样本计算 R7 阈值；`Threshold` 使用
`ColorScaleOptions::thresholds` 的升序有限边界。两者已共用 HeatMap、Calendar、
Choropleth 和实际离散图例。重复边界保留空档，等值归入后一档；`bin_extent`
和 `invert_extent` 分别按索引及首个匹配颜色反查，阈值首尾档以 `None` 保留无界端。
可选有限 `domain` 是图例窗口，不改样本分布或作者阈值；默认 clamp 匹配有限值的
D3 饱和映射，`clamp=false` 在显示域外返回 unknown。

输入沿用已有有限值与 20,000 项预算，不隐式转换字符串、不接受非有限源值；最多
256 档/255 阈值。空分位源明确输出 unknown，不复刻 D3 空样本中的未定义阈值。
只保留阈值与样本端点，不保留整份分布或访问历史。非默认插值和命名连续色带对
离散档位明确拒绝，调用者可先生成离散色板。

独立 d3-scale 4.0.2 参考已有 42 组配置、1,672 条实际查询/阈值/反查记录；
`scripts/discrete-color-reference.mjs --check` 只读验证冻结结果。原生专项还检查
隐藏后的分位重算、固定业务阈值、nullable 消费者、实际 CPU 图例像素和事务拒绝。
本批浏览器与完整检查的最终证据单独记录在 verification，不沿用旧矩阵数量。

等值线与二维密度、SequentialQuantile 连续分位色带、Conic/Albers 和六项球面工具已在后续轮次实现，专项与范围验收分别记录于 verification。
AlbersUsa 的有界地图／线网／宿主配置已验收，仍需更广球面拓扑和输入范围，以及多断点尺度在更多图族中的接线。分级配色或单项通过不代表所有图型的 Nivo props 已对齐。

### 可编辑 Brush

共享状态机、Surface、浏览器 SVG/Canvas 与 Glory CPU 图像宿主已完成本阶段接线和统一验收。
支持单轴/双轴、持续选区、移动与八方向调整、修饰键、程序控制、键盘微调和受检会话。
独立原版 D3 等价参考、有限极端输入、事务失败、当前数据、视口与清理均已验证；
原生 raw modifiers、真实触摸拖动设备和辅助技术继续按平台差距记录。
API 与恢复边界见 [使用说明](editable-brush.zh-CN.md)，最终成绩见 [验收记录](verification.md)。

### D3 Pow/Sqrt 连续尺度

`ScaleKind::Pow { exponent }` 和 `Sqrt` 已进入 ChartScale、连续配置、刻度和
域视口。`scales::PowScale` 提供独立受检映射、反算与 nice。正指数按 D3 保留
输入符号；负指数支持同号非零连续分支，内部对裁剪/导航使用递增坐标，实际
映射与反算恢复 D3 运算次序，保留取整边界及奇点零符号。刻度和 nice 仍在
原值域生成。零指数、非有限指数/倒数、变换溢出/塌缩以及 nice 扩展到奇点均拒绝。

共享入口已接入 Line、Bar、Combo、Scatter、Parallel、Swarm、BoxPlot、Bullet、
Radar 和 Stream；零基线柱和面积允许正指数，倒数的有限非零域不充当零基线。
空数据入口提供非零倒数域回退；BoxPlot 已修复遗漏的负指数分支，复用原 Log
同号规则，空源默认 `[1,10]`，全隐藏负样本使用 `[-10,-1]`。显式作者域仍优先，
零、跨符号或非有限域继续拒绝，不生成虚假箱体或交互点。新专项最小功能四项、
全功能五项及真实 CPU 轴墨迹通过。视口越过原分支奇点时保留原状态。
新增十项全功能专项（最小功能九项），实际 d3-scale 4.0.2 的 736 组配置、
895 条记录包含映射、反算、nice/round/clamp、原值刻度与 40 组反向缩放/平移。
实际 CPU 墨迹和跨图型几何/锚点已验证。两幅画廊与保留式 scaleProfiles 的
两个新 case 已完成统一验收：四引擎各十二个尺度配置专项、完整 162 套矩阵、
Chrome 五组及 Rust/原生流水线全部通过。最终冻结版本、独立 D3 对照、夹具
修复过程和范围见 [验收记录](verification.md)；这不代表所有 D3 尺度契约已完成。

### 共享尺度的下一层差距

Pow/Sqrt 的几何接线并未完成所有 D3 尺度契约。原连续数值入口保留两个端点，
新增 `PiecewiseScaleOptions` 保存完整数值/UTC 节点，已接 Line、Scatter 的实际轴、
几何、拾取和逐节点视口反算。域要求严格升降序；转向/平台输出保留前向和显式分支
反算，不提供不唯一的全局数据视口。重复域节点已补齐核心与 Line／Scatter 完整断点视口，并通过四浏览器专项；更多图型及更宽 D3 输入契约继续补齐。
参见 [D3 linear domain](https://d3js.org/d3-scale/linear#linear_domain)。

共享颜色目前有 Sequential、Diverging、Quantize、Quantile、Threshold，
新增 `ColorScaleTransform` 已提供 Log/Pow/Sqrt/SymLog 连续与发散颜色输入，
消费者与图例共用真实变换。`SequentialQuantile` 已提供连续样本右排名和精确数据轴图例，与离散 Quantile 档位保持独立契约。
空/全缺失源当前沿用通用 `[0,1]` 自动域，非零变换需显式合法域；后续自动非零域
策略须同时定义发散中点、正负分支与缺失源，不静默过滤真实零值或异号样本。
需要同时验证当前消费者、真实 legend 位置及反向/常值/奇点/unknown，
不能只增加枚举。参考 [D3 sequential](https://d3js.org/d3-scale/sequential)
和 [diverging](https://d3js.org/d3-scale/diverging)。

| 优先级 / 任务 | 最小可交付范围 | 独立验收重点 |
| --- | --- | --- |
| **P2-A/B：更多图型共享尺度** | 审计其余图型的尺度契约，逐一接入适用的共享选项。 | 验证原域、显示域、坐标轴、几何和拾取。明确基线、区间、数值和 UTC 的支持范围。 |
| **P2-B：自定义图形与图内控件** | Scatter、Radar、Line、PointLine、Bar、HeatMap 提供受检几何工厂；开发版另有 Funnel、Waffle、Bullet、Swarm 节点和 Network 连线工厂。Network 浏览器驱动按当前力状态重建自定义边。语义 Chart 和 BrowserChart 提供测量插槽，浏览器支持路径裁剪。继续接入其他图型及图像宿主导出。 | 单标记工厂与任意 Widget 生命周期分别处理；真实轮廓、裁剪、坐标映射、帧更新事务与焦点清理仍须逐角色接线。 |
| **P2-C：原生平台与字体** | Wry 增强导航与语义矢量图、硬件输入、屏幕阅读器、自动折行与更完整的文字布局、COLR v1 与 bitmap glyph。 | 分别验收宿主绘制、输入设备、语音播报和文字几何。显式多列、mixed 方向与 COLR v0 已有独立入口。 |

COLR v0 / CPAL solid layers 已接入共享彩层绘制、测量和纯 Scene 导出。layer 范围、palette 条目、命令与累计复制 clip 预算在发布前校验；`0xFFFF` 前景层使用标签颜色，其他 palette 层保留自身 alpha。COLR v1 的复杂 paint graph 与 bitmap glyph 仍是后续独立范围，不能从当前 v0 结果推导。

普通文字现已接通共享 family/weight：`Theme.font` 经 `Scene.typography` 继承，支持逐标签和 legend/tooltip 角色覆盖，`PresentationText.font` 为局部声明。SVG、Canvas、语义 Text 与字体感知测量共享解析结果；保留式 SVG 的字体缓存支持 font-only 更新。Pie、Hierarchy、Sunburst、AreaBump 的测量候选包含字体签名，受控换字体须重新测量；标签隐藏恢复和重索引保留配置。动画以两套真实字体交叉淡化，保留退出标签的字体。CPU 的显式 `bind_face` 绑定真实静态 face，按字体族顺序和 CSS 权重搜索规则选择同一份字节用于测量、CPU 和 paths，未绑定条目跳过、全部未绑定拒绝。未声明字体仍保留旧注册顺序。CPU 不查询系统字体，也不合成粗体或使用变量轴；完整 slant、自动折行和 COLR v1 仍待补。Nivo SVG 会传递完整 text style，而公共 Canvas helper 仅支持 weight/size/family，忽略 fontStyle；本轮范围是它们实际共有的配置，不宣称完整 CSS 文字排版。

普通 Text 的显式基线协议已落实：`Chart::semantic_text_measurer` 接收真实宿主度量，保留原 `Props::Text`，与 SVG/Canvas 的 alphabetic 锚点对齐。浏览器公开 `BrowserSemanticTextMeasurer`，Apace 示例读取真实 inline line baseline 并扫描实际墨迹。普通 Widget 主示例已默认使用该模式；未配置宿主测量的通用 Chart 保留兼容定位。统一的跨宿主自动测量查询仍可继续完善。显式字体动画的进入和退出 face 均须注册；由旧的无声明 profile 切入显式字体时，退出默认 Sans 也需要相应绑定。命名字体别名的 CPU 大小写匹配目前限 ASCII，完整 Unicode caseless matching 待补。

普通 Text 测量模式的已落实契约：

- 保留 `Label.(x,y)` 的 alphabetic 锚点，提供 advance、line_height、baseline_from_top 和相对锚点 ink_bounds。绘制与避碰共同使用结果，行盒与墨迹分别定位。碰撞先尝试墨迹步长，再尝试行盒步长，保留 padding 与 bearing；最终位移后的行盒也在发布前校验。
- 测量绑定文本、字号、解析字体、行高和宿主字体资源版本。字体加载、切换与回退变化使旧测量失效；拒绝时保留整帧，重试后保留原 Text 节点。
- 浏览器用独立的真实文字格式化上下文与外部零尺寸 inline-block 标记测量基线，避免标记改变末尾空格和空行盒。CSS 保持请求的正行高，实际量化行盒高度只用于布局；Canvas 提供保守墨迹边界，不充当 HTML 基线。
- Apace 使用 renderer inspect 与真实 inline line metrics.baseline，按 viewport scale 和内容区偏移换算逻辑像素；普通 Text 必须保持 `Props::Text`，不由 Glyph/图片替代。
- 原创 B 的 y=140、size=100 已在实际 Apace 与浏览器对齐；覆盖不同字体、400/700、fallback、重音/descender、三个锚点、真实普通 Text DPR1/2、隐藏恢复及避碰后的实际墨迹。另用下移八像素的负对照确认验收能检出基线错误。空串与纯折叠空白使用真实零盒、无墨迹及基线零偏移约定。

Nivo 的 SVG/Canvas top/center/bottom 基线有不同映射。当前共同模式为 alphabetic，central/top 等显式语义、完整 CSS 文本布局与未配置测量器的自动宿主接入仍是后续范围。真实 Glyph 路径和普通 Text 分开验收，后者的对齐由实际行盒与像素证据证明。

### Chord、Bump 与 Stream 的独立入口

| 图型 | 已实现的入口与行为 | 支持边界 |
| --- | --- | --- |
| Chord | `chord_layout` 分开提供群组、子组和双向弦带。较大方向决定来源及配色，独立配置环与弦带半径，并使用 D3 二次曲线和自环几何。 | 原矩阵须为有限非负方阵。弦带兼容标量为双向合计，原始方向值与双行提示适配器单独可读。原生语义控件可选水平标签，极坐标标签可用字形轮廓。 |
| Bump | `bump_layout` 使用类型化类别和离散排名点轴，提供转折肩部、首尾延伸、空值槽、共享符号及自定义点工厂。 | 每个系列的类别键须唯一。显隐保留完整类别与排名格。自定义点的最近锚点使用原始坐标，精确拾取使用实际墨迹。开发版另有类型化实时点组件；自动 spring 继续保留。 |
| Stream | `stream_layout` 复用共享 `stack`，提供原始 nullable 区间、共享比例尺、Nivo 曲线、三种点位置和面积或采样绑定。`StreamDotOptions.glyph` 提供共享符号，`stream_layout_with_dot_factory` 提供原始采样、继承样式与自定义点几何。 | 采样切片可保留关闭显示的零尺寸点，缺失值仍为 `None`。面积绑定按整个系列提供摘要，采样绑定按画面顺序提供逐列数据。自定义点沿用真实墨迹拾取与原始锚点协议，开发版已提供类型化点组件；图层组件和自动弹簧仍待补。 |

`Chart::data_control_labels` 可按稳定标记 ID 设置可访问按钮文本，并对标签与场景更新共同做准入检查。`BrowserChart` 已用它为类型化缺失值提供显式文本。

本轮落实了这些不能混用的语义：point scale是零bandwidth、无padding的n>1端点对齐/n=1居中；数字/字符串保留原identity。round遵守JavaScript `Math.round`负半值，axis/data/site共用映射，invert仍连续。log base影响ticks/nice，SymLog在原值域生成linearish ticks。Filled size是面积，Stroked size是D3感知参数；Triangle/Star/Wye origin通过显式Path支持点绑定，不取BBox中心。Exact使用真实轮廓，零半径站点用于明确的无点/slice或真实零尺寸Scatter协议，Exact仍须遵循实际cap有无墨迹，不能把logical site冒充填充。这些已是runtime实现；下一步是其他图族接线，不能仍列作缺失核心算法。

## 后续任务与平台边界

- **A/B 图型接线**：继续补充更多类型化比例尺配置、点和端标签自动避碰、环形布局和引线优化，以及图型专属自定义几何和组件入口。Radar 的非线性比例尺是 Kartis 的可选扩展。
- **径向标签契约**：Pie/RadialBar 已新增 `ArcLabelRadiusMeasure::RingThickness` 匹配 Nivo 的 `skipRadius`，默认 OuterRadius 保留既有行为；Sunburst 原有厚度语义不变。两者提供真实 Path 与极坐标中点的类型化绑定，Radial 的布局函数保存私有源状态供校验，外部应通过布局函数构造。Sunburst 测量后避碰是扩展，Pie 的同侧引线消交叉仍待补，不能列作 Nivo 已有但尚未移植的自动避碰。
- **A/B 共享协议**：Line、Scatter、Bar、Combo、Radar 和 Swarm 已接入共享尺度，Scatter 另有双 Point 轴。Line、Radar、Geo、Scatter、Swarm 和 Network 已接入共享符号。混色提供跨后端绘制和显式浏览器动画，图型便利配置待补。
- **C 框架与平台**。继续验收屏幕阅读器、物理设备和其他桌面系统，并扩展竖排和字形渲染范围。浏览器与原生宿主使用各自已支持的绘制和输入协议。

### 地理能力的进一步核对

Kartis 的十种投影已覆盖本次 Nivo `GeoProjectionType` 的名称集合，不能再
简单列为“缺 Nivo 地图投影”。新增的等积、等距、保角圆锥及 Albers 已有正式
Rust 入口，[中英文指南](geo-conic.zh-CN.md) 说明拟合、有限纬度和反算边界；
该受检范围已完成 Rust、四引擎 DPR1/2、原生渲染和同轮 183/183 回归；具体证据与保留限制见验收记录。
仍有两类实质差距：

- 已有投影的受检范围：方位类限制在不超过 90° 的前侧帽，Gnomonic 严格
  小于 90°；Mercator 类保留显式纬度截断。任意球面拓扑、对跖/自交消歧、
  更大可见帽和全部 Nivo projection props 的实际对应仍需独立验收。
- D3 的更多地理基建：复合 AlbersUsa 地图／线网／符号及真实宿主配置已验收，保留数值和拓扑边界，见[指南](geo-albers-usa.zh-CN.md)。球面距离、长度、面积、圆生成、重心和包含已有 typed 公共入口，详见[球面指南](geo-spherical.zh-CN.md)；数值与稳定性对照、真实WASM及新画廊受检范围已有验收；显式System字体配合主线程Offscreen的受检profile已通过，默认Firefox Sans全帧仍保留失败。它们保留原 D3 有向积分、Polygon 流长度与边界判据，不能由平面 Scene 命中替代，也不计算任意拓扑的地理并集。

本轮已实现当前投影上下文的受检反算，并对可见范围和拟合进行独立对照。
新增圆锥投影的相邻标准纬线可能让原始差商受平台三角函数舍入影响；实际 WASM
反例已记录，并新增标准纬线正弦区间的系数准入规则。原始公式与常规数值门槛
保留，端点余量属于工程保护；该版本浏览器与原生的受检范围已完成整体验收。
复合地图必须保留各分区裁剪、身份和反查，不由几张任意缩放图片替代。
参考 [D3 projections](https://d3js.org/d3-geo/projection)、
[conic projections](https://d3js.org/d3-geo/conic)、
[spherical math](https://d3js.org/d3-geo/math) 与
[spherical shapes](https://d3js.org/d3-geo/shape)。

## 最后处理：GPU / worker

按用户指示暂缓。GPU/worker、百万点增量布局、浏览器 compositor 端到端 FPS、完整活分配/显存归因都不能从已有 CPU/高密度/内存报告推导。它们也不是本地 Nivo 0.99 的所有图型通用承诺。先完成上述纯 Rust 图型与数据契约任务，再单独设计 GPU backend/worker 消息、取消/过期结果、预算及资源生命周期验收。

### 新一轮已核实的配置差距

RadialBar逐分类继承标签色、Sunburst构建期类型化文字，以及BoxPlot显式分类带均已实现和验收，见[标签与带刻度配置](chart-labels-and-bands.zh-CN.md)。它们不再列为待办。

Bump两端样式、Radar声明式点继承色、BoxPlot独立auto/explicit端点均已正式实现并验收，不再列为缺口；见[端点、点颜色与上下界指南](endpoint-point-bounds.zh-CN.md)。Radar点工厂和Box分类带此前已有，未重复计作新增。

Parallel分类轴属于Rust扩展，原Nivo变量同样是数值轴，不能记作Nivo功能缺失。空/全缺失颜色数据下Log或负指数域的自动非零回退也属于Rust扩展稳健性项：现限制仍须显式合法域，不将Nivo线性颜色配置冒称为该非线性参考。

### Bar分类图例与系列顺序

已实现 `BarLegendSource::{Keys,Indexes}` 和 `BarStyleOptions.series_order`。缺省仍为旧的按系列图例和词法顺序；显式顺序必须完整覆盖源系列，并一致决定分组/堆叠、palette槽及图例。分类图例来自首个可见有序系列的原始柱候选，包括None与零，不从过滤后的可见墨迹反推；None仍不创建mark/site。横向反转一次，分类窗口不截断源图例，隐藏系列不挤占palette身份。

共享 `Scene.legend_roles` 区分Series与Category：同名分类不自动隐藏系列。Chart、Browser SVG/Canvas与RasterSurface默认分类展示为静态内容；显式配置回调/legend_actions后提供原始ID的类型化激活意图。Series维持切换且先通知意图，再按当前接受帧复核后提议隐藏。角色切换、测量/自定义项和释放均进入本轮真实宿主验收；最终结果见verification。用法见[汇总域与分类图例](summary-index-legend.zh-CN.md)。

### BoxPlot自动域统计来源

已新增显式 `BoxPlotAutoDomainSource::SummaryQuantiles`，旧SampleExtent默认保持。样本[1,2,3,4]默认五分位数给出[1.3,3.7]，单侧fixed也使用选中的auto来源。新模式采用原Nivo有限插值表达式和稳定数值排序（含正负零相等的输入顺序）；仅原表达式中间溢出时使用Rust有限回退。UTC只对推断域端点做Date TimeClip，统计量保持连续；完整显式域绕过推断，但仍使用选中的统计计算模式。

已保存原函数1,046组输入参考与窄UTC域/正负零的真实修复前失败。原Nivo时间工厂对[0,0]的falsy回退保留为参考差异，Rust不复制该缺陷。最终合并的严格检查、全量回归与指南执行结果见verification；不是所有React props等价的承诺。

### 普通边框继承色与宿主修复

已实现BarOptions与HeatmapLayoutOptions的可选border_color_profile，保留未配置边框、原width/cap及预工厂numeric palette。Bar覆盖classic/styled及文字工厂stroke上下文，HeatMap保留显式NULL/零/opacity与颜色尺度。原Nivo完整resolver和实际Bar/HeatMap调用链已有原函数参考。128组默认场景的布局、SVG与工厂上下文保持；最终Canvas/CPU各112组完全相同，16组为另行核验的Normal整体alpha修复，不能泛称全部后端输出不变。语义Rect标签裁剪也已修复。

这批颜色角色、透明度合成与Rect标签能力已通过正式回归、SVGImage/Canvas与Rect裁剪专项，以及四引擎×DPR1/2的四宿主联合终验和独立原始图像复核。静态同源物理截图的抗锯齿差异作为诊断保留，完整DOM/CSS/接受状态及原生绘图资源仍严格比较，不宣称严格compositor通过。最终结果见verification。本轮原源参考不声称原Nivo所有后端等价。HeatMap标签颜色、Bar标签和Pie四个InheritedColor角色此前已有，未重复计新增。GPU/worker继续暂缓。

RadialBar扇段边框及BoxPlot的border/median/whisker继承色已实现并完成正式验收。原Nivo分别在ArcsLayer与BoxPlot hooks调用继承色生成器，Rust复用共享resolver并保留None默认固定stroke。Radial按解析后alpha决定styled收缩弧是否产生描边，Box须与端帽共用解析结果且不乘箱体fill_opacity；另修复BoxPlot可选箱体/离群点零宽边框先通过样式校验、后被Scene拒绝的问题。128组默认输出对照、288对整幅SVGImage/Canvas、最新四引擎九场景四宿主、完整原函数参考和全仓检查已通过，范围与物理截图诊断边界见verification。用法见[描边颜色指南](radial-box-strokes.zh-CN.md)。现有RadialBar标签和Funnel颜色角色不重复计新增。

BoxPlot已支持显式Group/SubGroup取色与完整域palette身份；Group图例按实际有效摘要资格生成，使用Category角色，保留原子组series/hidden身份。新增32配置／10边界的完整原hooks/getLegendData/transition实际参考，独立记录原数字图例ID、Nullable数值强制转换和SubGroup旧完整图例域的差异。受控BoxPlotInteraction可按同group或同subgroup关联，真实组合透明度在共享渲染器中合成一次，核心／默认输出／原生导出以及四类保留宿主的完整动态矩阵和独立审计均已完成。用法见[配色与悬停指南](boxplot-color-and-hover.zh-CN.md)。

该受控API不等于复制原Nivo整份g的DOM指针事件：controller.hit_test/key_for_mark可以识别所有实际部件，应用可显式路由；旧Browser/Raster绑定保留每箱一个body逻辑站点；新增 `whole_parts_scene_interaction()` 显式绑定把真实装饰部件拾取重定向到该逻辑站点，同时保留物理ID与原始outlier快照，已通过四宿主完整第三轮和独立验收。新增借用String图例工厂与显式ByLabel按文字去重模型已正式验收，保留首个完整代表及类型化Series／Category目标；不提供任意JS getter或React生命周期等价。借用的 typed Color 工厂提供实际摘要、原始观测索引和完整域身份，正文、继承角色及首代表图例共享解析颜色；悬停复用缓存，显式源替换重供工厂。它不自动保留任意 RawDatum 字段，也不复制 JS getter 或 React 生命周期。`legend_labels` 已提供按原身份的有界文字映射，核心、默认差分、原函数参考、双语程序及256宿主／1024阶段的四引擎DPR1/2真实图例动态矩阵和独立审计均通过；它保留重复／空文字的独立ID，不能冒称完整原按label去重模型。语义分组拓扑切换可以重建内部节点，弹簧／属性拓扑动画及Canvas/CPU嵌套隔离层也仍受明确限制。RadialBar的typed隐藏保留原槽位与累计值，不能冒充上游删除输入后的重新布局。GPU/worker继续暂缓。

逐摘要图例工厂的完整范围见[中文指南](boxplot-legend-factory.zh-CN.md)及[English guide](boxplot-legend-factory.md)。原Keys文字映射仍保留重复／空文字的独立行；ByLabel显式选择首代表去重，不把多个canonical目标并集。静态导出仍只携带图例元数据；更完整静态图例呈现属于后续共享配置范围。原版BoxPlot的colorBy公开类型仅Group／SubGroup，两者已覆盖；源颜色函数的 typed 摘要与调用者闭包适配见[颜色工厂指南](boxplot-color-factory.zh-CN.md)；任意 RawDatum 属性自动保留仍不属于此契约。

### FullYears 月份间距与 Radar 点标签

`CalendarLayoutOptions::month_spacing` 为 FullYears 的日期、月轮廓、bounds 和标签提供同源横纵偏移，默认零保持旧输出；Compact 拒绝正间距。原版 Sunday-Jan1 额外预留周与本库实际占用周列的差异仍明确保留，不冒全局坐标等价。见[月份间距指南](calendar-month-spacing.zh-CN.md)。

`RadarPointLabelContext` 及借用文字工厂提供稳定轴／系列身份、原始值和格式化值，可与点几何及共享尺度组合；只替换 owned label，保留 tooltip、逻辑锚点和已有调用顺序。回调不保留在布局中；任意 React 组件、自动文字避碰仍是后续范围。见[点标签指南](radar-point-labels.zh-CN.md)。

## 开发版增量：阶段几何与平行坐标筛选

这些新增 API 尚未包含在已经发布的 `kartis 0.1.0` 包中。

- `funnel_layout_with_stage_factory` 提供借用的原始 datum、源索引、计算阶段及继承画笔。工厂替换单个 body，显式控制原侧边框；偏移 Path 的精确拾取跟随墨迹，最近点仍使用原始中心。默认继承保留原 Scene、代表色及绑定。不是任意 Nivo React component 或实时 Widget 的移植。见[中文指南](funnel-custom.zh-CN.md)。
- `ParallelFilterController` 将多个闭合原始值区间按 AND 组合，并提供实际尺度的像素/Brush 转换。筛选保持完整源的尺度、配色、组图例和源索引，剔除行的几何、所属标签与拾取站点；坏筛选和坏数据替换原子保旧。这是可对接 D3 Brush 的 Rust 扩展；原 Nivo Parallel 未提供同名筛选 API。实时浏览器/原生手势桥与分类变量不计为本轮完成。见[中文指南](parallel-filtering.zh-CN.md)。

专项验收及边界见[本轮记录](verification.md#development-funnel-and-parallel-filtering)。GPU/worker 仍暂缓，其余差距保持。

## 开发版增量：真实测量自动折行

`wrap_label` / `wrap_scene_label` 根据借用真实测量器提供的完整候选 advance 自动选择 Words 或扩展字素断点；支持显式换行、长词策略、省略与工作预算。Scene 替换同时重映射字体、归属、clip 与 draw order，失败保旧；在复制重复 clip 前检查最终成本。`FontRegistry` 包装使用真实字体字形与家族/字重选择。普通 Scene 行可由原渲染器绘制；同字体原生 shaping 的路径副本用于确定的 SVG/Canvas/CPU 输出。

这是尚未发布的 Rust 扩展，不是 Nivo 原有自动折行 prop。宽度契约是 advance，非任意旋转墨迹 AABB。显式 `LabelWhitespace::Preserve` 已统一单行空格保留及控制字符归一，Scene 折行自动设置此模式；见[空白协议](text-whitespace.zh-CN.md)。浏览器字体量化/kerning 可与 CPU advance 不同：独立精确 advance 探测在 Firefox 保留失败，不能借路径专项声明原生字体完全等价。完整 CSS/UAX14、段落级 bidi 后换行、自动竖排仍属未完目标。见[中文指南](text-wrapping.zh-CN.md)。


## 开发版增量：Waffle 单元格与 Calendar 年份标签

这些 API 尚未包含在已发布 `0.1.0` 中。

- `waffle_layout_with_cell_factory` 补齐有类型的逐格几何/画笔扩展，包含真实空格。默认继承保持完整布局相同；偏移 Path 的 Exact 跟随墨迹，Nearest 与整格查询保留原槽。单格画笔不改组与图例源色，空格不生成数据站点。Kartis Areas 是实心组区域，只有实际 empty cell marks 调工厂；Nivo areas 是透明交互层，两者不冒称等价。单格实时组件已由 `waffle_cell_widgets` 提供；组区域工厂与区域实时组件仍待补。见[几何指南](waffle-custom.zh-CN.md)及[控件指南](cell-widgets.zh-CN.md)。
- `calendar_layout_with_year_legend` 支持 FullYears 的 Before/After、有限正负 offset 及借用年份文字工厂，并可组合共享颜色 profile。默认旧入口、年面板、日期、月份和 `Scene.legend` 不变；Compact 明确拒绝。原 Nivo 四种定位公式相对本库实际 panel bounds 核对，原占用周差异、header 预留和 rotated size/3 alphabetic 基线仍保留，不能声称完整 Nivo 坐标或字体像素等价。见[指南](calendar-year-legend.zh-CN.md)。

四浏览器导出专项与完整回归记录见[验收](verification.md#development-waffle-cells-and-calendar-year-labels)。GPU/worker 继续暂缓，较宽宿主、字体与图型差距仍保留。


## 开发版增量：Bullet 三类部件工厂

`bullet_layout_with_part_factory` 及共享值轴组合入口提供 Range/Measure/Marker 的借用几何与画笔回调。上下文区分原始行索引、可见行位置、原 source_index 与含省略项的排序 rank；实际绘制的合成年域尾 Range 也回调。隐藏、裁剪或零长度而未绘制的项不回调。所有 row/part 上下文保留原布局快照，接受后同步实际 mark 与 `part.geometry`；原始值、区间、身份、轨道 clip 及原行图例不变。

新包装保留旧 `BulletLayout` 字段，提供显式原部件中心站点。Legacy Geometry/Nearest 的 SceneInteraction 与 Glory `chart_layout` 所需的 PaintedGeometry 分开转换；后者先预检实色墨迹，拒绝不支持的渐变/图案及超限几何，仍受语义宿主的 Graphic/Text 限制。单个自定义部件并非任意 React 组件或 animatedProps 生命周期。原 Nivo 删除零端点并按过滤排序编号，本库继续保留零/负值及原始索引，不能冒称数据模型等价。用法与范围见[中文指南](bullet-custom.zh-CN.md)和[验收](verification.md#development-bullet-part-factories)。

本功能尚未包含在已发布 0.1.0 中。GPU/worker 继续暂缓，其余差距保持。


### 开发版已实现：Bump 两端独立文字

Nivo `bump/src/bump/types.ts` 的 `startLabel` / `endLabel` 公开独立系列 accessor。开发版 `bump_layout_with_label_factory` 及 `bump_layout_with_factories` 已提供 Start/End 类型化文字，并可组合已有点几何。上下文借用原系列、完整 computed 行、源索引、原文字及原线点／最终文字位置；只替换 actual owned endpoint Label 的文字，保留系列、图例、tooltip、归属和拾取身份。

隐藏、禁用、缺失端不调用；空串及零墨迹 owned slot 保留，返回文字按 1,024 UTF-8 字节限制。Rust 的逐系列 point→Start→End 调用次序与原 Nivo 按位置 hook、在排除 missing 前调用 accessor 不同，不冒称 React 副作用顺序或任意组件等价。原入口和旧 Options 字段保持。指南见[中文](bump-endpoint-labels.zh-CN.md)／[English](bump-endpoint-labels.md)，具体受检范围见[验收](verification.md#development-bump-endpoint-text)。此项未包含在已发布 0.1.0 中；自动避碰、更多布局和宿主差距仍保留。


## 已实现并专项验收：Albers USA 有界复合投影

已落实点／区知逆算、矩形 postclip、专用 World／Data 地图与跨分区／拟合轮次共享预算，
保留单一源 feature 与颜色域。线路／经纬网按连续 child 分流，完整墨迹取分区／plot 矩形
交集；多个真实分区 part 共享一个 canonical 业务控件。symbol／glyph／地点标签也按所选
分区裁剪，默认矩形使用 Glory 可接受的原生 `Mark.clip`，不退回关闭投影裁剪的弱配置。

真实 Memory、Raster 与四浏览器的 SVG／Canvas 配置已验收，包含原始 nullable 数据、
metadata-only tooltip 更新、主动拒绝事务、可编辑文字、真实指针／键盘／缩放／平移与区知
逆算。Lower48 大窗覆盖 inset 的问题由逐区矩形隔离处理，不使用 union 替代；默认复合
逆算的源点歧义保留 region provenance，known-region 入口可选回原组件。旧通用线流签名
仍拒绝复合 context，专用 `add_usa_lines`／`add_usa_graticule` 保留完整分区与归属数据。

原版 D3 跨插图 composite 在六例中交错 `lineStart`，参考完整保留，不把压平事件冒称无桥
几何。三个约 `1e-13` 像素的退化角接触差异也保留为未逐位对齐的数值边界；有效组件和双向
线段距离单独验收。更广球面拓扑、其他表中差距与 GPU／worker 仍保留。新 API 均属
Unreleased，未改旧投影枚举／Options 字段或已发布 0.1.0。详见[中文指南](geo-albers-usa.zh-CN.md)／[English](geo-albers-usa.md)
和[新验收](verification.md#development-albers-usa-streams-and-host-integration)。

## 开发版已实现：Swarm 节点与 Network 连线自定义绘制

`swarm_layout_with_node_factory` 在原碰撞布局后借用来源与站点，保留值坐标、组域、
原始碰撞圆和最近点锚点；Exact 使用新墨迹。隐藏／裁出不回调，原零尺寸及透明资格
保留。默认继承保持完整旧 Scene／metadata／binding；另有显式 PaintedGeometry 宿主入口。
Memory 和真实 Raster 发布回滚、SVG／Canvas／CPU、四浏览器均有专项证据。

`network_glyph_layout_with_link_factory` 提供当前边和端点状态，可组合节点 glyph；
显式 relation 变体保持图归属和独立边 ID。`BrowserNetwork::mount_with_link_factory`
在当前 tick、拖动、resize、源替换上重建连线，并共同接纳 Scene 与节点绑定。
普通连线仍为装饰，不产生假的节点控制或拖动目标。回调内部停止及预检内部停止已独立
复核，无借用崩溃且工厂只释放一次。

原 Nivo 的 Swarm SVG `circleComponent`／Canvas `renderCircle`，以及 Network
`linkComponent`／`renderLink` 公开能力已按共享受检几何角色接线。完整上游源码／MIT、
真实 Canvas 原函数参考及边界见[验收](verification.md#development-custom-swarm-nodes-and-network-links)。
这不复制 React／任意 Glory Widget 生命周期、原始 JS getter 或不同力算法的最终位置；
Funnel 的上游扩展入口是 custom layers，不能虚构为独立 partComponent。


## 开发版严格 Widget 组合

`PlotWidgetsComposition<T>` 和 `Chart::plot_widgets_composed` 为语义与 CPU 图像图表提供隐藏候选预挂载；BrowserChart 提供对应的 composed mount/set 接口。factory/build、源与测量校验或发布失败时，保留旧图、旧组件及其已接受数据。成功后才共同提交；仅替换集合变化也触发重绘。原 overlay 的局部修订失败协议保持独立。

`replace_ink` 只能包含真实插槽对应的 physical Mark ID。宿主建立独立绘制投影和物理拾取排除，authored Scene、所有标签、轴、标题和 canonical 逻辑数据控件保持；其他可见 alias 与显式选择／crosshair 不冒充已删除。0 透明度组使插槽隐藏、inert 并保状态；正透明度的具体 Widget 样式由组件负责。暂存新子组件累计有 256 上限，提交或放弃释放预算。

顶部层顺序仍不同于 Nivo Bullet 的 ranges→measures→axis→markers→title，任意控件内容的 Scene 导出、原生宿主图层交错、自动 spring 与完整组混色继承仍需继续实现；浏览器另有下节的 Scene 顺序组合入口。已发布 0.1.0 不包含此 API。见[双语指南](widget-composition.zh-CN.md)及[验收](verification.md#development-transactional-widget-composition)。


## 开发中：浏览器按 Scene 顺序嵌入 Widget

新增独立 `mount_with_plot_widgets_in_scene` 和对应 packet/layout setter。
共享 `widget_paint_plan` 保留原始 Mark/Label 顺序，支持透明绘制段、替换位置及整组一次性透明度。
SVG/Canvas 浏览器将真实组件放入同一分层树，跨组移动保留组件身份；非 Normal 源墨迹通过 CSS 与前方组件混色。
原生浏览器命中伴随层取实际 Canvas alpha 大于零的像素，覆盖区域阻止后方按钮的点击和 hover，孔洞及透明区允许穿透。
SVG 也采用该设备像素量化命中 profile，不承诺 SVG 亚像素边缘完全一致。

现有顶部层 API 保持其原有语义。原生宿主同层输入遮挡、任意组件导出、自动 spring/animatedProps
仍为未完成项；GPU/worker 继续后置。此开发 API 不包含在已发布的 0.1.0 中。
见[中英文指南](ordered-widgets.zh-CN.md)。


## 开发版：HeatMap 与 Waffle 实时单元格组件

`heatmap_cell_widgets` 与 `waffle_cell_widgets` 为原 Nivo `cellComponent` 角色构建
测量插槽及拥有所有权的类型化数据，可用于覆盖层、严格替换及浏览器 Scene 顺序组合。
HeatMap 保留原始 `HeatCell`、私有原输入索引、None/Some(0) 和当前计算单元格；稀疏对不补造。
Waffle 保留物理格位及可选原分组，空格没有虚构零值 group；同组选中格共享 Arc 快照。
完整来源/几何/分配校验先于任何回调，现有 custom geometry 与实际 Mark 继续提供当前画笔。

显式类型化包可以绑定空标签的真实装饰 Mark，原无类型入口规则不变，不因此生成源业务控制
或拾取站点。Waffle `Areas` 中占用格没有单独 cell Mark，只为真实空格建立组件；
组区域 Widget 工厂仍未完成；后续透明区域事件角色已通过显式绑定接入，见下文。
HeatMap 的默认公开外观编辑约定继续保留，
自定义工厂来源则使用原严格保护。语义 `chart_layout` 使用显式实色 painted binding 转换。

CPU 图像 Chart 新增 `chart_layout(frame).raster(...)` 显式 PaintedGeometry 绑定，
可与实时组件共同受 PNG 发布闸门约束。物理 View 接收指针，按已接受图像相机逆算一次；
保留源空值、物理别名与控制身份，详见[类型化图像输入](raster-chart-bindings.zh-CN.md)。
仅传 Scene 的旧入口仍没有类型化绑定，不可把 HeatMap 空值标量兼容零当成原值。
RasterSurface 保留独立接口，未新增实时组件入口。
自动 spring、animatedProps、任意实时组件导出与原生同层输入仍未完成。GPU/worker 继续后置。
参见[中英文指南](cell-widgets.zh-CN.md)。已发布 0.1.0 不包含这些 API。

## 开发版：Stream dotComponent 角色

`stream_dot_widgets` 为实际启用且非空的点生成类型化组件包，保留共享完整原系列、
源图层颜色、原索引、分类身份、原始值与格式化文字、堆叠区间和配置后的投影位置。
零值、零尺寸及自定义零透明度仍有点角色；空值和禁用点保留 slice 占位，不生成假组件。
完整私有来源、尺度配置及实际标记检查先于任何定位回调。

绑定 Chart 的 `AtDatum` 已接上类型化 Path 锚点，CPU 图像先解析逻辑来源再应用相机。
自定义偏移墨迹不改变组件原始定位。原版 Nivo 数值数据及 dotComponent 调用已用冻结
完整源码执行核对，不把 Rust 可空源、稳定业务 ID 或 Glory 生命周期冒充 React 等价。
自动 spring、animatedProps、其它角色组件及任意 Widget 导出继续保留；GPU/worker 后置。
参见[中文指南](stream-widgets.zh-CN.md)与[完整程序](../examples/stream_widgets.rs)。


## 开发版：Waffle 分组事件区域

`waffle_area_interaction` 为每个实际占用组建立完整格位区域，覆盖 padding 间隙；
Cells 保留首真实 cell 的 canonical 站点，其它 cell 成为物理别名，Areas 使用已有
实心并集站点。每组只有一个业务控制，空格、零分配和容量耗尽组不造数据。
Scene/SVG 绘制逐值保留；透明区域选择与真实物理命中分别报告。

公共 Surface、Glory 语义 Chart、CPU 图像 Chart 及浏览器 SVG/Canvas 共用类型化绑定。
区域受已接受相机、plot 和隐藏状态约束，无效候选保持旧帧。共享边采用半开区间，
重叠区域按逆声明顺序优先，不宣称与 Nivo inclusive-first 边界逐点等价。
区域坐标不自动跟随 spring；组 Widget 工厂、任意实时组件导出与原生同层输入仍保留。
详见[中文指南](waffle-regions.zh-CN.md)。GPU/worker 继续后置，0.1.0 不包含该 API。


## 开发版：Scatter nodeComponent 角色

`scatter_node_widgets` 以当前 `ScatterLayout` 和显式原 `ScatterSeries` 构建拥有所有权的
类型化组件包，不改变公开布局结构。每个选中系列只克隆一次并共享 Arc，保留完整原 datum、
Number/String XY、原数值符号位、索引、格式化结果及计算站点；真实 Mark 单独提供当前画笔。

整帧 raw/metadata/axis/site/mark/anchor 一致性预检先于任何定位回调。零尺寸、零透明度及
plot 外实际节点仍有角色；hidden、missing-size Skip、Point 域省略不造节点。
允许完全一致的公开帧编辑，不宣称能恢复未保存的原尺寸 accessor 或格式配置。

原 Nivo Nodes/Node/computePoints 的真实创建参数与零尺寸、交互处理器门控已保存完整源码并
可离线冻结核验，不冒称 React transition 运行时等价。语义 Chart、CPU 图像及浏览器
SVG/Canvas 使用已有事务组合协议；自动 spring、任意组件导出和原生同层输入仍保留。
参见[中文指南](scatter-widgets.zh-CN.md)与[完整应用](../examples/scatter_widgets.rs)。
GPU/worker 继续后置；0.1.0 不包含这些 API。


## 开发版：Bar barComponent 角色

`bar_widgets` 从 `CustomBarLayout` 构建实时类型化包，默认 `BarFactories` 继承普通 Bar
绘制，配置尺度与几何/标签工厂入口共用。私有来源保留完整可空系列、分类及原始矩形、
端点和位置；raw、displayed、interval 与当前实际画笔分别提供。标签和裁剪元数据可供
子组件读取，空文字及受控 dim 颜色保留，原 Scene 标签/总计继续独立拥有和绘制。

完整私有 accepted/source/binding 预检先于回调；None/hidden/cropped 不补造角色，
真实 Some(0) 和零透明度柱仍有角色。`bar_widget_frame` 原子提供 Scene、类型化 Path
锚点和准确 plot，空布局也不猜测 plot。默认公开 StyledBarLayout 结构保持不变。

Nivo 的实际 barsWithValue、组件图层门控、公共参数、四阶段 style 及矩形表达式已冻结
核验，不宣称 ReactSpring 动态、Rust 裁剪/标签资格与 Nivo 完全一致。
浏览器纯色 Button 样本用于角色、数据、布局、来源恢复及生命周期验收；原 font8 同色文字
在特定 Canvas compositor 下的颜色差仍单列未通过诊断，不算严格文字绘制通过。
详见[中文指南](bar-widgets.zh-CN.md)与[完整应用](../examples/bar_widgets.rs)。
自动 spring、任意组件导出、原生同层输入及 GPU/worker 继续保留；0.1.0 不包含这些 API。

## 开发版：截图坐标与文字合成诊断

已实现独立整视口截图采样：真实 DOM 矩形、分轴缩放、DPR 和单次物理 floor，截图前后
几何保护与越界拒绝。四浏览器 DPR1/2、48 帧、80 项硬颜色采样、24 项独立原生点标记、
120 项拒绝及四项变异负控通过；Chromium CI 子集已接通。旧元素截图补齐造成的邻行
误采样有独立反例，旧报告保持原范围，不倒推重分类。

原 Bar font8 G98 差异在正确坐标下仍存在；无 Glory/WASM、透明 Canvas 激活的原生
载体可复现，诊断用非 LCD 文字配置匹配局部字形。尚未实现生产修复，也不承诺任意
文字跨合成路径逐像素等价。Firefox 分数滚动后的 fixed-Canvas 编码颜色保留非门禁
诊断；独立点标记索引与宽色块仍严格核验。详见[采样指南](viewport-sampling.zh-CN.md)。
这些开发版工具不属于已发布的 0.1.0；GPU/worker 继续后置。

## 开发版：Bump pointComponent 角色

`bump_point_widgets` 将实际 Bump 点接入类型化组件包，每个选中系列共享一次 owned 原数据，
保留原源索引、Number/String 类别、rank 符号位、计算位置与点代表色。完整源/计算数据、
PointScale 域/range、折线分段、真实 Mark/site/anchor 预检先于首次定位回调。公开布局
结构不改变，自洽公开编辑可以接受，不冒称能恢复未保存的原配置或工厂。

真实零尺寸和零透明度点仍有角色，hidden/gap 不造组件。`bump_widget_frame` 提供准确
plot 及显式实际绘制拾取；偏移 Path 的组件位置仍用原始数据锚点。默认首尾标签与折线
保持 Scene 所有权。上游冻结参考执行实际 compute、hook/style、组件创建和 spring 配方，
不把这些表达式核验说成 ReactSpring 动态运行时。Rust 稳定类别 ID、组件自身输入与受控
opacity 的语义分别说明，详见[中文指南](bump-widgets.zh-CN.md)。自动 spring、任意组件
导出、原生同层输入与 GPU/worker 继续保留；0.1.0 不包含这些新 API。

## 开发版：层次图实时节点角色

`hierarchy_node_widgets` 为 Treemap、CirclePacking、Icicle 的实际 Mark 提供共享类型化
组件协议，保留原扁平树、原索引/权重与聚合值、节点、图族、焦点祖先和当前实际标签/clip。
私有原树及角色资格、完整拓扑/聚合/索引/几何/标签所有权预检先于回调；原树包内 Arc 共享。

`hierarchy_widget_frame` 保留准确 plot，包含透明/无画笔实际角色的类型化站点，物理拾取
仍仅依据实际墨迹。隐藏、零权重、焦点外、父节点/焦点配置省略不造角色。旧布局的输入
顺序无关相等语义保留，组件数据独立识别原权重 -0/+0 变化；画面相同也必须刷新原数据。

上游三家族的创建参数、transition、handler/label 表达式分别冻结核验，不冒称 React/
Spring 或 D3 布局运行时。Nivo 零面积/不可见节点仍可能创建组件，嵌套原数据、路径 ID、
非零父自重及标签所有权与 Rust 规则不同，明确保留差异。Motion target_layout 可用，
自动跟随采样 Scene 和退出动画尚未接线。详见[中文指南](hierarchy-widgets.zh-CN.md)。
这批 API 不含在 0.1.0；任意组件导出、原生同层输入与 GPU/worker 继续保留。

## 开发版：原始值变化独立于数值几何

Bump、Scatter、Bar、Stream、Heatmap 和 Bullet 类型化组件数据比较保留原始有符号零；
共享系列中的未选中原数据也能触发所选组件刷新。Bullet 同时保留当前原始区间端点。
Stream 数值类别及 Heatmap 原值篡改在回调前拒绝。业务键、Scene、比例尺及几何
继续使用原有数值相等规则。见[配对指南](widget-source-values.zh-CN.md)。

功能验收独立检查真实状态/DOM、SVG 计算样式、Canvas backing、来源 bits 和前景色；
此前整页 PNG 严格门禁仍未通过。无脚本、无 WASM、无候选更新的原生载体也可复现
一个 SVG 抗锯齿像素变化，整数对齐未消除。此诊断和既有 Bar font8 失败保留。
这些改动不属于 0.1.0；自动采样动画、任意组件导出、原生同层输入及 GPU/worker 后置。

## 开发版：Swarm 实时节点角色

`swarm_node_widgets` 为实际打包站点提供共享 owned 原输入、原/打包索引、当前分组、
碰撞圆盘、plot 与值尺度，`swarm_widget_frame` 提供显式 PaintedGeometry。完整来源、
分组、元数据、打包次序、当前尺度投影、Mark/anchor 一致性先于第一次定位回调。
隐藏/cull 不造角色；实际 size0、透明/无画笔角色保留，由宿主决定可见和物理输入。
来源 value/size 的有符号零独立触发数据更新；自洽公开编辑仍合法，不虚构原配置快照。

对应原版 circleComponent 的节点/style/handler 创建配方；原 Canvas renderCircle 另行
核对。Rust 单轴确定性打包不冒充 force 数值结果，显式组件事件不冒充 React/Spring
运行时；自动采样/退出组件、任意组件导出及原生同层输入仍未完成。见[中文指南](swarm-widgets.zh-CN.md)。
GPU/worker 后置，这批 API 不包含在已发布的 0.1.0 中。

## 开发版：Network 实时节点与 force 宿主

`network_node_widgets` 提供共享完整当前节点状态、源索引、视觉 site/物理半径、world 偏移、
ticks/alpha；空标签、零视觉尺寸和无墨迹角色可显式附着，不增加假的 canonical 来源站点。
`AtDatum` 按私有保存的真实 glyph 原点解析为 Rect，无须猜 Path bounds 中心。

BrowserNetwork 新入口把 force tick、真实未替换节点拖动、resize、图替换、revision/reheat
与严格节点组件共同准入，并可组合连线几何工厂。正向 CSS scale 保持逻辑 world 单位，
实际 client bounds 只作映射；外部 CSS 宽度本身不回滚。采用后的保留 Bond 与 owner snapshot
共享接受时点，预绘制失败保旧。纯事件空闲 RAF 不反复提交同 Scene 触发 hover token 环。

这对应 nodeComponent 角色，不冒称任意原 RawDatum、D3 force 轨迹或 React/Spring 运行时。
实时 linkComponent 已由后续开发版补齐受检范围；自动退出节点组件、任意 Widget 导出和原生同层输入仍待补，GPU/worker
继续后置。见[中文指南](network-widgets.zh-CN.md)；这批 API 不含在已发布 0.1.0 中。

## 开发版：Network 实时连线与混合角色

`network_link_widgets` 与 `network_widgets` 已提供完整当前节点/边快照、原始位、真实端点中点，
并在 BrowserNetwork 单个严格阶段共同接受 force/Scene/两类组件。来源顺序的遮挡与可信输入、
实际 tick/拖动/缩放/resize/换图、原点和自定义 Path bounds 分离、空源/revision 已完成专项。
边仍使用 force-link:index；重排保留索引本地状态，业务换边可用 revision 重建，不推断业务 ID。
显式任意业务边 key、自动退出组件、任意 Widget 导出、原生同层输入及 GPU/worker 仍待补。
详见[中文指南](network-link-widgets.zh-CN.md)与[验收记录](verification.md#development-live-network-link-and-mixed-node-components)。

## 开发版：Funnel 函数配置与实时层角色

`FunnelFactories` 补齐原始 datum 颜色、值格式化及已计算阶段的 border/label 颜色函数，
可与阶段几何工厂组合。显式标签色不会因自定义 body 再调用；旧入口的未使用 Contrast
边框仍保留原有准入条件。Rust 先批量计算尺寸，样式上下文含已计算 profile，不能冒称
原版逐 datum 回调调度或任意泛型 RawDatum 完全相同。

`funnel_widgets` 为实际 body、border、before/after separator Mark 提供全源输入、
计算阶段、分隔线与 owned 标签/clip/归属；真实零像素缺席 body 不造角色，尾分隔线
没有虚构 datum。原点由阶段中心或分隔线中点确定，自定义路径 bounds 不改它。
预检保证所供 source、公开 layout 与实际 Mark 自洽，不伪称不可修改的构造配置快照。

上游入口是 function layers，没有 partComponent、labelComponent 或 separatorComponent。
这里的有类型实时角色是 Glory 扩展，本身不能替代任意 layer 插入顺序；新独立
`BrowserFunnel` 的字面 SVG 整层能力见下节。React/Spring 调度、自动退出、任意 Widget
导出和更广原生同层输入仍保留。原始 profile 的独立几何工具要求
显式插值模式，结果可不同于自定义 body 的当前墨迹。见[中文指南](funnel-widgets.zh-CN.md)。
GPU/worker 继续后置，这批功能不含在已发布 0.1.0 中。

## 开发版：Funnel 字面 SVG 函数层（本轮范围已验收）

`FunnelSvgFrame` 通过真实构建保存 config/source/options、完整 parts/separators、
构造期样式结果与检查后的主体／边框／分隔线／标签映射，不根据任意 layout 猜配置，
也不重放函数。没有墨迹的角色仍保留显式函数结果；旧禁用标签的默认 `None` 和未使用
边框的颜色回退保持旧准入规则。原始四点 profile 与实际自定义／编辑 Mark 分开读取。

`FunnelSvgPlan` 和独立 `BrowserFunnel` 已实现 Separators／Parts／Labels／SceneExtras／
Custom 的真实字面顺序、省略与重复。不同 occurrence 引用同一来源，不伪造 Mark 或
业务站点；重复层有独立资源空间。Glory 整层子树在真实 SVG `foreignObject` 中保留，
稳定 key／registry／revision 保留本地状态；来源和各 runtime 的整帧 Cage 在 pump 前
共同采用。program／view／hidden 更新不重放来源函数，弱且受 Scope 约束的 current
能力排队真实来源更新。原生主体由实际 SVG `visiblePainted` 路径命中，不以纯色 alpha
非零近似替代，原通用 Scene／SVG／Canvas API 均保留。

这些 Unreleased 能力已通过五种浏览器配置 × 两种 DPR 的 40 组验收：1,610 帧、560 次
拒绝恢复和 20 个真实绘制／位姿破坏负控。WebKit 停留替换路径后可能不投递离窗事件，
独立普通 SVG 控制已复现；实际离开验收先真实移动到已接纳的新主体，此未投递边界仍保留。
失败 UI 永久隔离到 revision 重建或移除重挂，不以 pump 返回 Ok 自动恢复。
后续四个公开原生 part mouse hook、标准与保留式 Glory tooltip 已通过独立正式矩阵：
10 配置／40 profile、2,000 帧、160 次拒绝及 20 个真实 DOM 负控，具体软件配置与边界见
[事件与提示验收](verification.md#development-funnel-native-events-and-tooltips)。
绑定 annotations、泛型原始业务字段和逐属性 Spring 已有后文的独立实现；上游普通
keyed map 没有额外退出过渡。任意可变 D3 area/line 对象、builtin 内 Source Widgets
混排与任意 Glory UI 导出继续单列。
`SceneExtras` 是作者装饰，不冒充 annotations。GPU／worker 继续后置。
用法及错误阶段边界见[中文指南](funnel-svg-layers.zh-CN.md)与[English guide](funnel-svg-layers.md)。

## 开发版：Tree 节点与连线实时组件

`tree_layout` 在旧算法的同一次计算中捕获完整原始扁平输入、节点拓扑、深度/高度、
原权重/聚合权重、真实中心及连线端点/中点；旧 `tree_with_options` 仍返回同一 Scene。
`tree_node_widgets`、`tree_link_widgets` 与 `tree_widgets` 共享 owned 源、节点/边和当前标签池，
所有计算元数据、真实 Mark 与归属预检先于定位回调，混合角色共用 128 插槽预算。
合法当前 Path/画笔/标签编辑保留原始中心锚点，不取偏移路径包围盒中心。

新增精确零半径支持，实际零尺寸、透明和无墨迹角色仍有组件数据；禁用连线仅保留元数据，
不造 Mark 或角色。只有有标签的节点提供 canonical 来源，连线和空标签不伪造数据控件；
零墨迹逻辑行也不制造物理拾取轮廓。Tree 始终保留根，旧共享 `include_root` 字段在此图型
不控制根可见性。原始位值精确更新组件数据，兼容 Mark 标量仍按数值相等比较，控件文本
在仅零符号变化时可能保留 `-0`；非零数据更新仍须准确反映。

这对应真实 Nivo `nodeComponent`/`linkComponent` 用例，不声称原嵌套 raw datum、路径 UID、
React/Spring 或 D3 数字结果相同；Rust 径向第五方向是扩展。后续已增加[真实标签组件](tree-label-components.zh-CN.md)。
Canvas 原始回调和重复层见[后续专项](tree-canvas.zh-CN.md)，Glory SVG 函数层适配见[独立指南](tree-svg-layers.zh-CN.md)；自动退出动画、控件导出及原生同层输入仍待补，
GPU/worker 后置。见[中文指南](tree-widgets.zh-CN.md)。

## 开发版：Stream 八类函数配置

`StreamFactories` 提供 label、color、formatted_value、border_color 及逐点 size/color/
border_width/border_color 八个借用回调，可与原点几何工厂组合。原始投影、路径、身份和预算
先于回调；每层的 Some 原值只格式化一次，之后解析标签/颜色、完整层边框和完整层上下文中的
逐点样式。面积摘要复用末尾采样的格式化结果，末尾 None 不向前寻找替代值。
文字、图例、画笔、切片、提示和现有 typed dot Widget 数据来自同一布局结果。

显式空标签保留原意，不造面积 canonical 站点。隐藏层保留原输入和格式化/标签/颜色，
不调用未绘制面积或点的样式函数；未请求路径的隐藏层不计算无用曲线控制点。显式颜色函数
可以覆盖合法静态域的 Missing，但静态 ordinal 映射仍表示其声明域，不冒充函数结果缓存。
覆写颜色函数也不会先解析未使用的静态 Contrast。dot_size 与 glyph 面积/名义尺寸不能混用，
两者同时设置明确拒绝；其它样式函数可组合 glyph 和最终几何工厂，stroke-only glyph 仍要求
实际正描边宽度。几何工厂看到的 style 是原声明值，fill/stroke 是已解析画笔。

这覆盖真实 Nivo function props，仍不等于任意 Stream 自定义 layer 位置、React/Spring、
JS RawDatum 和 getter/runtime 协议。Rust 使用借用 Geometry、显式 nullable 系列和可控隐藏；
不同于原 SVG path 字符串与 JS null 数值处理。见[中文指南](stream-functions.zh-CN.md)。
GPU/worker、图层组件与任意实时控件导出继续保留，这批 API 不包含在已发布的 0.1.0 中。

## 开发版：Tree 五类基础函数配置

`TreeFactories` 支持真实 nodeSize、nodeColor、linkThickness、linkColor、label 对应的五个
借用函数。中间节点上下文含完整来源、原始 datum、父子索引、深度/高度、聚合值及真实投影位置；
连线函数读取两个最终计算端点，标签函数读取最终节点。节点按广度优先逐个计算尺寸/颜色，
再计算全部连线粗细/颜色；关闭连线绘制不跳过其计算。尺寸返回直径，显式颜色覆盖 None 填充，
零尺寸/透明节点及零线宽 Mark 保留真实角色。绘制标签可为空，不回写原始业务标签。

Rust 保留原静态样式 inset 投影，函数尺寸只改实际几何和标签间距；原 Nivo 的投影不依赖尺寸。
Rust 的统一 Scene 标签门禁与原 Canvas 无条件计算标签不同，文档和冻结参考分别记录。
后续开发版已补齐 active/inactive 四个修改器、固定深度颜色继承及[标签组件](tree-label-components.zh-CN.md)。
Canvas 原始回调和重复层见[后续专项](tree-canvas.zh-CN.md)；退出动画仍待补，不能把这五个基础函数解释为已完成所有 Tree 配置。
见[中文指南](tree-functions.zh-CN.md)。
GPU/worker 后置，这批 API 不含在已发布的 0.1.0 中。

## 开发版：Tree 活动修改器、颜色继承与所有者

`tree_layout_with_activity` 将四个 active/inactive 节点直径/连线宽度修改器与基础函数共同
计算。没有 current 时分类为 None 且四函数均不调用；有 current 后每角色为 true/false，
四个祖先/后代节点/连线开关独立。固定深度颜色继承按广度优先传播父色，并跳过深层颜色函数。
节点最终尺寸供连线、标签和带共享活动数据的 typed Widget 使用，静态 inset 不重新投影。
活动索引绑定原捕获构造，拒绝将另一合法布局整体移植进来的错误关联。

公开 `BrowserTree` 独立 RAF 合并真实指针意图，候选成功绘制与严格 Widget stage 采用后
才切换接受状态；空闲与 stationary 指针不重跑工厂，自身发布的 repick 不制造循环。
Geometry 使用具有业务绑定的实际墨迹，独立 nearest mesh 可选择零尺寸、透明及空标签节点，但不伪造
canonical 控件、物理命中或共享图表提示。鼠标/笔 move、leave 驱动活动；程序 focus 和
直接 SurfaceInput 只转发/处理共享图表，不冒充真实 pointer-current 输入。

这不等于原 D3 tie、React/Spring、任意 JS 数值阈值语法或 OS 触控协议。
标签组件见[后续专项](tree-label-components.zh-CN.md)；最近节点提示已有[显式 opt-in 接口与受检浏览器证据](tree-tooltip.zh-CN.md)；
Canvas 原始回调和重复层见[后续专项](tree-canvas.zh-CN.md)，Glory SVG 函数层适配见[独立指南](tree-svg-layers.zh-CN.md)；自动退出和实时控件导出仍待补；
键盘焦点需应用显式发布 activity，原生同层输入及 GPU/worker 后置。
见[中文指南](tree-activity.zh-CN.md)，所有新增功能保持 Unreleased。

后续[指针生命周期修复](tree-input-lifecycle.zh-CN.md) 在主鼠标/笔按压期间延后自动活动
与点击目标替换，真实拖动超过阈值仍允许重绘；接受的显式更新取消旧按压，提交前拒绝保留旧按压。
旧包四引擎 64 项基线保留 WebKit 的 10 项失败；同参数修复包 64 项通过，按下前不等待
悬停稳定、不重试。正常 pointer capture 本来也会将 up 定向到 Surface，不能单凭目标变化
归因。cancel/blur 的 DOM 注入仍只表示协议测试，不冒充 OS 焦点或取消投递。


## 开发版：Tree 真实标签组件

`tree_components` 将 Node、Link 和实际 Scene Label 纳入同一
`SceneWidgetsData<TreeComponentData>`；旧双角色 enum 保持兼容。
`tree_label_widgets` 提供标签专用 typed 包。稳定标签 key 不依赖当前 Label 索引，
数据包含原始输入、计算节点、完整池、当前字体、空白和裁剪。
空文字、透明标签、零尺寸节点不伪造 Mark，也不因此删除标签角色。
所有受管归属、字体与槽位身份预检先于放置回调，三角色共享 128 槽预算。

BrowserTree 新增三角色组件挂载入口，继续采用原所有者的活动和严格提交机制。
BrowserChart 的真实 Label carrier 支持任意 Glory 子树的仿射旋转、相机与独立 CSS
横纵缩放；显式 Rect 维持轴对齐，原生自动变换明确 Err。
替换 Label 文字保留节点／连线及 canonical 业务命中；标签默认 pointer-none 不等于 inert，
完整可见子控件可键盘聚焦或显式启用指针，部分裁剪仍保守被动。

这补充上文早期阶段所列 SVG `labelComponent` 用例，不代表 Canvas 原始 `renderLabel`、
React/Spring 动画或控件导出完成；后续 Glory SVG 函数层适配见[独立指南](tree-svg-layers.zh-CN.md)。Rust alphabetic 基线与投影默认值继续保留。
本轮实际检查结果单独记录，不能沿用上一轮 WASM 包的报告证明当前代码。
见[双语标签组件指南](tree-label-components.zh-CN.md)。

## 开发版：Tree 原生 Canvas 回调和可重复层

新增 `TreeCanvasFrame`、`TreeCanvasPlan` 和真实节点／连线／标签／自定义层绘制回调。
同一绘制过程共享原生 2D 上下文，重复层真实重复调用；变换、路径、alpha、裁剪和合成状态
在用户回调之间传递。`tree_canvas_layout_*` 完整捕获关闭的标签和连线样式，函数只计算一次，
旧布局入口的标签资格、Debug/Eq 和默认输出保留。

`BrowserTreeCanvasProgram` 在 Engine 状态借用之外准备绘制，封存位图与源数据、活动状态和
严格顶层 Glory 组件联合提交。相机／DPR／隐藏变化重新绘制而不重跑布局工厂，仅更换绘制程序
也不重跑布局／放置／组件工厂。失败恢复直接复制已接受位图，不重放回调；原生 PNG 导出包含
真实回调像素，不含 HTML Widget。

这一进展补充前文仍列为待补的原始 `renderLabel` 和 Canvas 自定义层。完整像素、所有者与
联合控件各用独立验收，详见[本轮记录](verification.md)及[双语指南](tree-canvas.zh-CN.md)。
原始上下文模式明确拒绝源顺序 HTML 交错和墨迹替换；SVG 源顺序组件仍使用原接口。
实际命中仍是源模型或节点中心，不能把任意回调形状称为物理命中几何。

最近节点提示的[类型化 opt-in 实现](tree-tooltip.zh-CN.md)已完成受检浏览器验收；Glory SVG 函数层适配见[独立指南](tree-svg-layers.zh-CN.md)。自动 Spring／退出动画、原生同层
输入及实时 Widget 导出仍待补。默认 Rust 图元、alphabetic 基线、主题子集、纵向 step 和每轮新生成器
不等于 Nivo／D3 数值或跨帧对象身份。GPU/worker 继续按用户要求后置；新增 API 保持 Unreleased。

## 开发版：Tree 最近节点提示（显式启用与受检范围）

新增 `TreeTooltipData` 与 BrowserTree 的显式提示选项／真实 Glory 内容接口，保留完整原始源、
最终计算节点及活动；raw 与 aggregate 分开，原始正负零保持可观察。原默认 Core 提示不变，
启用后必须有新的可信主鼠标／笔 Move 才建立受管目标，不从程序 activity 反推提示。

支持固定节点／光标定位、九种自然尺寸对齐及独立数据缓存。提示不制造 Mark、canonical 控件或
物理命中；它与源组件的数据共享接纳边界，但不进入 Canvas PNG 或源 Scene 导出。
同一最终包的五种浏览器／CI 配置和两 DPR 覆盖真实输入、源／提示 frame 同源、
九种对齐、实际组件像素及错误恢复；完整数据与像素证据见[验收说明](verification.md)。
Source Label carrier 仍要求可解析的 CSS 尺寸；持续的源构建／放置／原始绘制错误保留
共同已接受的源与提示，不通过 tooltip-only 更新绕过失败。
逐节点原生事件回调、任意泛型 Datum、React/Spring 生命周期与原 Delaunay 平局语义仍有边界；
GPU/worker 后置。用法见[双语指南](tree-tooltip.zh-CN.md)。

## 开发版：Tree SVG 函数层（Glory 适配）

`TreeSvgFrame` / `TreeSvgPlan` 与 `BrowserTreeSvgProgram` 提供字面省略、重复、顺序，
以及每个 occurrence 独立保留的真实 Glory 子树。内建视图保留原始业务身份；重复节点、
连线、标签不会复制业务 Mark 或 canonical 控件。完整计算池、源值、主题、活动和 SVG
连线生成器可用于自定义层。层、源组件与提示数据共同采用，revision 控制子树替换，
相机／隐藏更新保留局部状态。真实 Mesh 的字面位置决定最近节点输入覆盖，几何投影
排除遗漏、替换和无墨迹来源；自定义层不会被伪装成业务命中对象。

五种浏览器配置、两种 DPR 的同一构建完整矩阵通过 60 个配置、3690 帧，另有旧 Canvas、
标签与提示回归。实际 DOM 改色／相机偏移负控保持全部数据不变，仍被原像素门禁拒绝。
完整收据及独立读回范围见[验收记录](verification.md#development-tree-svg-function-layers)。

该适配使用真实 SVG `foreignObject`、XHTML、固定视口绘图区裁剪与 HTML 相机。
数据共同采用不代表 Glory 0.8 的受管后端 DOM 写入在采用前已完成；后端故障可能留下
已执行的前缀并使运行时 fault，须重新挂载。受检的采用前 carrier 拒绝恢复不替代该边界。
任意原始 React SVG/defs 插入、自动 Spring／退出、实时
Widget 与重复层导出仍是独立差距；GPU/worker 继续后置。新增 API 保持 Unreleased，
用法见[中文](tree-svg-layers.zh-CN.md)与[English](tree-svg-layers.md)。

## 开发版：Tree 类型化原生鼠标回调

节点与 SVG 连线各七个独立回调已完成受检范围验收；Canvas 提供节点七回调。
共享 `TreeEventFrame` 保留完整原始／计算数据、正负零、源索引、连线端点、样式与活动，
实际原生事件、已采用来源 generation 和 occurrence 身份各自明确。普通 SVG、字面层、
Mesh、Canvas 来源模型与显式 Glory 元素绑定分别规定资格和相位。

五种浏览器配置、两 DPR、九场景共 90 项采用同一包、同一脚本从头通过，
十个原进程退出码及独立完整读回均通过；原生与旧功能回归另有同源收据。
回调同步发生在实际 Window 晚冒泡阶段，不由 RAF／Activated 合成。
源、程序与组件 revision 的成功替换会使旧按压失效，采用前拒绝保留旧手势。

可选 NativeComponent 活动策略、处理目标的双击策略及显式绑定保留原默认行为。
默认浏览器菜单可能阻断后续输入；测试宿主明确阻止菜单以验证真实副按钮 Up，
不将未送达事件当作回调丢失，也不改变框架默认菜单行为。
SVG 连线托管 tooltip／独立 anchor 已有受检适配（见后节）；自动 Spring／退出、控件导出与更广设备验收仍单列；
GPU/worker 继续后置。API 保持 Unreleased，见[中文指南](tree-events.zh-CN.md)、
[English guide](tree-events.md)及[验收记录](verification.md#development-tree-native-mouse-hooks)。

## 开发版：Tree SVG 连线提示

已新增 `TreeLinkTooltipData` 与 SVG 专用的真实保留式 Glory 内容，完整保留两端原始值、
计算节点、样式、来源索引与 generation。光标定位、Nivo 五种 anchor 解析和四角 Rust
扩展与节点提示共用采用边界；两类组件各自保留状态，Link-only 配置保留原 generic Node 提示。

五配置、两 DPR、60 场景已完成同一包／脚本的从头矩阵，覆盖 3470 步、1680 个截图检查点。
Chromium 测试显式使用 `--disable-gpu`；加速 GPU 下的逐像素稳定性仍在本轮范围之外。
来源、层与当前提示的完整数据、实际组件像素、拒绝恢复及清理见
[验收记录](verification.md#development-tree-svg-link-tooltips)；API 用法见
[中文](tree-link-tooltip.zh-CN.md)与[English](tree-link-tooltip.md)。

自动 Spring／退出、实时控件导出、泛型 Datum 与更广设备／绑定路线继续单列。
新 API 为 Unreleased，GPU/worker 继续后置。

## 受检完成：Tree SVG 逐属性 Spring

`TreeMotion` 的 7/9/3 属性、保速重定向与 Live／Leaving 来源现已接入 Cartesian SVG
BrowserTree：自动 RAF／精确手动时钟、动态配置、实际采样绘制与拾取、目标事件数据、
Source／层／tooltip 共同采用及真实 Glory 退出 Scope 保留均已实现。重入保留组件本地
状态；`.custom_animated` 与重复层共享数值时钟。固定 plot 裁剪不因 WebKit 键盘聚焦
发生滚动；CSS 隐藏和原生按住暂停动画，显示与松开恢复。

独立模型／配置／绘制共 29 项专项，包括 60 组形状／方向／曲线组合；默认 1986、全特性
2322、文档 18 项通过。Chromium／Firefox／WebKit × DPR 1／2 的 18 个真实应用配置、
450 步通过，含物理值闭式参考、实际 DOM／PNG、组件状态和候选拒绝。

前述 Tree 小节中的自动 Spring／退出缺口由本节限定范围收口。业务 ID 身份与 Nivo
祖先 UID、解析求解与 React Spring 积分的区别仍明确保留；Node／Label／Link 不因
数值存在而获得原生输入资格。编辑几何需显式逐帧 renderer；原始 React SVG/defs、
任意 Widget 导出、更广设备／原生同层输入仍单列。Nivo Canvas Tree 直接绘当前数据，
Canvas／Radial Spring 和 GPU/worker 继续后置。
见[中文指南](tree-motion.zh-CN.md)、[English guide](tree-motion.md)和[验收记录](verification.md)。

## 开发版：Funnel 绑定注解与原生文字描边

`FunnelSvgFrame::with_annotations` 将类型化 ID 或 predicate 绑定到完整计算来源，
提供 Circle／Dot／Rect、相对和绝对 note 坐标、连接线、真实文字 halo 与自定义 Scene
note。独立 Annotations 字面层保留省略、重复和顺序；完整匹配先于 note 回调，重新绑定
替换旧 paint，来源 capture 与派生 bundle 身份保持明确。

原生 SVG 注解可以覆盖主体，但不获得业务 part hook 或 canonical site。
来源、层和当前 tooltip 共同采用；函数和预算拒绝保留旧接纳数据、组件与呈现。
共享 `LabelStroke` 接入 SVG／Canvas／CPU raster，零尺寸形状保持无墨迹，零宽或透明
描边保持原彩色字体填充。五浏览器／双 DPR 的 40 组、900 帧正式矩阵与完整独立读回
通过，含 120 次拒绝、20 个真实 DOM 负控和 40 个文字描边贡献检查；同构建旧功能回归通过。

泛型应用 raw datum 由下节实现；本节不包含注解 Spring／退出、逐 note 保留式 Widget、自动避碰
或任意实时 UI 导出；GPU／worker 继续后置。上游原生 circle 与有限 Scene path 的
全边缘栅格结果不承诺相同。细节见[中文指南](funnel-annotations.zh-CN.md)、
[English guide](funnel-annotations.md)和[验收记录](verification.md#development-funnel-bound-annotations)。

## 开发版：Funnel 泛型应用数据全链路

`FunnelDataDatum<D>` 显式拥有原始业务数据和 `FunnelDatum` 投影，`FunnelDataFrame<D>`
保留共享 raw pool 与完整计算帧，`BrowserFunnelData<D>` 接入同一专用 SVG owner。
不要求 D 实现 Clone／PartialEq／Debug／Serialize／Send／Sync，纯数据模型也可借用
非 static 数据；浏览器保留来源须为 static。既有未泛型化入口与 `FunnelSvgFrame` 的 Send／Sync
能力保留。样式、尺寸、阶段、注解、自定义 Scene note、旧角色 Widget、整层组件、
命中快照、原生鼠标 hook 和标准／自定义 tooltip 均可读取对应完整来源。

仅原始字段变化仍产生新的构造 capture 与 source generation，保留组件更新真实 UI
并保持本地状态；program／view／hidden 不重放来源工厂。private SourceEnvelope 将
raw 与 projection 一起进入接受帧、各 Layer／Tooltip Cage 和原生 capture，避免接受后
单独写入 raw sidecar。旧输入保持历史来源，tooltip 按仍有资格的业务 ID 重新绑定。

原生默认 2056、全特性 2401、文档 18 项及严格检查已通过；同包五项旧功能回归与六个
双语 Rust 示例编译通过。五浏览器／双 DPR 的 40 组、2010 帧、2020 张原始 PNG、
360 次拒绝和 10 个真实 DOM 负控均通过完整独立读回，最终证据见
[验收记录](verification.md#development-funnel-owned-application-data)。

该模型不推断 opaque D 与投影的业务一致性，不深冻结 D 内部可变性，也不回滚用户
闭包的外部副作用。共享后端 adopt 后故障仍使用既有隔离边界。动画与时钟见下节；
任意可变 D3 generator、逐 item Source Widget 混排、任意实时 UI 导出与 GPU／worker
继续单列。使用见[中文](funnel-data.zh-CN.md)与[English](funnel-data.md)。

## 开发版：Funnel 逐属性 Spring 与原生动画帧

`FunnelMotion` 区分接受的目标 Frame 与实际采样 Scene；路径各用独立的 0→1 Spring，
中断时从上次提交的目标插值，新命令拓扑与数字 ordinal 配对遵循原版 D3 字符串规则。
颜色、边框、标签、分隔线、注解轮廓／连接线／内置 note 分别保留物理轨道；原始
样式 opacity 与文本即时采用目标，自定义 note 和无角色 extras 保持目标绘制。
部件和标签以 typed ID 保留，分隔线另区分 side／terminal，注解按 binding index。
新角色即时出现，移除即时消失；原版 Funnel 没有需要补造的退出组件。

专用 SVG owner 提供自动 RAF、精确 Manual step、系统／显式动画偏好与隐藏时钟暂停。
来源、采样、Layer／Tooltip Cage 和原生输入接纳证书共同采用，物理 tick 不重跑业务
工厂。真实 path／text 节点按角色复用；位置、属性、子节点和焦点受 placement journal
保护。`BrowserFunnelData<D>` 复用相同协议，不对 D 增加 trait 要求。

原始 Nivo 组件／hook 请求、key 与 D3 插值参考已实际执行；9 项专项与严格编译、
六个双语 Rust 示例编译通过。完整五浏览器／双 DPR 的 40 组、1930 帧、200 次拒绝、
10 个真实 DOM 负控和 2340 张原始 PNG 已通过原始进程检查和完整独立读回，2472 批
原始事件完整保留并逐批核对，见
[验收记录](verification.md#development-funnel-property-springs)。API 保持 Unreleased。
本实现使用解析阻尼 Spring，不运行 React Spring。body／border 轨道已按 source key
保留，零宽／零 opacity 边框及完全收拢 body 的无墨迹期间仍接收目标并推进；恢复时
保留速度及上次目标几何。显式全来源工厂为自定义收拢状态捕获真实描述符，旧构造器
未调用过的自定义目标在启用动画时明确拒绝。12 项原生专项及 40 组休眠专项独立读回
已通过；同包完整动画回归 40 组／1930 帧／2340 PNG、200 次拒绝与 2432 批完整事件
也通过正式独立读回。见[休眠状态指南](funnel-dormant-motion.zh-CN.md)。
可变 D3 generator、逐 item Widget
混排、任意实时 UI 导出与 GPU／worker 继续单列；用法见[中文](funnel-motion.zh-CN.md)
和[English](funnel-motion.md)。


## 开发版：Waffle 字面 SVG 层与专属 owner

新增 `WaffleSvgFrame`／`WaffleSvgPlan` 与 `BrowserWaffle`／`BrowserWaffleData<D>`，按字面顺序省略、重排或重复 Cells、透明 Areas、Legends 和保留式 Glory 整层。原生默认 2095／全特性 2440／18 doctest 与严格检查通过；完整矩阵通过 10 个浏览器／DPR 上下文、40 场景、2280 capture、2160 操作、280 次候选拒绝及 20 项真实 DOM 负控，来源不变。独立读回亦通过，覆盖 7340 次 raw read、2368 次 effect、712 个 hook、34720 行 peer 与 3100 张解码 PNG。见[中文指南](waffle-svg-layers.zh-CN.md)、[English guide](waffle-svg-layers.md)及[验收记录](verification.md#development-waffle-literal-svg-layers)。

实际 cell Mark、完整槽位分组 polygon 和透明原生事件平面分别保存。透明 Areas 与 handlerless Cells／legend 背景保留 SVG 原生遮挡，关闭 interactive 只禁业务回调／提示；空格不伪造分组。默认图例配置为空，显式固定 item box 的原生图例在其 occurrence 中绘制。自定义 UI 使用完整 Scene／margin 坐标，不加隐式 plot clip。

新 SVG IndependentRound 允许显式 total 小于来源值总和，并保留请求／容量裁剪区间；新 profile 合并角接触轮廓。旧纯 layout 和 Vec DataFrame 的准入、来源存储及轮廓规则保持。旧 `WaffleMode::Areas` 有色 union 不冒作新透明 Areas；literal owner 只接纳 Cells mode。

shared-source 构造让任意 D 不增加 Clone／Eq／Debug／Serialize／Send／Sync 要求；完整 source、Layer／Tooltip Cage 与 native admission 共同采用。四种原生观察保留实际 MouseEvent 与捕获 GEN／publication／occurrence，回调可排队 setter 而同步 flush 拒绝。源 hidden 重排需要重建；camera／program／tooltip 位置不重放业务函数。候选拒绝保留旧来源与 owned DOM／focus；用户外部副作用不回滚，采用后 UI 故障隔离 carrier 并要求 revision 重建。

原 12 像素拒绝差异已由独立纯 DOM tooltip 显隐对照复现；native-content 的唯一 `will-change:transform` paint 边界修复与同配置冷基线核对，不归因特定 GPU，整图回滚零差门保留。固定 pattern 使用作者参数独立原生参考／整 tile 零差及三项错误 motif 负控，不宣称所有图元跨引擎 AA 等价。真实 ResizeObserver 与 stop 后弱 handle／原位置点击守卫也已进入当前验收。

Canvas 专属 owner、Waffle Spring／退出、builtin 内逐 item Widget 混排、组区域 Widget 工厂、HTML 层等价与任意实时 UI 导出继续单列，GPU／worker 后置。既有 Waffle 函数／cell／regions 的历史验收不改写，也不能代替本轮新 owner 验收。
