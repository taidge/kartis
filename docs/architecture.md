# Kartis 实现规划与当前结果

## 定位与设计依据

Kartis 是 Glory 的下游图表库，保持独立 Cargo workspace。Glory 无需依赖 Kartis，也无需修改 framework 的语义节点协议。图表数据与布局结果均为 Rust 类型；渲染和输入处理不依赖 JavaScript 图表库。

本次直接检查的本地参考快照：

- `E:/Repos/d3`：`db118c93a429e82b83f091dbaa18fd0a82c06130`；`package.json` 标示 7.9.2。该 checkout 是聚合包，`src/index.js` 导出独立的 d3-scale、d3-shape、d3-hierarchy、d3-force 等模块，并不包含这些依赖的完整算法源码。
- `E:/Repos/nivo`：`0dae2c32052a573f9f1d66ec1b453b572119b265`。重点参考 `bar/src/compute/{grouped,stacked}.ts`、`line/src/hooks.ts`、`pie/src/hooks.ts`、`bar/src/BarCanvas.tsx`、`network/src/hooks.ts`、`chord/src/compute.ts`、`geo/src/hooks.js` 和各图表的布局/类型文件。

采用 nivo 的“数据规范化 → 比例尺/计算 → 渲染层 → 外围交互”分层。运行实现均为 Rust，不调用 React hooks 或 D3 runtime，也没有把 d3 的 selection/DOM 模型引入 Glory。算法参考与 Rust 后端归属见[third-party notices](third-party-notices.md)；独立参考测试可在开发时执行原 Nivo hooks，测试本身读取冻结产物。

默认 Rust 依赖图除 Glory 外新增 Spade **2.15.1** 及其 robust **1.2.0** 自适应谓词，用于真实 Delaunay 三角剖分；不再宣称默认只依赖 Glory。这与开发时执行的 D3/Nivo oracle 分开。Raster、浏览器、serde 与时区仍按各 feature 启用，具体归属和许可证见上述 notices。

## 当前新增：Conic 采样进度保护

新增 Conic 分支在 quarter/mid/three-quarter 弦误差之外，复用中点沿弦参数的进度触发细分，以覆盖近端急弯；旧十种投影路径和原工作量/点数/深度预算保持。默认地图入口仍为粗采样与曲边保守包络，显式 path options 才启用像素自适应采样。修复前约九像素的 Rust 盲点与原 D3 自身约 0.000485 的 near-pole 偏差分开保留，不再将前者归为原函数差异。

新版本整体验收见[验收记录](verification.md)，未沿用修复前 featureAccepted=false 基线的通过项。BrowserChart、RasterSurface 和 SVG 导出支持 Scene 任意蒙版；Glory 语义 Graphic/Text 无该操作。AlbersUsa、球面工具、任意拓扑与设备仍需推进，GPU/worker 暂缓。

## 当前新增：独立 typed Scatter

`src/scatter_layout.rs` 及其 `axes.rs` 子模块从源数据直接生成轴、节点、标签和metadata，不调用Line再删路径。旧 `cartesian::scatter` 保持原半径5及默认行为。新 `ScatterDatum::new(id, x, y)` / `ScatterSeries::new(id, data)` 接 `CategoryKey::{Number,String}`；节点ID按UTF-8长度前缀编码 `(series,node)`，允许不同series使用相同局部node ID，不因hidden或重排改身份。`series_index/source_index/global_index`记录当前完整源的位置，hidden源也保留metadata。

`ScatterAxisScale::{Continuous,Point}` 两轴独立解析。Continuous携`ContinuousScaleOptions`和`ScatterBound::{Auto,Fixed}`，默认Linear/min0/maxauto/niceDefault；UTC只接有界整数epoch millis。Point使用真正D3点格，typed显式domain或可见源首遇顺序、padding/align/round/reverse及ordinal窗口；Y默认top→bottom，与Nivo pointScale一致。`ScatterAxes`可分别配置top/right/bottom/left；轴、glyph和sites共享解析映射。Point轴当前接domain和tick_count，不把数值tick_values/calendar猜成字符串类别。

`ScatterNodeShape::Circle { diameter }`、`FilledGlyph { area, .. }`、`StrokedGlyph { size, .. }`显式表达直径、几何面积、D3 nominal参数。`ScatterSize::Linear { values, sizes, clamp, missing }` 使用实际线性尺寸映射，默认外推，constant域取中点；missing可Error/Skip/Fixed。size0仍为实际退化几何和logical site，负数/溢出/正尺寸不可表示Err。stroke-only需显式可见stroke且无fill/paint，节点颜色覆盖实际stroke；图例代表色、Site和InteractionSite均取实际ink，不能用不可见fill色代替。

`ScatterLayout`含Scene/plot/sites/anchors/x_scale/y_scale/全源metadata。已映射未hidden节点不因中心在plot外而预删；glyph真实相交部分由`Mark.clip`裁切。Exact检测真实fill/stroke/空隙，Nearest以glyph origin为显式站点、查询限制plot，并尊重当前opacity/clip。Path支持点来自工厂的anchor_indices，不使用bbox中心或透明hit代理。尺寸Skip与显式Point domain遗漏只留源metadata；类别Y在Scene有限标量中兼容0，真实类别保存在ScatterSite.raw_y，binding.value为None且label含formatted XY，不输出内部像素适配值。

`ScatterLayoutOptions.node`是全局样式，`series_styles`为完整系列override；`ScatterSeries.label/color`和`ScatterDatum.label/color`提供源别名及颜色。Filled/Circle的颜色优先级为node color→style fill→series color→Theme，stroke-only用node color→显式stroke color；图例使用该系列代表ink色。x_format/y_format与轴自身format可分开配置，datum/point标签归属原series；隐藏重排删除sites和owned labels，完整源图例仍可用于恢复。文字不自动测量或避碰。

`interaction_binding()` / `Into<SceneInteraction>` 与现有Browser/Surface共用；`viewport_controller(limits)`从实际域与range推导反向，`viewport_scale_options(request.domains)`保持typed类别域、回填ordinal window/固定连续域，并关闭重复nice。`DomainViewport::with_y_max_zoom`与原X cap独立，单类别Y不再阻止X缩放；continuous constant/空Point域不可导航并明确Err。该布局最多256系列/20000源节点，series/node ID最多256字节、raw类别/文字1024字节，源文字及各显式类别域各限4M字节，真实Scene的200k路径/16384渐变stop及label/mark预算继续前置检查。

专项8all/8no、独立9all/8no（CPU1）及Ycap3all/3no通过；9profiles/76points/18domains来自实际Nivo/D3执行，182静态Chrome/Firefox通过。共享Circle0 typed Exact/Canvas差异修复后，正式pipeline已实测841项Rust+9doctest=850、no-default733；较前轮813项Rust新增28项。四consumer严格/release/bindgen及native Apace均通过；Rust1.95 native full/minimal all-targets check亦有通过记录，最终小分支后的日志刷新待根验收。动态六case×两phase、72入口、最终host端帽专项及114矩阵仍待最终包结果；不将已归档822/707/176/111基线当新接口验收。

## 已实现的分层

前一完成检查点新增 `scale_options.rs` / `line_scales.rs` 与 `glyph.rs` / Geo glyph adapter；正式原生813+9=822、no-default707和四consumer流水线通过，176画廊正式Chrome及三家新专项、66导航/32extras/Chrome五CLI组合已通过；111矩阵在同一次fresh完整运行全部通过并已归档。更早778/no-default668/164/102作为历史保留，不视作本轮Scatter接口的验收结果。

`ResolvedContinuousScale` 分别保留 source domain 与 nice 后 display domain；reverse 作用于 range，round 使用 ECMAScript 负半值规则，invert 使用未舍入的连续映射。log base 真正驱动 ticks/nice，SymLog 的 ticks/nice 在原值域生成；UTC 使用有界整数 epoch millis。`line_layout_with_scale_options` 将同一解析尺度交给实际轴和data/site，`viewport_controller` 采用 canonical domain，窗口重排不重复nice。`PointScale` 采用零bandwidth格，numeric/string CategoryKey 身份分开；`point_line_layout` 保存原source index、类别raw和格式值，显式遗漏类别成为gap，窗口是点格的仿射投影而非Band槽位。旧入口不自动改为新profile。

`glyph_geometry` 返回真实 `Geometry`、D3 origin、几何bounds、paint_mode及Path支持点indices；Filled七种size表示面积，Stroked七种使用原D3感知公式。checked `glyph_mark` 拒绝fill/stroke契约错配、伪造metadata和正尺寸不可表示。size0保留D3退化命令；typed Exact排除无墨迹的零长Butt glyph，Round/Square端帽保留实际墨迹，旧无binding拾取策略不变。

单点退化stroke是显式跨后端profile：零半径Circle、宽高同时为0的Rect，或全部Move/Line/Cubic支持点重合且含Line/Cubic或Close的Path，由 `render::point_stroke_footprint` 得到实际Round/Square cap区域并一次fill。`M Z`属于闭合零段，仅Move不属于；任意bbox零面积、但支持点不重合的路径不冒充单点。Butt、zero alpha/width或off dash得到空墨迹，既有Scene stroke-width准入不变。static/retained SVG、Script/typed Canvas、CPU与typed Exact共享此判定，避免原生SVG/Canvas对零长度dash/closed path处理不同。空/全零dash为solid；按SVG2理想stroke端点规则，零长度 `[3,3]` offset0/1/3/6为on，4/-1为off；`[0,3]`仅周期边界为on。

