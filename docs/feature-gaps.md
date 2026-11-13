# Nivo / D3 差距审计与下一轮任务

原始审计日期：2026-11-10；共享字体与可编辑 Brush 更新：2026-11-11。范围是本地快照的公开图表数据契约、配置与实际计算；这是可实施清单，不是逐个 React prop 的移植承诺。已有通用 `Scene`、图层、格式器或调用者手改 mark 能实现某种效果，不等于对应图型已经提供稳定的 typed API。

## 当前差距摘要（2026-11-13）

BoxPlot整部件指针本轮已正式验收：显式PaintedGeometry及原始outlier快照、Glory有限坐标通道与四宿主接线完成；完整第三轮8上下文／128宿主／512阶段、独立1888来源像素与6480逻辑PNG通过。旧body-only入口保留；Geometry solid-paint、有界曲线／clip、平台transform与真实设备边界见[整部件指南](whole-part-pointer.zh-CN.md)及[验收记录](verification.md)。逐摘要String图例工厂／显式ByLabel也已完成正式受检范围：类型化行到目标、缓存／显式替换、完整第四轮8上下文／224逻辑宿主／288实际mount／896阶段及独立审计通过。借用的逐摘要源颜色工厂已落实，实际受检范围见[颜色工厂指南](boxplot-color-factory.zh-CN.md)。FullYears 月份间距与 Radar 逐点标签工厂也已补齐，详见下方配置说明。GPU／worker继续暂缓。

Conic 中点进度盲点已修复，修复前基线 featureAccepted=false 已归档；新版本已按同源终结证据完成受检范围验收并归档，未沿用修复前成绩。全新完整 cohort 2863059d-9ad6-4490-a2be-a0f1caf966bf 通过 183/183，Firefox、WebKit、Edge 各完成明确选取的 61 套，整个 piecewiseScales 仍排除。四组系统 Chrome CLI 独立通过，host 严格 compositor CLI 未计入。新版首轮 182/183 因 WebKit scaleProfiles 等待滚动稳定超时而保留为失败；原脚本在新旧宿主的四次独立对照未复现，原因仍未确定。最终仅采用三份完整新子报告，不复用部分通过项，也未更改断言、超时或像素门槛。详细新版本状态见[验收记录](verification.md)。

普通 Nivo 图族已有对应布局，主要差距已转为配置完整度、D3 基建和宿主能力。
以下按当前实现区分，完成证据与历史检查点见后文和 verification。

本次重新对照原版 Nivo 后，已把 Bar 分类 BandScale 配置、Bar 单柱和 HeatMap 单元格的类型化几何/画笔工厂落实到正式源码。使用方式见 [分类尺度](bar-category-scale.zh-CN.md)、[自定义 Bar](bar-custom-geometry.zh-CN.md) 和 [自定义 HeatMap](heatmap-custom.zh-CN.md)。这三项只覆盖受检的尺度与单标记契约，不替代任意组件、更多角色配置或宿主验收。

