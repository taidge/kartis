# 对照功能差距继续完善

该目标保持进行中。此清单区分已实现、已测试的推进和仍未完成的功能；不以示例数量或编译通过代替 nivo 等价性。

## 当前推进：Stream dotComponent 与绑定 Chart 锚点

- [x] 类型化真实点角色、共享原始系列、采样/堆叠元数据及独立源图层颜色；保留零值与零尺寸资格，不将空值或禁用点占位冒充组件。
- [x] 完整来源、尺度配置与实际 Stream 标记预检先于定位回调；自定义 Path 使用逻辑采样锚点，绑定 Chart 与 CPU 图像共同接线。
- [x] 原版 Nivo 函数与门禁冻结源执行、只读离线复核和八类参考篡改拒绝。
- [x] Memory 状态/身份及双 DPR PNG 发布联动回滚，明确区别真实 OS 输入。
- [x] 四浏览器八上下文、208 画面、64 组可信输入、112 次严格拒绝回滚；完整 1,643 minimal / 1,961 all / 18 doctest 与质量门禁通过。16 个禁用零高度面积边界探测单列诊断，不据此宣称逐像素等价，见[验收](verification.md#development-stream-dot-components-and-bound-chart-anchors)。
- 图层组件、自动 spring、任意 Widget 导出及原生同层输入继续保留；GPU/worker 后置。

## 当前推进：BoxPlot 逐摘要图例工厂与类型化目标

- [x] 显式Keys／ByLabel、借用String工厂、首代表去重与完整域稳定排序；显示行与Series／Category隐藏目标分开，四宿主处理重入、排队和退役意图。
- [x] 控制器缓存与显式工厂源替换；最小1392／全特性全部目标1649／16doctest、严格native／WASM、实际Rust1.95、四消费者和双语完整程序通过；152默认配置、176CPU、228完整导出、248SVG及index逐字节保留。
- [x] 原Nivo32主＋8资格真实参考；Native56帧／112CPU及448对整幅SVGImage／Canvas通过，最大均值0.1781684＜0.5，来源RGBA≤2及实际位图恢复0差。
- [x] 全新完整第四轮与独立验收通过：8上下文／224逻辑宿主／288实际mount／896阶段，5152真实图例输入、2688非法返回／2816拒绝、26272来源像素及原DOM／CSS／资源严格复核；前三轮失败完整保留，不拼接。
- [x] 双语指南和README保持portable，公开struct literal迁移与String／React／静态导出边界明确；详见[验收记录](verification.md)与[逐摘要图例指南](boxplot-legend-factory.zh-CN.md)。
- [ ] 下一项真实差距：BoxPlot `colors(summary)` 借用工厂及与ByLabel／继承色的组合。已在隔离候选准备实现、原函数参考与测试，正式合入和完整验收仍待完成；`colorBy` 的Group／SubGroup枚举已实现，不另造“任意colorBy访问器”缺口。
- [ ] 更广图型配置、D3复合投影、静态图例呈现及平台边界继续按实际差距推进；GPU／worker按用户指示最后处理。

## 前轮完成：BoxPlot 整部件指针与 Glory 坐标通道

- [x] 显式 `PaintedGeometry` binding 及真实边框／中位线／须／帽／离群点 ownership；每箱一个逻辑站点，物理 mark 与原始 outlier 快照分开，成功替换使旧 token 失效，非法候选保持原帧。
- [x] Glory 有界 `PlotPointer`、Web／Apace 接线，原 Graphic Chart、Browser SVG／Canvas 与 Raster 共同接纳 Scene＋binding；无透明命中代理，真实元素blur不取消仍存活的Window捕获。
- [x] 最小1357／全特性全部目标1603／16doctest、Glory1277＋12doctest、严格native／WASM、实际Rust1.95、四正常消费者和完整双语程序通过。152默认配置／176CPU与248画廊SVG及index逐字节相同；200对原生整幅比较最大均值0.41603＜0.5。
- [x] 全新完整四浏览器×DPR1/2第三轮及独立验收通过：128宿主／512阶段／1024非法操作／128擦除恢复与释放／32真实焦点回归，1856真实部件命中、1888来源像素、6480逻辑PNG。前两轮失败与输入资格纠正完整保留；WebKit原生client整数化与框架真实小数local分开记录。
- [x] 该后续String工厂／ByLabel及独立row→typed target已在当前推进章节验收；Keys caption Map继续保留自身契约。GPU／worker、strict compositor与真实设备边界不据此改绿。

## 既有检查点：BoxPlot 图例文字

- [x] `BoxPlotLayoutOptions.legend_labels` 按所选完整原始域配置文字，保持显示行ID、角色、颜色和原资格；None／空map保留默认，显式空文字不触发回退。
- [x] 真实controller source替换与非法键／UTF-8预算原子拒绝；原Nivo属性访问器及Lodash原函数30配置／7资格快照对照，明确重复／空文字在原按label去重模型与当前稳定身份模型间的差异。
- [x] 最小1326／全特性1569／16doctest、严格native/WASM、Rust1.95、四消费者和完整双语程序通过。128legacy＋24受控Group默认配置／176CPU输出逐字节保持，248画廊与index保持，新增128对独立SVGImage/Canvas保持原0.5门槛。
- [x] 四浏览器×DPR1/2完整第二轮及独立复核通过：256宿主／1024阶段／3072非法操作／256擦除恢复与释放；真实改名、转义、重复／空文字、Category静态与动作、Series按ID点击及Enter／Space隐藏恢复均通过。caption-only的128对完整Canvas／Raster缓冲和64对实际SVG绘制树保持；首轮可见性校准错误完整保留，未改产品或像素／回滚门槛。
- [x] 该后续逐摘要String工厂／ByLabel模型现已按当前推进章节验收，Map仍是独立Keys配置；整部件pointer保留前轮证据。GPU／worker仍暂缓。

## 既有检查点：BoxPlot Group 配色与受控关联

- [x] `BoxPlotColorBy::{SubGroup, Group}`：完整原始域分配颜色，Group 图例按实际摘要资格生成，Category 身份与子组系列／隐藏分开。
- [x] `BoxPlotInteraction`：从真实观测值构建，按同组或同子组关联；未知身份、非法因子与公开布局篡改原子拒绝，替换源数据时保留或清除已存在的激活键。
- [x] 共享 `MarkOpacityGroup` 对真实箱体部件组合应用一次透明度；独立／保留式 SVG、Canvas、CPU 与语义 Chart 接线，后者可关闭独立单 mark 弱化。索引在帧接纳时构建，覆盖最大 mark 预算的拾取与导航。
- [x] 原 Nivo 完整 hooks/getLegendData/transition 的32配置与10边界；最小1318／全特性1556／16doctest、native/WASM严格Clippy、Rust1.95、四正常消费者与双语完整程序通过。128默认场景及CPU逐字节相同，248默认画廊SVG与index逐字节相同，192对整幅SVGImage/Canvas保持原0.5门槛。
- [x] 四浏览器×DPR1/2的6配置×4宿主动态终验及独立审计完成：192宿主、768帧、2304非法操作、192擦除恢复／释放、5376来源像素、14880份PNG。正常语义发布仅对框架生成的节点／anchor身份作已验证的比较；非法更新仍为完整原始DOM/CSS/状态/资源精确门槛。Canvas采用实际位图1:1复制观测，避免直接读回改变后续绘制策略，整缓冲零差门槛未变。两轮失败和独立诊断完整保留。
- 后续图例显示文字见上方当前推进；任意属性访问与完整子部件宿主指针路由仍按[差距清单](feature-gaps.md)推进，GPU/worker继续暂缓。

## 既有检查点：RadialBar 与 BoxPlot 描边继承色

- [x] `StackedRadialBarOptions.border_color_profile`：原分类palette、独立track、Some/None/零/隐藏资格，以及按解析后alpha决定收缩弧描边。
- [x] `BoxPlotLayoutOptions` 的border/median/whisker三角色：原子组palette、各自固定Theme回退、须与端帽同色，独立于fill_opacity。
- [x] 128组真实修改前后默认场景的完整布局、绑定、SVG、Canvas与CPU PNG逐字节相同；四浏览器×DPR1/2的288对新配置SVGImage/Canvas整幅比较通过，门槛仍为0.5。
- [x] BoxPlot箱体/离群点的可选正负零宽边框统一禁用描边，原填充和正宽样式保留；真实RED→同测试GREEN，并新增真实Some(width0)宿主场景。
- [x] 最终最小1293/全特性1523/16doctest、native/WASM严格检查/Rust1.95、四消费者及完整双语程序通过。最新9场景×四浏览器×DPR1/2共288宿主/1152阶段/576非法构建/288擦除恢复与释放通过，独立核8624个PNG。完整原Nivo函数参考和默认画廊逐字节检查也通过；物理截图诊断不等于严格compositor验收。见[验收记录](verification.md)与[指南](radial-box-strokes.zh-CN.md)。
- 后续的BoxPlot Group配色、实际getLegendData参考与受控关联见上方当前推进；更宽配置仍按[差距清单](feature-gaps.md)推进，GPU/worker继续暂缓。

## 既有检查点：普通边框继承色、整体透明度与矩形标签裁剪

- [x] Bar与HeatMap的可选 `border_color_profile`，保留预工厂数值颜色、NULL/零值、固定stroke宽度和cap；未配置边框仍不启用边框。
- [x] Normal多paint的部分透明度按对象整体合成一次；Canvas/CPU复用有界隔离层，第三方Canvas能力在绘制前检查。
- [x] Glory语义Chart支持Rect标签裁剪及仅裁剪信息的动态更新；保留原文字/字形，Circle/Path标签蒙版继续明确拒绝。
- [x] 正式最小1271/全特性1496/16doctest、native/WASM严格检查/Rust1.95、四消费者和完整双语程序通过；320对SVGImage/Canvas和56帧真实Rect裁剪专项已独立复核。
- [x] 四引擎×DPR1/2的四宿主完整新矩阵通过：320组合、1280阶段、640次非法构建拒绝、320次擦除/资源恢复和释放检查；8800宿主PNG独立核验。物理Chart/SVG截图差异单独保留诊断，canonical资源与主颜色/整帧门槛继续严格执行，不宣称严格compositor通过。见[verification.md](verification.md)。
- [x] RadialBar扇段边框及BoxPlot的border/median/whisker已在后续批次实现，验收状态见上方当前推进。

## 既有检查点：BoxPlot汇总自动域与Bar分类图例

- [x] 显式SummaryQuantiles域及有限原式插值、正负零稳定排序；保留SampleExtent默认和UTC连续统计。
- [x] Bar Keys/Indexes图例及完整series_order，原始None/zero候选、palette/source索引与隐藏语义分开保留。
- [x] 共享Series/Category角色、Chart与Browser SVG/Canvas、RasterSurface类型化激活接口；分类默认静态，显式激活不修改series Hidden。
- [x] 正式最小1244/全特性1461/16doctest、native/WASM严格检查/Rust1.95、四消费者和双语程序全部通过；四引擎×DPR1/2共32宿主组合、188 PNG、32擦除恢复/32拒绝更新与64旧DOM失效检查通过独立终验。见[verification.md](verification.md)。
- [x] Bar与HeatMap普通边框InheritedColor角色已在后续批次实现，验收状态见上方当前推进；已有固定边框/工厂和标签角色不重复计新增。

## 既有检查点：Bump端点、Radar点画笔与BoxPlot双端域

- [x] Bump两端独立padding/InheritedColor，未指定字段继承共享配置，保留原端点资格和alpha。
- [x] Radar点fill/border继承颜色角色；原palette、polygon swatch与custom实际画笔代表色契约分别保留。
- [x] BoxPlot独立auto/fixed上下界；保持统计量、原非零/UTC/裁剪语义，双fixed与单侧空态分开处理。
- [x] 正式最小1215/全特性1429/16doctest、native/WASM严格检查与Rust1.95、四个Glory消费者通过；9新场景×4浏览器×DPR1/2的72绘制对照和320真实负控通过。见[verification.md](verification.md)。
- [x] 后续实现Bar分类图例的独立展示/回调语义，未把分类ID伪装成series隐藏键。

## 既有检查点：径向标签配置与 BoxPlot 分类带

- [x] RadialBar逐分类InheritedColor标签角色，保留原palette与独立alpha。
- [x] Sunburst类型化文字回调，在测量候选和签名前构建文字；空槽位与受管来源规则保留。
- [x] BoxPlot共享分类带round/align/间距及显式反向，保持None默认输出；96组原D3夹具可重生成。
- [x] 正式最小1190/全部特性1404/16doctest、native/WASM严格检查和Rust1.95、四个Glory消费者通过；新四浏览器8上下文/48绘制对照/160实际负控通过。独立终验见[verification.md](verification.md)。
- [x] 上述Bump/Radar/BoxPlot三项配置已在后续正式检查点完成，见[端点、点颜色与上下界指南](endpoint-point-bounds.zh-CN.md)。GPU/worker继续暂缓。

## 既有检查点：Bar 分类尺度与 Bar / HeatMap 工厂

- [x] `BandScaleOptions` 和 Bar `category_scale` / `category_reverse`：D3 round/align/独立 padding、共享 step/bandwidth、完整范围取整后投影窗口。
- [x] HeatMap 单元格与 Bar 单柱的 Geometry/FillPaint 工厂，保留原始值、逻辑站点、真实墨迹、受控高亮和受管来源校验；NULL 与零值分别遵守各图型数据模型。
- [x] 独立复核修复 HeatMap 标签归属和默认 NULL→零值冒充来源两个问题；Bar 默认/缩放、受控悬停及公开帧篡改拒绝已独立通过。
- [x] 正式整合版本：最小特性 1120、全特性 1331、15 doctest；native/WASM 严格检查和 Rust 1.95；四个 Glory 消费者与三组中英文完整指南。新四浏览器 × DPR1/2 × 两图族共 16 上下文全部通过，228 PNG、80 真实破坏负控已独立核验；失败首轮保留，最终成绩见 [verification.md](verification.md)。
- [x] 开发版类型化实时 Widget 数据包和 Bullet range/measure/marker 覆盖层，保留稳定子组件与当前已接受数据；完整范围见[指南](bullet-widgets.zh-CN.md)。
- [ ] 继续接入其他角色的自定义 Widget、更多颜色/标签配置与输入范围；任意原图层交错、控件快照导出及 GPU/worker 仍保留。

Bar/HeatMap owned-label 的共享 `InheritedColor` 已正式验收：该检查点最小 1141/全特性 1353/15 doctest、四个 Glory 消费者及完整双语指南通过。四浏览器8上下文/232PNG/32绘制对照/64负控独立核验通过；独立SVG仅单实色填充改用相同数值的 fill-opacity，WebKit 原失败的8帧均值为零。原失败保留，旧 retained/font/compositor 不改绿。

逐源文字 factory 与显式 opt-in 零值轴 extent 标签已合入正式源码，独立组合核心检查和双语完整指南通过；正式回归最小 1170/全特性 1383/16 doctest、四个 Glory 消费者通过，新六场景 × 四浏览器 × DPR1/2 全部通过。产物独立终验见 [verification.md](verification.md)，不沿用上一检查点计数。

## 上一轮完成记录：Conic 中点进度修复

- [x] 定位并修复近端急弯漏采盲点；仅新增 Conic 分支使用沿弦中点进度触发细分，不放宽预算。
- [x] 两个真实 Chrome WASM 二进制成对验证 9.01848→0.4967817 像素；原 D3 自身 near-pole 约 0.000485 误差单独保存。
- [x] 修复前 1126 路径/183 项已选回归及原功能证据不可变归档，明确 featureAccepted=false。
- [x] 新 Rust/消费者：最小特性 1082 项、全部特性 1285 项、15 个 doctest、native/WASM 严格 Clippy、Rust 1.95 和四个新构建的浏览器消费者全部通过。
- [x] 新四引擎：Chrome、Firefox、Playwright WebKit、Edge 各自完成 DPR1/2 的 SVG/Canvas：208 项普通/typed 用例，另有 96 次接缝、32 次绘制对照、32 次非法候选、16 次发布故障及 32 次 resize 检查（16 次合法、16 次实际发布失败）。独立核验 288 PNG、96 对原始回滚记录、144 marker 像素、480 可信输入包和 11 个篡改反例。最大绘制平均差为 0.207068，原门槛仍为 0.5；源码与 HTTP/WASM 指纹一致；新原生：正常 target 的格式及严格 Clippy 通过，headless Apace focused/full 各通过 16 case、各保留 32 PNG；独立解码全部 64 PNG，核验 3476 个精确来源颜色像素与 175 个原生源码指纹，前后源码指纹一致；新画廊：实际 246 幅场景保留旧 244 幅独立 SVG 字节不变。两张新卡通过可移植原 D3 几何/绘制检查，包括每图 761 个经纬网作者节点的 1e-9 对照、来源色与缺失值、plot/projection 蒙版及低于 0.5 的 SVG-image/Canvas 平均差。修复后旋转 Albers 可见经纬网的双向采样距离为 0.056184 和 0.495643 像素；不称原生细分逐位相等或全局 Hausdorff 保证，另以 25001 个原 direct-function 点验证已修复急弯。
- [x] 新正常回归及四 Chrome CLI：全新完整 cohort 2863059d-9ad6-4490-a2be-a0f1caf966bf 通过 183/183，Firefox、WebKit、Edge 各完成明确选取的 61 套，整个 piecewiseScales 仍排除。四组系统 Chrome CLI 独立通过，host 严格 compositor CLI 未计入。新版首轮 182/183 因 WebKit scaleProfiles 等待滚动稳定超时而保留为失败；原脚本在新旧宿主的四次独立对照未复现，原因仍未确定。最终仅采用三份完整新子报告，不复用部分通过项，也未更改断言、超时或像素门槛。
- [ ] 后续仍有 AlbersUsa、更广拓扑/配置/设备及旧严格 compositor；六项球面工具与显式 CanvasScript Offscreen API 已于后续轮次完成各自受检范围，默认 Firefox Sans 仍未通过。GPU/worker 留最后；整体目标继续进行。

## D3 分级配色

- [x] Quantile 使用真实有限样本的 R7 分位阈值；Threshold 提供作者升序边界。
- [x] 复用 HeatMap、Calendar、Choropleth 与硬边图例；支持重复阈值、缺失值、原始区间反查和有限显示窗口。
- [x] 42 组 d3-scale 配置、1,672 条独立记录，以及隐藏重算、CPU 墨迹和拒绝事务专项通过。
- [x] Chromium、Firefox、Playwright WebKit、Edge 各通过五阶段 SVG/Canvas 色标切换专项。
- [x] 后续轮次已完成 Pow/Sqrt、等值线/密度与可编辑 Brush 的受检范围；各轮成绩独立记录，图型配置接线和平台文字差异继续推进，整体目标未完成。

## 当前推进的图型点、标签、尺度与裁剪

- [x] Line 和 PointLine 增加自定义点几何，连续与类别输入保留原始数据身份及格式。
- [x] BoxPlot 增加 Linear、Log、SymLog 和 UTC 值轴。UTC 分位数保留小数毫秒，刻度保持整数毫秒。
- [x] AreaBump 可传入实际文字边界，自动在原侧调整端标签、避开障碍并选择性绘制引线。
- [x] 浏览器图内控件支持路径裁剪和孔洞，随相机和响应式布局更新，保留插槽身份。
- [x] 四个浏览器通过 203 幅画廊，以及 Line、AreaBump、图内路径裁剪的同包保留式专项。
- [x] 统一构建通过 954 项 Rust 测试、9 项 crate 文档测试、824 项精简测试及四个浏览器消费者和 Apace headless。
- [x] Chrome 五组 CLI 通过，Firefox、WebKit 和 Edge 对正式冻结产物各通过 48 套，共 144/144。

## 类别配色、雷达点与子弹图尺度

- [x] 类别色标支持 11 种 Nivo 类别色盘、ColorBrewer 官方色板、自定义色板、显式键域和受控未知值。Bar、Line 与 Pie 的图形和元数据共用颜色。
- [x] Pie 支持逐数据颜色工厂、D3 亮度与透明度修饰，以及按填充和实际背景选择标签对比色。
- [x] Radar 点工厂支持真实偏移路径、裁剪、原始数据锚点和样式继承。Bullet 支持线性、Log、SymLog 与 UTC 值轴。
- [x] 四个浏览器通过 200 幅画廊及五幅新增场景的实际几何与像素检查。
- [x] 四个浏览器通过保留式 Radar 双阶段专项，覆盖 SVG／Canvas 像素、指针拾取、缩放后的切片和拒绝后的保留路径。
- [x] 统一流水线通过 936 项 Rust 测试、9 项 crate 文档测试、809 项精简测试及四个浏览器消费者和 Apace headless。
- [x] Chrome 五组 CLI 与正式冻结产物的三引擎矩阵通过，Firefox、WebKit 和 Edge 各 46 套，共 138/138。

## 径向比例尺、颜色配置与浏览器控件

- [x] Radar 接入 Log/SymLog 值轴、真实映射半径、顺逆时针布局和可选等距反算刻度。8 组 D3 配置及 CPU 像素通过。
- [x] 共享颜色尺度支持可选 RGB gamma、Lab、HCL、HSL、Cubehelix 和 38 条命名色带。连续色带及量化色板可用于同一 HeatMap、Calendar 和 Choropleth 布局。CPU 回归确认连续图例的半像素样本颜色。
- [x] 保留式 SVG/Canvas 图内控件、两种混色动画模式和 195 幅画廊均在 Chrome、Firefox、WebKit、Edge 通过独立专项。
- [x] 统一流水线通过 915 项 Rust 测试、9 项 crate 文档测试、790 项精简测试以及四个浏览器消费者和 Apace headless。
- [x] Chrome 五组 CLI 通过，Firefox、WebKit、Edge 对正式冻结产物各通过 45 套，共 135/135。
- [ ] 继续验证原生窗口、物理输入、辅助技术、竖排与彩色字形。GPU 和 worker 排在最后。

## 已验收：自定义节点与原生双向文字

- [x] 散点纯几何工厂接收原始数据、已解析大小与映射坐标。精确拾取与最近点锚点保持各自的语义，验证了偏移路径、真实裁剪、描边、透明色、预算和兼容动画。
- [x] 189 幅阶段画廊与真实 SVG/Canvas 自定义散点拾取在四浏览器通过。
- [x] `raster-bidi` 支持单行双向排版，共用测量、CPU 和字形轮廓。已通过 Unicode/HarfBuzz 对照、真实位图与 Glory 语义轮廓。Apace headless 实测混合方向标签轮廓像素。
- [x] `MarkBlends` 实现 SVG、Canvas 和 CPU 的逐标记艺术混色。CPU 与独立 Canvas 对照验证了 15 种模式和不透明、半透明背景。
- [x] `Chart::plot_widgets` 增加按数据标记保留状态的语义控件插槽。原生语义与四浏览器正式包专项覆盖实际裁剪、焦点、点击和失败替换。
- [x] 统一流水线通过 893 项 Rust 测试、9 项 crate 文档测试及 771 项精简测试。Chrome 五组 CLI 全部通过，Firefox、WebKit 和 Edge 对同一冻结产物各通过 43 套，共 129/129。画廊 190 幅，配置入口 72 个。

## 共享尺度与符号阶段

- [x] Bar 的 Linear/SymLog、反向、整化域与像素取整共用值轴与实际柱、合计和拾取几何。全功能专项 4 项及旧 Bar/Cartesian 14 项通过。
- [x] Swarm 共享数值/UTC 值轴、逐点尺寸、组槽及类型化交互。真实符号与描边参与保守包络，仅沿另一轴确定性避碰。全功能 9 项、无默认功能 8 项通过。
- [x] Network 按 ID 配置节点符号和真实轮廓拖动，保留独立模拟半径、连线端点和抓取偏移。全功能 6 项、无默认功能 5 项及旧模拟/拖动/关系 29 项通过。
- [x] 187 幅阶段画廊通过四浏览器完整静态验证，新增 5 项定量几何检查全部通过。
- [x] Combo 的连续 X、独立左右 Y、分类窗口与真实交互锚点。6 项全功能、6 项无默认功能及 26 项旧回归通过。
- [x] 加入 Combo 后，188 幅阶段画廊和 6 项新几何检查均通过四浏览器验证。Swarm 和 Network 的实时 SVG/Canvas 专项也在四家隔离构建通过。
- [x] 统一构建通过 866 项 Rust、9 项 crate 文档测试及 756 项精简测试。Rust 1.95、四消费者、Apace headless 和 Chrome 五组 CLI 通过。
- [x] 同一冻结产物完整复验 120/120。Firefox、WebKit 和 Edge 各 40/40，原阈值保持。

## 此前推进：独立 typed Scatter

- [x] `scatter_layout` 独立计算节点，长度前缀的series/node业务身份，raw typed XY、格式、局部/系列/global源索引；新默认Circle直径9，旧 `scatter()` 半径5不变。
- [x] `ScatterAxisScale`：Continuous Linear/Log/SymLog/UTC与X/Y Point、四侧轴、原域/显示域与typed类别；Point Y按Nivo从上到下，canonical数据域导航不重复nice。
- [x] `ScatterNodeShape`区分Circle直径/Filled面积/Stroked D3 nominal；动态线性尺寸默认外推、constant中点、missing策略；真实glyph Path anchors、域外中心的部分墨迹、零尺寸logical sites、Exact/Nearest不同契约及plot内查询。
- [x] `DomainViewport::with_y_max_zoom`独立约束Y ordinal窗口，不让单类别Y阻塞数值X缩放；与原X cap并存，constant continuous导航明确Err。
- [x] 基础8all/8no-default、独立9all/8no（CPU1）、Ycap3all/3no专项通过；实际Nivo9profiles/76points/18domains oracle与182静态Chrome/Firefox检查通过，旧176保留。
- [x] 修复后正式pipeline实测841项Rust+9doctest=850、no-default733，较前轮813项Rust新增28；四consumer格式/严格/release/bindgen及native Apace通过。Rust 1.95 原生全功能与精简配置编译检查已刷新通过。
- [x] 共享单点stroke规范化：零Circle/双零Rect/同点可绘Path（含`M Z`，仅Move除外）复用真实cap footprint，一次fill；遵SVG2理想stroke端点，零长度 `[3,3]` offset3仍为on。Scene身份/metadata不变、普通正长路径及bareScene旧hit不变；专项7all/6no、真实semantic Widget1all/1no、Chrome/FF canonical每engine60row×3cap=180比较mean0已通过，原始228row×两engine观测另存。
- [x] 统一包 72 个配置入口、Scatter 六组双阶段及 12 组实际端帽通过。完整矩阵 113/114，WebKit 的平行坐标悬停是唯一失败，三引擎的 Scatter 均通过。
- [x] 按指针区域与图面悬停所有权修复迟到的按钮进入或离开。在四浏览器的隔离构建验证了 SVG/Canvas 事件顺序和原平行坐标检查，统一包完整复验仍需执行。

## 已完成：共享尺度与 Filled/Stroked glyph

- [x] additive ContinuousScaleOptions / ResolvedContinuousScale：source/display domain分开、reverse/nice/round/clamp/log base；真实Line轴/几何/sites和viewport配置接入，旧默认保留。
- [x] PointScale / point_line_layout：typed数字/字符串类别、零bandwidth点格、padding/align/round/window、原source identity与格式metadata。
- [x] glyph_geometry / glyph_mark：Filled七种与Stroked七种、真实D3 origin/支持点/边界/绘制模式；252组实际d3-shape3.2.0 recording-context、1440记录，填充面积和描边感知size分开。
- [x] Line/Radar真实glyph与Path anchor、Geometry Exact凹口/hollow/零尺寸cap语义；Geo add_glyphs共享投影、nullable metadata、同形大小/颜色图例、实际projection mask和hidden归属。
- [x] Glyph核心6项/Geo7项专项（含CPU3）及native/WASM严格Clippy通过；原Geo62项回归通过。
- [x] 新静态画廊176幅生成/严格检查/独立Chrome通过，新增12幅包括共享scale/glyph10幅和Geo2幅；新定量几何检查不使用外部图表renderer。
- [x] 正式813项Rust+9doctest=822、no-default707，完整pipeline/四consumer通过；Scale四case、Glyph四case和Geo两case的双phase Chrome专项已同源实测通过。
- [x] 最终Chrome五CLI组合通过，176场景/66导航/32host专项与未挂载或Stop后snapshot清理合同实际验收。
- [x] 37×3=111最终矩阵在同一次fresh完整运行全部通过：Firefox150.0.2/WebKit26.0/Edge157.0.4341.46各37/37，每家176场景/66导航；保留初轮incomplete诊断57条状态（56过1测量失败），不拼接结果或放宽阈值。
- [ ] GPU/worker仍留最后；图内Widget、blendMode、复杂字体/实际原生平台与其余图族scale接线继续保留。

## 上轮已完成：Pie、Line/Radar定位与关系集合

- [x] 独立 `pie_layout` / `pie_with_layout_options`：NivoRequested/D3Applied/LegacyAngularGap、Auto/Fixed pad radius、真实内外 asin 与圆角，visible zero pad、collapsed border、metadata与exact-cardinal sector fit；旧 `pie` 默认保留。
- [x] `PieLayout::place_link_labels`：当前签名与显式真实测量、有限候选/预算、文字与leader一起移动或隐藏，拒绝不提交；先place再改公开Scene，编辑后再place明确Err保帧；无测量不假装自动避碰。Browser示例实际用Canvas测量，静态示例不虚构测量。
- [x] `LineLayout` / `InteractionBinding` / `SceneInteraction`：raw/format/source index/当前位置、11种crosshair、typed resize/viewport与Scene/plot/binding事务；旧Scene更新清metadata，动画保当前目标业务值，world坐标只inverse一次。
- [x] `RadarLayout` / `PolarSliceIndex`：真实圆内角度分区、顺逆/旋转/seam、stable axis和point IDs、多系列排序rows/格式/颜色、零值无点sites、真实spoke与共享键盘/tooltip协议。
- [x] `RelationGraph` / `RelationLayout`：Tree祖先/后代独立开关、Network邻居、Chord incident、Sankey同向parallel边；显式mark/label归属、hidden traversal、当前数据替换与受控paint，不从series猜图关系；Network adapter为边提供raw-source series保证Surface可达，双端隐藏仍用Graph。
- [x] 新增13幅独立typed状态快照；静态画廊164幅生成/严格检查通过。四组独立native/oracle专项见[verification](verification.md)。
- [x] 最新原生769项Rust+9doctest=778、no-default668，新增43项Rust；完整pipeline/四consumer/native Apace/严格WASM与rustdoc通过，最终Chrome五CLI及四家新专项已过（164场景、56导航）。
- [x] 动画目标/真实enter-exit union与owned overlay联合准入，完整decorated Scene提交前检查；tiny dash/累计dash预算3项native unit通过，避免Model/Cage先行；预排队拒绝保旧motion/clock，末段帧失败恢复旧数据帧并取消animation/gesture/captures。
- [x] 最终34×3=102/102在同一次fresh完整run通过；每家gallery164/配置56，原阈值不变，不拼接初轮成绩。
- [x] 首轮101/102诊断已保留；Hierarchy只修测量fixture，随后预算修正/最终pkg从零重跑102全绿，未拼接旧通过项。

## 此前推进

- [x] `axes.rs`：Linear/Log/SymLog/UTC、四侧轴、刻度/网格/legend/格式；`dual_axis` 左右独立值域的折线/面积。
- [x] `format.rs`：显式 FormatLocale、分组/货币/数字替换、specifier 子集；接入 ValueFormat，预算和不支持格式明确拒绝。
- [x] 数值格式补全：固定 SI PrefixFormat / ValueFormat::prefix、显式字符串 c、JS精确十进制中点舍入、20/21位精度与大整数基数；发布版 d3-format 3.1.0 oracle 对照。
- [x] `datetime.rs`：TimeLocale（en_us/zh_cn/自定义）、DateTimeFormat、Gregorian/ISO周、负epoch、padding与有界递归模板；ValueFormat::date_time 接轴/tooltip，默认UTC/固定分钟偏移无新增依赖。
- [x] opt-in `timezone`：静态 jiff-tzdb IANA2026e，TZif/POSIX尾规则支持远期DST，不读取宿主时区库；Named约-9999..=9999与明确越界错误，历史秒偏移保留在civil字段，%Z向零截断为±HHMM。纽约跳时/回拨、上海历史DST、Kolkata、巴黎历史秒偏移及2101北/南半球已测。
- [x] `Label.rotation` 与数值/分类轴 `tick_rotation`：SVG/Canvas/BrowserChart 围绕锚点旋转，显式后端能力准入与场景弹簧角度插值。
- [x] `clipping.rs`、`Mark.clip`：线性段边界求交，SVG/Canvas/Widget/命中与框选共享裁剪。极大有限对角端点保留单位范围内的真实交线。
- [x] 曲线：StepBefore/After、Cardinal、Natural、Basis、CatmullRom(alpha)、Bundle(beta)；整段计算后按语义预算分块。
- [x] D3 open/closed、Cardinal tension、MonotoneX/Y Steffen、BumpX/Y；真实周期控制点、长 closed line 连续分块及长 closed area 完整单路径。
- [x] `stack.rs`：五种 order、五种 offset，含 Wiggle；stream 接入 InsideOut 等排序。
- [x] `mesh.rs`：平衡 KD-tree 最近点、Voronoi 单元与 slices；与暴力查询对照。
- [x] 层次：Squarify、多种 tile、Tidy tree、五种方向；FrontChain + 支持集最小包围圆（含退化切圆处理、包含性及计算预算）；显式 Shelf profile。
- [x] `Resquarify`：持久化行成员/方向，尺寸与权重变化复用、局部拓扑重建、可配置目标长宽比、零值拓扑保留、有界缓存与失败原子拒绝。
- [x] Sankey：Left/Right/Center/Justify，真实源深度/终点高度分列；断连/悬空/零流量拓扑、迭代重排与松弛。
- [x] Widget：受控图例/选择、数据按钮 hover/focus tooltip、导航、当前数据回调、清理与重入测试。
- [x] `presentation.rs`：LegendOptions 横/纵/换行/对齐/符号/文字/别名，TooltipOptions 纯文本模板、多datum、标题/格式/缺失值/条数限制；Widget tooltip_marks 和 BrowserOptions 共用配置。
- [x] `motion.rs`：兼容几何/文字插值、拓扑/enter-exit 淡入淡出；场景投影速度 retarget / Reset，独立 SpringValue 物理速度 Preserve / Reset，reduced motion。
- [x] `clock.rs`：FrameDriver 调度票据与取消、过期帧/单调时间检查；ExplicitClock 手动推进。BrowserChart 单独拥有实际 RAF 调度。
- [x] `PropertyMotion`：完整独立物理属性轨道、兼容 ID 的逐属性速度保持、最短旋转、可配置 enter/exit/topology 淡入淡出、原子预算检查；BrowserChart 默认 Properties RAF，FrameDriver/ExplicitClock 同样支持。
- [x] `combo`：柱/线/面积/散点共享分类或数值 x，左右独立值轴，分组/正负堆叠/百分比与稳定 ID/颜色、显式点身份。
- [x] `surface.rs`：共享拾取、选择、隐藏、变换、平移/框选及事务式输入模型。
- [x] `DomainViewport`：原始域与当前连续窗口分离，Transform/Zoom/Pan/Brush/Reset，Linear/Log/SymLog/UTC、左右值轴共用Y窗口，UTC一毫秒下限；坏输入原子拒绝。
- [x] `BrowserChart::on_viewport`：显式数据域重排与固定轴，BrowserViewportLayout 含尺寸/plot/domains/hidden，refresh_viewport 复用窗口；Relayout 避免重复Scene变换，重排直接发布，失败保留已接受帧。后续已补分类、独立X/Y与动态plot（详见本轮补全）；默认仍为整份Scene的Transform。
- [x] 双指手势：中点世界锚、缩放约束、单/双指重基线、点击抑制、Touch/Mouse互斥；浏览器真实 nonprimary Pointer Events/capture、取消/resize/隐藏/Stop/Drop 清理。
- [x] `browser.rs`（opt-in web + wasm32）：实际 SVG/Canvas surface、真实 Glory 外围控件、有界输入队列、ResizeObserver、DPR、tooltip、wheel/pointer/keyboard、RAF 动画和 stop/Drop 清理；运行见 [browser-host](../examples/browser-host/README.md)。
- [x] `geodata.rs`：可选 GeoJSON/TopoJSON 解码，默认图无 serde。
- [x] `geo.rs`：十种投影、反经线/地平线裁剪、受限球面几何 profile；新增TransverseMercator与显式自适应路径入口。
- [x] `GeoMapLayout`：底图与地点共用拟合 projection context，面积/半径映射、zero/missing、正射背面剔除、实际绘制颜色/大小图例。
- [x] `calendar.rs`：多年闭区间日期、双方向、稳定日期身份和统一色域。

- [x] `ForceSimulation`：Barnes–Hut、有限附加力、碰撞、固定节点、显式时钟与可复现性能/误差基准。
- [x] `LayerPlan` / `DrawItem`：命名层、实际几何/文字交错顺序、标签归属；Widget/Motion/注释全链路重映射。
- [x] 真实 Glory Web Widget 浏览器验收：控制图例、hover/focus、实际导航焦点、当前数据、mixed paint order、标签过滤、退场清理与重挂载。

具体测试与浏览器证据以 `verification.md` 和当前命令输出为准。

- [x] Browser SVG 增量 DOM：路径按 mark ID、文字按槽位保留，属性差异更新、原序保留/重排、clip切换、退场清理；未变化帧 MutationObserver 零记录。

- [x] CPU RasterCanvas：tiny-skia实际RGBA/PNG，显式字体+LTRshaping/回退、clip/alpha/DPR/旋转与事务预算；Chart.raster接GloryImage与现有控件，真实PNG消费者验收。
- [x] 当地日历刻度：ZonedTimeScale/TimeCalendar、field对齐、DST gap/fold策略、真实day/week/month/year边界与轴接入。

## 本轮补全

- [x] 展示布局：显式测量的图例多列/换行、八方 tooltip viewport/禁区避让、优先级 label 位移/隐藏；Chart 接入测量和纯布局，Browser tooltip 使用真实 DOM 测量与超大内容滚动。自定义 legend item/symbol/tooltip Rust Widget renderer 通过 Bond 绑定当前数据，外层保留受控语义与清理。
- [x] 分类域导航：CategoryViewport、绝对 BandScale 窗口、bar/combo 可见刻度和边缘柱裁剪、完整分类 ID/堆叠/颜色保留；X 分类缩放上限与 Y 值轴独立。
- [x] 浏览器非等比 X/Y：request_viewport、TransformXY、真实 wheel/pinch 相对窗口、每轴锚点限幅；on_viewport_with_plot 动态 insets 与过期回调抑制，布局失败原子保留。
- [x] 日期 parsing：严格全部现有 directive、冲突/非法日期/精度校验、明确 IANA gap/fold 策略；d3-time-format 4.1.0 全部34官方locale、918条独立D3输出和解析oracle；calendar当地日期转换与 Sum/Count/Mean/Min/Max 日聚合。
- [x] 单轮廓跨拓扑 morph：按 ID 明确授权、有界曲线细分与弧长采样、closed 起点/方向策略、显式 enter/exit 几何、活动轨道速度保持和精确终态。开放/封闭互转保留 Fade，多轮廓/孔洞未推断。
- [x] 地理扩展：真实 AzimuthalEqualArea/AzimuthalEquidistant/Stereographic（明确90°前半球裁剪）；Circle/Square/Diamond/Triangle/Cross 五种等面积符号，真实同形状图例/命中，官方 D3 raw golden 和 CPU 像素校验。
- [x] 循环 Sankey：SCC/反馈排序、四对齐、正向交叉目标不增、真实回流 ribbon、自环/平行边、统一流宽、可查布局/路由 metadata、有限/极值预算与 CPU 像素校验。
- [x] 原生可移植 RasterSurface：owner guard/weak handle、唯一已接受 PNG 帧、显式 offset/DPI 坐标输入、图例/选择/tooltip 控件桥与事务发布；独立 Apace headless 消费者使用显式 bounded PNG/font provider，实际解码/CPU绘制/命中/键盘/清理。
- [x] 原生高密度release基准：1k/5k/10k scatter与SVG、20k Scene/mesh与CPU RGBA、first/warm median/p95/min/max、128次暴力nearest核对及预算拒绝；[实测范围与重现](density-benchmark.md)。

## 进一步补全（GPU 暂缓）

- [x] 显式多轮廓/孔洞morph：固定key槽位、输入bindings、缺槽collapse、重排/enter/exit/速度/精确终态；真实BrowserChart Properties RAF与reduced/retarget/drop验收。
- [x] 地理复杂裁剪：整个outer+holes有向边界重接、非凸分离轮廓/实际缺口、小圆双交点与整cap、Gnomonic和共享clip_angle；数值有界开放半球profile，歧义拒绝。
- [x] 投影轮廓内的符号填充clip：Circle lens与polygon/disk重接，五种shape、拟合/旋转/命中/像素；显式fill-only准入，图例完整、文字不球面clip。
- [x] 图形样式：线性/径向FillPaint覆盖SVG/Canvas/CPU栅格；渐变定义身份/退休、shader-only拾取与动画、透明色、累计预算；bar/combo圆角/border与真实裁剪边。
- [x] 语义Chart旋转文字：显式字体outline Graphic、完整glyph/holes、原Scene/slot/安全文本保留；实际Glory SVG与Apace Graphic像素验收。
- [x] 浏览器高密度变化帧：SVG/Canvas 1k/5k/10k/20k、64warm样本/8cold、DOM/backing/JS heap范围、真实命中/清理；修复浮点client坐标被整数getter截断的薄mark漏选。
- [x] Firefox/WebKit/Edge可移植验收：复用gallery/widget/outline/paint/contour/raster六套，保留原像素阈值，记录实际浏览器/driver差异；不替代CDP触摸或Safari设备验证。

## 本轮样式与组合补全

- [x] PatternFill：Dots/Squares/Lines、错列/旋转/偏移/透明色；SVG定义与Canvas/CPU瓦片复用、累计像素/操作预算。
- [x] StrokeDash：SVG数组/offset、奇数/负offset/零长cap、闭合接缝、有界曲线/弧长误差；SVG/Canvas原生、CPU/Graphic展开，间隙拾取及动画拓扑。
- [x] 显式路径mask：NonZero/EvenOdd、多轮廓孔洞、圆与矩形，完整fill+stroke与文字裁切、矩形交集、保留SVG节点与定义退休；CPU真实alpha mask。语义Graphic直接plot明确拒绝路径mask，raster或plot(false)可用。
- [x] Combo区间和系列样式：ErrorBars/BoxPlot/RangeBand共享分类/连续x与左右y；真实缺段/越界裁剪、五数箱体/中位/须、误差帽、线/点/边框/透明度/标签。
- [x] 带stroke地理符号与地点文字的投影平面mask，图例保持完整；文字不沿球面弯曲。

## 交互与配置补全

- [x] Tree配置：五连接曲线、五方向、节点形状/叶与内部样式、标签位置/旋转、dash与plot clip，稳定ID、当前数据及保留DOM验收。
- [x] 平行坐标配置：独立Linear/Log/SymLog与反向轴、横纵布局、四种missing策略、曲线、系列样式/刻度/格式、分组legend与owned labels。修复业务ID含-grid-时轴/网格角色混淆。
- [x] 网络拖动闭环：可移植控制器+BrowserNetwork实际Pointer/RAF/capture、64项手势FIFO、camera/resize rebase、动态图latest-slot、隐藏暂停恢复、坏帧保旧与回调重入/Stop清理。
- [x] [内存实测](memory-measurement.md)：原生进程工作集/生命周期峰值/private commit、浏览器拥有PID计数器、实际WASM线性内存容量及显式GC对照；不作为活分配/显存/FPS声明。

## Pie、Radar、Sankey 与 Line 配置补全

- [x] `PieOptions.style`：真实 tangent 圆角 annular arcs、顺逆时针/小角/薄环/整环、边框与独立填充 alpha、显式 active 内外半径偏移；Label/Id/Value/Percent、skip angle/radius、半径位置及文字样式；真实引线、不可选装饰、slice-owned labels 和稳定 ID。
- [x] `radar_with_options`：显式/自动线性域、Circle/Polygon 网格/辐条/刻度、轴标签与系列样式、真实闭合曲线、五种等面积点符号及归属文字；隐藏保持比例尺与共享轴，实际命令数预算准入。
- [x] `sankey_with_style` / `circular_sankey_with_style`：横纵方向、显式链接 ID、Source/Target/Fixed/Gradient、曲率/透明度、节点圆角/边框和内外/居中标签；返回真实节点矩形、宽度/端口、中心线及回流 metadata；隐藏后不重分配流量。
- [x] `line_with_style`：复用原曲线/域/裁剪，系列线/点/边框/面积 paint 与独立 opacity、归属点标签、显式 `connect_missing` 和透明 mesh sites；保留原 gap x 域、源点索引、隐藏系列图例身份。

专项数值、坏输入、CPU 像素与实际浏览器记录见 [verification.md](verification.md)。旧 `radar` / `sankey` 等入口保留原 profile，这些 typed 配置不代表所有 Nivo props 已覆盖。

## P0 共享颜色、HeatMap、Bar 与极坐标

- [x] 共享 `ColorScale`：Sequential/Diverging/Quantize、自动/反向/常值domain、unknown/clamp和palette；真实连续/离散色图例，Geo/Calendar及HeatMap共用，300条独立D3 oracle。修复1ulp边界及极窄域自动legend tick重复叠绘；病态阈值用稳定fallback保持域内/单调，半透明legend与尺度颜色CPU对照通过。
- [x] `heatmap_layout`：NULL/零分离、稳定raw row+column身份、Rect/Circle/size variation/独立padding/force_square、边框/圆角/文字/四轴；精确五种关联集合及事务active，Controlled浏览器真实高亮。
- [x] `bar_with_style` / `bar_layout_with_style`：分类inner/outer padding、分组pixel gap、系列/分类色、Start/Middle/End标签与raw/displayed格式、Net/PositiveNegative totals、独立tooltip格式；隐藏后重布局保留真实rounded端点。
- [x] 公共 `polar_axes` / `append_polar_axes`：径向/圆周tick/grid/format、真实文字旋转、部分/反向/整圈；`polar_bar_with_axes` 接入原几何与面积半径；新增 `stacked_radial_bar` 每ring沿角度堆叠categories、tracks、paint/owned labels、稳定ID和可查守恒metadata。
- [x] inline SVG实例定义隔离：`to_svg_with_namespace`与BrowserChart owner作用域覆盖gradient/pattern/clip及typed引用，业务ID/安全文字不改写；独立 `to_svg` 保持原入口。修复前真实复现、native回归、Chrome及三引擎live截图隔离专项通过，原通道±2阈值保持。

专项与最终统一验收以 [verification.md](verification.md) 的实际记录为准；不将编译或option字段本身视为视觉/交互验收。

## P1 新变体、层级focus与统计配置

- [x] `layout_area_bump`：逐x按值排名、共同厚度比例尺、Linear/Smooth闭带、Connect/Gap、align/spacing/x padding、起止文字、slice/rank/bounds metadata及事务active；保留旧显式rank `bump`/全局layer `stream`。
- [x] `treemap_layout` / `TreemapLayoutState`：parent/leaf/per-ID样式、真实inner/outer与四侧header预留，复用旧五tilers及状态Resquarify；`circle_packing_layout`真实子树投影，`icicle_layout`四向焦点重排。
- [x] `HierarchyFocusController`：root/parent/back、祖先/当前metadata，删焦点恢复最近旧祖先或新根；布局/数据/缓存候选验证后提交，有界历史；不把Scene camera当数据focus。
- [x] `calendar_layout` / `calendar_layout_zoned`：Compact/FullYears、firstWeekday、nullable/zero、方/长方格、横纵/文字样式及真实月union轮廓；旧完整年份入口与显式zone聚合保留。
- [x] `boxplot_layout` / `bullet_layout`：nullable真样本group/subgroup/R7/Tukey/Extrema与离群点样式；Bullet共享/逐行域、横纵reverse、相邻端点区间/marker及真实axis/title归属，隐藏重布局。

上轮四项P1加上P0共8项的历史验收保持；下面三组实现补齐最初队列剩余的数据契约和基础算法，但不把最初10项优先任务等同于全部目标。当前专项与最终统一验收分开记录于 [verification.md](verification.md)。

## 独立 Marimekko、Delaunay 与地理线流

- [x] `marimekko_layout` / `marimekko_with_options`：独立category thickness与dimension矩阵、None默认及五种offset、两套nice、outer/inner padding、四轴/实际分类中心、paint/border/标签、raw/端点/displayed/source indices；零厚度仍参与padding/domain却不造ink，维度和类别隐藏语义分离，旧面积守恒入口不变。
- [x] `LinearScale::nice`：D3式稳定步长迭代/负倒数小数增量，finite fallback；4项专项与644条独立D3查询，保留D3 decimal非严格幂等行为。
- [x] `Delaunay`：Spade2.15.1/robust1.2.0、重复点ownership、三角形/半边/inedges/凸包/邻居/links、均匀fit真实Scene；共线链与共圆合法确定剖分、可逆二次幂坐标profile、预算和事务拒绝。
- [x] 地理线流与经纬网：TransverseMercator、`geographic_lines`、`geo_graticule`、原projection context追加/behind层、反经线/极点/cap切段、完整stroke mask；新`*_layout_with_path_options`提供真实有界弦误差细分，旧两度profile保留。

上述为前轮完成记录，勾选表示已有实现和对应专项证据。该轮最终日志确认726项Rust+9doctest=735、no-default629，新增37项Rust；native/WASM严格检查及WASM rustdoc通过，完整流水线、四consumer及Chrome五CLI通过（151场景、41项导航、三家新专项）；三引擎各30套、共90/90在同一次fresh完整重跑全部通过。首轮88/90的Geo指针probe歧义仅修fixture测量，新宿主严格构建与Chrome重验后完整重跑，不拼接旧成绩或修改阈值。默认Rust依赖已增加Spade及robust；参考算法与许可证见[third-party notices](third-party-notices.md)。

## 后续差距

具体任务、参考源码、优先级与完成条件见 [Nivo / D3 差距审计](feature-gaps.md)。

- [ ] 更广的图表组合和全部 Nivo 样式 props；固定轴回调仍由应用负责数据重建。静态时区规则更新需重新构建。
- [ ] 共享颜色的更多图型角色、图型自定义 Geometry/Widget 接线；现有 38 条命名连续色带与 RGB gamma/Lab/HCL/HSL/Cubehelix 插值已实现。HeatMap 关联需应用接事件，nullable metadata 与 Scene 有限标量契约不同。Bar 隐藏聚合需重布局；角堆叠 RadialBar 已有 D3 圆角与 pad_radius 间距，更宽 props 和图型专属组件仍待补。
- [ ] 已有图型的更多typed配置与共享scale接线仍按feature-gaps继续；当前关系集合已有独立协议与factory，图内自定义组件仍保留；Marimekko signed offset受明确profile限制，Delaunay共圆不承诺D3相同对角，地理误差为采样判据而非任意投影精确证明。AreaBump非负ordinal x，Hierarchy标签避碰/导航动画不自动推断，Calendar有界Gregorian，BoxPlot UTC统计可保留小数毫秒，源值与刻度仍为整数。
- [ ] 自动业务语义对应、任意轨迹无自交保证；现有多轮廓morph显式配置、有采样近似，默认继续保留Fade。
- [ ] 全球任意球面拓扑/对跖与自交消歧、文字沿球面变形；样式/legend/label不承诺所有场景无碰撞。现有方位类符号/文字 mask 使用投影平面 disk，新增 Conic 使用真实有限扇形/环形 footprint。
- [ ] 全局路径优化及更多 Nivo 布局/样式 props；循环流程可用，但端口附近 ribbon 仍可能交叉，不声称全局最小交叉或平面嵌入。
- [ ] Pie tight annulus/custom组件与全局最优引线排布；已有D3/Nivo asin及真实测量贪心避碰，不保证任意文字和leader零交叉。active显式控制，Browser普通图例过滤不重算角度，需应用重布局。
- [ ] Radar 的更多径向比例尺、任意图内 Widget，以及 Pie/Radar/Line 点标签的自动避碰。Radar 点 Geometry 回调已实现；当前只接受真正的 periodic closed 曲线，数值必须落在显式域内。
- [ ] Sankey 的更多回流路径形态与交叉优化；当前回流保留正交清障通道。Scene source-owned 图例过滤与 `style.hidden` 的两端 incident 隐藏语义不同，后者需调用者重布局，均不改变原流宽与端口。
- [ ] Glory语义Canvas节点、真实Wry/Apace OS窗口、辅助技术与物理设备验收；Apace headless/Chrome触摸注入不能替代实机。
- [ ] 完整原生/WASM活分配归因、显存及浏览器compositor端到端帧率；已有OS工作集与容量观测，尚不据此声称无泄漏或60FPS。

## 暂缓

- GPU/worker：按本轮用户指示留到最后，本轮不实现，也不引入相关依赖。

这些项仍属于原目标，后续工作应继续减少它们，而非删除差距来宣称全部完成。


## 本轮新增共享配置

- [x] BoxPlot 借用逐摘要源颜色工厂：完整域身份、实际统计量、继承描边与首代表图例共享颜色；控制器缓存及显式源替换。
- [x] Calendar FullYears 月份间距：日期、月轮廓、bounds、标签和拾取同步，默认零输出保持，Compact 正间距拒绝。
- [x] Radar owned point-label 文字工厂：typed index/key/value/格式化上下文、几何组合与共享尺度，保留既有 tooltip 和锚点；无回调存留。

详细受检范围和保留的上游差异分别见[颜色工厂](boxplot-color-factory.zh-CN.md)、[月份间距](calendar-month-spacing.zh-CN.md)、[点标签](radar-point-labels.zh-CN.md)及[验收记录](verification.md)。GPU／worker继续暂缓，前述更宽差距仍保留。

## 未发布开发增量

- [x] Funnel 借用阶段 Geometry 工厂；保留旧入口及独立原侧边框策略，原中心锚点与自定义墨迹区分。
- [x] Parallel 数据/像素/Brush 逐轴筛选控制器，完整源尺度/配色身份与事务更新。
- [ ] Parallel 实时宿主 Brush 接线、分类变量；Funnel 的 builtin 内 Source Widgets 混排及更广 React 行为等价仍单列。独立字面 SVG 整层 Glory 组件、原生 part hook／提示已完成专项验收；逐属性 Spring 与时钟见后续动画节，上游无退出过渡。

这些开发 API 未包含在 crates.io `0.1.0` 中。指南与受检边界见[差距清单](feature-gaps.md#开发版增量阶段几何与平行坐标筛选)。其它未完成项不因这两项增量而关闭。

## 未发布开发增量：测量折行

- [x] 真实候选 advance 驱动 Words/Graphemes 折行；有界测量、max-lines、长词/溢出策略。
- [x] 事务 Scene 行替换、字体/归属/clip/draw-order 维护与真实 FontRegistry 包装。
- [x] 显式单行空白协议：SVG、Canvas、CPU、Glory Text 与真实测量一致传递模式；保留默认 Legacy 行为。
- [ ] 完整 CSS/UAX14、段落 bidi 后换行与自动竖排。

受检 profile 和原生字体差异见[折行指南](text-wrapping.zh-CN.md)，不将显式路径结果等同于任意宿主文字一致。


## 未发布开发增量：Waffle 与 Calendar 年份标签

- [x] Waffle 借用单元格几何/画笔工厂：占用及空格、原始槽位锚点、实际墨迹拾取和最终帧预算。
- [x] Calendar FullYears 年份标签四向 Before/After、正负 offset、借用文字工厂及颜色 profile 组合。
- [x] Waffle 实际单元格及空格的实时组件适配，遵守测量、身份与宿主边界。
- [ ] Waffle 组区域工厂与区域实时组件；Calendar 完整 Nivo 排版及跨后端 baseline 等价。

这些 API 不属于已发布 0.1.0；受检范围见[Waffle](waffle-custom.zh-CN.md)与[Calendar](calendar-year-legend.zh-CN.md)。


## 未发布开发增量：Bullet 部件

- [x] Range/Measure/Marker 借用几何/画笔工厂、原始数据与排序身份、实际墨迹和原部件中心区分。
- [x] 共享值轴组合、最终帧预算、显式实色 PaintedGeometry 预检及 Glory 类型化接线。
- [ ] 任意 React/Widget 生命周期与 animatedProps 对象协议；既有较宽样式和平台差距仍保留。

指南见[Bullet 部件工厂](bullet-custom.zh-CN.md)。这些开发 API 未包含在已发布 0.1.0 中。


## 未发布开发增量：Bump 两端文字

- [x] Start/End 独立借用文字工厂；保留原 label、图例、tooltip、owner、样式和逻辑身份。
- [x] 点／文字组合入口，明确逐系列调用顺序、missing/hidden/disabled、空槽及 UTF-8 预算。
- [ ] 自动端点文字避碰、任意实时组件和更宽宿主验收继续按差距清单推进。

用法见[Bump 两端文字](bump-endpoint-labels.zh-CN.md)。不将显式字体路径导出等同于普通宿主文字像素完全一致。


## 已实现并专项验收：Albers USA 有界复合投影

- [x] 三分区固定投影与区域逆算内核、原像素 epsilon 和区域 provenance。
- [x] 完整平面矩形裁剪基础：凹轮廓、穿边界的孔洞、整窗覆盖、开放线重入及有界工作量。
- [x] 共享拟合 context 的复合模式、三路共同 World/Data fit、共享预算与单一源 feature 身份。
- [x] glyph／地点标签所选区域 mask、共享颜色域与 browser／raster 分区逆算接口。
- [x] World／Data 地图及 symbol／glyph 导出在四浏览器、DPR 1/2 的实际 SVG／Canvas／CPU 比较，原生缩放坐标链与类型化控件验收。
- [x] 分区线路／经纬网、完整描边 mask 与 canonical ownership。
- [x] 新分区逆算的浏览器宿主运行时、可编辑标签及真实控件／拒绝事务验收。

以上开发 API 仍属 Unreleased；本项按受检有界输入实现，不承诺任意球面拓扑或逐位相同的
原版退化接触，详见[指南与验收边界](geo-albers-usa.zh-CN.md)。更广地理拓扑、其他图型差距
以及 GPU／worker 继续按原计划保留。

## 已实现并专项验收：Piecewise 重复域节点

- [x] 非严格单调／全常量域、变换坐标选段、全域 clamp、显式逆分支及 UTC TimeClip。
- [x] Line 的投影后裁剪、全常量折线与面积基线，Scatter 的全断点重排。
- [x] 原始局部像素 range 视口反算、全常量窗口、原 controller 重置及无界 inverse clamp。
- [x] 原版 D3 的 9,496 配置核心矩阵及 4,128 条 Line／Scatter 视口记录。
- [x] 真正编译 WASM 的全部核心矩阵与浏览器 Math.pow 路径。
- [x] Chrome／Firefox／WebKit／Edge、DPR 1/2 的 SVG／Canvas、指针／键盘和拒绝事务。
- [x] 锁定参考依赖、CI 接线与双语说明。

本项属于 Unreleased。数值与宿主证据、首次失败和平台数学限制见
[重复域节点指南](piecewise-repeated.zh-CN.md)及[验收](verification.md#development-repeated-domain-scales-and-consumers)。
更多图型接线、既有普通文字／严格 compositor 差距与 GPU／worker 保留。

## 已实现并专项验收：Swarm 节点与 Network 连线工厂

- [x] Swarm 借用原始来源、组与已排布站点，自定义几何／画笔不反向改变碰撞圆或值轴。
- [x] 原数据锚点、Nearest、实际墨迹 Exact、零尺寸／透明资格及累计预算。
- [x] Network 借用当前边／源目标状态，组合节点 glyph 与显式关系身份，平行边／自环独立。
- [x] 浏览器保留连线工厂，按 tick／拖动／resize／替换重建；失败候选保留旧帧和物理位置。
- [x] 回调读取、嵌套请求受检拒绝、内部 stop 及预检 stop 的安全释放；独立审计复现缺陷后修复。
- [x] 原版 Nivo 真实 Canvas 函数参考、Memory／Raster／CPU 与四浏览器 DPR1/2 专项。
- [x] 双语指南、默认继承回归及 CI 接线。

新功能属于 Unreleased。[Swarm](swarm-custom.zh-CN.md) 与
[Network](network-link-custom.zh-CN.md) 工厂提供共享 Scene 绘制，不等于任意组件或
另一力布局的坐标／动画轨迹相同。更广实时 Widget 生命周期及既有平台差距继续保留。


## 开发版：事务式 Widget 墨迹替换

- [x] `PlotWidgetsComposition<T>` / `PlotWidgetCompositionContext<T>`，新 key/revision 先隐藏构建，成功发布后提交；旧子组件在此之前保留已接受数据。
- [x] 语义 Chart、`raster_chart` 与 BrowserChart SVG/Canvas 共同准入；指定 physical mark 绘制与拾取排除，source Scene、全部标签和逻辑控件保持；同 Scene 模式切换恢复原墨迹。
- [x] 组 0 透明度隐藏／inert 但保状态，暂存新组件预算、清理重入与停止、PNG／Canvas 发布拒绝及新 key 失败已验收。
- [x] 1,595 minimal / 1,897 all / 18 doctest，严格 native/WASM 检查；四浏览器八上下文、88 画面对照与 120 条拒绝记录通过。参见[指南](widget-composition.zh-CN.md)及[验收记录](verification.md#development-transactional-widget-composition)。
- [ ] 任意 Scene 原层级与 Widget 交错、控件墨迹快照／Scene 导出、更多图型角色和设备／字体差距继续推进；GPU/worker 暂缓。


### 开发中：浏览器 Scene 顺序真实 Widget

- [x] 共享受检 Paint/Widget/Group 计划，保留字体、裁剪及原始标签位置。
- [x] SVG/Canvas 浏览器按原始顺序组合，整组透明度只应用一次，CSS 源混色可读取前方 Widget。
- [x] 保留作用域的跨父节点移动与受检失败回滚；alpha 像素的有界精确矩形分解。
- [ ] 原生宿主同层精确输入遮挡、任意 Widget Scene 导出及自动 spring。
- [x] 四浏览器八上下文、256 截图、320 原生输入检查、128 拒绝及 80 生命周期记录；全量 1,614 minimal / 1,916 all / 18 doctest。见[指南](ordered-widgets.zh-CN.md)及[验收](verification.md#development-scene-order-live-widgets)。


### 开发版：HeatMap / Waffle cellComponent 角色

- [x] HeatMap 原始可空数据、原输入索引、当前 computed cell 与真实 Mark 的类型化组件适配。
- [x] Waffle 实际单元格及空格组件、共享 owned group 快照、格位身份与重新分配元数据。
- [x] typed 装饰 Mark 准入，不伪造业务标签、canonical 控制或站点，旧无类型规则保持。
- [x] 真实 Memory 与 DPR1/2 PNG 发布拒绝的来源/组件回滚，最小配置完整示例。
- [x] Waffle 透明 area 事件角色：完整占用格位、真实 canonical 站点和物理墨迹分离。
- [ ] Waffle 组区域实时 Widget 工厂（Nivo 无 areaComponent prop，属于扩展角色）。
- [x] 后续 CPU 图像 Chart 已接通完整绑定指针和实时组件组合，验收见 [图像输入指南](raster-chart-bindings.zh-CN.md)。
- 自动 spring、任意 Widget 导出和原生同层输入继续保留，GPU/worker 后置。
- [x] 两种角色各四浏览器八上下文、416 合计画面；全量 1,627 minimal / 1,931 all / 18 doctest。完整 Chrome 宿主 CLI 含原严格尺度门禁通过，旧跨平台总矩阵不重分类。见[指南](cell-widgets.zh-CN.md)及[验收](verification.md#development-cell-widgets-and-complete-chrome-host-cli)。


### 开发版：Scatter nodeComponent 角色

- [x] 显式原系列与当前公开布局一致性预检；共享 owned 原 datum、完整 series 与 computed site。
- [x] 原始 XY 类型及数值符号位保留；零尺寸、透明和 plot 外实际节点角色，缺失/隐藏/省略不补造。
- [x] 使用既有测量、覆盖/严格替换及原绘制顺序组合，完整 Memory 示例及 CPU 发布拒绝回归。
- [x] 冻结完整 Nivo 原源码执行组件参数、computePoints、transition style 配方及交互处理器门控。
- 自动 spring、React animated props、任意组件导出和原生同层输入仍保留，GPU/worker 后置。


### 开发版：Bar barComponent 角色

- [x] 类型化工厂来源捕获完整可空系列/分类、原几何及绑定；继承默认 Bar 输出和全配置入口。
- [x] raw/displayed/interval、Some(0)、原数值符号位、未裁剪端点和 Path 原锚点角色。
- [x] 当前标签/clip 元数据、空文字、受控 dim；原 Scene 标签和总计所有权保留。
- [x] Memory 状态/焦点/标签数据和 DPR1/2 CPU PNG 发布拒绝回归，完整便携应用。
- [x] 冻结实际 Nivo 组件创建、公共参数和四阶段 style 配方，离线验证及篡改负控。
- 特定 Canvas compositor 中的 font8 字形颜色差保留独立未通过诊断；纯色 Widget 内部像素不替代文字绘制验收。
- 自动 spring、任意组件导出、原生同层输入和 GPU/worker 继续保留。

### 开发版：准确截图与字形诊断

- [x] 只读整视口采样、真实 DOM 分轴坐标、DPR、单次 floor、几何竞态及越界拒绝。
- [x] 四浏览器双 DPR 原生独立矩阵、补齐截图反例、四项变异负控和 Chromium CI 子集。
- [x] Bar font8 正确坐标复现、无 Glory/WASM 原生载体、透明 Canvas 与非 LCD 对照。
- [ ] 任意 Widget 文字跨合成路径严格等价；当前仅完成诊断，原失败仍未通过。
- 自动 spring、任意组件导出、原生同层输入及 GPU/worker 继续保留。

### 开发版：Bump pointComponent 角色

- [x] 原系列共享 owned 快照、原索引、类型化类别与有符号 rank、实际点代表色。
- [x] 全帧预检先于回调，普通/偏移 Path 原锚点、准确空 plot 和实际绘制绑定。
- [x] 实际零尺寸/透明点组件，hidden/gap 不造角色，`None` 跳过与错误拒绝。
- [x] Memory 状态/焦点保留、DPR1/2 CPU PNG 发布拒绝回滚和完整便携应用。
- 自动 spring、任意组件导出、原生同层输入及 GPU/worker 继续保留。
- [x] 四浏览器双 DPR、192 帧、11,856 颜色采样、48 真实输入组及 112 拒绝；独立四项变异负控通过。

### 开发版：Treemap / CirclePacking / Icicle 节点组件

- [x] 共享完整原扁平树、原 source_index、raw/aggregate、图族及焦点元数据。
- [x] 源/拓扑/聚合/资格/几何/受管标签整帧预检先于回调，实际透明/无画笔角色不省略。
- [x] 准确空 plot、PaintedGeometry 站点，标签测量后定位/隐藏/恢复组合。
- [x] 保留旧布局的排列相等语义，原树 raw-bit-only 变化独立更新组件数据。
- [x] 三家族 Memory 状态/焦点与 DPR1/2 CPU PNG 发布拒绝回滚，完整便携应用。
- [ ] 自动跟随 HierarchyFocusMotion 采样帧与退出动画；当前只支持静态/target_layout。
- 任意组件导出、原生同层输入及 GPU/worker 继续保留。
- [x] 三图族×四浏览器×双 DPR、624 帧、20,832 颜色采样、144 真实输入组、336 拒绝及独立四项负控通过。

### 开发版：类型化组件原始值更新

- [x] 六角色 payload 的原始有符号零比较，包括共享系列、数值类别和 Bullet 原端点。
- [x] 真实 Memory 保留身份/计数/焦点、CPU 图像发布失败回滚及来源预检。
- [x] 配对便携指南和完整可运行示例。
- [ ] 整页 PNG 严格稳定：原生 SVG 单像素变化仍未解决，不以功能验收替代。
- 任意字形等价、自动采样动画、任意组件导出、原生同层输入及 GPU/worker 继续保留。
- [x] 六角色×四浏览器×双 DPR 功能矩阵：48 场景/480 帧、4320 前景采样、独立 Chromium 子集及隐藏 Canvas backing 负控。

### 开发版：Swarm circleComponent 对应角色

- [x] 实际打包节点类型化组件、完整来源共享与原始/打包索引、分组、碰撞圆盘和值尺度。
- [x] 全帧预检先于回调；实际 size0/透明角色、hidden/cull/skip 和有符号原值更新。
- [x] SVG/Canvas Scene 顺序宿主接线、完整便携应用及配对指南。
- [ ] 自动跟随采样动画及退出节点组件；任意控件导出、原生同层输入和 GPU/worker 后置。
- [x] 真实 Memory 状态/焦点与 DPR1/2 CPU 发布失败回滚；四浏览器8上下文/272帧、独立 Chromium 子集与五项负控/单独深层 witness 变体。
- [x] 原版组件创建/transition/handlers 与 Canvas command 参考、独立离线/篡改核验；不冒称 force/React/Spring 动态运行时。

### 开发版：Network nodeComponent 与动态宿主

- [x] 当前 force 状态共享、源索引、真实 glyph 原点、独立物理半径及空标签/零视觉角色。
- [x] tick/源拖动/resize/图替换与严格组件共同接受，保留 snapshot 与 live Bond 一致。
- [x] CSS 逻辑单位、停止回调截断、连续 reheat 最大值及冷 hover 空闲处理。
- [x] 完整 Memory 示例、配对指南与浏览器宿主接线。
- [x] 实时 linkComponent 与节点/连线混合严格阶段已由后续专项完成。
- [ ] 自动退出动画、任意 Widget 导出、原生同层输入及 GPU/worker。

### 开发版：Network 连线组件与混合严格提交

- [x] 完整节点/边快照与精确原值位、edge 索引、原端点中点和真实几何分离。
- [x] links→nodes 单包/128 总预算，旧 node-only 类型不变，混合 BrowserNetwork 单阶段提交。
- [x] 真实源遮挡与可信输入；换图、索引重排、revision、停止、错误回滚及 owner/Bond 一致。
- [x] 完整 Memory 示例、配对指南、原始 Nivo 参考与离线/篡改核验。
- [x] 四浏览器32上下文/832帧、CI8/208、旧节点API8/304及8负控，独立PNG解码通过。
- [ ] 显式业务边身份、自动退出动画、任意组件导出、原生同层输入与更多样式；GPU/worker后置。

### 开发版：Funnel 函数与类型化层角色

- [x] 原始颜色/值格式器、已计算阶段的边框/标签色函数，可与自定义几何组合。
- [x] 实际 body/border/前后分隔线 Mark 的实时角色，共享完整原值/轮廓/标签及精确来源位。
- [x] 全帧预检先于定位、真实零像素角色资格、尾分隔线无虚构 datum、原点与偏移路径分离。
- [x] 原始 profile 面积/侧边几何工具、Memory 完整示例及配对便携指南。
- [x] 原始 Nivo custom layers/function props 参考与独立离线/篡改核验；上游不存在 partComponent。
- [x] 四浏览器32上下文/1280帧及CI8/320，独立54720样本、完整来源/几何/颜色普查、六项负控通过；整页PNG差异仍为诊断。
- 字面 SVG 整层插入、省略与重复已由下节独立 `BrowserFunnel` 实现；此处既有逐 Mark 角色 API 保持自身范围。
- [ ] 泛型 RawDatum 扩展字段、自动 Spring/退出、任意控件导出与更广原生同层输入。
- GPU/worker 后置，开发版不改已发布 0.1.0。

### 开发版：Funnel 字面 SVG 整层组件（本轮范围已验收）

- [x] 权威 `FunnelSvgFrame` 保存真实构建来源／配置／options 和无墨迹角色的显式样式函数结果；原 profile、当前 Mark 与静态 Scene 装饰区分，不重放函数或猜测配置。
- [x] literal Separators／Parts／Labels／SceneExtras／Custom 顺序、省略与重复；原来源索引与业务身份不复制，复合透明度组和重复资源工作量有界校验。
- [x] 独立 SVG `BrowserFunnel` 及 `foreignObject` 中真实保留式 Glory 整层子树；稳定 occurrence／registry／revision、构造期 None 与各 runtime 整帧 Cage 共同采用。
- [x] 来源重建与 program／camera／hidden 仅呈现更新分开；弱 Scope-bound current 排队提交，实际原生 SVG body 命中与装饰／custom UI 边界分开。原通用 Scene／SVG／Canvas API 保留。
- [x] 五种浏览器配置 × 两种 DPR、40 组、1,610 帧、560 次拒绝恢复、20 个真实像素／位姿负控；失败 UI 永久隔离及 revision 重建、实际 Glory batch 控制纳入验收。
- [ ] WebKit 停留替换原生路径后可能不投递离窗事件；普通 SVG 控制复现，当前仅对实际已投递的离开输入保证清除。
- [x] 四个公开原生 part hook、标准与保留式 Glory tooltip；后续独立正式矩阵通过 10 配置／40 profile、2,000 帧、160 次拒绝与 20 个真实 DOM 负控。原事件、历史 Frame／GEN／publication／occurrence、NativeComponent 活动策略、共同采用和错误隔离见[事件指南](funnel-events.zh-CN.md)与[验收记录](verification.md#development-funnel-native-events-and-tooltips)。
- 泛型原始业务字段、绑定 annotations 与逐属性 Spring 由后续独立开发节补齐。上游 part keyed map 即时移除，不另造退出过渡。
- [ ] 任意可变 D3 generator、builtin 内 Source Widgets 混排和任意实时 UI 导出。
- 新 API 保持 Unreleased，GPU／worker 继续按用户要求后置。实际用法与边界见[双语指南](funnel-svg-layers.zh-CN.md)。

### 开发版：Tree nodeComponent 与 linkComponent

- [x] 四浏览器32上下文/2048帧与CI8/512，独立65680前景/源采样及八项负控通过；整页PNG差异保留为诊断。

- [x] 单次权威布局捕获完整扁平原源、节点拓扑/深高/原值/聚合值、连线端点和真实中点。
- [x] 节点/连线/混合类型化组件，共享源/当前标签池、统一预算及回调前完整预检。
- [x] 原中心与当前偏移 Path 分离，精确零半径准入，透明/无墨迹/空标签和禁用连线资格明确。
- [x] 默认 Scene/SVG 保留，旧 root 行为与 Nivo UID/raw 数值模型差异明确。
- [x] 实际 Memory 状态/焦点及 CPU DPR1/2 发布拒绝回滚，完整应用和中英文指南。
- [x] 原版 UID/计算/组件创建/handlers/Canvas 源参考，离线和篡改核验。
- 标签组件见[真实标签专项](tree-label-components.zh-CN.md)，Glory SVG 函数层适配见[独立指南](tree-svg-layers.zh-CN.md)。
- [ ] 自动 Spring/退出及实时控件导出；原生同层输入和 GPU/worker 后置。

### 开发版：Stream 八类函数型配置

- [x] 四浏览器8上下文/304帧及CI2/76，独立21120像素采样最大误差0，八项负控通过；整页差异保留诊断。
- [x] 标签、源色、原值格式化、完整层边框以及逐点尺寸/颜色/边框宽度/边框颜色函数。
- [x] 全源准入先回调、完整已计算层上下文、末采样格式复用、None 和隐藏资格明确。
- [x] 实际 Mark/图例/切片/提示/typed Widget 同源，组合原几何工厂而不重放函数。
- [x] 修复 Missing 静态域覆盖、隐藏长标签与未绘制曲线的默认准入，旧二进制边界证明保留。
- [x] 原 Nivo 实际函数参考、独立 Memory/CPU 拒绝回滚、可运行完整程序与中英文指南。
- [ ] 任意原 custom layer 插入/生命周期、自动 Spring/退出及控件导出；原生同层输入、GPU/worker 后置。

### 开发版：Tree 五类基础函数配置

- [x] 节点直径/填充、完整端点的连线粗细/颜色、最终节点的绘制标签函数，借用且不保留。
- [x] 全源/静态路径先预检、广度优先顺序、禁用连线仍计算、零尺寸/零线宽及空标签资格明确。
- [x] 静态 inset 投影保留，实际节点/标签间距/typed Widget 共享解析结果，原始业务标签不回写。
- [x] 200 组实际旧/新二进制完整布局与 SVG 输出逐字节一致；Memory/CPU DPR1/2 拒绝回滚通过。
- [x] 原 Nivo 五函数及 active/fix-depth 边界冻结，48 计算与 384 标签定位 profile、离线和篡改核验。
- [x] 四引擎8上下文/368帧与CI2/92通过；16160个受检屏幕点及72720个原生截面像素最大误差0。
- [x] 八项真实负控准确失败，包括只篡改实际 Canvas 线宽、元数据保持不变的独立轮廓拒绝；原始作者错误与屏幕边缘诊断保留。
- 原 Canvas 边内纯色点仍为诊断：760点中15点超1byte；真实线宽另由420个独立基元截面硬门禁验证，未宣称修复 CSS 合成或严格整页等价。
- active/inactive 四个修改器与固定深度颜色继承由后续专项补齐，见下节。
- 标签组件由后续[真实 Label 组件](tree-label-components.zh-CN.md)专项补齐。
- [ ] 自动退出动画与控件导出；Canvas 原始回调和重复层见后续已完成专项，Glory SVG 函数层适配见[独立指南](tree-svg-layers.zh-CN.md)，原生同层输入和 GPU/worker 后置。

### 开发版：Tree 活动修改器、颜色继承与浏览器所有者

- [x] 四个借用修改器、独立祖先/后代节点/连线开关、无 current 的中性状态及固定深度父色继承。
- [x] 最终节点供连线/标签/类型化组件共享，活动状态绑定原布局捕获，拒绝合法布局之间的整体移植。
- [x] 公开 BrowserTree 合并原生主鼠标/笔意图，独立 RAF 和严格 Widget 提交；Geometry 与独立 nearest 活动策略边界明确。
- [x] Memory 状态/焦点与 CPU DPR1/2 发布失败回滚，完整应用及配对便携指南；200 组旧默认输出和 34 组旧基础函数输出逐字节保留。
- [x] 冻结原 Nivo 活动/继承/事件参考，独立离线和有效篡改核验。
- [x] 四引擎加独立 Chromium CI：20 上下文/3300 帧/720 拒绝；229640 个精确颜色点与 1291230 个原生 Canvas 截面像素均最大误差 0，八项负控通过。
- 35660 个颜色族 witness 与 61420 个理想点诊断单独计数，不冒称 SVG 全 AA/空间白界或整页等价；31 对整页 PNG 差异保留。
- [x] 为 BrowserTree 快速 move/down/up 保留原生按压与点击目标，自动活动延后至 click 完成；旧包 WebKit 10 项失败保留，相同四引擎 64 项对照修复后通过，不以稳定悬停或重试替代。显式更新与错误边界见[指针生命周期](tree-input-lifecycle.zh-CN.md)。
- 标签组件另见[三角色组件](tree-label-components.zh-CN.md)。
- [ ] 自动 Spring/退出及实时控件导出；Canvas 原始回调和重复层见后续专项，Glory SVG 函数层适配见[独立指南](tree-svg-layers.zh-CN.md)，最近节点提示的 opt-in 实现已有受检浏览器证据（见[指南](tree-tooltip.zh-CN.md)），原生同层输入和 OS 触控协议仍待补。
- GPU/worker 后置；新增 API 为 Unreleased，不改已发布的 0.1.0。


### 开发版：Tree 真实 Label 组件

- [x] additive 三角色 Scene packet 与标签专用 typed adapter，当前 Scene 顺序、完整原始数据与真实 Label/font/clip，稳定 key 与索引分离；旧双角色 API 保持兼容。
- [x] 节点／连线／标签共享事务与 128 槽预算；Label-only 替换不掩盖节点命中，未归属注释保留普通绘制。
- [x] BrowserTree 三角色 owner 接线及 BrowserChart 任意 Glory 子树仿射 carrier；明确 axis-aligned Rect 与原生 transformed-label Err。
- [x] 本轮 Rust minimal1904/all2240/doc18；同一最终包的五种浏览器／CI配置、两DPR共2520帧统一矩阵通过，独立Python/Pillow逐原件读回；原失败和作者错误分别保留。
- [ ] 自动 Spring/退出及控件导出仍单独推进；Canvas 原始回调和重复层见下节，Glory SVG 函数层适配见[独立指南](tree-svg-layers.zh-CN.md)，GPU/worker 后置。

### 开发版：Tree 原生 Canvas 程序

- [x] 真实 `renderNode` / `renderLink` / `renderLabel` 及可重复自定义层，共享一个原生上下文，保留用户回调之间的绘制状态。
- [x] 权威 `tree_canvas_layout_*` 计算池、完整源与活动、关闭／零宽连线样式及全部标签；旧入口 Debug/Eq 和默认输出回归。
- [x] 私有候选与封存位图、停止／重入门禁、相机／DPR／隐藏重绘、接受位图 PNG 导出；恢复不重放有状态回调。
- [x] 与真实 Node／Link／Label 顶层 Glory 组件联合提交；稳定键和 revision、局部状态／焦点、四类应用拒绝及实际绘制后的 DOM API 拒绝恢复。
- [x] 五种浏览器配置、两 DPR 的原生像素／所有者／联合控件专项矩阵；独立 PNG 解码、严格源数据及原生状态控制、真实跨源污染拒绝。具体范围见[验收说明](verification.md)。
- [ ] 自动 Spring／退出、原生同层输入及实时 Widget 导出继续单列；Glory SVG 函数层适配见[独立指南](tree-svg-layers.zh-CN.md)，最近节点提示的受检范围见下节，GPU/worker 后置。

### 开发版：Tree 类型化提示组件

- [x] 显式启用的完整原始／计算节点数据、独立原生目标、固定／光标及九种自然尺寸对齐；旧默认保持。
- [x] 真实保留式 Glory 内容的可空 frame Bond、revision 身份与源／提示共同接纳；物理命中和程序 activity 分离。
- [x] 受检 opt-in 浏览器输入／像素、源与提示 Effect 同源、错误恢复、CSS 缩放、明确隐藏边界与清理验收；实际矩阵、独立像素读回及负控见[验收说明](verification.md)。
- Source Label carrier 需要可解析的 CSS 尺寸；持续源错误保留共同已接受的源与提示。该范围不等于任意隐藏源或 tooltip-only 降级。
- [ ] 原 Nivo 逐节点事件回调、任意 Datum 与 React/Spring 等价性不由本接口替代；控件导出及 GPU/worker 保持后置。
- API 保持 Unreleased，用法与边界见[Tree 提示指南](tree-tooltip.zh-CN.md)。

### 开发版：Tree SVG 函数层（Glory 适配）

- [x] 字面省略、重复与顺序；完整计算池、原始值、主题、活动、SVG 连线生成器及显式准入预算。
- [x] 每个 occurrence 的真实保留式 Glory 子树与 Node／Link／Label 源组件，独立 revision／Scope／局部状态；业务 Mark 与 canonical 来源保持单份。
- [x] 真实 native SVG 与 foreignObject/XHTML 混排、固定视口绘图区裁剪及 HTML 相机；Mesh／几何／自定义控件按实际层叠路由输入。
- [x] 源、层与提示数据共同采用，采用前 owned carrier 拒绝恢复、停止／清理及同一最终构建的旧入口回归。
- [x] 五种浏览器配置、两 DPR、60 配置／3690 帧完整矩阵通过；真实 DOM 改色／相机偏移保持元数据不变仍被像素门禁拒绝。最终收据与独立读回见[验收记录](verification.md#development-tree-svg-function-layers)。
- 数据共同采用不等于完整受管 DOM 事务：Glory 0.8 延后的后端写入仍可能留下前缀并 fault，须重新挂载；不将 owned carrier 的采用前回滚扩大为此保证。
- [ ] 原始 React SVG/defs、自动 Spring／退出及实时 Widget／重复层导出；更广原生输入与 GPU/worker 后置。
- 新 API 保持 Unreleased，见[双语指南](tree-svg-layers.zh-CN.md)；已发布的 0.1.0 保持原发布状态。

### 开发版：Tree 类型化原生鼠标回调

- [x] 节点／SVG 连线各七种独立回调，Canvas 节点七回调；共享完整事件数据与真实原生事件。
- [x] 已采用 generation、字面 occurrence 与显式 Glory 绑定；原生／几何／Mesh／来源模型分别定义相位。
- [x] Window 晚冒泡同步调用、同节点 Move 不重放布局／工厂、注册替换／Err／停止及旧手势退役。
- [x] 五配置、两 DPR、九场景完整矩阵与独立读回，同包原生测试、旧输入／Canvas／标签／提示／SVG 回归通过。
- [ ] 自动 Spring／退出、控件导出与更广设备验收继续推进；SVG 连线托管提示及独立 anchor 见后节，GPU/worker 后置。
- API 为 Unreleased，使用与验收边界见[事件指南](tree-events.zh-CN.md)和[验收记录](verification.md#development-tree-native-mouse-hooks)。

### 开发版：Tree SVG 连线提示

- [x] 完整 source/target 原始及计算数据、独立 link tooltip options、光标定位与九种自然尺寸 anchor。
- [x] Source／SVG 层／两类真实 Glory 提示共同采用；revision 保留、角色仲裁、候选拒绝与清理。
- [x] 同一构建、同一脚本的五配置／两 DPR／60 场景矩阵及真实 DOM 改色／偏移负控；Chromium 显式采用 `--disable-gpu` 测试配置。
- [ ] 自动 Spring／退出、实时 Widget 导出及更广设备／绑定路线；GPU/worker 后置。
- 受检范围见[验收记录](verification.md#development-tree-svg-link-tooltips)，API 保持 Unreleased。

### 受检完成：Tree SVG 逐属性 Spring 与自动组件动画

- [x] 显式时钟的独立 7/9/3 属性、保速重定向、结构化 Link 键、正确退出配方与原始来源保留。
- [x] 原版六个 transition 函数及 keys 的真实提取执行、独立闭式物理数值与十二项专项。
- [x] BrowserTree 自动 RAF／精确 Manual clock 与采样绘制、动态保速配置，真实 Glory animated-property Bonds 及退出 Scope 保留。
- [x] 实际物理帧与目标数据的拾取／事件／提示协议、重复与 animated SVG 层、Source／层／tooltip 共同采用。
- [x] Chromium／Firefox／WebKit × DPR 1／2，18 配置、450 步；真实组件本地状态、重入、相机／CSS／hidden、零宽链接提示、按住与隐藏时钟和错误回滚通过。
- [x] 原生默认 1986、全特性 2322、文档 18；60 几何组合及 11 个绘制专项、严格 Clippy、格式／拼写／工作流与发布门禁检查。
- 见[双语指南](tree-motion.zh-CN.md)。Canvas／Radial Spring、任意 Widget 导出、更广原生设备与 GPU/worker 保持独立边界。

### 开发版：Waffle 分组函数与原始应用数据

- [x] 原版 Nivo 源码实际提取执行，核对逐组颜色→格式→边框及最后一次空格边框模板。
- [x] 隐藏／零值／溢出组保留源序；Cells／Areas／图例／类型化交互与组合单元格工厂共用最终样式。
- [x] WaffleDataFrame<D> 拥有任意应用字段，类型化单元格组件保留同一完整来源；捕获身份让原始字段及正负零变化可观察。
- 失败候选不替换旧帧，应用闭包的外部副作用不回滚；数值 SceneInteraction 不携带应用类型。
- [ ] 组区域实时组件与 Spring；字面 SVG 整层实现及当前验收状态见后续独立节，GPU/worker 继续后置。
- 新 API 保持 Unreleased，见[使用指南](waffle-functions.zh-CN.md)。
- [x] 最终 10 个浏览器／DPR 上下文、40 场景、240 帧和 160 次候选拒绝；同包 Tree 注释 60 步及 Spring 回归 450 步通过，完整证据见[验收记录](verification.md#development-waffle-functions-and-owned-data)。

### 开发版：Funnel 绑定注解与共享原生文字描边

- [x] 基于原构造 capture 的不可变 Frame／bundle、typed ID／predicate、完整计算 part 与原始数值身份。
- [x] Circle／Dot／Rect、offset／绝对坐标、连接线、文字 halo、自定义 Scene note 及显式工作预算。
- [x] Annotations 字面省略／重复／顺序、原生绘制覆盖与业务输入分离、来源／层／提示共同采用。
- [x] SVG／Canvas／CPU raster 原生 LabelStroke；零尺寸无墨迹与 inactive COLR 兼容修复。
- [x] 原版 144 案例／708 bindings 参考、独立重放、默认 2046／全特性 2391／文档 18 项及严格检查。
- [x] 同一包／脚本的五浏览器、两 DPR、四 profile：40 组／900 帧、120 次拒绝、20 个真实 DOM 负控与 40 个文字描边贡献检查；独立完整读回及原始终态均通过。
- 泛型应用原始类型及绑定注解 Spring 由后续小节实现；逐 note Widget 与任意实时 UI 导出继续单列；GPU／worker 后置。
- API 保持 Unreleased，见[双语指南](funnel-annotations.zh-CN.md)。

### 开发版：Funnel 泛型应用数据全链路

- [x] `FunnelDataDatum<D>` 显式配对业务数据与投影，拥有共享来源的 `FunnelDataFrame<D>` 不增加 D 的 Clone／Eq／Debug／Serialize／Send／Sync 要求；纯模型支持非 static 借用。
- [x] 类型化样式、尺寸、阶段、注解、自定义 Scene note 与旧 Widget 角色访问完整来源；既有纯 `FunnelSvgFrame` 保留 Send／Sync。
- [x] `BrowserFunnelData<D>` 将完整来源共同交给 SVG 层、提示、命中和原生事件；仅业务字段变化也产生新的来源，历史事件保留原帧。
- [x] 失败候选保留旧接受帧、真实组件状态与焦点；program／view／hidden 的同步操作不重放来源工厂，后续真实 Enter／Leave 另行处理。
- [x] 默认 2056／全特性 2401／文档 18 项、严格检查、六个双语 Rust 示例编译及同包五条旧功能回归通过。
- [x] 五浏览器／双 DPR 的 40 组、2010 帧、2020 张原始 PNG、360 次拒绝及 10 个真实 DOM 负控；完整独立读回和原始进程终态均通过，见[验收记录](verification.md#development-funnel-owned-application-data)。
- 逐属性 Spring 与 source-key 休眠记忆见后节；上游普通 keyed map 没有额外退出过渡。
- [ ] 任意可变 D3 generator、内层逐 item Widget 混排和任意实时 UI 导出；GPU／worker 后置。
- API 保持 Unreleased，见[中文指南](funnel-data.zh-CN.md)与[English guide](funnel-data.md)。

### 开发版：Funnel 逐属性 Spring

- [x] 目标来源与采样 Scene 分离，body／border 各自独立路径 progress，从上次提交 TARGET 重置；不同拓扑按真实 D3 数字 ordinal 插值。
- [x] 样式、标签、分隔线、按 binding index 的注解几何／连接线／note 轨道；新角色直接显示，移除即时退场，无退出 ghost。
- [x] 自动 RAF／精确 Manual 时钟、保速配置、系统／显式减弱动画、隐藏暂停和恢复基线。
- [x] typed／legacy owner 的来源／动画／层／提示／原生 capture 共同采用，真实 native path/text 复用及 placement journal 回滚。
- [x] 9 项专项、原版真实 hook／组件／key 请求与 D3 参考、六个双语 Rust 示例编译；完整运行验收进度见[验收记录](verification.md#development-funnel-property-springs)。
- [x] 原生默认 2065／全特性 2410／文档 18、严格 native／WASM 检查与同包六条旧功能回归。
- [x] 同包同脚本的五浏览器、双 DPR、40 组／1930 帧／200 次拒绝／10 个真实 DOM 负控，原始协调器、五条浏览器进程和完整独立读回均退出 0；2340 张原始 PNG 与 2472 批完整事件全部核对。
- [x] source key 仍在时保留无墨迹 body／border 的属性和路径；恢复保留速度，路径从上次真实目标重置。全来源自定义捕获与未知 legacy 自定义目标的拒绝边界见[指南](funnel-dormant-motion.zh-CN.md)。
- [x] 新增 12 项独立原生回归；默认 2077／全特性 2422／文档 18 及严格 native／WASM 检查通过。休眠专项 40 组／560 帧／880 操作由独立 Python／D3 读回通过；同包完整动画回归 40 组／1930 帧／2340 PNG、200 次拒绝、2432 批完整事件也通过正式独立读回。
- [ ] 任意可变 D3 generator、逐 item Widget 混排、任意实时 UI 导出；GPU／worker 后置。
- API 保持 Unreleased；纯采样 Scene 可供 Canvas／raster，专用 owner 为 SVG。见[双语指南](funnel-motion.zh-CN.md)。


### 开发版：Waffle 字面 SVG 层与专属 owner

- [x] sealed 源 capture、实际 cell／透明 group polygon／原生 legend 分离；Cells／Areas／Legends／Custom 字面重排、省略和重复，来源仅一份。
- [x] `BrowserWaffle`／`BrowserWaffleData<D>`、保留式 Glory 整层与完整同源 Cage；shared raw source 无新增 D trait，真实四 hook／标准或保留式 tooltip 共用来源证书。
- [x] 全 Scene／margin 自定义 UI、真实 native 遮挡、默认空图例；source hidden 重排与 camera／program retarget 分开，业务函数不在呈现更新重放。
- [x] 新 SVG IndependentRound 超额请求与角接触轮廓 profile；legacy layout／Vec 存储和准入保持，旧有色 Areas union 不与透明 Areas 混同。
- [x] live／pending／retiring 共享预算、owned placement／focus journal、错误后 carrier 隔离与 revision 恢复；真实 ResizeObserver 与 stop 后弱 handle／原位置输入守卫。
- [x] 通过独立 tooltip 显隐复现确定有限 paint 边界修复，保留原 12 像素回滚零差门；固定原生 pattern 整 tile 参考与错误前景／间距／缺失 motif 负控，不扩为通用 AA 等价承诺。
- [x] 本轮 native 默认 2095／全特性 2440／18 doctest 与 native／WASM 严格检查；完整矩阵 10 个浏览器／DPR 上下文、40 场景、2280 capture、2160 操作、280 拒绝及 20 项真实 DOM 负控通过，来源不变。
- [x] 独立读回通过 7340 次 raw read、2368 次 effect、712 个 hook、920 个 data frame、34720 行 peer 与 3100 张解码 PNG；80 次真实 ResizeObserver、40 项 retiring tooltip 证书与 pattern 全 tile／错误控均核对。CI 同 producer 的 Chromium 双 DPR 步骤及递归证据上传已接线，不代表远端 CI 已运行。
- [ ] Canvas owner、Waffle Spring／exit、builtin 内逐 item Widget、组区域 Widget 工厂、HTML 等价及实时 UI 导出；GPU／worker 保持后置。
- 新 API 为 Unreleased，见[中文指南](waffle-svg-layers.zh-CN.md)。此前函数／cell／regions 的历史通过项保持，不拼接为本轮新 owner 的验收。