原Scene geometry、业务ID/value、raw metadata、anchor、clip、opacity保持；仅实际paint表示可改为footprint。因此不能拿零尺寸Mark导出的SVG `d`仍为全同点作为不变量，应分别验证源Geometry与实际墨迹。多段重合的单点路径不重复叠色。普通正长dash/path继续原管线，bareScene legacy hit不自动改为typed Exact。修复专项7all/6no与semantic Widget1all/1no通过；后者原有pure-dash展开已正确，无需修改生产Widget。Chrome/Firefox canonical staticSVG/ScriptCanvas各60row、每row3cap，分别180次cap比较mean0；这是独立canonical证据，最终host专项/矩阵仍待重验，原生引擎观测不是互相等价的golden。

`InteractionBinding.anchors` 显式关联Path拓扑和支持点；构造时验证支持点均值确为请求origin，动画时按同一支持点的当前位移恢复anchor，不能用BBox中心替代Triangle/Star/Wye的D3 origin。Exact仍对真实glyph轮廓拾取；Nearest/slices才使用该anchor定位。Line/Radar均直接使用工厂Geometry，不附加透明proxy冒充Exact。GeoGlyphLayer也提供同一binding，nullable raw与Scene兼容标量分开保存。

跨shape默认Fade用`SurfaceModel::set_interaction_frame(T,binding,R)`：先验证目标业务Scene T的binding，再与当前物理帧R原子安装。同ID Circle/Star/Wye/Triangle可合法切换，typed logical focus/selection/controls在enter0仍保留；physical hover/Exact继续要求当前墨迹opacity>0，退出ghost不生成业务controls。PointMesh::from_scene_at按显式anchor建最近点格，pointer另检查真实clip/path，不因anchor位于当前裁剪外就预删一个仍有可见墨迹的glyph；旧无binding路径不变。normalized/canonical sampled morph若与目标anchor拓扑不兼容仍Err，不猜索引对应。

`GeoMapLayout::add_glyphs` / `GeoProjectionContext::add_glyphs` 在clone候选上复用原投影，返回GeoGlyphSite对应的actual mark ID与GlyphGeometry。域/palette按完整源确定，hidden只过滤新glyph及其owned labels，不重拟合或删除旧地图；实际大小/颜色图例使用同shape/rotation/paint_mode。新Path投影裁剪使用真实mask保留凹形和原anchor，边界stroke不被重造；零半径Geo marker沿用旧省略规则。所有新命令、mask、label和legend仍计入Scene总预算，拒绝不修改原Scene。

| 层 | 文件 | 职责 |
| --- | --- | --- |
| 场景契约 | `src/scene.rs` | 配置、主题、矩形/圆/路径、文本、图例、有限值和预算校验、命中与插值 |
| 比例尺 | `src/scales.rs` | Linear/Log/SymLog/Band、反向范围、invert、ticks、clamp、有界D3稳定步长nice |
| UTC 时间 | `src/time.rs` | epoch millis、逆映射、UTC 日期格式化、按日历对齐的时钟/日/周/月/季度/年刻度 |
| 日期 locale / 时区 | `src/datetime.rs` | 编译日期模板、Gregorian/ISO 周、显式 locale、UTC/固定偏移、可选静态 IANA TZif/POSIX 规则 |
| 轴与裁剪 | `src/axes.rs`、`src/clipping.rs` | 实际图表比例尺/刻度/格式、鲁棒线段边界求交与共享 plot clip |
| 数值格式 | `src/format.rs` | 有界 specifier 解析、显式 locale、分组/货币/数字替换、数值输出预算 |
| 图形样式 | `src/paint.rs` | 线性/径向渐变、显式stop/坐标预算、内容确定性基础ID与SVG实例namespace；bar/combo真实圆角与border |
| 堆叠与 mesh | `src/stack.rs`、`src/mesh.rs` | order/offset、Wiggle、KD-tree、Voronoi/slices |
| 图层与动画 | `src/layers.rs`、`src/motion.rs` | 命名自定义层、任意混排、显式时钟弹簧与进退场 |
| 帧时钟 | `src/clock.rs` | FrameScheduler / FrameDriver 票据契约、取消与过期帧处理、ExplicitClock |
| 地理/日期/力模拟 | `src/geodata.rs`、`src/geo.rs`、`src/calendar.rs`、`src/network.rs` | 可选解码、球面投影/裁剪、多年日期、Barnes–Hut |
| 笛卡尔布局 | `src/cartesian.rs` | 折线、面积、柱、散点、热力、流、bump |
| 独立 typed Scatter | `src/scatter_layout.rs`、`src/scatter_layout/axes.rs` | 业务node身份、typed XY、连续/UTC/双Point与四侧轴、尺寸单位、实际glyph/anchors、全源metadata及导航 |
| 图表样式适配 | `src/line_styles.rs`、`src/pie_styles.rs`、`src/radar_styles.rs`、`src/sankey_styles.rs` | 复用布局和比例尺，生成真实圆角/闭合曲线/点/标签/渐变；显式角色、ID和归属 |
| 显式饼图几何 | `src/pie_layout.rs`、`src/pie_layout/pie_arc.rs` | D3/Nivo pad 分配与 asin 补偿、sector fit、带签名的测量引线候选与事务放置 |
| 当前数据定位 | `src/crosshair.rs`、`src/radar_interaction.rs` | Line/Radar typed site bindings、11种crosshair、真实半径/角度扇区与多行tooltip |
| 图关系集合 | `src/relations.rs`、`src/relations/charts.rs` | 显式raw identity/邻接/归属、Tree/Network/Chord/Sankey关系集合与受控paint |
| 数据颜色与数值图例 | `src/color_scale.rs`、`src/color_legend.rs`、`src/color_charts.rs` | Sequential/Diverging/Quantize/Quantile/Threshold、unknown/常值；真实连续/离散图例，Geo/Calendar接入 |
| HeatMap 与 Bar 配置 | `src/heatmap_layout.rs`、`src/bar_styles.rs` | 可缺失单元/关联集合/形状/轴，柱band/标签/totals/颜色和稳定身份 |
| 公共极坐标 | `src/polar_axes.rs`、`src/radial_bar.rs` | 半径/角度ticks和实际轴网格、文字旋转；ring内按类别沿角度累加的RadialBar及metadata |
| 变厚排名带 | `src/area_bump.rs` | 逐x值排序、共同厚度比例尺、Linear/Smooth闭带、Connect/Gap与slice/series metadata |
| 层级展示与导航 | `src/hierarchy_layout.rs`、`src/hierarchy_focus.rs` | 保留原tilers/FrontChain的padding与样式、真实子树投影/重排、ancestor/back与事务数据替换 |
| 紧凑日历 | `src/calendar_layout.rs` | Compact/FullYears、nullable civil日期、周起点/文字/真实月边界、显式zone聚合 |
| 样本统计配置 | `src/boxplot_layout.rs`、`src/bullet_layout.rs` | nullable样本分组/R7/Tukey/离群点；共享或逐行域、相邻端点区间、横纵轴和归属 |
| 独立厚度 Marimekko | `src/marimekko_layout.rs` | 显式category thickness与维度矩阵、stack offsets、nice、四侧轴、实际中心标签、归属与原值/端点metadata |
| Delaunay 拓扑 | `src/delaunay.rs` | Spade/robust后端、重复点ownership、三角形/半边/凸包/邻居、均匀fit Scene |
| 地理线流与采样 | `src/geo/path.rs`、`src/geo/stream.rs` | 有界投影弦误差细分、minor great-circle线流、major/minor经纬网与原context图层追加 |
| 混合笛卡尔 | `src/combo.rs` | 统一 x/双值轴、柱线面积散点、堆叠/槽位、稳定系列及点身份 |
| 属性动画 | `src/motion/tracks.rs` | 独立物理属性轨道、速度保持、进入/退出/拓扑淡入淡出及有界临时场景 |
| 地图符号 | `src/geo_symbols.rs` | 共用底图投影上下文、非负量面积/半径映射、实际绘制颜色/大小图例 |
| 扩展布局 | `src/advanced.rs` | 保留旧面积守恒Marimekko、SwarmPlot、PolarBar |
| 径向/层次/流程 | `src/radial.rs` | pie、radar、waffle、层次、DAG Sankey 等 |
| 其他布局 | `src/specialized.rs` | 网络、弦图、地图、箱线、bullet、平行坐标、日历 |
| 渲染 | `src/render.rs`、`src/render/browser.rs` | 独立 SVG、Canvas trait 回放、Rust/WASM Canvas 适配器、安全脚本回放 |
| Glory 接入 | `src/widget.rs` | 唯一 Widget 契约、Cage/Bond/Vary、逐帧准入、keyed 标记、图例和可访问数据控件 |
| 交互与注释 | `src/interaction.rs`、`src/annotations.rs` | renderer 无关选择/隐藏、坐标转换、注释、缩放/平移/框选 |
| Plot 模型 | `src/surface.rs` | SurfaceModel 事务式拾取、选择、视图变换、拖动/框选、索引和输入校验 |
| 数据视口 | `src/viewport.rs`、`src/category_viewport.rs`、`src/browser/viewport.rs` | 连续/分类窗口、独立X/Y、锚点限幅、固定轴和动态plot、事务式浏览器回调 |
| 图例 / tooltip | `src/presentation.rs`、`src/presentation_layout.rs` | 安全文本模板、测量布局、八方禁区避让、优先级label、当前数据Bond驱动的自定义Widget |
| CPU 栅格 | `src/render/raster.rs`、`src/raster_widget.rs`（`raster`） | tiny-skia路径、显式字体LTRshaping、RGBA/PNG、Glory Image桥及一致帧准入 |
| 语义旋转字形 | `src/text_outline.rs`（`raster`） | 显式字体shaping与outline Graphic、完整glyph/holes预算、保留原Scene/slot/安全文本 |
| 当地刻度 | `src/zoned_time.rs` | civil边界反解instant、字段对齐、DST gap/fold策略、轴tick_calendar |
| 浏览器宿主 | `src/browser.rs`（`web` + wasm32） | BrowserChart 的直接 SVG/Canvas、真实 Glory 外围控件、RAF、ResizeObserver、有界事件队列与清理 |