| 范围 | 当前状态 | 继续补齐的重点 |
| --- | --- | --- |
| Nivo 图族 | 已有普通图族对应入口，保留不同数据模型 | 逐图型共享配置、自定义几何/Widget、标签与图例的角色接线 |
| 等值线与二维密度 | 正式功能验收通过：网格拓扑/孔洞、加权 Float32 场、稳定身份与阈值复用、SVG/Canvas/CPU、真实拾取及发布恢复 | 更广的数据与宿主 profile；新图型专属配置继续按预算扩展 |
| Brush | X/Y/XY 持续选区、移动/边角调整、修饰键、会话生命周期已实现；四引擎专项、165 套矩阵及原生 headless 接线统一验收通过 | 原生原始修饰键事件需应用显式转发；更多真实触摸设备与辅助技术验收 |
| 数值尺度 | Linear/Log/SymLog/UTC/Point/Pow/Sqrt；多断点已接 Line、Scatter 和完整断点视口，原生与功能专项已有证据；整体严格跨帧基准仍未通过 | 更多图型接线；重复域节点等更宽的 D3 输入契约 |
| 颜色尺度 | 连续/发散、Quantize/Quantile/Threshold、非线性输入；SequentialQuantile 样本右排名及精确数据轴图例、三消费者和四引擎功能验收已完成；较早整体严格跨帧基准仍未通过 | 更多消费者角色；空样本非零自动域策略 |
| 地理 | Nivo 十种投影、有界裁剪/线流/经纬网；Conic 三家族与 Albers、曲边拟合、反经线重连和真实符号轮廓；六项球面工具已有原版 D3、高精度数值及真实导出专项 | AlbersUsa 复合投影、更广拓扑和可见范围；默认 Firefox Sans 绘制差异与较早严格跨帧基准仍未通过，显式 System/Offscreen 专项不替代默认字体结果 |
| 文字 | 真实测量、字体/字重、普通 Text 基线、字形轮廓、显式 bidi/竖排、COLR v0 | 自动折行、更完整 CSS/基线语义、COLR v1、bitmap glyph |
| 平台与性能 | SVG/Canvas/CPU、浏览器矩阵及部分原生窗口已有证据 | 更多真实输入设备/读屏/系统验收，完整分配归因；GPU/worker 按用户指示最后处理 |

保留式 SVG 的五种已复现包装节点事务故障已完成正常四引擎专项与 180 项范围验收；任意持续 DOM 故障、Canvas 及 legacy phase/mode 应用缓存原子性不在此结论内。

等值线/密度已完成正式功能交付，见 [中文指南](contour-density.zh-CN.md) 与 [English guide](contour-density.md)；[原计划](d3-contour-density-plan-2026-11-11.md) 保留准备阶段证据。

计划与参考数据不单独计入已交付功能。高级尺度整体浏览器验收仍未通过：第五轮矩阵 173/174，Chrome 严格跨帧屏幕像素基准失败；专项通过不能替代整体结果，详见 [当前验收检查点](verification.md)。可编辑 Brush 的实现入口与边界见
[使用说明](editable-brush.zh-CN.md)，原要求保留在 [Brush 计划](d3-editable-brush-plan-2026-11-11.md)。

## 新增独立图型与逐轴布局