常见图表入口返回 `Result<Scene>`，typed layout 入口返回包含 Scene 和明确 metadata 的 `Result<...Layout>`。失败在输出前报告，不用 NaN 或空占位图掩盖无效输入。布局层不做 I/O、DOM 测量、全局调度，也不读取外部随机源；浏览器功能由显式 `web` feature 提供。相同布局输入得到相同输出，可直接单元测试、SSR、导出 SVG，或交给其他 Canvas 后端。

FillPaint以Scene逻辑坐标定义线性/同心径向渐变，覆盖Mark.fill；每个渐变支持2..512个停点，端点为0/1，单场景累计预算为16384个停点。基础定义ID由规范化float bits和色值组成（-0归一化），不使用hash或全局计数；同一SVG owner内复用，retained SVG退休未使用定义。Canvas使用真实gradient，CPU使用tiny-skia shader。能力准入在begin前完成；语义Graphic不支持shader时明确拒绝，可改用raster。样式定义变化走Fade，不伪造shader插值。圆角bar用四段cubic，先保留真实端点再clip；极值超出时只在viewport外保留足够圆角/描边余量，不在裁剪边伪造cap。

HTML中的SVG定义ID按文档解析，不能依赖每个内嵌SVG天然隔离。独立文档继续用 `render::to_svg`；多实例inline导出使用 `to_svg_with_namespace`，由调用者选择唯一的有界ASCII namespace。只有生成的gradient/pattern/clip定义及typed引用加前缀，原始业务ID、label/title和文本中的 `url(#...)` 不改写。BrowserChart retained SVG由每个owner建立自己的作用域，同实例内容缓存/相同帧零Mutation和节点身份保持不变，其他实例的隐藏、defs退休或Stop不应改变本实例绘制。

`Scene.draw_order` 为空时默认 marks→labels；显式 DrawItem 顺序可任意交错几何和文字，所有渲染与命中遵守同一顺序。LayerPlan 提供事务式命名层与自定义计算。标签归属由 label_series 显式声明，不根据标签文本猜测。Scene.legend 属于元数据，Glory Widget 渲染在外围，SVG export 的描述区包含其标签。`LegendOptions` 控制外围排列、换行、对齐、间距、项宽、符号和文字样式；它不自动生成 SVG export 的图例几何。GeoMapLayout 的大小/颜色图例则明确生成实际 Scene marks/labels。

`TooltipOptions` 用 `TooltipPart::{Text,Label,Series,Value,Id}` 构造纯文本行，支持标题、系列别名、ValueFormat、字形/背景/padding 与条数限制。Widget 的 `.tooltip_marks(...)` 可接受受控 ID 列表；BrowserChart 按当前 hover/slice ID 生成多 datum 文本。`TooltipDatum` 允许 None，分别执行 HideTooltip/OmitRow/Placeholder；非有限 Some 值拒绝，Scene marks 仍始终有限。每 datum 最多16模板行、每行32片段，最多显示256 datum、单行1024字节、总文本16384字节。字符串始终作为安全文本；自定义 renderer 返回 Rust Widget，接收当前数据 Bond，并保留外层图例按钮/受控状态/清理。

`layout_legend`、`place_tooltip`、`layout_labels` 是无I/O有界纯布局，输入明确测量与矩形，不用字符数冒充字体测量。Chart 的 text_measurer 接入语义布局；直接SVG/Canvas/raster可显式预处理Scene标签。Browser tooltip 使用实际DOM矩形、八方候选、禁区与viewport交集，超大内容限制宽高并可滚动。复杂布局或禁区无解时返回最小重叠候选/隐藏标签，不声称所有输入可零碰撞。

## 当前 typed 几何与定位协议

`PieOptions.style` 保留真实环形圆角、独立填充 alpha/边框、显式 `active_ids` 与内外半径偏移、弧标签和引线。旧 `pie` 的角度式 padding 与默认输出保持不变。新增 `pie_layout` / `pie_with_layout_options` 接收 `PieLayoutOptions`：`PiePadding::D3Applied` 用实际分配 pad，`NivoRequested` 用原请求 pad，两者按内外半径分别 asin 补偿，可选 Auto/Fixed pad radius；`LegacyAngularGap` 明确保留旧语义。新默认是 NivoRequested + NivoSector，不能据此改变旧入口默认。`PieFit::NivoSector` 按包含中心的完整基础扇区和精确 cardinal 极值拟合，不是 tight annulus，也不复制 Nivo 整数角四舍五入的 overshoot。零值会参与新模型的 padding；fill-only 塌缩弧不造 ink，显式 border 保留真实径向线。

`PieLayout.link_candidates` 提供当前文字、leader 与签名；应用用真实字体测得相对锚点/旋转后的 bounds，再调用 `place_link_labels`，有界贪心移动或隐藏文字及对应引线。应在应用自行给公开 `scene` 加层或改样式前完成放置；已编辑Scene后再次place显式拒绝并保帧，不能静默丢弃调用者改动。缺失、过期、坏测量同样拒绝且保留旧布局；不按字符数伪造测量，不承诺全局最优或引线全无交叉。Browser 示例使用实际 Canvas text metrics；静态画廊只展示几何/交互状态快照，不声称已测量。

`line_layout_with_style` 返回 `LineLayout { scene, plot, sites, slices_x, slices_y }`，含源 index、raw x/y、独立格式化坐标和当前像素位置；`.interaction_binding()` 与 RadarLayout 共用 `InteractionBinding`。`CrosshairType` 支持 X/Y/Cross 及八方组合共11种，输出真实可裁剪/dash 的 Geometry；SliceX/SliceY 强制全高/全宽线，普通 point 使用配置形状。

`radar_layout(..., &RadarLayoutOptions, &RadarInteractionOptions)` 进一步返回 `RadarLayout`：center/radius/domain、每个原轴的稳定 ID、起止角/tip、按原始值排序的多系列 rows、raw/formatted 值、颜色及 point mark ID。默认值降序，同值反转输入系列顺序，与实际 Nivo RadarSlice 一致；可选升序或源顺序、顺/逆时针与 spoke 样式。`PolarSliceIndex` 按 atan2、半径和 directed half-open 扇区判定，不借用 nearest；圆心确定选第一轴；键盘按轴导航，单值选择使用该轴当前首个可见排序成员，tooltip保留整轴多行。`points=false`以零半径透明站点服务slice/keyboard；旧shape入口继续原decorative glyph/site模型。显式`points.glyph=Some(...)`则将真实glyph本身绑定typed anchor，`slices=false`启用实际Path Exact轮廓拾取；旧radar/radar_with_options默认保留。

`radar_layout_with_value_scale` 让实际网格、点、面和类型化锚点使用同一解析比例尺。网格环可以由值轴刻度给定，也可以等距放置并反算原始数值。显式 Log 域拒绝零和跨零值域。SymLog 按实际投影绘制零环，原始值和轴顺序继续用于提示和键盘导航。

`BrowserOptions.interaction_data` 原子绑定 Scene 与当前 typed 数据；`set_scene_with_interaction` / `animate_to_with_interaction` 更新同一候选，`set_layout(SceneInteraction)` 还可一起更新 plot。`on_resize_with_interaction` / `on_viewport_with_interaction` 及动态 plot 变体返回 `SceneInteraction { scene, interaction, plot }`，避免尺寸、轴域和元数据分离提交。普通 Scene setter/callback 清除旧 binding；动画使用已接受目标的业务值/格式，坐标与 overlay 从当前 Scene 读取，ghost 不参与 business picking。Transform 模式只逆变换一次到世界坐标；Relayout 使用当前已布局坐标。Polar binding 自动聚合多行 tooltip 与中心到轴 tip 的 spoke，`polar_spoke=None` 可关闭。元数据不是对旧 Scene 的不受校验补丁：绑定 ID/series/value/站点位置、成员、预算都须匹配。 目标/union在排队前被拒绝时，旧running motion/clock/model保持；末段逐帧decorated预检失败则恢复上一已接受Scene、hover、binding、focus，取消当前动画及live gesture/captures，避免无穷RAF重试，不让已终止contact复活。