| 图型 | 已实现的入口与行为 | 支持边界 |
| --- | --- | --- |
| Funnel | `funnel_layout` 提供横纵布局、阶段前后宽度、线性与平滑边缘、真实间距与分隔线、受控高亮、原始阶段和拾取绑定。 | 自定义宽度接收有限像素值。重绘高亮由调用者提交。图型专属组件插槽待补。 |
| Waffle | `waffle_layout` 提供四向网格、独立取整或最大余数分配、隐藏后重排、圆角单元格和实心分组区域。 | 独立取整溢出时分别保留请求区间和实际占用区间。精确拾取按墨迹，全格区域查询包含间距。 |
| Sunburst | `sunburst_layout` 提供 D3 等面积分区、内部节点权重、父色继承、弧形圆角和间距、节点路径及类型化拾取。类型化文字工厂在测量签名前构建所属文字，保留原datum和数值。测量标签支持优先级、偏移、隐藏恢复及 Plot/ClipToArc/RequireFullInkBounds 三种策略。 | 隐藏节点保留原角度区间。真实文字墨迹须相对原锚点测量；严格 AABB 可能额外省略旋转文字。扇环蒙版保留实际 pad、圆角与内孔，路径查询另设工作预算。 |
| 平行坐标 | `parallel_layout_with_scale_options` 提供独立端点推断、Linear、Log、SymLog、UTC、刻度与行几何共享比例尺，并返回原始值、投影顶点和拾取绑定。 | 显式或推断域在隐藏行时保持稳定。刻度预算包含实际日历区间刻度。逐轴筛选刷与类型化分类轴仍待评估。 |

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
| HeatMap 扩展 | `heatmap_layout` / `HeatmapLayoutOptions`：NULL、稳定row/column ID、shape/size、两轴padding/四侧axis、边框/圆角/labels；`related`与事务`set_active`精确Cell/Row/Column/RowColumn/All；新增 `heatmap_layout_with_cell_factory` 与显式颜色 profile 工厂、真实墨迹和原始 nullable 拾取绑定 | 稀疏未供给pair不补NULL；明确 NULL 会调用工厂，零值保持零。工厂返回单个受检 Scene 标记，隐藏/resize 通过重建；不等于任意 Widget 插槽。受管来源和标签归属必须保持，完整整合验收见 verification。 |
| 极坐标轴与径向堆叠柱 | `PolarFrame`、数值和类别刻度、`polar_axes` / `append_polar_axes`、`polar_bar_with_axes`。`stacked_radial_bar` 按系列建环，按类别累计角度。`layout_stacked_radial_bar_with_arc_style` 为柱段和轨道接入 D3 圆角与半径感知间距。 | 角轴支持非负线性值，隐藏类别保留完整域和栈槽位，正值角投影失真时返回错误。图型专属自定义组件待补。 |
| Bar 展示配置 | `bar_with_style` / `bar_layout_with_style`：group pixel gap、系列/分类色、值标签与 Net/PositiveNegative totals；`BandScaleOptions` / `category_scale` / `category_reverse` 使柱、刻度、合计共享 round/align/inner/outer/step/bandwidth 与分类窗口；`series_order` 统一源系列顺序，`legend_source` 选择Keys/Indexes | 显式类别 profile 先完整范围取整，再投影窗口且不二次取整；`None` 保留旧行为。Keys图例控制series；Indexes分类只独立展示/激活，隐藏系列需重布局才能重排slot/totals。选中的聚合必须可表示，不能把域外 total 挪到 crop 边。 |
| Bar 自定义标记 | `bar_layout_with_bar_factory` / `bar_layout_with_scale_options_and_bar_factory` 提供原始 series/category/index、区间、可见与未裁剪信息、单个 Geometry/FillPaint、私有来源和受控高亮 | `None` 不造标记；零值保留原值及逻辑站点。自定义墨迹不移动普通柱的固定逻辑站点。真实墨迹拾取与逻辑最近点分开，受管整帧变更需工厂重建；不等于任意 Widget 插槽。 |

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
| Scatter 自定义节点 | `scatter_layout_with_node_factory` 以原始数据、映射坐标、尺寸和继承样式生成单个自定义标记。精确拾取使用真实绘制与裁剪，最近点保留原始数据锚点。 | 可用路径或居中的图元。兼容路径动画跟随指定端点的平均位移，提交帧使用精确映射坐标。图内 Widget 另需测量与生命周期支持。 |
| Line 自定义点 | 连续 Line 与 PointLine 支持同一自定义几何工厂、原始数值或类别键、配置值轴和类别配色。 | 精确拾取按实际裁剪后的墨迹，最近点与切片保留数据锚点。工厂返回单个图形。 |
| 双向、竖排与彩色文字 | `FontRegistry::with_bidi`、`with_vertical_layout` 提供双向、显式多列和 Unicode mixed 方向。COLR v0/CPAL solid layers 可显式选择 palette；CPU 与 `Chart::painted_labels` 共用实际彩层。`scene_with_painted_labels` 为独立 SVG/Canvas/CPU 导出生成路径副本，保留业务标记、归属、绘制顺序及裁剪。 | 默认横排、彩色 glyph 拒绝。透明前景不抹去其他 palette 层。路径副本不保留可编辑原文字，原 Scene 由应用保留。自动折行、完整 CSS bidi/BASE baseline、COLR v1 绘制图及 bitmap glyph 待补。 |
| CPU 图像交互宿主 | `RasterSurface::mount_layout` 与 `RasterSurfaceHandle::set_layout` 原子接纳绘图区、Scene 和类型化绑定，支持自定义路径原始锚点、nullable 控件与原始提示行。metadata-only 更新刷新提示且保留 PNG。 | 应用提供真实宿主坐标边界与图像源发布。图片发布或绑定校验失败保留原始图像、控件和模型。这里的宿主坐标接线不等于已验收硬件输入设备或屏幕阅读器。 |
| Windows 桌面宿主 | Apace 显示语义矢量图，Wry 显示 CPU 位图并使用 `Chart::basic_data_controls`。两者的窗口绘制、键盘、图例、更新和拒绝由实际窗口与 Rust 事件记录验证。 | Windows 辅助功能树可读出图表、图例和数据按钮。Wry 增强导航和语义矢量图取决于宿主支持。硬件鼠标、触摸、屏幕阅读器播报和其他桌面系统待验收。 |
| 艺术混色 | `MarkBlends` 为标记指定 Normal 或 15 种艺术混色模式。SVG、Canvas 和 CPU 将填充、描边与透明度作为一层合成。BrowserChart 提供显式混色动画入口，过渡实例保留各自模式。 | 默认语义 Graphic 使用普通绘制。旧动画入口要求普通绘制配置，图型专属便利配置待补。 |
| 图内 Widget | `Chart::plot_widgets` 和 `BrowserChart::mount_with_plot_widgets` 提供测量矩形、稳定身份和当前帧。CPU 图像宿主也支持实时插槽，随 RasterOptions.view 调整且独立于 DPR，并与 PNG 发布共同准入。浏览器覆盖层支持带孔路径裁剪。 | 部分矩形裁剪控件显示墨迹并暂停输入。图像宿主支持矩形 mask 与路径 Full/Outside，任意 partial mask 仍需 browser mask host。导出的 Scene PNG 不含实时控件内容，控件快照导出和语义宿主任意路径蒙版待补。 |

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
仍需补齐 AlbersUsa、更广球面拓扑和输入范围，以及多断点尺度在更多图族中的接线。分级配色或单项通过不代表所有图型的 Nivo props 已对齐。

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
反算，不提供不唯一的全局数据视口。更多图型、重复域节点及更宽 D3 输入契约继续补齐。
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
| **P2-B：自定义图形与图内控件** | Scatter、Radar、Line、PointLine、Bar、HeatMap 提供受检几何工厂。语义 Chart 和 BrowserChart 提供测量插槽，浏览器支持路径裁剪。继续接入其他图型及图像宿主导出。 | 单标记工厂与任意 Widget 生命周期分别处理；真实轮廓、裁剪、坐标映射、帧更新事务与焦点清理仍须逐角色接线。 |
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
| Bump | `bump_layout` 使用类型化类别和离散排名点轴，提供转折肩部、首尾延伸、空值槽、共享符号及自定义点工厂。 | 每个系列的类别键须唯一。显隐保留完整类别与排名格。自定义点的最近锚点使用原始坐标，精确拾取使用实际墨迹。 |
| Stream | `stream_layout` 复用共享 `stack`，提供原始 nullable 区间、共享比例尺、Nivo 曲线、三种点位置和面积或采样绑定。`StreamDotOptions.glyph` 提供共享符号，`stream_layout_with_dot_factory` 提供原始采样、继承样式与自定义点几何。 | 采样切片可保留关闭显示的零尺寸点，缺失值仍为 `None`。面积绑定按整个系列提供摘要，采样绑定按画面顺序提供逐列数据。自定义点沿用真实墨迹拾取与原始锚点协议，图型专属 Widget 插槽待补。 |

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
- D3 的更多地理基建：复合 AlbersUsa 仍待实现。球面距离、长度、面积、圆生成、重心和包含已有 typed 公共入口，详见[球面指南](geo-spherical.zh-CN.md)；数值与稳定性对照、真实WASM及新画廊受检范围已有验收；显式System字体配合主线程Offscreen的受检profile已通过，默认Firefox Sans全帧仍保留失败。它们保留原 D3 有向积分、Polygon 流长度与边界判据，不能由平面 Scene 命中替代，也不计算任意拓扑的地理并集。

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