`RelationGraph` 用显式 raw node/edge ID、mark/label 归属和邻接表建立关系；`RelationLayout` 维护不可变 authored Scene，`set_focus` / `set_hidden` / `replace` 在候选通过后重算当前集合与 paint，避免透明度反复相乘。`RelationOptions::tree/chord/sankey` 与 `RelationMode::Neighbors` 分别提供 Tree 独立祖先/后代节点/边开关、Network 直接邻居、Chord incident 和 Sankey 同向平行边语义；可选 Visible/ThroughHidden traversal。`tree_relation_layout`、`network_relation_layout` / `ForceSimulation::scene_with_relations`、`chord_relation_layout`、`SankeyStyledLayout::relation_layout` 复用原布局。它们不靠 series 文本推断关系；应用把当前 `scene()` 交回宿主，Browser 示例设置 `BrowserHighlight::Controlled` 保留关系 paint，并真实监听事件。Network adapter将edge的Scene系列明确归属raw source，使真实Surface pointer/keyboard可达；双端incident隐藏由Graph负责，普通source系列过滤不能替代它。关系隐藏移除 ink，不重新分配原几何；Node/Edge 两种 focus 的含义按 mode 明确，focus 被新树删除时清除。

Browser的旧/typed Scene setter、layout和resize/viewport在提交前还复用`validate_motion_scene`验证当前PropertyMotion/ContourBindings准入。`SurfaceModel::set_layout_for_motion`是显式辅助入口；直接原生调用者须主动使用并管理时钟，现有native-raster consumer没有自动接入这一新增辅助接口，不能由Browser验收推导已接入。

Owned overlay准入同时检查目标与真实动画enter/exit union，而不只看目标mark数。`crosshair::overlay_scene_admission`在候选Scene副本上按full frame两条Cross保留原stroke/dash，补齐draw order后执行完整`Scene.validate`，包括累计dash展开/work预算；单独合法的业务dash和overlay组合也可能超限，必须提前拒绝。逐帧再对克隆Motion/Model的完整decorated Scene预检，通过后才推进Model/Cage发布；目标/union预排队拒绝不入队并保旧running motion/clock/model；末段帧预检失败恢复已接受Scene/Cage、binding、hover、focus及绘制，同时取消当前animation与live gesture/captures。Geometry Transform视图仍为既有min1/max100配置边界，不能以scale0.01构造不可达的world放大反例；本轮没有改变该边界。原生3项unit覆盖tiny dash、合法mixed order保留和累计预算，不据此宣称新的性能/FPS结果。

## 响应式更新与资源归属

应用维护业务数据并重新计算 Scene；语义 `Chart` Widget 观察 `Vary<Scene>`。新场景先做完整验证，含 Graphic 单路径预算，再发布给内部 accepted Cage。非法更新不替换 accepted frame；`on_reject` 接收具体错误。没有注册回调时通过 Scope 报错。

图形按 mark ID keyed，更新数据不依赖数组索引恢复身份。文字按显示槽位更新，不承诺跨重排的文字控件身份。Scope 负责绑定和回调清理，不保留已卸载图表的数据历史。当前每次场景发布会复制完整已接收 Scene；PointMesh 提供显式重建的 KD-tree 空间索引，但没有宣称支持百万数据点的增量布局。

`BrowserChart` 通过 `kartis/web` 在应用拥有的空容器中直接创建 SVG 或 Canvas surface，并在独立子容器挂载 Glory 图例/数据控件。它是有生命周期的浏览器控制器，不是新的 Glory 语义 Canvas 节点。原生监听器只入有界队列，RAF 执行模型更新、绘制及应用回调；弱 BrowserHandle 支持回调中的更新与停止。stop / Drop 清理 RAF、监听器、ResizeObserver、指针捕获和所属控件。直接 SVG 保留根、标题、描述、背景与绘制组；路径以 mark ID、文字以 slot 复用，逐属性比对后才修改 DOM，按实际 DrawItem 顺序移动已有节点。裁剪切换只调整所属 viewport，路径身份不变；退场删除缓存、重进创建新节点，不保留历史DOM。

`BrowserChart::mount_with_plot_widgets` 在输入图面旁挂载独立 Glory 覆盖层。测量矩形和类型化数据锚点使用已接受的布局，随相机、动画和重排更新。矩形裁剪同时约束显示与事件，部分裁剪的控件进入 inert。浏览器按场景宽高分别映射到图面的 CSS 宽高。发布时释放控制器状态借用，允许子控件回调重入或停止图表。数据退出后清理对应插槽，图例隐藏则保留同一插槽。

浏览器路径裁剪将数据标记的蒙版投影到插槽包围盒，并使用图表独占的 SVG 定义裁剪 HTML 控件。偶奇填充保留孔洞。完全位于填充内部的矩形保留输入。与曲线误差带相交的插槽采用保守判定，保持显示并暂停输入。部分路径裁剪的帧矩形是保守包围范围，实际墨迹按路径绘制。

动作队列最多256项，resize和系统reduced-motion分别有一个合并配置槽；snapshot.pending包含这些待处理项。媒体偏好在RAF排程成功后才提交槽值，并在动作前消费最新值，避免满队列丢配置后永久停留在旧策略。stop/scheduler也检查和退休该槽。浏览器Pointer/Wheel从真实JS Number读取f64 client坐标，避免legacy i32 getter把亚像素几何漏掉。

容器尺寸是布局输入。BrowserChart 的 `on_resize` 在 RAF 中调用应用布局函数，拒绝保留旧帧；低层 host 自行接 resize。DPR 只决定 Canvas backing store，不进入业务坐标。`SurfaceModel` 提供几何、最近点和 X/Y slice 拾取、选择、隐藏、滚轮缩放、拖动平移与框选的可移植状态；浏览器宿主负责实际事件接线、tooltip 和键盘控件。默认 `SurfaceViewMode::Transform` 将轴、文字与数据一起变换；可指定 plot，默认覆盖整份 Scene。立方曲线路径命中用 24 段细分，不是解析几何精度。

固定轴使用 `DomainViewport`，仿 D3 rescaleX/rescaleY 将 plot-local 手势像素逆映射到原始连续比例尺，保持原始域、当前域和连续窗口分别存储。Transform/TransformXY 是原始帧绝对变换，Zoom/Pan/Brush/ViewportGesture 是当前窗口增量操作；Reset 恢复原始域，resize 保留窗口。X/Y 独立导航，左右值域共用 Y 窗口，先分别限幅倍率再按真实锚点和中点平移应用手势。控制器使用严格升序、非常量的canonical Linear/Log/SymLog/UTC域；UTC最小一毫秒。ViewportDirections表达实际range反向，布局helper从源域/range推导方向；退化/不可表示连续域明确拒绝。独立with_x_max_zoom/with_y_max_zoom限制ordinal轴，不让一轴到界阻断另一轴。CategoryViewport 用完整分类 ID 与索引空间域，BandScale.windowed/bar/combo 映射分数窗口，分类 X 上限独立于 Y。

`BrowserChart::on_viewport(controller, axes, layout)` 注册显式重新布局回调，优先于 on_resize。回调收到 `BrowserViewportLayout { width, height, plot, domains, hidden }`，应用按这些值重新运行 line/combo 等布局，让刻度和 clip 保持固定位置。`SurfaceViewMode::Relayout` 保留手势状态，但绘制/拾取使用新 Scene 坐标，不再重复套用 zoom；`BrowserHandle::refresh_viewport()` 用当前窗口重算业务数据，snapshot.domains 暴露已接受域。回调在控制器 borrow 外运行；场景、域、hidden 和手势候选通过后一起提交，失败保留已接受帧，终止事件仍退休对应手势。重新布局取消场景动画并直接发布。

浏览器绑定要求 bounded 且 data zoom extent 在 1..=100；Both 可保留非等比窗口，request_viewport 可程序化单轴导航。snapshot.view 只是虚拟标量输入基线，实际域以domains/controller为准。虚拟输入允许宽范围，不让一轴到界阻断另一轴；DomainViewport 独立约束真实域。on_viewport 固定insets，on_viewport_with_plot 每次解析动态plot；当前输入先按已接受plot解释，resize再保留所解domain。两个应用回调均在borrow外运行，过期成功/错误结果都丢弃。回调仍由应用负责数据访问和重建，不从Scene反推业务模型。

TouchStart/Move/End/Cancel 与鼠标协议互斥，最多保留两触点。pinch 使用初始中点的世界坐标和两点距离比，同时支持中点平移；结束一指后按剩余位置重建单指 pan 基线且禁止该手势变成 click。浏览器接受 nonprimary 第二触点，各自捕获，第三触点忽略。丢失捕获、队列溢出、尺寸变化/归零、停止和销毁都会退休相应手势；尺寸变化独立于布局回调是否存在或成功。

## 日期格式与时区边界

`DateTimeFormat` 构造时编译D3风格directive和locale模板；TimeLocale包含d3-time-format4.1.0固定快照的全部34官方locale（来源/commit/ISC许可随typed数据保存）及自定义数组。918条独立D3输出oracle覆盖全年月份、weekday、period与Unicode。旧zh_cn()便利构造保留原输出。Gregorian/ISO周、padding、epoch按整数日历算术计算；%Y/%G保留扩展有符号年份。模板1024字节、展开512tokens/8192扫描字节、深度8、输出1024字节，循环拒绝；无全局locale或宿主时区读取。

parse/parse_with 严格消费全部输入并交叉验证已提供字段，包括epoch/民用日期/星期/ISO周/UW/offset，非法溢出与精度损失报错。IANA gap/fold默认Reject，可明确NextValid/Earlier/Later；显式offset决定instant。fr-CA官方空period可用于24h，只有12h且无可消歧字段时明确拒绝。CalendarInstant经指定时区转当地日期，再用Sum/Count/Mean/Min/Max按当地日聚合，不假设每个当地日都是24小时。

UTC 与分钟级 FixedOffset 默认不增加依赖，支持 ±8,640,000,000,000,000 epoch milliseconds。`timezone` feature 仅启用 optional Jiff（alloc，关闭默认 features）和 jiff-tzdb；当前锁文件为 Jiff 0.2.42、静态 IANA 2026e。实现直接以 bundled bytes 构造 TZif TimeZone，不访问系统 zoneinfo 或全局数据库；POSIX 尾规则继续推导未来 DST，包含2101年的北/南半球测试。Named 使用 Jiff Timestamp 范围（约 -9999..=9999），`iana_timestamp_bounds()` 给出精确毫秒界限，越界 Err。`%Z` 为 ±HHMM，历史秒偏移向零截断到分钟，civil 字段保留精确秒。数据库版本可查询 `iana_time_database_version()`；规则更新需要更新依赖与重新构建。

UtcTimeScale保留UTC默认边界；ZonedTimeScale/TimeCalendar按当地字段floor/offset并反解instant，通过AxisOptions.tick_calendar接轴。gap取真实transition并dedup，fold默认小单位Both、大单位Earlier，可显式更改策略。分钟/小时/day/month步长在父字段边界重置；输出上限2000tick/20000边界工作量，轴上限1000，超限报错。calendar_range_zoned同时提供当地日期数据聚合。

## CPU Canvas 与图像宿主

`RasterCanvas` 实现同一 Canvas trait，使用 tiny-skia 的真实路径/字形 outline 栅格化，输出 straight RGBA8；PNG 编码不经过 SVG 或浏览器截图。所有几何、clip、stroke、文本锚点/角度共用 DPR/view 物理变换。新 buffer 的完整 draw/finish 成功才替换已接受帧；像素上限64Mi、操作预算、字体/字形/物理坐标预算先后检查，拒绝保留旧帧。

字体由 FontRegistry 显式注册，限制32 face、单文件16MiB、合计64MiB。rustybuzz 提供字距、连字和组合字形。缺字按注册顺序回退，Error、Skip 和 Notdef 为显式策略。可选 `raster-bidi` 提供单行双向排版，CPU 与轮廓共用排版结果。跨字体的上下文塑形、竖排及彩色或位图字形仍待实现。

`Chart::raster` 用同一个已接受模型驱动Glory Image和图例/数据/tooltip控件。同步render/PNG/sourcepublication失败时保留图像、隐藏状态、options和控件；任意source回调调用前已挂ownercleanup，首次发布中销毁owner也安全退出。custom provider后续异步decode/transport错误属于host观察，不承诺回滚业务模型。默认dataURL在web真机验收，原生host需显式provider和源发布器；这里没有新Glory Canvas语义节点、原生设备窗口或pointer capability声明。

`RasterSurface`（raster）补充原生可移植宿主：owner-scoped guard、weak handle、单一SurfaceModel和已接受PNG帧，dispatch/set_scene/hidden/selection先render与publisher成功后提交。widget()提供Glory Image与受控外围控件，dispatch_host用显式image bounds将宿主offset/DPI坐标映射到Scene。它不读取字体、不加载图片、不安装系统事件或窗口。独立native-raster消费者通过应用内有界asset provider授予图像能力；Apace headless的真实document/解码/CPU绘制/输入路径与OS窗口、设备实机证据分开记录。

## 时钟与物理速度

`FrameDriver<S: FrameScheduler, M: SceneMotion>` 管理单个待执行票据、单调时间、过期帧和取消，支持 SpringMotion、PropertyMotion 或应用自己的场景动画。`ExplicitClock::from_motion` 为已开始的轨道建立零时刻，首步不会丢失时间。BrowserChart 则直接拥有自己的 RAF，默认 PropertyMotion，也可显式选择共享进度的 SpringMotion；它没有在内部使用通用 FrameDriver。默认暂停间隔钳制为 0.25 秒，ExplicitClock 可完整推进至 60 秒。

`animate_to_with_blends` 将目标场景和 `MarkBlends` 一并提交。兼容轨道沿用实际模式和物理状态。模式变化创建分属退出和进入的物理实例，各自保留模式和既有绘制顺序。目标与已启用透明度的过渡实例先通过预算检查，实际 Canvas 帧在提交前准备合成缓冲。重定向或手势中断使用当前物理帧，已经退出业务数据的实例继续参与绘制直至退场。

SpringMotion 仍用一个归一化弹簧进度驱动整帧。默认 `RetargetPolicy::PreserveProjectedVelocity` 保留兼容几何旧速度在新轨迹上的投影；Reset 可显式清零。拓扑、绘制顺序、标签归属变化使用淡入淡出并重置投影速度；绘制钳制 overshoot。`SpringValue` 提供独立物理单位的标量位置/速度，`ScalarVelocityPolicy::{Preserve,Reset}` 可用于应用组合属性轨道。它不会自动为所有图表属性分配独立弹簧，组合结果仍需通过 Scene 约束。

PropertyMotion 为每个兼容 mark ID、label slot 维护独立 SpringValue。位置、尺寸、路径各控制点、RGBA、opacity、stroke、clip、文字位置/字号/角度和视口分别推进；重定向保留每个实际属性速度，目标设为当前绘制值也不清零动量。最短旋转保留未归一化物理等价角，仅绘制时归一化；尺寸/颜色等约束也不反馈到物理轨道。不兼容形状、文字或标签归属以 ghost 退出和新目标进入；绘制顺序置换采用完整有序快照交叠。目标、时钟和临时 union 预算先在候选状态验证，失败保留旧帧及全部轨道。enter/exit/topology 可分别选择 Fade/Immediate，presence 弹簧可独立配置。

GeometryTransitions通过按ID的morphs/enter/exit显式配置几何。单轮廓弧长采样最多256点、细分最多8192点/轮廓、递归16、共享4m工作量；可选择closed循环对齐或允许反向，不推断业务语义和孔洞对应。活动采样轨道重定向保留速度，重新建立对应时明确重置几何轨道，settled恢复原目标几何。默认不授权morph，继续拓扑Fade；reduced motion直接终态。

contour_morphs提供最多32个显式MorphContour槽位，ContourBindings声明输入Move顺序；缺席槽位按声明点collapse，closed/winding角色不推断、禁止非退化翻转。canonical固定命令拓扑使重排/出现/消失仍保留真实物理轨道速度，settled回到精确目标与原顺序。Browser PropertiesWithContours保存当前帧/目标bindings；中断、reduced、rollback和再次启动均保留其对应，不将已采样Scene注入当作原生多轮廓RAF。应用resize/viewport新Scene回调采用默认声明顺序。

## P0 颜色、矩阵与极坐标契约

`ColorScaleOptions::resolve` 先校验最多20,000个源值，再确定显式或自动有限域。自动空/missing域为[0,1]，常值域保留常值，支持反向域。Sequential为2..256个等距色锚点，Diverging为3色并有数据空间midpoint，Quantize为1..256档且等阈值进入下一档；连续色按非预乘RGBA8逐通道舍入。缺失/非有限查询用unknown，源NaN仍拒绝。连续clamp=false按通道饱和外推；Quantize unclamped域外unknown是本库明确扩展，不混称D3默认。普通有限Quantize阈值保留D3加权表达式的bit舍入顺序，避免1ulp边界误分；若计算溢出、越域或非单调则整组回退稳定lerp。该极值健壮profile不宣称所有病态域与D3逐bit相等。极大跨度用缩放差值避免溢出，极窄域legend自动tick按真实代表值去重。

`resolve_with_profile` 可在作者色板上选择 RGB gamma、Lab、HCL、HSL 或 Cubehelix。HCL、HSL 和 Cubehelix 提供最短色相路径和直接路径。可选的 38 条命名色带自行提供颜色并要求钳制的连续尺度。离散量化可使用 `ColorScheme::palette` 返回的指定尺寸 ColorBrewer 色板。HeatMap、Calendar 和 Choropleth 的 `*_with_color_profile` 入口把同一份解析色标交给数据单元和图例。

`Quantile` 对非缺失有限样本排序并求 R7 分位阈值，仅保留阈值和原样本端点；
`Threshold` 接受 `ColorScaleOptions::thresholds` 的最多 255 个升序有限边界，
颜色数必须恰好为阈值数加一。重复阈值保留空档，边界等值进入下一档；非默认
插值或连续命名色带拒绝。两者的有限 `domain` 为升序图例窗口，不重写分布或
作者阈值。`bin_extent` 以可选端点保留 Threshold 的无界首尾档；按颜色反查遵循
首个相同颜色的 D3 规则。`bins` 和实际图例只裁剪显示窗口内的非空区间，不把
无界端点送入 Scene。空 Quantile 源明确映射 unknown。默认饱和映射与有限值的
D3 配置对照；`clamp=false` 的窗口外 unknown 与健壮极值插值是明确扩展。

`OrdinalColorOptions` 将类型化类别键映射到固定范围。显式域为隐藏项保留位置，数字与同名字符串是独立键。图表读取已解析的色标，应用可显式追加新键。Bar、Line 和 Pie 将结果同时用于数据颜色和图形。Bar 与 Line 的显式颜色优先。Pie 可按原始数据调用颜色工厂一次，再为描边、引线和文字分别应用继承色。对比色以实际切片填充或图表背景为基础，并计算候选文字的透明度。

`append_color_legend` 将连续色标绘成一个完整渐变，保留色板锚点和发散中心，避免半像素边界的拼接细缝。查表色带以成对停点保留原始颜色桶，量化色阶仍绘制为独立色块。未知色为装饰色片，轴和色块不进入数据交互。每份图例最多32个刻度和2..256个连续采样步，最终受Scene累计预算约束。失败保留上一份场景。它追加图形和标签并扩展显式画序。`choropleth_with_color_scale` 复用既有投影几何，所有供给feature值先定域再做投影裁剪；`calendar_range_with_color_scale` 以日期源表区分缺失格与观测0，沿用原日期和布局验证。

`heatmap_layout` 输入显式 `HeatCell`，最多20,000个有限/NULL单元，稀疏pair不补齐。raw row/column组成长度编码ID，默认lexical顺序，显式顺序须为含隐藏身份的完整排列；按隐藏集合重建时，band及自动颜色/大小域使用可见数据。Rect/Circle/圆角、padding和size variation都作用于真实Geometry；NULL采用unknown色和完整size因子，不等同0。metadata保留Option及nullable `tooltip_datum()`，Scene有限标量以0兼容、label明确missing。`related` 仅返回实际可见source cells；`set_active` 校验公开缓存结构后原子更新mark/label opacity，不动比例尺或几何。浏览器通过 `BrowserHighlight::Controlled` 接受应用的真实关联高亮，仍保留事件、焦点、选择、tooltip及清理。

`bar_layout_with_style` 同时求解分类band、分组像素gap、正负堆叠/百分比、labels及totals。颜色fallback按lexical series/category ID，不随数据重排或隐藏改变；显示metadata区分raw/displayed、true interval/band、visible rect。Net和正负分开totals使用补偿归一化求和；不需要输出的溢出聚合不拒绝独立分组柱，选中要显示的量必须可表示。totals锚在显示域真实端点，不把域外total挪到裁剪边；raw格式与percent轴分离。总数是聚合而非一个series所有，隐藏必须重新布局，普通Scene过滤不能重算它。

`PolarFrame` 约束有向span为(0,一圈]、有限中心/半径，角度允许跨零和反向。公共半径tick直接带逻辑radius、圆周tick带unwrapped angle，数值tick共用ChartScale和ValueFormat，categorical tick显式保留ID/文本。圈/辐条grid和start/end、inner/outer轴均是真路径，文字Fixed/Radial/Tangential绕自身anchor旋转。事务append按真实grid索引组成背景→旧场景→轴/文字画序，既有label归属/clip槽位不动；每方向最多1000tick，整圈重合首尾去重。`polar_bar_with_axes` 只替换旧grid/文字，不改bar几何，面积尺度的radius按平方根实际映射。

`layout_stacked_radial_bar` 的每个 `BarSeries` 是径向band，显式categories沿角度累计。默认270°、inner ratio0.3、band padding0.2，支持非负有限值、反向/部分/整圈、有界颜色/FillPaint/标签。metadata保留ring原ID、category原ID、累计值与未扣pad的角度；paint按每段对称扣angular pad，太小段可被显式pad消耗，零值不造ink。隐藏仅过滤类别ink与owned labels，完整源总量/域/track/槽位不变，与Scene图例过滤严格一致。最多256ring、256category且矩阵积≤8192；ring总和溢出或正值角度不可表示明确拒绝，最后完整Scene准入。旧单值 `radial_bar` 与径向堆叠 `polar_bar` 保留原数据契约。

## 图表样式与布局边界

`line_with_style` 先校验原始 Series，再按 `connect_missing` 筛选定义点并保存源索引表；自动 x 域仍使用原数据（含 gap 的 x），y 域、baseline、hidden 规则保持既有 line 管线。生成点、stroke、area 按已知系列序号前缀识别，再改为包含原 series ID 长度和源索引的稳定 ID，不拆分业务 ID。标签可独立于可见点生成，按最终槽位写入 `label_series` / `label_clips`。`interactive_sites` 显式生成零半径透明 Circle，仅供 mesh/slice 索引；不伪造几何墨迹。输入仍最多256系列、10,000源点，样式映射最多256项，最终 Scene 统一准入。

Pie 的 `PieOptions.style` 将圆角限制在半环厚度，并根据小角度收紧内外切圆半径；圆角与圆弧共用不超过90°分段的 cubic 表达，圆弧径向近似误差上界约为半径的0.000273。整环使用相反 winding 的独立内外轮廓，避免 border 产生径向接缝；顺/逆时针、小角和薄环均经过有限值检查。`fill_opacity` 只乘填充颜色的 RGBA8 alpha，边框保持自己的 alpha。active 内半径偏移向内缩孔，最低为0，外半径偏移后须保持正环厚。弧标签半径在内外半径间插值，引线按中角产生径向段和水平段；引线 label 为空且属于 slice 系列，实际文字显式归属 slice。slice/leader 最多512 marks、最多512 labels，仍需通过统一 Scene 预算。

`radar_with_options` 使用独立 `RadarLayoutOptions`，旧 `radar` 不变。周期曲线直接复用经过 D3 golden 检查的 closed 曲线内核，不把 open spline 用直线强行封口。Circle/Polygon 网格、辐条、刻度、轴文字、每系列 RGBA8 填充/边框和五种等面积点符号均产生真实 Scene 图形。自动域包含隐藏系列；显式域必须严格升序并包含全部源值，不 clamp 或外推。3..128轴、最多128系列、1..64层网格，先按实际曲线与点符号命令数检查20k marks/labels及200k commands；默认半径比0.85为外侧文字留空间，不替代标签避碰。

`sankey_with_style` / `circular_sankey_with_style` 在原求解器外围使用显式 link ID，返回节点/边/中心线和实际 Scene 的对应关系。Vertical 先交换 viewport 与 margins 再求解，最后转置几何、端口及回流方向；标签旋转另行配置。正向 curvature 改变真实 cubic ribbon 和中心线，回流保留原正交通道清障。Source/Target/Fixed/Gradient 颜色、独立 opacity、圆角节点/边框和节点标签均复用最终几何。DAG 输入最多2,000节点/8,000链接，环图保留256节点/1,024链接限制；默认 legend 最多256项，更大 DAG 须关闭图例，且仍受 Scene 总预算约束。无法表示的正流量投影下溢明确拒绝。

Sankey 的隐藏不参与求解，保持原拓扑、宽度、端口和 metadata。每条链接的 `series` 为 source，所以普通 BrowserChart 图例只过滤该 source 的节点、文字及出边；要同时过滤隐藏节点的入边，应把隐藏集传入 `SankeyStyleOptions.hidden` 并重新调用布局。该入口在完整求解后标记两端可见性，不重新分配余下流量。

## AreaBump、层级导航、紧凑日历与样本统计

`area_bump` / `layout_area_bump` / `AreaBumpOptions` 使用每列独立值排名和一个共同高度尺度；spacing、align、x padding、Linear/Smooth、Connect/Gap、起止标签和 active paint 都生成真实 Scene。`AreaBumpX` 区分数字与字符串类别，显式 x_order 可保留空列，默认 tie 按输入顺序，也可显式 SeriesId。metadata 提供每个 source_index 的 rank/y0/y1 和 slice total/offset；不使用 stream 的全局排序代替。隐藏先移除系列再重算厚度/排名，保留完整 x 身份和图例，应用需重布局。源值非负，最多256系列/4096列/16384输入cell；Scene命令与坐标预算继续生效。全零域给零高度，不沿用D3 constant-domain半高；singleton/全零run不制造phantom ink。普通布局有actual Nivo compute点与实际areaPoints+D3 Linear/Basis完整路径oracle，不能据此声称blendMode/图内Widget/标签避碰均已实现。

`HierarchyLayout` 同时返回 Scene 和全树 raw ID、parent/children、原depth、焦点relative_depth、当前叶总量、投影geometry及owned label slots。未选子树保留 metadata，未生成 ink；祖先/后代按需查询，避免每节点复制完整祖先路径。样式区分parent/leaf及完整per-ID覆盖，支持solid/palette/FillPaint、边框、透明度、文字格式/rotation/skip和节点mask。精确隐藏节点保留其分配及子节点几何，只移除该节点和owned labels。

`treemap_layout` 为无状态五种tiler入口；`TreemapLayoutState::layout` 额外接受 Resquarify，使用旧算法的真实行成员/方向，在padding后的content内重新放置行。inner/outer padding和Top/Right/Bottom/Left父标题是实际可用矩形保留，不是文字metadata。state提案先clone，直接子ID集合改变时局部失效，删除的父缓存退休，ratio改变清缓存；Scene校验通过才提交，focus/back可复用尚有效的行。缓存不承诺剧烈变值后的最优长宽比。

`circle_packing_layout` 复用完整树的FrontChain/Shelf和支持集包围圆，然后以焦点圆投影所选子树到viewport内切圆。同一局部packing helper保留parent-relative offsets，先算sqrt(value)/sqrt(rootTotal)、每父局部归一化，避免先除下溢或先投screen再相减丢失微小子树；两遍算法保留完整树padding的相对语义。原circle_packing走旧计算路径，默认行为保留。focus投影另有≤2000000次包含/兄弟分离检查；1e-300与最小subnormal叶、MAX兄弟、两/三叶、零/微小正/普通padding和两算法的实际成功投影均有回归。`icicle_layout` 对焦点局部宽度/深度带重新分区并支持四方向；两者都不修改Scene camera。

`HierarchyFocusController` 的focus/root/parent/back/replace先计算有效布局再提交数据、历史和缓存；历史最多128项，删除焦点恢复最近仍存在的旧祖先，再用新根，空树清空。调用者若还需宿主发布事务，可先clone controller，成功发布后接纳候选。层次节点≤8000、深度≤128；packing≤2000节点，原enclosure检查预算保持，非正packing焦点明确拒绝，treemap/icicle零焦点输出无节点ink。padding使正值矩形无面积、半径不可表示或投影不变量失败会拒绝；未保证任意微小全树节点在像素上可见，标签碰撞和导航动画不自动推断。

`calendar_layout` 使用Gregorian civil闭区间。Compact仅生成实际需要的周，FullYears扩成年panel并对齐周列；可指定first_weekday、移位weekday ticks、横纵、方/长方格、day/month/year文字及样式。每月最多31个pitch矩形经坐标压缩求真实union外边界，保留凹角、跨周分离和padding后合并；outline padding≤最小pitch/4，并非月组spacing。`CalendarCell.value`为Option，metadata的provided区分显式NULL和补齐缺失；共享ColorScale自动域仅用可见非NULL值。day ink/labels归属year，月轮廓/月年weekday装饰共享；隐藏保留panel位置。当地instant入口复用现有显式zone转换/五种聚合，无宿主I/O。最多12000日期/32年，年份1..9999，空源推域需显式端点。拾取继承Scene普通undashed stroke的width/2+2px容差；配置border的圆角附近不承诺严格逐paint像素命中，移除border后才按fill轮廓判断。

`boxplot_layout` 对`BoxObservation`的非NULL样本按group/subgroup统计，R7默认五分位数为10/25/50/75/90%，可选Tukey或Extrema，保留extrema/mean/outlier/source IDs。横纵、分组slot、独立箱体fill/border、中位/须/端帽及三种离群点都是实际几何；完整subgroup控制palette和canonical legend ID。主体与median交互值为median，outlier为当前raw样本；mean仅保留metadata，whisker/cap/border为不交互装饰，避免中位label配均值scalar。最多10000样本、每维256类、2048组合，Linear/Log/SymLog；UTC 源值和刻度使用有界整数毫秒，摘要分位数可保留小数毫秒。`bullet_layout` 的ranges/measures先排序成相邻区间，保留source index，自动补到domain max的区间source_index为None；域外片段裁剪，域外marker不生成边缘墨迹。支持Shared/PerRow域、横纵/reverse、axis/title、大小和样式，行轴几何及文字均归属该行；最多256行、各数组64项、总source parts20000。两图隐藏重排/自动域须重新布局。旧specialized与Combo预计算五数入口保持。

## 独立厚度、拓扑与地理线流契约

`marimekko_layout` 使用 category 的独立非负 thickness，维度沿 `BarSeries` 对齐；默认 Zero/none、outer0/inner3与两套nice10，旧面积守恒入口不变。padding按全部未隐藏category计数，零厚度仍进入stack/domain但不造ink。hidden_dimensions只重算stack；hidden_categories先删除列再重算厚度和offset，metadata继续映射原source index。raw/interval/displayed分开，Diverging的displayed为端点正差，Scene标量仍为signed raw；None与零不混同。类别中心轴使用真实gap后坐标，数值thickness轴保持Nivo的压缩线性坐标。输入≤20000cells/256维度，Scene预算、gradient总stops、标签字节与正厚度可表示性另行检查。

`LinearScale::nice` 按D3稳定步长最多迭代10次，包含负倒数表示的小数增量；常量、零count或非有限扩展保留旧域。D3自身decimal舍入可能使重复nice继续扩域，不能额外要求严格幂等。4项专项与644条实际d3-scale4.0.2/d3-array3.2.5查询核对；这些参考包不进入Rust运行图。

`Delaunay` 以ID排序后的sites构建；重复xy含±0归最小ID，raw_owners映射到canonical sites。Spade自适应谓词处理共线/近退化；triangles保持CCW、最小ID起点和排序，halfedges/inedges用Option表示缺失，hull与neighbors/links次序明确。共线给完整链，共圆只承诺固定输入及固定后端版本下的确定合法剖分。有限坐标必须能用同一可逆二次幂进入Spade的0或[2^-142,2^201]有效绝对值范围；过宽动态范围拒绝，禁止抖动。输入≤20000raw sites；`diagram`均匀fit保欧氏metric，原值不被屏幕坐标替换，输出还受Scene预算限制。

Delaunay图示的links/triangles是带稳定series的装饰，label为空，不产生伪0值控件或面内数据命中；canonical sites才是数据点。是否显示边、面和标签由图示options控制，与triangulation拓扑查询分开。

第十种投影TransverseMercator采用D3的+90°相机frame与稳定atanh。旧地图入口保留两度采样；`*_layout_with_path_options`与新line streams用`GeoPathOptions`显式自适应细分，默认0.5逻辑px、depth20/最高24、180000点、4m工作量。判据是quarter/mid/three-quarter到弦偏差及角度guard，不是任意投影的形式化Hausdorff证明；未收敛或预算不足返回Err。旧十种投影的 Data fit 保留 coarse 点再 refine，最终 scale 只减；新增 Conic 可使用更细叶包络，相比初始保守 coarse 拟合允许放大，仍保守覆盖曲边。Overlay 继承原 context，不重拟合底图。

`geographic_lines`、context/map的`add_lines` / `add_graticule`返回line/source/采样metadata，完整stroke受cap mask和plot clip，behind层保持原DrawItem次序。经纬网的angular precision决定平行线源折线密度，与屏幕px precision不同。line/grid允许反经线、极点和圆帽切段；近对跖单边、真正塌缩或歧义输入仍拒绝。polygon继续原simple/open-hemisphere与hole限制，不能由线流支持推导任意全球polygon都可用。

## 已交付算法与 nivo 的差异

| 能力 | 当前实现 | 仍有差异 |
| --- | --- | --- |
| 折线/面积 | D3 open/closed、Cardinal tension、CatmullRom alpha、MonotoneX/Y Steffen、BumpX/Y、Bundle beta；line_with_style 提供系列 paint、点/标签、显式缺失连接和 mesh sites | closed area 保留完整单路径以保持 winding，超过语义 Graphic 节点预算需直接 SVG/Canvas；点标签不自动避碰，Scene 总预算与坐标预算仍适用 |
| Pie | 真实 annular fillet、顺逆时针、整环/薄环、border/独立填充alpha、显式active偏移、弧标签格式/skip/半径、真实引线与归属 | 旧入口保留角度padding；新pie_layout提供D3/Nivo asin及sector fit、显式真实测量放置；非tight annulus/全局最优，active由调用者更新 |
| Radar | Linear、Log 和 SymLog 径向值轴，顺逆时针布局、网格、周期曲线与点符号，支持极角切片和原始值提示 | 每幅图共用一条径向比例尺。专属点组件工厂和全局标签避碰待补 |
| bar / stream / 双轴 / 混合 | 正负堆叠、百分比、双向、Wiggle/InsideOut、dual_axis/combo；ErrorBars/BoxPlot/RangeBand、系列样式/标签、圆角/border、稳定身份/分类窗口 | 组合最多5,000点；箱体使用预计算五数；数值柱x为Linear/UTC、y为Linear/SymLog；任意组合和完整Nivo样式props未覆盖 |
| 绘制样式与mask | 渐变/重复pattern、SVG虚线与CPU有界展开、真实NonZero/EvenOdd路径mask、完整stroke/文字裁切；retain SVG定义复用与退休 | Pattern瓦片及dash/mask展开受累计预算限制；直接语义Graphic不支持shader或mark路径mask；标签支持Rect裁剪，非矩形label mask仍拒绝，可显式raster；复杂曲线拾取有界近似 |
| 比例尺与格式 | 数值/分类、UTC、四侧轴、旋转文字；数值/日期format、parse、34locale/IANA/当地ticks；语义Chart可显式font outline旋转 | 有限输入/输出、不隐式coercion；outline为显式LTR字体、完整glyph≤256命令，不包含完整bidi/跨字体shaping |
| 数据颜色 / HeatMap | typed sequential/diverging/quantize与真实图例、Geo/Calendar接入；nullable单元、shape/size/padding、四轴和精确关联集合 | 非全部D3插值空间/内置scheme；稀疏pair不合成NULL；Generic Scene标量有限兼容，nullable tooltip须用metadata；关联高亮由应用接事件 |
| Bar / 极坐标 | bar独立band/gap、分类/系列色、标签与raw/displayed totals；公共polar轴/网格；series×category角度堆叠RadialBar | Bar聚合隐藏需重布局；新RadialBar为非负线性角尺度和angular pad，尚无全部Nivo圆角/自定义组件；不是径向polar_bar别名 |
| AreaBump | 逐列值排名与共同厚度尺度、Linear/Smooth、Connect/Gap、对齐/spacing/起止标签/active metadata | 非负值、数字/字符串x为ordinal；隐藏重算需应用重布局，完整Nivo callback/custom组件仍有差异 |
| treemap / icicle | 五种tile、状态Resquarify与ratio；parent/leaf样式和四侧header保留；icicle四方向及真实focus重排，独立事务导航 | 极端权重变化后行拓扑不保证最优比例；隐藏仅精确节点ink，标签碰撞和导航动画不自动推断 |
| circle packing | FrontChain/Shelf、支持集最小包围圆；circle_packing_layout真实focus子树投影与样式、全树metadata | 共享2,000,000检查预算、最多2000节点；只绘选定子树，零/不可表示焦点明确拒绝；全部Nivo组件未覆盖 |
| tree | 原 tree 为 Cluster；tree_with_options 提供 Buchheim/Reingold–Tilford Tidy、五方向、五种连接、节点形状/分类样式、标签位置/旋转与plot clip | 字体基线采用共同近似；任意业务节点组件与全部Nivo props仍未逐项覆盖 |
| Sankey | DAG与循环、自环、平行边；四对齐、双方向、SCC反馈排序、统一流宽、端口重排；节点/链接/文字样式、渐变、真实回流ribbon与metadata | 两端隐藏需应用重布局，source-owned Scene过滤不等同此策略；回流保留正交路线，正向曲率不作用其上；不保证全局交叉最优或节点端口附近所有ribbon互不相交 |
| network | ForceSimulation 提供 Barnes–Hut、link/center/collision；NetworkDragController 映射/抓取/热度；BrowserNetwork拥有SVG/Canvas、Pointer capture、RAF、动态拓扑替换、隐藏恢复及错误恢复 | 每RAF1..16tick；统一电荷/弹性中心/迭代碰撞，拥挤输入不保证最终无重叠；适配器仅Scene Transform，节点/边全部Nivo样式仍有差异 |
| parallel coordinates | 独立Linear/Log/SymLog/反向变量轴、横纵布局、缺失策略、曲线、格式/轴样式、分组/legend/owned labels；原API兼容 | 新API明确数字维度、无UTC；最多64维/2048行、8192轴刻度及Scene总预算；不自动推断字段语义 |
| SwarmPlot | 确定性 tangent-interval 贪心排布，不移动值轴坐标，已接受场景保证无碰撞 | 对拥挤输入保守拒绝，不保证接受所有理论上可排布的输入 |
| Marimekko | 新独立thickness/dimensions入口，五种offset、nice、四轴/标签/样式、raw与端点分离；旧面积守恒入口保留 | 厚度非负；Expand/Silhouette/Wiggle非负，Zero/Diverging支持signed；数值thickness轴不含gap位移，隐藏需重布局，全部Nivo组件未覆盖 |
| Delaunay | triangles/halfedges/inedges/hull/links/neighbors、重复点ownership和真实Scene | Spade/robust为Rust运行依赖；有界可逆坐标缩放，共圆不承诺D3相同对角；图示受Scene预算限制 |
| 地图 | GeoJSON/TopoJSON、十种既有投影及三个 Conic 家族、Albers；复杂polygon rejoin、小圆cap、Gnomonic/TM；共享context线流/经纬网/自适应弦细分，符号与文字mask | polygon开放半球与拓扑限制仍在；线流近对跖/真实塌缩Err，精度为有界采样判据；文字不沿球面变形 |
| calendar | 旧单/多年及新Compact/FullYears；firstWeekday、方/长方格、nullable、真实月union边界/文字；显式zone当地日聚合 | Gregorian年份1..9999/12000日期/32年；outline pad非月组spacing，空源推域需端点，非全部Nivo callback |
| BoxPlot / Bullet | 真样本分组/R7/Tukey/Extrema与离群点；Bullet共享/逐行域、相邻端点区间、横纵/reverse/样式/轴归属 | 样本有限且无插值UTC；隐藏重新排位/自动域须重布局；旧预计算Combo与specialized入口独立 |
| 交互与展示 | SurfaceModel/BrowserChart pointer/wheel/双指/brush/键盘；测量图例布局、tooltip禁区避让、自定义Widget、label碰撞；连续/分类窗口、非等比X/Y、动态plot | Browser bounded1..100；输入/布局有限预算，应用负责数据重建；Chrome触摸注入不等于物理触屏验收 |
| 动画 | 独立物理轨道/速度、最短旋转、presence、enter/exit、显式key多轮廓/孔洞morph、时钟/真实RAF；旧共享进度可选 | 不自动推断业务语义/孔洞角色；不保证任意轨迹无自交，采样近似、终态精确 |
| SVG / Canvas 宿主 | 增量SVG、浏览器Canvas、CPU RGBA/PNG、Chart.raster、RasterSurface显式宿主桥与Apace headless消费者 | 仍无Glory语义Canvas节点/GPU/worker；OS窗口、辅助技术和其他浏览器实机验收分开推进 |

层次数据采用 flat parent ID 结构，单根，内部节点 value 必须为 0，权重只来自叶节点；拒绝环、缺失父节点、超深层次。此约定避免内部权重与叶子权重重复计数。

## 验收与继续推进

当前验收包括数值/坏输入测试、面积与流量守恒、donut 孔洞、circle packing 包含/不重叠、路径预算、响应式旧帧保留与恢复、SSR、资源清理、WASM 编译、实际浏览器 Canvas 绘制及 SVG/Canvas 像素差异检查。编译通过不视为原生设备渲染或屏幕阅读器验收。

历史检查点包括735/no-default629/151场景/90矩阵，以及778/no-default668/164场景/102矩阵。前轮共享scale/glyph正式原生813+9=822、no-default707，四consumer/native Apace与Chrome五CLI已通过；176画廊/66导航/32host专项、三家新专项同源实测通过，111跨引擎矩阵已在2026-11-10 15:33:25.063→15:46:34.500 UTC同一次fresh完整运行通过。Rust1.95 native全features与no-default的all-targets check均通过，未运行该版本WASM或测试。准确日志与矩阵记录于[验收记录](verification.md)，不从纯算法或静态生成推导。

后续按照可观察结果推进：

1. 更多图表组合与Nivo样式配置应逐项提供真实布局和测试，现有实现不代表所有props等价。
2. 全球任意球面拓扑、文字球面clip和自动业务语义匹配需要单独定义；当前以明确有界profile与显式对应保证可验收行为。
3. 原生OS窗口、Wry、辅助技术、物理触屏和其他浏览器分别验收，不以headless或Chrome注入替代。
4. 高密度性能与[独立内存采样](memory-measurement.md)分别记录变化帧、DOM、OS工作集/private commit、实际WASM容量与显式GC对照；工作集合计含共享页重复，容量不等于活分配。完整allocator归因/显存/compositor仍待测，不外推任意数据规模/跨机器排名或60FPS。GPU/worker按用户指示暂缓。

## Normal 对象透明度与语义标签矩形裁剪

普通 Normal 标记若同时包含可见填充和描边且整体透明度介于0和1，CanvasScript、BrowserCanvas与CPU先在有界透明源层中绘制，再将 `Mark.opacity` 及目标裁剪各应用一次。这与SVG对象透明度、Glory语义包装容器一致。颜色自身的RGBA alpha保持独立，artistic blend路径不重复应用透明度。既有scratch/frame像素预算与预先能力准入继续生效；归一化点和明确零面积primitive保留单paint快路，复杂Path可保守隔离。

`Canvas::supports_group_opacity` 默认false。自定义适配器需声明能力并实现prepare/begin/end隔离协议；不能表达该语义时在begin触及目标前拒绝。此次修复会有意改变旧版逐paint alpha导致的部分Canvas/CPU像素，布局、源值、SVG和工厂上下文不变，不能把颜色配置None的兼容结论延伸为所有旧raster字节不变。原Nivo HeatMap的SVG和Canvas本来也存在透明度合成差异，颜色角色参考不等于原生跨后端像素等价。

Glory `Chart` 对 `Scene.label_clips` 中的Rect构建真实overflow-hidden矩形包装盒，文字和outlined/painted字形保留原坐标、字体、源身份与alpha；仅修改裁剪映射也会更新包装盒，None与Rect转换重建所需结构。无裁剪路径保持原结构。Circle/Path标签遮罩、mark路径mask和其他既有不支持的旋转/shader组合仍明确拒绝并保留已接受帧，用户可使用SVG/Canvas/raster对应能力。

## 月份间距与逐点文字工厂

FullYears 的 `month_spacing` 先从周轴预算中预留十二份间距，再按零基月份移动单元格、月份 union 和 bounds；标签与拾取使用同一实际几何。默认零与旧 Debug 保持一致；Compact 非零、非有限／负值、超过维度预算或剩余日格面积不足均拒绝。原有 occupied-week 策略保留。

Radar 的文字工厂借用 `RadarPointLabelContext`，其中 `index` 是轴 ID，`key` 是系列 ID，`point` 保留原逻辑位置和 paint 上下文。新入口先检查已知共享 Scene、格式、tooltip 和投影错误，实际自定义几何先运行，然后为该 owned label 求文字；返回值仍需通过文字与最终 Scene 预算。未隐藏且启用点与标签的槽位才求文字，透明或合法零墨迹几何不会自动删除 owned label。布局仅保存 String、owner、几何和 binding，不保存用户回调；旧 None 路径与错误顺序保持。
