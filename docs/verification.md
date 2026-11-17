# 验收记录

## Kartis 改名验收（2026-11-13）

项目、当前标识与文档现统一使用 Kartis。Git origin 配置为 `https://github.com/taidge/kartis.git`；修改本地配置不代表已迁移或发布 GitHub 仓库。历史验收产物保留原始字节与原路径（包括路径中的旧名称），下方先前各日期记录仍仅属于各自的源码版本。

当前 Kartis 版本重新通过最小配置 1424 项（248 个套件）、全特性／全部目标 1691 项（262 个套件）、17 项文档测试、native／WASM 严格 Clippy 及实际 Rust 1.95 三组检查；248 场景画廊已重新生成。Root 正式执行以 terminal／passed／exit 0 结束，终态为 `target/kartis-rename-root-final-state.json`，逐命令日志为 `target/kartis-rename-root-first.log`。

10 个字体由原生成器重新生成并检查；轮廓、字宽、字符映射与颜色表精确保持，仅 name／head 字节变化，见 `target/kartis-font-regeneration/receipt.json`。原算法数值参考重新执行并检查，见 `target/kartis-rename-reference-regeneration/check-receipt.json` 与 `additional-producers-receipt.json`。四个浏览器消费者重新构建并通过严格检查，原生 headless Apace 消费者实际运行通过，见 `target/kartis-consumers/browser-build-receipt.json` 与 `native-build-receipt.json`。四引擎 Typography、普通文字基线与 COLR 专项共 12 个上下文通过（`target/kartis-consumers/font-browser.json`）；16 个示例加载上下文全部通过（`target/kartis-demo-smoke/report.json`）。这些结果不代表严格 compositor 或完整 D3／Nivo 等价。当前总收据为 `target/kartis-rename/acceptance.json`。

## Calendar／Radar／字距策略新版本（2026-11-13）

本次已整合 Calendar 月间距、Radar 拥有标签文字工厂与 Canvas 字距一致性五份核心源码；正式应用记录为 `target/viora-final-application/receipt.json`。这是下述颜色工厂接受后的新源码版本，旧 `0418…` 冻结记录及颜色产物仍保留其历史范围。正式新版本最小配置 1424 项（248 个套件）、全特性／全部目标 1691 项（262 个套件）及 17 项文档测试通过，native／WASM 严格 Clippy 和实际 Rust 1.95 检查亦通过；终态为 `target/viora-final-root-final-state.json` 的 terminal／exit 0。

`CalendarLayoutOptions::month_spacing` 默认零；FullYears 的正间距沿周轴分隔月份，同时移动日期、月边界及标签，Compact 明确拒绝正值，月轮廓 padding 保持独立。Radar 借用的 `RadarPointLabelContext` 工厂只替换实际可生成的拥有标签文字，保留原始值、逻辑位置、tooltip、交互身份和颜色；几何工厂与文字工厂可组合，回调不被布局／悬停缓存持有。见 [Calendar 指南](calendar-month-spacing.zh-CN.md)与 [Radar 指南](radar-point-labels.zh-CN.md)。

来源及静态绘制证据分别保留：Calendar 的独立八上下文、32 帧 SVGImage／Canvas 检查最大均差 0.15039879261363637，23424 次来源 RGBA 读回最大差零，32 次透明擦除与保存位图恢复零差；这是静态输出范围，见 `target/viora-calendar-independent-review/execution-receipt.json`。组合候选 `target/viora-combined-validation-stage/READY.json`（`9cdc06…`） 绑定五份现已应用的核心源码，八个静态上下文最大均差 0.31948153409090907，仍严格小于 0.5；来源像素、PNG／gzip 读回及位图恢复均有独立记录。这些结果不是新增四宿主交互矩阵，也不替代本版本正式全库检查。

原 Radar 静态检查的 Chrome DPR2 均差 0.5215916193181818 仍按失败保留。真实旧 Root 构建再仅替换六份拥有文字，与原标签工厂输出的完整 Scene／Layout／Binding Debug、SVG 和旧 CanvasScript 逐字节相同，排除了将该失败归咎于新文字工厂几何的说法。修正使用显式 `fontKerning = normal`，保留 alphabetic 基线、原字体与字号，并只在 Canvas 原型提供该成员时设置；未支持时不新建或覆盖 expando。较早无能力 guard 的失败亦保留。

直接 Rust `render::draw_canvas` 的独立真实八上下文记录共 108 次 `fillText`：六个原生支持上下文执行 72 次 normal 绘制，两个 WebKit 上下文执行 36 次不支持回退，不能写成八个上下文都原生支持 normal。18 帧 PNG／gzip RGBA 相等，原型缺失与已有 own setter 的负控证明不创建／写入 expando。现有 begin/reset 仍拥有帧状态；此处不承诺保留调用者整个 Canvas 状态。独立收据为 `target/viora-text-kerning-browser-consumer/execution-receipt.json`（`7b04343a…`）。

兼容性按表示区分：受检布局、Debug、SVG 与 CPU PNG 保持；CanvasScript 有意增加能力 guard／normal 字距策略，22 份组合导出中的 17 份非 JS 相同，五份 JS 只含该 renderer 策略差分。因此不能把更早 152 个配置／228 份导出的全字节证明延伸到修正后的脚本。新默认差分收据 `target/viora-final-default-regression/receipt.json`（`9efd16ae…`）另核 483 份文件：482 份逐字节相同，唯一 records.json 的 128 个配置内 Canvas 字段只有 1230 处预期 guard／normal 插入，896 个字段及整份 JSON 严格复核；152 个默认配置的 176 张 CPU PNG 与其余导出保持，255 份颜色导出也全部原字节相同。 严格物理 compositor、任意字体／设备、完整 React 生命周期和 GPU／worker 仍不在本条接受范围。

## BoxPlot 逐摘要颜色工厂验收（2026-11-13）

颜色工厂受检范围已接受：借用的逐摘要 RGBA 工厂可与图例文字工厂组合；源色贯穿 body、默认离群点、继承描边与 ByLabel 首代表图例，固定描边／离群点覆盖仍独立。控制器缓存结果，真实悬停和图例操作不重调工厂，源替换显式提供所需工厂。用法见[中文指南](boxplot-color-factory.zh-CN.md)与[English guide](boxplot-color-factory.md)。总收据为 `target/viora-colors-function-acceptance.json`（`89948aae…`）。

- **颜色工厂受检源码版本**通过最小配置 1407 项、全特性／全部目标 1671 项和 17 项文档测试，以及 native／WASM 严格检查和 MSRV 检查。这些是该版本的成绩，不是后续 Calendar／Radar／文字渲染整合后的最新全库成绩。204 项源码／包／Native 输入在接受时保持；冻结窗口现已释放，后续变更属于新版本记录。
- 原 Nivo 参考实际执行 32 个主配置、16 个资格配置及 81 次冷调用；部分域预热回调与实际摘要回调分开记录。Rust 每个真实摘要的借用上下文、None 与原 Number(null)、类型化 RGBA 的透明 RGB、非法 CSS／任意属性访问边界均保留明确差异，不宣称完整 React 生命周期或回调次数等价。
- Native 实际生产 63 帧、126 张 CPU PNG、160 次拒绝；独立核验含 1200 个 CPU 色样。四引擎 × DPR1/2 的 504 对 SVGImage／Canvas 整幅比较最大平均通道差为 0.09683333333333333，严格小于原 0.5 门槛；9600 次来源 RGBA 读回保持通道差 ≤2，504 次擦除后位图恢复为整缓冲零差，不冒称 renderer 重绘恢复。

四宿主采用同一完整第二轮：Chrome、Firefox、Playwright WebKit、Edge × DPR1/2 的八个上下文，256 个逻辑宿主、288 次实际挂载、1024 帧、5120 份原始 UTF-8 拒绝返回、6080 次真实图例 pointer／Enter／Space 输入及 992 次真实物理命中全部通过。独立有理数模型重算完整原始输入顺序、R7、full-index band／值投影、函数源色、固定／继承角色、首代表与规范目标；84384 个 body／median-over-body／离群点色样最大通道差 2，包含透明 body 的白色负样本，不以透明墨迹伪造正命中。3552 组正常 Canvas／Raster 整幅恢复、2560 组拒绝操作位图比较及 128 组实际擦除恢复均为零差。

独立产物读回核验 1134384 份 CAS 逻辑／历史引用、41920 个逻辑 PNG 引用（全局 3844 种 PNG 内容），并解析核对 26208 份完整原始 DOM／computed CSS／资源捕获。计数与来源见 `target/viora-colors-function-independent-host-audit/final-receipt.json`（`ec2f70a5…`）及 `terminal-receipt.json`（`6b679dd5…`）；不能把别名数当作独立图像数。

首轮 Windows 报告原子替换 EPERM 及中止的进程、原始报告和图像均保留；第二轮四个原进程与四个独立审计进程均实际退出 0，没有拼接首轮通过项。隔离诊断证明持读句柄可阻止本机 rename、关闭后同一文件可发布，支持与首轮一致的失效机制，但没有直接证明首轮唯一根因。重跑只停止运行中的报告读取，驱动／包／用例与 0.5／RGBA 2／零差门槛均未改。

物理 Chart／SVG 截图仍只作诊断，完整 DOM／CSS／资源及 Canvas／Raster 位图检查保持硬门槛。本条不接纳严格 compositor、物理设备认证、任意原始 getter／React 生命周期完全等价或 GPU／worker；整体 D3／Nivo 目标仍未完成。下文历史条目保留各自源码版本的语境。

## 先前命名版本验收（2026-11-13）

此条记录的是先前命名版本的项目目录、Rust/npm 包、示例入口、CSS/DOM 标识、双语文档及字体检查；1193 个有效源文件的检查数量与下列通过结果仅属于当时版本。原始验收收据保持不变。

该历史版本改名后重新通过最小配置 1407 项、全特性/全部目标 1671 项及 17 项文档测试；native/WASM 严格 Clippy、Rust 1.95 三组检查、四个正常浏览器消费者的严格检查及 WASM 发布构建均通过。248 场景画廊通过 Chrome 原验收入口；四个示例在 Chrome、Firefox、WebKit、Edge 共 16 组加载检查通过，原生 headless Apace 示例也实际运行成功。README 的 15 个完整示例与颜色工厂指南在最小和 raster 配置下运行通过；中英文代码块一致，353 个文档相对链接及锚点有效。

字体使用四个原生成器真实重新生成并执行检查。10 个字体的 Family/PostScript 名称已更新，轮廓、字宽、字符映射和颜色表保持一致；其中 6 个字体还发生 GSUB 原始表重排，独立解码后的表语义一致，未声称只有 name 表字节发生变化。D3 地理包装及 SequentialQuantile 参考重新执行原算法生成并重封来源记录，数值和 CSV 保持精确一致。

字体生成后另重建实际嵌入这些字体的 host/raster 两个示例包；当时四份 WASM 的 ASCII/UTF-16 更早品牌检查均为零。重建后四引擎各完成 Typography、普通文字基线与 COLR 字体三个原专项，共 12 组通过。此补验不更改颜色专项冻结的 Widget 包。

颜色工厂加入前的 152 个默认配置与改名后的同一生产程序输出共 228 份文件通过比较：176 张 CPU PNG 逐字节一致；227 份完整文件相同，另一份 JSON 只有明确的品牌替换。此比较没有将改名前后的所有文本冒称为原字节相同。总收据：`target/viora-rename/acceptance.json`。颜色工厂的新浏览器交互及原生 SVGImage/Canvas 专项另行验收；改名通过不替代该功能的验收，也不代表所有 D3/Nivo 差距已消除。

## BoxPlot 逐摘要图例工厂与类型化目标（2026-11-13）

本轮受检范围已通过功能验收。新增显式 `BoxPlotLegendMode::{Keys, ByLabel}`、借用的 String 图例工厂与普通／共享值尺度入口。ByLabel 按实际非空可见摘要回调，按完整原始域索引稳定排序，精确文字去重保留首个完整代表；空文字仍是有效行，不合并多个代表的隐藏目标。`Scene.legend_targets`、`LegendActivation.target` 与 `LegendRenderContext.target` 将显示行身份和实际 Series／Category 目标分开，四宿主对排队、重入和退役意图复核接纳帧。控制器缓存图例，悬停不重调工厂；显式源替换携带工厂，空源及 legend=false 仍保留构造策略。用法和兼容性见[中文指南](boxplot-legend-factory.zh-CN.md)及[English guide](boxplot-legend-factory.md)。

正式核心最小1392、全特性／全部目标1649、16doctest、native／WASM严格Clippy及实际Rust1.95检查通过；四个正常消费者和完整双语程序的五条正式验收命令通过。修改前后同一生产程序的152配置、176份CPU图像、228份完整导出逐字节相同，248张画廊SVG及index亦相同。默认Keys与Debug保留旧输出；新增公开结构字段仍需要调用者调整完整struct literal，不能把默认Debug兼容冒充完整源码兼容。

原始参考真实执行完整Nivo hooks／legend／property accessor及Lodash：32主配置、8资格，共40个主冷快照，108份computed summary、100份回调上下文和62行图例。Rust测试逐项区分None与原Number(null)、隐藏源过滤、String返回及属性访问边界；冷适配器不是完整React生命周期或回调次数等价证明。新Native共享应用7配置×4阶段，另28份真实Surface Move悬停，共56帧／112份CPU PNG；84次API尝试与4次额外工厂策略拒绝合计88次。独立数学读回、真实悬停与blank恢复、PNG CRC／filter解码和408个整body内点通过，源像素最大差1≤2。Native非法操作导出保留原始拒绝标志，不冒称每条均有完整before／after图像；宿主原子性另由下述矩阵验证。

同源Native导出在四浏览器×DPR1/2完成448对原SVGImage／Canvas整幅比较，最大平均通道差0.1781684027777778，门槛保持严格小于0.5；1632个独立来源位置的双buffer RGBA检查最大差1≤2。448次实际透明擦除后只用保存位图putImageData恢复，整缓冲0差，没有调用renderer重画。独立复核2688份逻辑PNG（全局441种内容）、4935个逻辑CAS项及614个对象。结果见`target/box-legend-factory-independent-native-browser/terminal-receipt.json`。Native库、生产程序、字体和绘制资源未随宿主报告存储修复变化，freeze4到freeze5的唯一非Native observer差异另有逐字节输入等价记录。

最终采用完整第四轮四个原进程实际退出0的结果，独立离线四个原进程也均实际退出0：8上下文、224逻辑宿主、896阶段、64次真实动作重挂载，合计288次实际mount；5152次真实native图例输入、2688份原始非法返回及2816次控制器拒绝通过。26272个来源像素最大RGBA差1，19040份原始DOM／computedCSS／资源捕获严格核对，3024组Canvas／Raster完整位图比较保持0差。32640份逻辑PNG逐一核验原字节并CRC／inflate／filter解码；各引擎unique合计4442、全局内容3106，不把别名当作独立图像。451156个最终CAS逻辑项另有451636份历史引用，全局38343种对象内容。最终独立收据为`target/box-legend-factory-independent-host/final-receipt.json`与`terminal-receipt.json`；最终三份验收文案更新前，代码、Glory、完整包和Native资源的2454项冻结闭包逐hash保持；更新后为2451项保持及3份明确文案差分。

准备及前三轮失败均保留。原预检的重复factoryTrace逐位浮点比较误拒约1e-14差异，仅改为既有1e-8几何容差。首轮合法输入在首次flush／RAF之后仍有两条待排观察；独立证据分别为五个row0 Space、两个row1 pointer和一个row0 pointer，只为合法输入补第二次flush／RAF，非法与零发布路径不改。第二轮夹具清空App却仍持有旧WebApp，动作重挂载被真实root claim拒绝；修复为实际take／drop guard，没有手删所有权属性，也没改Glory。第三轮Firefox／WebKit完整通过，Chrome／Edge在大型报告JSON.stringify触及单字符串上限而失败，整轮不计通过。最终仅改分块完整JSON写入与独立读取，字段、schema、全部数学／输入／像素门槛不减；实际597689338字节压力样本与230739309字节真实上下文逐字节对照通过，最终Chrome／Edge大于旧单字符串上限的报告正常终结。第四轮没有拼接历史通过项。

物理Chart／SVG截图仍只作诊断，不宣称严格compositor、任意字体／设备／平台或GPU通过。case6的旧DOM click是显式合成负控，没有配置viewport回调profile，不能据此宣称私有外层viewport guard已被该运行直接覆盖；空Category行是合法调用者Scene元数据，不是伪造BoxPlot摘要。静态导出保留图例元数据，不直接绘制交互图例UI。逐摘要颜色函数正在独立候选验证，尚未合入本轮正式版本；GPU／worker继续暂缓。

## BoxPlot 整部件指针与 Glory 有限坐标输入（2026-11-13）

新增显式 PaintedGeometry 绑定，把实际箱体边框、中位线、须、帽和离群点映射回每箱唯一的逻辑 body 站点。SurfacePhysicalHit 分开保存物理 mark、逻辑站点与真实离群点原始 ID／值；成功源、隐藏或视图替换使旧 token 失效，非法候选保持接受帧。拾取检查真实 solid RGBA 墨迹、stroke/cap/dash、clip、绘制顺序与组合透明度，不增设透明代理 mark 或额外键盘 datum。原 body-only 入口保持；新 chart_layout、BrowserChart 和 RasterSurface 共用同一受检 binding 与模型。见[中文指南](whole-part-pointer.zh-CN.md)与[English guide](whole-part-pointer.md)。

Glory 新增 View 的 PlotPointer 有限坐标通道，保留原 Pointer／PointerWithin 契约。 `SurfaceEvent` 与 Glory `Event` 增加事件变体，外部穷尽match需补充分支；`InteractionBinding` 新字段可用 `Default` 保持旧profile。Web 保留真实 fractional client 数值及同次 viewport 范围；Apace 接实际引擎输入。事件通过正常队列分发，用户回调期间不持有模型／renderer 借用。语义 Chart 继续绘制原 Graphic tree，数据按钮保留键盘与 focus，严格配置关闭其 pointer 命中；Raster 只使用真实 image-host View 坐标。source／dispose 与捕获终结有独立生命周期，Web 仅真实 Window blur 取消 lease，子元素 blur 不吞掉尚未分发的 down。

正式最小1357、全特性／全部目标1603、16doctest、native／WASM严格Clippy、实际Rust1.95与四正常消费者编译／发布包通过。Glory 工作区1277项测试与12doctest通过，2项真实平台检查保持忽略，不声称已运行设备验收。五条双语完整程序命令通过。152份默认配置、176份CPU图像、248张画廊SVG及index逐字节保持。独立原生200对SVGImage／Canvas整幅均值最大0.41602951388888887，保持0.5门槛；800来源位置的两renderer通道误差最大1，保持2门槛。1250个PNG路径及800份gzip RGBA已独立解码核验。原生擦除后通过实际位图putImageData恢复，不能称为renderer重绘恢复。

Chrome、Firefox、Playwright WebKit、Edge × DPR1/2的同一完整第三轮四个原进程均实际退出0：128宿主、512阶段、1024次真实非法controller操作拒绝、128次实际擦除恢复及释放、32次真实元素blur／Down／Up回归。独立重算1856次正命中与768次可信Move负点／64次可信Leave终结；1088次同owner部件转换保持绘制，384份源替换历史保留旧快照。1888个来源色样本（1856主步骤＋32焦点补充）最大RGBA通道差2，门槛仍为2；1136组Canvas／Raster全缓冲比较精确，完整原始DOM、所有枚举computedCSS、接纳状态及资源严格比较。6480个逻辑PNG引用逐一核验原始对象并独立CRC／inflate／filter解码，各引擎unique合计696、全局不同内容499；13123个cohort本地CAS对象对应全局9695种内容，不把重复存储或别名数冒充独立图像。128个实际host各有可信native→actual viewport→delivered小数局部坐标见证，最大误差2.842170943040401e-14，1856个截断反事实均拒绝。独立收据见`target/box-parts-pointer-independent-host-audit/final-receipt.json`，汇总为`target/acceptance-summary-box-parts-pointer.json`。

Native生命周期夹具最初误用了不合法的disabled View声明，修正为有效Fieldset传播与非法声明原子拒绝；没有放宽Semantics，也没有保留临时disabled检查旁路。Web子元素blur吞掉待分发Down则是实际产品缺陷，本轮已修复并通过32次真实焦点回归。

首轮完整四引擎×DPR1/2因夹具try_invalid入口额外flush推进Browser revision而失败，8个上下文和原始源码／包／CAS完整保留。独立复核确认无效controller操作发生于该次重绘之后，拒绝分支没有publish；修订仅移除一条夹具前置drain，并新增driver断言前保存原始返回字节，全部原比较门槛保持。首轮返回值未在断言前落盘的证据限制明确保留；完整新轮结果独立计算，不拼接首轮通过项。

第二轮所有128宿主／512阶段均走完，但WebKit两种DPR在末尾“native client本身必须含小数”资格断言失败。实际可信事件的client为整数，而actual border-box偏移含小数，SurfacePhysicalHit已逐点保留相减后的小数局部坐标。独立复核区分浏览器交付值与框架View-local契约；新资格要求真实native／viewport／delivered hit严格一致且每宿主有实际小数local见证，客户端分数计数仍原样报告，不声称WebKit自身交付分数client。全部原point mapping、RGBA、零差回滚与0.5比较门槛保持，完整第三轮从新上下文运行，第二轮按失败保留。

本轮只接纳 Geometry 的 bounded solid-paint profile；Nearest／Slice／polar、owned渐变／图案、严格Raster与editable brush组合均明确拒绝。Web仅支持正向轴对齐二维scale／translate，Apace当前拒绝含computed transform的祖先链；曲线和clip有各自有界近似。物理截图保留诊断，严格compositor、任意平台变换、真实触摸／笔／辅助技术设备、默认Firefox Sans与GPU／worker均未因此改绿。完整逐摘要legendLabel factory／ByLabel继续按差距清单推进，Map不替代该模型。

## BoxPlot 按身份配置图例文字（2026-11-13）

`BoxPlotLayoutOptions.legend_labels` 按当前 Group／SubGroup 配色所选的完整原始域配置文字。None／空map保留旧回退，显式空文字保留；未知键和超过1024 UTF-8字节的文字在建图前拒绝，包括隐藏或保留键。图例原ID、Series／Category角色、颜色、顺序和资格保持，真实controller源替换仍原子。重复或空文字保留独立身份，不冒称原Nivo按label去重模型；完整逐摘要legendLabel factory另列为未完目标。见[中文指南](boxplot-legend-labels.zh-CN.md)与[English guide](boxplot-legend-labels.md)。

最小1326、全特性／全部目标1569、16doctest、native／WASM严格Clippy、实际Rust1.95、四正常消费者及双语完整程序均通过。原版属性访问器及Lodash原函数实际运行30配置、7边界，保留97快照、220份computed box和660次真实回调；差分明确区分唯一文字、重复／空文字及一个key产生多label的资格。152份完整旧默认配置、176份CPU PNG、248张画廊SVG与整份index逐字节保持。16新帧的128对独立SVGImage／Canvas整幅比较通过，最大平均通道差0.03575347222222222，门槛仍为0.5，并独立核对来源像素的2通道门槛。

Chrome、Firefox、Playwright WebKit、Edge × DPR1/2 的同一完整第二轮全部实际终态0：256宿主、1024阶段、3072次非法操作、256次擦除恢复及释放。2560次真实图例操作含1024次pointer、768次Enter和768次Space；Series按ID隐藏恢复，Category默认静态、显式动作保持分类与隐藏分离。独立核对10816个来源像素、2752个转义文字见证、9984份capture manifest和全部27442个CAS对象；29536个逻辑PNG引用保存为原始字节对象与完整索引，不把别名数冒充独立图像数。只改caption的阶段另核128对Canvas／Raster完整RGBA相同、64对实际SVG完整绘制树与属性相同，仅允许预期title／desc叶文字改变。6对物理截图不完全相同保留为诊断，严格compositor不据此改绿。

首轮driver误要求语义隐藏删除路径；原始DOM与computed CSS证明正确行为是祖先display:none。修订仅让校准读取真实可见路径，8份原始树及24个负控离线核验通过，模型、颜色、像素和完整非法更新回滚门槛未改；第二轮从头运行，不拼接首轮通过项。首次widget编译的handle调用及模块格式问题已修正并保留原日志。1055条代码／夹具冻结路径、实际package与执行模块图在验收前后保持。独立收据见`target/box-legend-labels-independent-audit/final-receipt.json`，汇总为`target/acceptance-summary-box-legend-labels.json`。整部件真实宿主指针与完整按label模型继续推进，GPU／worker继续暂缓。

## BoxPlot Group 配色与受控关联（2026-11-13）

`BoxPlotColorBy`保留SubGroup默认，Group模式使用完整原始组域选色并按实际有效摘要生成Category图例。`BoxPlotInteraction`保存构建源与完整接纳帧，激活关系来自同group或同subgroup；组合透明度覆盖body、border、median、whisker、cap和实际离群点，成员RGBA与fill_opacity不重复相乘。独立SVG和保留式SVG使用真实父g，Canvas／CPU使用有界透明源，语义Chart使用CSS父层。组0参与可见性过滤，单帧组因子索引避免全图遍历中的二次扫描。

最小1318、全特性／全部目标1556、16doctest、native/WASM严格Clippy、实际Rust1.95与四正常消费者的严格检查和release重新构建均通过。完整双语Rust程序在Rust1.95下分别运行最小与raster配置，并通过WASM严格检查。首轮全量严格检查发现新增测试一处iter().copied().collect风格警告，唯一改为to_vec后重新完整检查通过；生产代码未因此变化。最终1042路径冻结修订及实际状态／日志见`target/box-colorby-hover-core-audit.json`。

同一未修改的基线生产程序分别在修改前与最终库运行128配置，完整布局、Scene、绑定、SVG、Canvas、pretty与128份CPU PNG逐字节相同。248张默认画廊SVG和整份index也逐字节相同。共享新应用6配置×4阶段保留48份CPU PNG；四浏览器×DPR1/2的192对SVGImage／原生Canvas整幅比较通过，最大平均通道差0.026265625，门槛保持0.5，并有48次真实擦除和整缓冲恢复。原生导出工具曾沿用旧的builder计数字段名称，已更正为真实controller操作拒绝；97份渲染／字体文件逐字节不变，修正元数据后使用新目录完整重跑，没有拼接首轮上下文。

新增原函数参考来自完整原hooks、transition和getLegendData的实际执行：32配置、64基态／激活快照、10边界、272份computed box记录。Rust的三项参考测试按24非nullable配置／80箱体核对统计、几何和RGBA，直接把原memo记录的activeKeys数字映回原身份再比较关系集；8 nullable配置与10边界独立记录差异。未用重写OR谓词产生参考期待，也未把旧210个Group角色结果当作新图例证据。

四类真实保留宿主的最终完整矩阵及独立审计已通过：8上下文、192宿主、768帧、2304次真实controller非法操作、192次实际擦除恢复及释放检查。5376个来源像素保持RGBA2门槛，Canvas／CPU的完整位图恢复和非法更新保持零差；完整原始DOM、CSS、接纳状态和资源也精确比较。独立审计解码14880份PNG（431种唯一内容），另核18368个来源mark的RGBA／成员透明度、9184份实际SVGpath paint，以及2976份黑色CDF标定的3490848个条带像素与探点区间绑定。最终证据见`target/box-colorby-hover-host-browser-third/tool-terminal-observations.json`与`target/box-colorby-hover-independent-host-audit/final-receipt.json`。

前两轮失败完整保留。首轮正常恢复时，Glory因Group↔Mark拓扑变化重分配内部节点与anchor编号；8份真实前后树离线核对后，其余字节完全一致，只为语义正常发布引入数字alpha-renaming，保留类型、位置、数量及唯一且LIFO平衡配对，6种实际负控仍拒绝opacity、path和anchor结构破坏。第二轮Chrome/Edge出现5260通道／2051像素、最大31的真实Canvas重绘差异；独立50实验／856捕获证实，反复读回后只重发同一Scene也足以复现，不需要分组切换。Canvas首次显式willReadFrequently配置与该变化相关，但未直接追踪内部GPU切换。

最终观测从实际live Canvas位图按1:1、identity transform、无插值复制到独立sRGB观测画布，再读取RGBA和PNG；没有依据geometry／facts重绘。相同帧copy/direct对照精确，反复复制观测不会复现直接读回的重绘干扰，正常位图全alpha255、擦除位图全零也逐缓冲核验。WebKit未返回的context属性不冒称已实测；请求配置、实际可读属性与不透明像素资格分开保存。源期望、22个数学／事务函数和每阶段三个非法操作块逐字节保持，RGBA2、整缓冲零差及整幅0.5门槛均未放宽。最终第三轮从全新8上下文完整运行，未拼接此前Firefox/WebKit成绩。物理Chart/SVG截图仍仅作诊断，不能据此宣称严格compositor、任意读回／GPU／software profile、系统字体或物理设备通过。

本批接受的是受控状态关联、真实整组合成和controller对全部实际部件的直接命中查询。每箱一个body站点的标准Browser／Raster绑定不自动为须、帽等装饰部件提供整g指针路由；程序调用set_active不等于原Nivo DOM hover已经对齐。任意legendLabel／颜色访问器、数字图例身份、上游null强制转换和stale active对象、语义分组拓扑的节点身份、spring/property动画和嵌套隔离层均有独立边界；GPU/worker继续暂缓。完整用法见[中文指南](boxplot-color-and-hover.zh-CN.md)与[English guide](boxplot-color-and-hover.md)。

## RadialBar / BoxPlot 描边角色与零宽边框（2026-11-13）

已实现RadialBar扇段边框与BoxPlot箱边、中位线、须的可选继承色。固定默认、width/cap、原始palette身份及各图型透明度规则保留；须与端帽共用颜色，Radial styled收缩弧使用解析后的stroke alpha判断墨迹。两份完整指南见[中文](radial-box-strokes.zh-CN.md)和[English](radial-box-strokes.md)。

正式旧库与最终新库使用同一原生生产程序运行128组默认场景：64组RadialBar legacy/styled、64组BoxPlot legacy/scaled，完整Scene/布局/绑定/SVG/Canvas与128份CPU PNG全部逐字节相同。新共享模型9配置×4阶段的288对SVGImage/Canvas整幅比较通过四引擎×DPR1/2完整新运行，最大平均通道差0.161629464，小于原0.5门槛。保留72份实际CPU渲染；含128组默认前后图像在内的704个PNG已独立校验与解码，不以此声称CPU与浏览器整幅字体相等。

最终最小特性1293项、全特性/全部目标1523项、16 doctest、native/WASM严格Clippy、实际Rust1.95检查，以及四个正常Glory消费者的严格检查与重新构建全部通过。中英文完整Rust程序在最终库实际完成两次原生运行和WASM严格检查；README旧15对代码块保留，配对指南与README无机器环境依赖表述。1010个代码/夹具路径在终验期间冻结，248张默认画廊SVG与整份index逐字节保持。

最新四宿主完整新矩阵的四个进程均实际退出0：8上下文、288宿主组合、1152阶段绘制、576次真实非法配置构建拒绝，以及288次实际资源擦除/完整资源恢复和释放检查。独立审计解码7920份宿主PNG、复核14336个来源颜色探点、1584次独立黑色矩形坐标标定、2448组比较，并直接核7328个SVG路径和5600个stroke属性；与原生/default证据共8624个PNG。新增零宽case直接把Some(width0)送入公开BoxPlot builder，128帧配置记录和448个整像素外侧白底见证证明无边框墨迹；与None的Scene/SVG/CPU及各宿主原生缓冲一致。

原Nivo完整hook、transition和实际JSX组件参考已在正式目录重新生成检查通过：32组Radial配置/128个计算段、28组BoxPlot配置/140个箱体/420个角色输出，另有11组资格、6个原生错误和3个任意属性边界。Rust实际builder对照检查可绘制Radial角色24项及4项alpha-zero差异、28项禁用边框；Box角色190项及20项alpha-zero差异，须和端帽分别核验。原Group模式的210个角色结果独立保留为后续差距，未冒称已实现group colorBy。原Nivo零范围弧、Number(null)、上游删除与Rust typed缺项/隐藏的差异均明确记录。

原函数对照还复现了BoxPlot的旧问题：可选箱体/离群点边框width为正零或负零通过布局样式校验，随后却被Scene拒绝。新增测试先真实RED，修复仅把这两处零宽描边归一化为无描边，同测试GREEN；离群点填充、正宽/cap、角色颜色预检和median/whisker原零宽规则保留。Radial的Some(width0)仍拒绝，使用None禁用。旧8场景矩阵、导出工具陈旧显示计数和首轮对照失败均保留；最终新9场景与动态真实计数重新生成验收，不拼历史成绩。

Chart/SVG物理截图的16组非完全相同结果单列诊断，最大3385个通道变化、最大通道差30；它们不被笼统归因于轻微抗锯齿，也不被称为物理屏幕精确回滚。完整canonical DOM/资源、相关CSS和已接受状态严格保持；Canvas/Raster原生完整缓冲与资源恢复严格一致。来源色误差2和原生整帧0.5主门槛保持。汇总为 `target/acceptance-summary-radial-box-stroke.json`。严格compositor、默认Firefox Sans、平台SVG量化和物理设备仍未因此改绿，GPU/worker继续暂缓，整体对齐目标保持进行中。

## Bar / HeatMap 边框颜色与宿主修复（2026-11-13）

已实现两图型的普通边框 `InheritedColor`、Normal多paint整体透明度单次合成，以及语义Chart的Rect标签裁剪。未配置边框不自动启用，颜色仍取预工厂数值palette；NULL/零值、自定义stroke覆盖和独立opacity保留。完整用法见[中文指南](border-color-roles.zh-CN.md)与[English guide](border-color-roles.md)。

正式最小1271项、全特性/全部目标1496项、16 doctest，native/WASM严格检查、Rust1.95、四个Glory消费者及双语完整程序均已实际通过。四引擎×DPR1/2的320对整幅SVGImage/Canvas比较通过，最大平均通道差0.0500595，原门槛0.5不变；56帧真实Rect裁剪、仅裁剪更新、负原点/零宽度、Circle拒绝回滚与恢复通过。613个PNG已独立完成SHA、CRC、解码与尺寸校验。

全新Chrome、Firefox、Playwright WebKit、Edge × DPR1/2的四宿主联合矩阵通过，四个实际进程均以退出码0结束。8个上下文覆盖320个宿主组合、1280次阶段绘制、640次真实非法配置构建拒绝、320次实际绘图擦除/完整资源恢复及320次释放检查。8800份宿主PNG与gzip RGBA、5120个来源颜色探点、1760份独立黑色矩形坐标标定和2720组比较已独立复核。原始NULL/零值、几何、numeric palette与工厂画笔、owned label及回调顺序分别核验，不仅检查两点或编译结果。

静态无操作控制复现了DOM、CSS和已接受状态完全相同时的物理截图波动，因此使用完整canonical资源与已接受状态的精确比较判定事务身份；Chart/SVG物理截图逐通道差异保留为诊断，不宣称bitexact或严格compositor通过。最终69组非完全相同的物理截图均明确记录，未逐场景扩大噪声门槛。Canvas全缓冲、Raster原src与解码RGBA仍完全恢复，主来源颜色误差2和原生整帧0.5门槛不变。首轮Normal对象透明度、Rect标签准入、坐标栅格假设及圆角处相邻描边叠加造成的错误取样均保留；仅真实产品缺陷修改生产代码，测量修正不改原图形/opacity/边框或主门槛。最终采用同一冻结源码/夹具/pkg的完整新矩阵，不拼接历史部分成绩。

128组旧库默认对照的布局、SVG及工厂上下文全部一致；Canvas/CPU各112组完全一致，16组变化均对应旧SVG实际要求的部分透明度fill+stroke整体合成。248张画廊SVG保持不变，index的7段Canvas脚本为33个合格mark补齐隔离合成，其他脚本及非脚本HTML逐字节相同。983个正式代码/夹具路径在终验期间保持冻结；完整汇总见 `target/acceptance-summary-border-color-role.json`。GPU/worker、默认Firefox Sans、平台SVG量化与严格compositor均未因此改绿；整体对齐目标继续进行。

## BoxPlot 汇总自动域与 Bar 分类图例（2026-11-13）

本轮已实现并验收 `BoxPlotAutoDomainSource::SummaryQuantiles`、`BarLegendSource::{Keys,Indexes}`、显式完整 `series_order`，以及共享 `LegendRole::{Series,Category}` 与四宿主的类型化激活。旧SampleExtent、Keys及词法顺序默认保留。Summary使用原Nivo有限插值与稳定数值排序，仅对auto UTC域端点做TimeClip；窄UTC域1毫秒和正负零顺序的真实修复前失败均保留。原时间工厂[0,0]的falsy回退不复制。Index来自原始nullable候选，包含None/zero/category但不造None墨迹；分类和series同名也不自动隐藏。分类缺省静态，显式启用后通知原始ID；Series先通知意图，再按当前接受帧复核并提议Hidden。完整用法见[中文指南](summary-index-legend.zh-CN.md)和[English guide](summary-index-legend.md)。

正式最小特性1244项、全特性/全部目标1461项、16 doctest、native/WASM严格Clippy、实际Rust1.95.0检查，以及四个正常Glory消费者的严格检查和release构建全部通过。完整双语程序在最小和raster原生实际运行，WASM同时开启web/raster编译两个mount helper；不把helper编译当作运行。README中英文15对Rust代码及新指南一对完整Rust程序/一对TOML配置相同，56个相对链接可解析，README与本轮配对指南无机器路径或本机环境依赖措辞。

全新Chrome、Firefox、Playwright WebKit、Edge×DPR1/2的8上下文全部通过：32个宿主组合、72次真实mount、72张根截图和各上下文内去重后合计116张绘图PNG，共188张。本轮验收实际挂载Chart、Browser SVG/Canvas与RasterSurface；验证静态分类、pointer/Enter/Space激活、role-only重建、同名series显式隐藏、更新label后原ID、源数据更新、32次真实非法角色拒绝、32次实际绘图资源擦除/完整恢复，以及64次旧分类/释放后DOM节点失效检查。Semantic Chart另实测受控Hidden拒绝/接受、自定义内容与真实Canvas测量。报告保留原始事件和测量；Browser/Raster没有虚构外部受控Hidden或自定义插槽能力。

绘图检查先以独立的源数组、唯一bar身份/索引、完整Band/线性域公式和两种原始palette值验证metadata，再核实际像素。完整RGBA/PNG与实际Canvas全缓冲、Raster原src、SVG DOM资源哈希支持拒绝回滚和擦除恢复；恢复由driver当场检查完整DOM/src/全buffer哈希及实际画面PNG，不只比较两点；离线独立审计重算已落盘PNG的SHA/RGBA，未存DOM原字符串，不声称离线重算全部DOM。Semantic截图按Playwright实际enclosing integer clip保留完整图像并映射分数CSS坐标，未缩放截图或放宽颜色门槛。Canvas按真实设备DPR，SVGImage解码与CPU Raster配置为480×320；DPR2浏览器上下文不冒称CPU二倍渲染。此为功能与资源保存专项，不是跨renderer全幅均值相等或严格compositor验收。

原Nivo生成器在正式目录重新check通过：Bar32个profile/212个原始bar（106个null）/128项分类图例；Box基础24组及1046组插值边界参考。独立数学审查另核5230个统计量bits、2092个域端点和5230个投影结果。963个正式代码/夹具路径在最终检查后保持冻结，248张默认画廊SVG和整份index逐字节一致。38项源码迁移之外，两处消费者新增事件match分支及测试Widget的raster条件导入修复均有独立diff；未迁入候选manifest、alias、cache或构建产物。

所有首轮失败保留：候选数学红例、错误依赖版本被pin拒绝、测试特性导入及示例枚举缺分支、夹具无效name/按钮子树、观察函数多余flush，以及截图整数clip假设。修正后使用完整fresh矩阵，不拼接前几轮的局部通过项。缓存删除命令曾被自动审批以blocked by policy拒绝且未执行；磁盘空间外部恢复后依各workspace正常配置完成构建，无缓存路径重定向。完整汇总为 `target/acceptance-summary-summary-index-legend.json`。GPU/worker暂缓，默认Firefox Sans、历史严格compositor和保留式SVG量化仍保留原结论，整体对齐继续进行。

## Bump 端点、Radar 点颜色与 BoxPlot 独立上下界（2026-11-12）

三项已正式实现并验收：Bump 的 `start_style/end_style` 可逐字段覆盖两端 padding 与 InheritedColor，保留共享字号、源文字、缺段规则及 alpha×series opacity；Radar 的 `fill_color_profile/border_color_profile` 为普通圆点、旧 shape、共享 glyph 和工厂上下文解析点画笔，保留 polygon/series swatch 与普通站点、custom paint 代表色的区别；BoxPlot 的 `domain_bounds` 为每端分别选择自动或固定值，保持原样本与统计量并沿用值轴、nice、UTC、裁剪及离群点规则。完整契约与可运行程序见[中文指南](endpoint-point-bounds.zh-CN.md)和[English guide](endpoint-point-bounds.md)。

正式回归最小特性 1215 项、全部特性/全部目标 1429 项、16 doctest、native/WASM严格Clippy、实际Rust1.95.0，以及四个正常Glory消费者的严格检查和release构建全部通过。双语程序主体在正式库实际运行通过，既有正常消费者仅追加导出后缀生成九场景。README中英文15对Rust代码块保持一致，配套指南各一段相同完整程序；52个仓库链接均可解析，README和配套指南不含机器路径或本机环境依赖措辞。

全新Chrome、Firefox、Playwright WebKit、Edge×DPR1/2的8上下文通过72对完整SVGImage/Canvas绘制比较，最大平均通道差0.044575602，原门槛0.5保持。320个实际DOM/Canvas负控检查源文字、颜色、端点padding、真实路径与像素缓冲保存；72次实际白画布均恢复完整原始RGBA。24次稀疏擦白（Bump两帧及Radar描边帧×8上下文）的整幅均值低于0.5，因此擦白明确依靠原native缓冲精确保存门槛拒绝，不能声称所有擦白都超出均值阈值。

450张PNG（288原生/解码/恢复/白缓冲、144根场景截图、18CPU）及408条原生fillText的独立终验见汇总。几何核验使用实际Circle四段Cubic、D3描边Plus和工厂Rect完整支持点；同包围盒错形另有独立负例。Box原始极值1..4、Q1=1.75/Q3=3.25/median2.5在三个独立域中保持原义。原Nivo参考包括18条Bump端标签与48组Box上下界，可由冻结源码和许可证重生成；None/空覆盖的Bump128组、Radar64组以及Box96组完整输出/32项原严格错误均在真实旧库上对照，不复用本轮改动后的库冒充旧基线。

933个正式代码/夹具路径在最终检查期间保持不变，248张默认画廊SVG和整份index逐字节一致。本轮只迁20个生产源/测试/小参考文件，候选manifest、依赖alias、缓存与产物均不迁入。候选首轮NaN比较、无PartialEq类型断言、测试语法/非法nice假设及一处等价Clippy let-chain修正均保留原日志；未通过放宽数值或像素阈值修正。汇总为 `target/acceptance-summary-endpoint-point-bounds.json`。GPU/worker暂缓，默认Firefox Sans、历史严格compositor和保留式SVG原平台量化不据此改绿，整体对齐仍在进行。

## 径向标签配置与 BoxPlot 分类带（2026-11-12）

正式实现并验收三项具体配置：RadialBar 的 `RadialBarLabels.color_profile` 按原分类 palette 解析共享继承色，保留标签 alpha 独立于扇段 opacity 的原契约；Sunburst 的 `sunburst_layout_with_label_factory` 在捕获测量签名前确定所属文字，原 datum、聚合值、formatted value、tooltip 与几何保持原义，空串保留槽位；BoxPlot 的显式 `category_scale` 将圆整、对齐和间距用于完整分类带，轴、子箱、中位线、须和离群点共用该结果。None 默认输出保留；Box 反转与额外共享带配置明确列为 D3/Rust 扩展。见[中文指南](chart-labels-and-bands.zh-CN.md)与[English guide](chart-labels-and-bands.md)。

当前正式版本通过最小特性 1190 项、全部特性/全部目标 1404 项、16 doctest、native/WASM 严格 Clippy、实际 Rust 1.95.0，以及四个正常 Glory 消费者的严格检查和 release 构建。双语完整程序在正式库实际运行通过，再由既有正常消费者生成六个普通/显式配置场景；没有复制或重定向构建缓存。Root 首次格式检查发现新测试多余尾部空行，原日志保留；修正后重跑整套正式回归，未改生产核心。

全新 Chrome、Firefox、Playwright WebKit、Edge × DPR1/2 的8个上下文全部通过，共48对整幅 SVGImage/Canvas 绘制比较，160个实际 DOM/Canvas 破坏负控，48次全白画布后的完整缓冲恢复。最大平均通道差 0.031424397，低于原0.5门槛；WebKit所有帧为零。208条原生 fillText 按原脚本逐槽位核对，包含8条真实空文字调用。300张PNG（192原生缓冲、96根场景截图、12 CPU图像）均已独立校验SHA、CRC、尺寸及RGBA；50个非空CPU字形区域含2,904个正确来源色的内部像素。空Sunburst标签保留原橙色弧形背景，不将其误称为白色背景或文字墨迹。

旧库差分分别覆盖 RadialBar64、BoxPlot32、Sunburst128组默认场景；BoxPlot96组原D3带刻度夹具可由保留许可证的原源码重生成。916个正式代码/夹具路径在最终检查期间保持不变，248张默认画廊SVG及整份index逐字节不变。Root合入只迁9个源文件/测试/小夹具，另单独记录测试格式修正；临时候选manifest、依赖alias、source bridge和产物均未迁入。完整汇总为 `target/acceptance-summary-chart-options.json`。

Sunburst沿用受管标签签名，不将未管理根节点扩大成整棵层级的冻结协议；依赖具体颜色上下文的错误可能发生在文字回调之后，外部副作用不回滚。BoxPlot显式保留的可见子组槽位即使无观测值仍要求正宽度。这里不接受物理设备、全平台字体或历史严格compositor，默认Firefox Sans、保留式SVG平台量化仍未改绿。GPU/worker继续暂缓，整体对齐目标仍进行中。

## Bar / HeatMap 逐源标签文字与零值轴长度标签（2026-11-12）

三项已完成正式实现和验收：HeatMap 的 `HeatmapFactories`、Bar 的 `BarFactories` 可组合类型化文字与几何回调；`BarLabels.zero_extent` 默认 `Skip`，显式 `ValueAxis` 支持正类别宽度下的真实零值轴长度或像素取整坍缩标签。回调接收原始数据、索引、默认文字与逻辑/文字位置，保留 NULL/零值、raw、tooltip 和 formatted source；空文字保留 owned slot，闭包只在建图时借用，高亮与恢复不重调。两个图型仍有不同的可见/标签资格规则，未将 Rust 的 eligible-only 回调次数冒称为所有 Nivo accessor 调用。见[中文指南](typed-labels.zh-CN.md)与[English guide](typed-labels.md)。

最终正式版本通过最小特性 1,170 项、全部特性/全部目标 1,383 项、16 个 doctest、native/WASM 严格 Clippy、实际 Rust 1.95.0，以及四个正常 Glory 消费者的严格检查与 release 构建。双语完整程序在正式库实际运行通过；原程序主体保留，另一个正常消费者追加导出后缀生成六个 base/active/restored 场景。两份 README 各保留 15 段一致 Rust 示例及 22 个有效相对链接，不含机器路径或环境依赖说明。

全新四浏览器 × DPR1/2 的 8 个上下文通过 48 对同首次保留缓冲的完整 SVGImage/Canvas 比较与 168 项真实破坏负控，48 次全白画布后完整位图恢复。348 PNG 独立核验：240 native 缓冲、96 根场景截图、12 CPU 图像。完整 360 条 native `fillText` 按各原脚本的顺序、次数和坐标精确匹配，其中 144 条 owned-label RGBA 包含 24 个真实空标签调用。32 对 base/restored 后端缓冲完全相同。最大平均通道差为 0.008948，低于原 0.5 门槛，WebKit 全部为零。

独立核验原始 NULL/零值、原 tooltip、metadata、18 个矩形路径及实际绘制来源；零值柱的 72 条非填充查询均为 false、432 个实际背景像素保持原色，三帧合成填充矩形的真实负控拒绝，未创建假拾取面积。30 个 CPU 字形区域包含 4,266 个正确合成色像素，6 个空标签区域全部保持背景且没有预测字形。这里只核字体提供的实际 CPU 墨迹与来源颜色，不声明 CPU/浏览器全帧字体相等或物理设备输入通过。

旧 API 默认输出、上下文、回调错误顺序和资源拒绝由真实双库及独立组合探针核对；当前 909 个代码/fixture 文件在终验期间保持字节不变。真实 248 张画廊 SVG 和整份 index.html 均与上一已接受检查点逐字节相同。完整汇总为 `target/acceptance-summary-typed-labels.json`，各独立候选及前轮失败继续保留，不拼接旧成绩。

本批接受受检的类型化文字、几何组合和显式零值轴标签。任意组件/Widget、全图文字避碰与更宽 Nivo props 仍分别受模型和宿主约束；默认 Firefox Sans、历史严格 compositor 及保留式 SVG 原平台量化未因此改为通过。GPU/worker 继续暂缓，整体对齐目标不据此宣称完成。

## 标签继承颜色与独立 SVG 单次实色透明度（2026-11-12）

Bar 的 `BarLabels.color_profile` 和 HeatMap 的 `label_color_profile` 已完成正式整合验收，复用已有 `InheritedColor`，不重写颜色算法。标签按工厂绘制前的数值 palette 取色，保留固定样式/主题回退、NULL 的 unknown 色、原始数据、工厂透明度与受控高亮。None 默认颜色路径已与旧库实际对照；类型化 alpha-zero RGBA 保留隐藏通道，与原 Nivo CSS 解析的差异单独保留。见[中文指南](inherited-label-colors.zh-CN.md)与[English guide](inherited-label-colors.md)。

最终正式版本通过最小特性 1,141 项、全部特性/全部目标 1,353 项、15 个 doctest、native/WASM 严格 Clippy，以及实际 Rust 1.95.0 检查；四个正常 Glory 浏览器消费者重新完成严格检查和 release 构建。新完整指南在正式库实际运行通过，两份 README 各有 15 段一致 Rust 示例及 21 个有效相对链接，保持独立文件与可移植说明。

全新 Chrome、Firefox、Playwright WebKit、Edge × DPR1/2 的 8 个上下文共完成 32 对完整 SVGImage/Canvas 绘制比较、64 个真实破坏负控及 32 次完整位图恢复。232 PNG 全部独立校验：160 native 绘制缓冲、64 根场景截图、8 CPU 图像；另核验全部 192 条 native `fillText` 的原脚本顺序/次数，其中 80 条 owned-label 的 RGBA、位置和 globalAlpha 精确符合预期。CPU 的 20 个独立来源区域包含 1,166 个正确字形内部像素。最大平均通道差 0.027958，小于未改变的 0.5 门槛；WebKit 的全部 8 帧差异为零。没有将 CPU 字体与浏览器字体全帧相等或物理设备行为计入结论。

首轮仍保留失败：6 个上下文通过，WebKit 的 2 个上下文在 HeatMap 大面积半透明背景上未达 0.5。独立真实 None-profile 和无标签单矩形复现相同差异：对象/组透明度 0.4 使 SVG RGB 各少 1，单矩形平均差 0.75；填充透明度与 Canvas 相同。这不是新标签颜色角色导致。修复仅在最终 `paint_mark` 为一个实色 fill、无描边、无 paint server、Normal 混合且 opacity 非 1 时，把独立 SVG 的属性名改为 `fill-opacity`。数值、路径、颜色、裁剪、透明背景及输入范围不变；多次绘制继续保留对象透明度。未改 CanvasScript、CPU 或保留式 SVG，也未加 epsilon、预合成背景、缩小图形或放宽门槛。

独立精确分数验证 37,632 项单填充等价及多绘制反例；实际双库 64 场景的 Scene/CanvasScript 全部一致，256 份 SVG 中 152 份仅合法属性替换、104 份逐字节一致。真实 248 张画廊中，22 张仅有 95 个透明度属性名替换，226 张逐字节不变。完整 Edge gallery CLI 在修正旧 reader 和 246→248 的过时 ready-count 后新一轮通过；两次旧 reader/精确计数失败保留，未改 45 秒超时或几何/像素阈值。最终汇总为 `target/acceptance-summary-owned-label-colors.json`，首失败快照及独立 WebKit 诊断另存。

本批只接受标签颜色角色与受限的独立 SVG 单填充修正。保留式 SVG 的原平台量化、球面场景默认 Firefox Sans 与历史严格 compositor 仍不改为通过；逐源标签文字工厂和零尺寸 Bar 标签资格继续独立实施，GPU/worker 暂缓。

## Bar 分类尺度与 Bar / HeatMap 自定义标记（2026-11-12）

本批正式实现 `BandScaleOptions` 与 Bar 共享类别尺度，以及 Bar/HeatMap 的单标记 Geometry/FillPaint 工厂。原始值、来源身份、逻辑站点与真实墨迹分别保留；受控悬停和类型化绑定校验受管来源。HeatMap 独立复核发现的标签归属和默认 NULL→零值冒充来源两个问题已修复，并以相同反例重新验证拒绝。Bar 默认、缩放及公开帧篡改的独立复核通过。用法见 [类别尺度](bar-category-scale.zh-CN.md)、[自定义 Bar](bar-custom-geometry.zh-CN.md) 和 [自定义 HeatMap](heatmap-custom.zh-CN.md)。

正式 Rust 检查通过：最小特性 1,120 项、全部特性 1,331 项、15 个 doctest、原生/WASM 严格 Clippy，以及 Rust 1.95 最小特性、全特性和 WASM。四个正常 Glory 浏览器消费者均重新完成格式、严格 WASM 检查与 release 构建。三组中英文指南的完整 Rust 程序实际运行通过；两份 README 各保留 15 段一致示例和 20 个有效相对链接，不包含机器路径或环境依赖说明。正式画廊仍为 248 幅，另有类别尺度四场景、Bar 工厂四场景和 HeatMap 工厂两个受控帧的可运行导出示例。

全新第二轮 Chrome、Firefox、Playwright WebKit、Edge × DPR1/2 × 两图族共 16 个上下文全部通过，检查 48 个 SVG/Canvas 绘制对照和 80 项真实 DOM/Canvas 破坏负控，均恢复原内容。独立解码核验 228 张 PNG：120 张 native 绘制缓冲、96 张完整场景截图和 12 张 CPU 图像；另检查 160 个作者来源位置像素、原始值、D3 分类位置、几何及截图前后文档坐标盒。首次 helper 测量最大平均通道差为 0.059426，低于原 0.5 门槛。Bar 归档 native PNG 是后续独立捕获，并非首次测量的同一快照；归档缓冲重新计算的最大平均差为 0.013550，分别记录，不将其替换为首次测量结果。

第一完整浏览器轮次保留为失败：8 个 HeatMap 上下文在检查器错误要求 SVG 圆必须为 `<circle>` 时失败；现有导出器始终用 `geometry.commands()` 输出 Path。修正只触及检查器，严格核对圆的 M+4C+Z、26 个坐标及同边界框错误控制点，未改 Rust、例子、0.5 门槛或像素容差。首轮 355 文件/324 PNG 快照保留，第二轮全部重跑，未复用首轮 Bar 成绩。正式汇总为 `target/acceptance-summary-nivo-factories.json`。

这项验收覆盖共享类别尺度和类型化单标记工厂。任意图内 Widget、全部 Nivo 标签/颜色角色、物理输入设备和 BrowserChart 专属事件仍须分别推进。六项球面场景的默认 Firefox Sans 差异及历史严格 compositor 仍未通过；本批小图字体结果、显式 System/Offscreen 专项均不改变这些结论。GPU/worker 按用户指示暂缓，整体目标继续进行。

## 中点进度修复与验收范围

上一版画廊审计发现了真实的 Conic 采样盲点：四分之一、中点和四分之三点的弦误差虽低于请求精度，靠近端点的短急弯仍可能偏离约九像素。修复仅为新增 Conic 投影增加沿弦的中点进度细分条件，不改变工作量、深度或点数预算。Chrome 中两个独立 old/new WASM 二进制的实测误差由 9.01848 降至 0.4967817 像素，低于原 0.5 目标；这项成对诊断不能替代新版本完整功能验收。原 D3 自身约 0.000485 的近极点细分偏差单独保留为参考诊断，不能用来解释或豁免已修复的 Rust 盲点。

修复前 1126 路径 cohort、183 项已选回归通过及原功能证据已完整归档且不可变；该基线明确 featureAccepted=false。不将修复前通过项或图片拼入修复后版本成绩。

新源码 Rust 与消费者检查：最小特性 1082 项、全部特性 1285 项、15 个 doctest、native/WASM 严格 Clippy、Rust 1.95 和四个新构建的浏览器消费者全部通过。新四引擎 SVG/Canvas 与 DPR1/2 功能检查：Chrome、Firefox、Playwright WebKit、Edge 各自完成 DPR1/2 的 SVG/Canvas：208 项普通/typed 用例，另有 96 次接缝、32 次绘制对照、32 次非法候选、16 次发布故障及 32 次 resize 检查（16 次合法、16 次实际发布失败）。独立核验 288 PNG、96 对原始回滚记录、144 marker 像素、480 可信输入包和 11 个篡改反例。最大绘制平均差为 0.207068，原门槛仍为 0.5；源码与 HTTP/WASM 指纹一致。新原生 focused/full：正常 target 的格式及严格 Clippy 通过，headless Apace focused/full 各通过 16 case、各保留 32 PNG；独立解码全部 64 PNG，核验 3476 个精确来源颜色像素与 175 个原生源码指纹，前后源码指纹一致。新画廊几何、拟合、蒙版及导出：实际 246 幅场景保留旧 244 幅独立 SVG 字节不变。两张新卡通过可移植原 D3 几何/绘制检查，包括每图 761 个经纬网作者节点的 1e-9 对照、来源色与缺失值、plot/projection 蒙版及低于 0.5 的 SVG-image/Canvas 平均差。修复后旋转 Albers 可见经纬网的双向采样距离为 0.056184 和 0.495643 像素；不称原生细分逐位相等或全局 Hausdorff 保证，另以 25001 个原 direct-function 点验证已修复急弯。新版本正常回归与 Chrome CLI 收口：全新完整 cohort 2863059d-9ad6-4490-a2be-a0f1caf966bf 通过 183/183，Firefox、WebKit、Edge 各完成明确选取的 61 套，整个 piecewiseScales 仍排除。四组系统 Chrome CLI 独立通过，host 严格 compositor CLI 未计入。新版首轮 182/183 因 WebKit scaleProfiles 等待滚动稳定超时而保留为失败；原脚本在新旧宿主的四次独立对照未复现，原因仍未确定。最终仅采用三份完整新子报告，不复用部分通过项，也未更改断言、超时或像素门槛。

普通/typed 用例计数与额外的接缝、parity、invalid、publication、resize 检查口径分开；合法 resize 与真实 resize 发布失败分别计数。原生源码指纹与浏览器源码、HTTP/WASM 指纹也分开记录。只有完整同版本终结报告支持验收，部分通过项及早期运行均不拼接。

默认地图与 choropleth 入口保留固定粗采样及球面曲边保守包络；带 path options 的入口可显式启用像素自适应采样及细叶包络。不承诺与 D3 逐点细分或紧致拟合逐位一致。AlbersUsa 组合、球面面积/长度/质心/包含/圆生成 API、任意球面拓扑及更广投影配置仍待推进。投影蒙版应使用 BrowserChart、RasterSurface 或 SVG 导出，语义 Graphic/Text 不支持任意蒙版。GPU/worker 暂缓，较早严格 compositor 仍未通过，整体 D3/Nivo 等价目标保持进行中。详见[验收记录](verification.md)和[差距清单](feature-gaps.md)。


## 保留式 SVG 包装节点：正式功能验收（2026-11-12）

本轮修复矩形 clip 移除、path mask 移除、成功 detach 后 reparent 失败、Label mask 移除和首次 blend wrapper 样式发布失败五条事务路径。保留包装节点引用直到可失败操作完成，回滚按实际父子关系恢复原业务 path/Label，首次 blend 候选在完成样式和子节点组装后才提交。仅对已实测的一次性故障给出恢复结论，不承诺任意持续 DOM 故障。

正常工作区新 WASM 在 Chrome、Firefox、Playwright WebKit、Edge 各完成 SVG 与 DPR1/2，共 40 项事务、80 张 PNG、80 份原始 HTML。独立核验只删除 surface 根 data-revision，检查完整 DOM/错误计数/revision/几何及浏览器记录的节点身份；两种 probe 另有 16 项已提交 Scene/原值和 typed 值 7、控件、32 个可信 pointer/Enter 包及成功 retry。三种 legacy blends 的 phase/mode 仍只是提案记账，不用来证明 accepted 应用缓存原子性；伴生 Canvas 未计入覆盖。

PNG 经 CRC、完整解码、来源尺寸与同版本浏览器 CSS 捕获盒独立重现核验；原运行未保存 bbox，不倒称原时刻位置已经重建。Playwright 的 enclosingIntRect 对小数 y 向外取整，使截图比 80/120 逻辑高度多一 CSS 行，这不是更大的 Scene/backing。40 对固定业务填充内部 3×3 区域共 720 个像素保持 RGBA；没有整屏位级一致或新的 AA 容差承诺。18 项篡改负控全部拒绝。

同一 fresh cohort 并行完成三引擎各 60 项显式范围专项，共 180/180；整个 piecewiseScales 专项仍排除。Chrome 四组 CLI 另行通过，整个 host 严格合成器 CLI 未计为通过。本轮重新通过正常 WASM 严格 Clippy、Rust 1.95 和四个消费者格式/检查/release 构建；未重新运行 native 全量测试，前轮 1055/1258/14 数字仅为历史证据，共享库源码除 WASM SVG 模块外保持字节相同。

汇总为 target/acceptance-summary-svg-wrapper.json，绑定 1002 文件冻结、实际 HTTP/磁盘 WASM、同 cohort 三份完整原始报告及本轮独立产物。格式失败、早期 RED、f2 混合结果、独立 stage 候选与 StageQuad 接线证据全部保留且不拼入正常工作区成绩。此前严格跨帧屏幕基准仍未通过，Conic/Albers 仍为准备，GPU/worker 继续暂缓，整体 Nivo 等价目标尚未完成。

## 连续分位色带：正式功能验收（2026-11-12）

SequentialQuantile 已完成样本右排名、R7 分位数、不可变完整分布、按数据值定位的精确硬边图例，以及 HeatMap、Calendar、共享颜色 Choropleth 接线。空样本、单样本、非有限输入、512 图例 stop 上限和 f32 跳点碰撞均有明确受检行为；真实业务值及 None 不被排名替换。详见[中文指南](sequential-quantile.zh-CN.md)和[English guide](sequential-quantile.md)。

四引擎 Chrome、Firefox、Playwright WebKit、Edge 各自独立完成 SVG/Canvas 与 DPR1/2，共 816 条检查、96 张屏幕 PNG、336 次无效候选拒绝、48 次发布故障和 48 次 resize 故障。408 个实际报告的绘制平面平均通道差最大为 0.333434，小于原 0.5 门槛；24 个错误平滑色带负控最小差为 1.079727，均被拒绝。独立离线复核覆盖 PNG 解码像素和已保存的完整故障状态，不宣称重算未保存的逐状态绘制缓冲。

当前 Rust 最小特性 1055 项、全部特性 1258 项、14 个 doctest 通过；严格 Clippy、WASM、Rust 1.95、四个浏览器消费者和完整原生回归通过。最终原生三消费者 × DPR1/2 的 24 张 PNG、126 个业务像素和 60 个图例像素独立核验。画廊为 244 幅，旧 241 幅独立 SVG 字节不变；32 幅 CPU/Glory 挂载 smoke 保留旧 29 幅并追加三幅。中英文 README 各有相同的 14 段可执行示例，保持独立文件和可移植说明。

冻结版本的新范围回归通过 177/177，即 Firefox、WebKit、Edge 各 59 项；明确排除 60 项目录中的整个 piecewiseScales 专项。Chrome 157 的画廊、Canvas、Widget 和 raster 四组 CLI 通过，未计 host 严格基准。首个 CLI 驱动误选 bundled Chromium 141 后出现 dash 精确不透明度失败，原记录保留，最终四组来自显式 Chrome 通道的新运行。

故障注入实际发现并修复 SVG 图形和文字节点的缓存短路；同一不可变脚本在旧二进制双 RED、新二进制双 GREEN，属性和位置在回滚后恢复。另有五例结构回滚故障待修复：原宿主的三例 clip/mask 操作，以及独立探针中的文字 mask 移除和首次 blend 创建。候选尚未应用；本轮不作任意 DOM 故障恢复承诺。原 D3 单样本插值器可以抛异常，oracle 如实记录，Rust 仍返回 unknown。所有早期失败与原始源码/运行产物保留。

当前功能汇总为 target/acceptance-summary-sequential-quantile.json，绑定 1000 文件冻结、实际 HTTP/WASM 与独立证据。整体 Nivo 等价目标仍未完成；较早严格跨帧屏幕基准仍未通过，Conic/Albers 只有候选准备，GPU/worker 继续暂缓。

## 等值线与加权密度：正式功能验收（2026-11-12）

本轮正式根版本已完成等值线与加权二维密度的功能验收：包含网格拓扑与孔洞、不可变 Float32 密度场、原始 Scatter 身份、阈值与绘制更新复用、SVG/Canvas、CPU/Glory 图像宿主，以及拒绝发布后的恢复。实现与边界见 [中文指南](contour-density.zh-CN.md) 和 [English guide](contour-density.md)。这项交付不代表全部 Nivo/D3 对齐目标完成。

修复后的完整 Rust 检查通过：最小特性 1,031 项、全特性 1,231 项、文档示例 13 项；严格 Clippy、四个浏览器消费者、Rust 1.95 原生/最小特性/WASM 和 WASM rustdoc 通过。两个 README 各有 13 个有效对应链接和 13 段相同 Rust 示例；两份新指南各 3 个示例也实际通过。原版 D3 参考覆盖 546 个网格、500 个密度配置、844,260 个精确 Float32 单元和 3,527 个阈值层。

Chrome、Firefox、Playwright WebKit、Edge 的独立专项均完成 SVG/Canvas 与 DPR1/2。96 张 PNG、16 份发布故障记录及 32 份原始 HTML 经独立核验；包含真实孔洞和源点墨迹、可信指针/Enter、阈值身份与网格复用、权重/带宽/源隐藏/resize 重算、无效 Scene/绑定拒绝，以及 SVG 更新和 resize 发布故障恢复。SVG/Canvas 平均通道差均小于 0.01425，原门槛仍为 0.5；删除轮廓的负控均拒绝。

补齐画廊后重新冻结并重跑的范围回归通过 174/174，即 Firefox、WebKit、Edge 各 58 个专项；明确排除当前 59 项目录中的 piecewiseScales 专项。Chrome 的画廊、Canvas 示例、Widget 和 raster 四组 CLI 检查通过，没有将含已知严格屏幕基准的 host CLI 组记为通过。两幅新增画廊有独立原版 D3 几何、孔洞、坐标轴、图例和 200 个前景散点检查，并通过十次页面破坏负控。画廊现为 241 幅，旧 239 幅 SVG 字节不变；CPU PNG 与 Glory 挂载 smoke 保留旧 27 幅并追加两幅。

完整 headless Apace 原生流程保留普通 Text、Brush 及既有交互检查，并通过 Contour/Density × DPR1/2 四组新案例。原生指针采用实际 Apace UiEvent、命中树与图像 Bounds，再由应用显式 dispatch_host；键盘通过原生 datum 按钮。未将这些检查称为真实触摸设备、读屏或 OS 窗口验收。

失败证据单独保留：独立审查发现并修复 metadata/绘制 mark/拾取 site 不一致；原生指针桥先处理旧按钮退出事件，再提交当前坐标。发布缓存事务经代码审查修正并做实际故障注入。正式浏览器首轮因 live DOM 的根 data-revision 遥测变化失败，完整前后内容证实其余字节相同，最终仅规范化这一属性且保留原值。漏更新画廊数量导致的 Chrome 失败及中断矩阵也保留，最终成绩来自修正后的新运行。

当前功能汇总为 target/acceptance-summary-contour-density.json；各版本源码、WASM、实际 HTTP 文件、报告与截图哈希均绑定在汇总中。专项采用 973 文件冻结，后续画廊回归采用 974 文件冻结，两者仅四个画廊验收脚本不同，产品源码和运行产物相同；不是不同产品版本成绩拼接。此前高级尺度严格跨帧屏幕基准仍未通过，五轮历史失败没有被覆盖或计为本轮成功。GPU/worker 继续暂缓。

## 高级尺度与等值线迁入时的检查点（2026-11-12）

下方各章节按其当时版本保留。高级尺度的原生统一流水线已通过：最小功能 1,012 项、全功能 1,211 项、文档示例 13 项，以及严格 Clippy、消费者构建、Rust 1.95/WASM、239 幅画廊和 27 幅原生绘制 smoke。上述数字属于等值线迁入前的冻结版本，不能作为新增代码的验收结果。

高级尺度的整体浏览器验收仍未通过。第五轮冻结版本的三引擎矩阵为 173/174，WebKit 唯一失败发生在测试夹具跨异步调用对齐时，队列 pending 从 0 变为 1；场景、DOM、局部几何及其余状态相同。Chrome CLI 在严格跨帧屏幕像素检查中出现最大通道差 36，虽然对应绘制输入、DOM、状态与几何相同，也不能记为通过。四引擎新增专项 12/12 和旧功能专项 12/12 独立通过，不能拼接成整体验收。五轮失败及原始 PNG 分别不可变归档；第五轮归档含 71,325 个文件，独立校验未发现缺失或不可解析引用。未放宽像素阈值。

等值线与加权密度正在正式迁入。根库核心的最小特性 16 项、raster 17 项、Rust 1.95 raster 17 项及两种配置的严格 Clippy 已通过；原版 D3 参考检查覆盖 546 个网格、500 个密度配置、844,260 个精确 Float32 单元及 3,527 个阈值层。隔离阶段的浏览器成绩不计入正式根版本，正式消费者与浏览器验收另行更新。GPU/worker 继续暂缓。

## 可编辑 Brush 与空 BoxPlot 负指数域（2026-11-11）

本轮统一流水线通过：最小功能 986 项、全功能 1,178 项、文档示例 10 项，后两项合计 1,188 项。严格 Clippy、四个浏览器消费者、原生实际绘制、Rust 1.95 原生/最小功能/WASM 检查及 WASM rustdoc 通过。两个根 README 各有 11 个有效相对链接、10 段完全一致且通过 doctest 的 Rust 示例，可移植性检查通过。

共享 `EditableBrush` 已接入 Surface、浏览器 SVG/Canvas 和 Glory CPU 图像宿主：X/Y/XY、选区移动、八方向边角调整、越边翻转、Alt/Shift/Space、程序设置/清空、键盘微调和明确会话退休。当前数据 ID 使用类型化锚点；选区保持宿主逻辑像素，转换数据视口时只减一次 plot 原点。隐藏、resize、回调重入、过期输入、原有手势仲裁和可恢复的宿主发布失败均有专项；具体恢复边界见 [使用说明](editable-brush.zh-CN.md)。

原版 d3-brush 3.0.0 独立参考保留 76 个案例、279 步、279 个实际通知及 1,848 个手柄记录；Rust 等价子集为 67 个案例、253 步、234 个通知、1,648 个可见手柄。另有 49,152 步有限极端输入和 3,072 项非法输入/错误指针事务检查。原 D3 的 Alt 后 Space 可越出 extent；此独立案例与 Rust 保持实际受限尺寸的安全行为分开记录，不并入普通等价成绩。参考脚本使用随项目保存的原版源码及完整许可；原生 Chrome、四引擎精确事件重放和独立越界案例的只读复验全部通过。

最终重新冻结后，Chrome 五组 CLI 全部通过；Firefox、Playwright WebKit、Edge 各通过完整 55 套，共 165/165。四引擎 Brush、普通 Text 和尺度独立专项全部通过，前后宿主 WASM 哈希一致。每个 Brush DPR1/2 上下文覆盖 SVG/Canvas 共 134 条轨迹、506 步，并检查实际手柄墨迹、正容器变换、数据视口、DOM 发布故障、终止输入、公开导航和清理；汇总实际核对 32 份独立 PNG。画廊为 233 幅，原配置导航保持 72 项，新增三个 Brush 模式另行检查。

原生 Apace headless 的四组为 DPR1/2 × camera1/2，验证真实图像绘制、八手柄 DragCapture、Tab/Enter 操作按钮、键盘微调、发布失败、回调权威、隐藏及宿主边界变化取消、stop 清理。原有 34 组普通 Text 和 21 幅 CPU PNG/Glory vector/image smoke 也通过。原生 raw modifiers 仍由应用显式转发；浏览器可信触摸 tap 与注入的触摸拖动分开标识，不宣称真实设备触摸拖动或读屏验收。

空 BoxPlot 的负指数 Pow 自动域已修复：空/缺失源使用非零域，隐藏的负值源保留负分支；显式域优先，非法奇点输入仍拒绝。最小功能四项、全功能五项新专项包括实际 CPU 坐标轴及关闭坐标轴后空白画面，不生成虚假盒体。

首轮完整矩阵为 164/165，唯一失败是 WebKit 关系图键盘阶段。实际轨迹证明：聚焦 Chord 节点按钮后自动滚动 65 像素，静止鼠标随后产生可信移动并悬停弦带；按钮仍保持焦点，最新悬停按既定语义覆盖键盘高亮。仅在节点/边键盘聚焦前各增加一次移开鼠标的调用，共 54 字节；逆移除后与原脚本完整字节相同。所有原断言、flush、像素阈值及鼠标覆盖键盘专项保留。诊断、因果轨迹、对照与字节证明分别保存为 `target/relations-focus-webkit-*`、`target/relations-keyboard-pointer-isolation-proof.json`；首轮原报告及哈希保存在 `target/editable-brush-before-relations-focus-review/` 和 `target/editable-brush-relations-first-failure.json`。最终成绩来自修正后另起的完整矩阵，不拼接首轮通过项。

最终汇总为 `target/acceptance-summary-editable-brush.json`，矩阵为 `target/gallery/cross-browser-editable-brush-report.json`。汇总严格核对 632 份冻结源码/产物、22 个实际 HTTP 文件、原生报告、截图、参考生成器及独立失败历史；前后再次确认源码和汇总辅助脚本未变。宿主 WASM SHA-256 为 `c9b539177c1afcd34e09478de0b9faa1cc7d748f27edc67a80cf3f0f2bdad511`。GPU/worker 按要求暂缓，其余差距见 [差距清单](feature-gaps.md)。

以下章节保留此前各阶段的验收检查点，数字按当时版本记录。

## 普通 Text 的真实基线、避碰与容器缩放

基线模型八项全功能专项、Widget 十一项专项通过。当前统一 Rust 三段结果为最小功能 956 项、全功能 1,141 项、文档示例九项，后两项合计 1,150 项；包含同期分级色标与 Pow/Sqrt 的实现。Rust 1.95 的原生库、最小功能全部目标和新补的全功能 WASM 库检查分别通过；WASM rustdoc 与双语 README 可移植性检查通过。

`SemanticTextMeasurement` 绑定文本、字体、字号、请求行高和资源 revision，绘制与避碰共用实际 advance、行盒高度、alphabetic 基线和墨迹边界。行盒通过墨迹位移同步移动；发布前还校验最终行盒，避免合法墨迹位移却使行盒越界。CSS 使用原始请求的正行高，实际量化行高用于布局。空串与纯折叠空白保留实际零盒、无墨迹与零基线偏移。普通 Widget 主示例已接入，通用未配置测量的 Chart 仍保留兼容模式。

实际原生 Apace 的 34 组覆盖两个真实静态字重、ABeeZee 重音和 descender、字体回退、三个锚点、DPR1/2、两种行高及空行盒。B 的十二组按全图像素对照 CPU，普通 Text 保持 `Props::Text`，未由 Graphic 或 Image 替代。基线从真实 inline line metrics 读取，墨迹来自实际 Raster 扫描；过期和失败测量、重试、显隐、节点保留及捕获清理通过。另验证不同字体的真实彩色墨迹避碰，已查看实际 PNG。

四引擎各在实际普通 HTML DPR1/2 执行十五组案例，共 120 组基础场景。SVG、Canvas 的原像素阈值保留，原 B100 与高 110 障碍使用真实 +104 位移；将普通 Text 下移八像素的负对照能检出 966 个差异像素。截图原点先归整以消除截图重采样相位，原始文本与 Label 坐标保持不变。

公用浏览器适配器独立测量 Text 格式化上下文，以外部基线标记避免改变末尾空格和空行。复现并修复了父级 scale 将屏幕度量放大后误用作本地 CSS 度量的问题。新增 32 组正缩放、非均匀缩放、平移缩放和 zoom 场景，实际普通 Text 与相同变换 SVG 墨迹差异均为 `[0,0]`。zoom 保留宿主真实量化后的基线，逻辑及物理锚点分别严格核对；未强迫使用未 zoom 的基线。另有 64 组旋转/skew/反射/透视/3D 拒绝和 16 组 Shadow DOM/slot 证据，包含实际祖先检查、四点几何校验与 Drop 清理。

233 幅画廊已生成；21 个相关场景通过 CPU PNG 与 Glory vector/image 挂载 smoke。新增两个 Pow/Sqrt 场景分别有 5,624 和 5,182 个彩色像素，已查看实际 CPU PNG。统一消费者构建和冻结后跨浏览器矩阵已通过相同源码、WASM 与实际服务哈希核对。

完整矩阵发现新增 Pow/Sqrt 第四个配置在真实滚轮缩放后无法选出测试目标。实际域裁切已移除所有 alpha sites，beta 的两个可见点仍分别通过九个真实鼠标位置的命中检查；这是夹具只选择 alpha 的限制。修复前现场、源码与哈希保存在 `target/text-baseline-before-scale-target-fix/`。修复保留原 alpha 优先顺序，追加其他当前系列的候选，并保留九邻点命中、滚轮强度、原始数据和全部 JavaScript 断言。修复后重新冻结并执行完整验收，不拼接修复前的通过结果。

后续四引擎专项继续检出保留节点夹具的前提错误：第四个配置保存的 alpha 点在滚轮后已被合法裁切，reset 后的同 ID 节点是新建节点。实际 DOM 诊断确认最早断开时刻；第五个配置单独执行原两阶段全部断言通过。只将第四个配置的固定点纵值由 `4 + phase × 5` 调整为 `8 + phase × 5`，使节点在真实交互中持续可见；独立 d3-scale 4.0.2 核对新坐标 `[343,260]`、`[210,233]`，并核对 37 个已捕获的 phase0 视口，无 clamp 掩盖。对应两处坐标期望同步更新，节点身份和其他逻辑断言保持。第二次失败、实际 detach 诊断与 D3 核对保存在 `target/text-baseline-before-retention-fixture-fix/`。

第三次专项中 Chrome/Edge 的十二个配置通过，Firefox/WebKit 仍拒绝旧节点身份。Firefox 的独立轨迹确认第四个配置已通过，第五个配置在 phase0 平移结束时将原 `x=4` 裁切：真实滚轮 scale 为 1.3698，随后平移的横轴上限为 3.7719，并非 brush 导致。只将第五个配置的固定横值改为 `±3`，真实 D3 核对 `[455.73333333333335,185]`、`[456,260]`，并在捕获的 40 个视口关闭 clamp 检查可见性；其中 phase1 只有三个初始快照，其余手势由最终完整专项复验。保留数据之外的全部断言。第三次失败及诊断见 `target/text-baseline-before-reciprocal-retention-fix/`。

真实测量器接入普通 Widget 后，旧的隐藏 `#app` 启动页会拒绝不可布局的探针。默认启动现在跳过明确带 `hidden` 属性的容器，显式挂载仍遵守真实布局要求。普通、轮廓、虚线和 plot-widgets 四组原有验收通过；隐藏启动的 children/probes/live 都为零，移除 hidden 后显式挂载和探针清理通过，未采用估算基线回退。

最终重新冻结 611 份源码及产物后，Chrome 五组 CLI 验收全部通过；Firefox、WebKit、Edge 各通过完整 54 套，共 162/162。四引擎普通 Text 专项及各十二个尺度配置的独立专项全部通过，前后宿主 WASM 哈希一致，执行时间均晚于最终冻结；最终完整矩阵包含相同专项。全流程日志为 `target/check-text-baseline.log`，最终汇总为 `target/acceptance-summary-text-baseline.json`。汇总严格校验 22 个实际 HTTP 文件、全部冻结文件、原生 34 组、21 个 smoke、233 幅画廊及参考生成器，且再次核对源码未变。最终宿主 WASM SHA-256 为 `bc8a56ab758b6049cb14fbf6ac44010603f96bdf7ab215667b8bb58fb580d91b`。三次失败报告与最终成绩分别保存，未拼接旧通过结果；其夹具变更通过逐字节比较限制为已说明的数据及独立坐标期望。

## D3 分位数与显式阈值色标

新增八项全功能专项（最小功能七项）。独立 d3-scale 4.0.2 的 42 组配置、
1,672 条冻结记录对比真实查询、R7 阈值、首尾无界档、重复边界和按颜色反查；
`node scripts/discrete-color-reference.mjs --check` 再次运行原函数并只读核对。
额外覆盖隐藏后重算分位、作者阈值保持、nullable HeatMap/Calendar/Choropleth、
极端有限值、单色与空样本、有限图例窗口、真实 CPU 图例像素和拒绝事务。
既有连续色标保持原有相等性与输出，不记录无关样本分布。

`scripts/check.ps1 -BuildBrowser -RunNative` 完整通过，日志为
`target/check-discrete-colors.log`。工作树普通文字基线随后更新，因此追加当前源
复验：1,131 项全功能测试、947 项最小功能测试、严格 Clippy 和全功能 WASM
库检查通过，分别记录在 `target/discrete-colors-current-{tests,minimal,clippy,wasm}.log`。
统一流水线的九项 crate 文档示例、四个浏览器消费者构建和 Apace headless 的实际
PNG 解码、绘制、事件与清理也通过。此处不把文字基线的并行测试计为本批新增项。

最终 Chromium 141.0.7390.37、Firefox 150.0.2、Playwright WebKit 26.0、
Edge 157.0.4341.46 各通过 gallery 和 colors 两套验收，共 8/8。每家检查
231 幅 SVG/Canvas 画廊以及连续、分位、显式阈值、量化和日历五阶段保留式切换。
新图的颜色与四档宽度由独立已知阈值核对；专项保留严格像素差条件、原节点、
当前 hover/键盘激活、相同帧零 DOM 修改、拒绝后旧帧和销毁清理检查。

首轮图集仅因固定预期仍为 229、实际增加为 231 而拒绝；保留
`target/discrete-colors-initial-count-mismatch.json`，修改为 231 并新增图名和
实际配色/档宽断言后重新完整运行，没有降低像素阈值或拼接首轮结果。
两幅新图已实际截图查看，并修正窄色档附近的刻度文字重叠。

最终报告为 `target/discrete-colors-final-browser-report.json`，汇总为
`target/acceptance-summary-discrete-colors.json`。汇总核对关键源码/参考/产物哈希、
最终两套浏览器结果、原生报告和 HTTP 实际返回的宿主 WASM、JS、画廊 HTML。
此批不宣称完整 53 套跨浏览器矩阵、全部 Nivo props、GPU 或物理设备验收。

## 共享字体族、字重与同字体测量

Rust 全功能 1,104 项测试、9 项文档示例，共 1,113 项通过；最小功能 924 项通过。本轮新增 32 项全功能测试。严格 Clippy、WASM 库检查、四个浏览器消费者的格式/Clippy/发布构建，以及原生 Apace 实际绘制通过。MSRV 1.95 的全功能库和最小功能全部目标、WASM rustdoc、双语 README 的十个相对链接及九段一致示例分别通过。

公共模型和测量协议的十二项专项覆盖独立继承、角色覆盖、过滤重索引、字体签名、四种图族受控换字体、隐藏恢复、Pie 类型化绑定、最长合法名字与 Unicode 签名预算。六项后端专项覆盖 XML/JS 安全引用、显式 400 重置、Canvas 字体能力的发布前拒绝、绘制顺序。九项字节字体专项及一项 CSS 查找顺序测试核对真实静态 400/700 face、400–500 特殊规则、分数字重、族序/覆盖回退、未绑定拒绝、旋转、竖排、双向与 COLR。四项控件专项核对保留节点、展示角色、测量回调与拒绝重试、两种动画的退出字体，以及前置注释重索引。

原创几何 A/B 字体由 FontTools 4.67.0 生成并独立回读 hmtx、OS/2 与墨迹边界。原生 Apace 的 Glyph 图与 CPU 在两个字重下逐像素核对；真实普通 Text 另验 60/100 advance、40/80 墨迹宽度和节点保留，未用 Glyph 或图片代替普通 Text。字体缺失拒绝、资源释放和 owner 清理同时通过，报告新增 `semanticTypographyFaces`。普通 Text 仍使用既有近似基线定位；该边界已记入差距清单，不能从 Glyph 像素相同推导普通 Text 基线也相同。

229 个画廊场景已生成；17 个相关场景执行实际 CPU PNG 与 Glory vector/image 挂载 smoke。两个新字体场景分别有 9,295、11,584 个彩色像素，已查看实际 PNG。四引擎字体阶段验收核对真实 FontFace、SVG 截图与 Canvas 墨迹、400/700 和 500/600 匹配、安全 quoted family、900 字重宿主污染、font-only 节点保留、同字体测量、图例/提示框、整帧拒绝和清理重挂。Firefox 的 Canvas advance 有实测尺寸量化；以独立仅 Regular/仅 Bold 的 FontFace 作同引擎参考，SVG 仍严格比对字体表的 60/100，墨迹条件未降低。

最终 Chrome 五组 CLI 全通过。Firefox、Playwright WebKit、Edge 在同一轮新运行中各通过 53 套，共 159/159，覆盖 229 幅画廊、72 个配置示例及原有全部专项。汇总程序核对 600 份冻结源码与产物、22 个实际 HTTP 服务文件的响应哈希、冻结后的四引擎字体专项、原生和 17 场景 smoke。冻结范围补齐 Canvas 消费者源码、Node 配置及实际验收辅助程序。两个字体生成器的只读检查和原 Nivo 半径过滤 12 组参考由汇总程序实际再执行并核对。结果为 `target/acceptance-summary-typography.json`，完整矩阵为 `target/gallery/cross-browser-typography-report.json`，未拼接旧结果。

## 径向标签、Sunburst 测量、COLR 彩层与位图控件

统一 `scripts/check.ps1 -BuildBrowser -RunNative` 已通过。全功能 1,072 项 Rust 测试与 9 项文档示例合计 1,081 项，最小功能 902 项，本轮新增 40 项全功能测试。首轮严格 Clippy 指出了 Sunburst 新测试的三处写法问题，改用引用切片和 `as_chunks` 后完整重跑通过，断言与像素条件不变。最终流水线按单个 Cargo job 执行，保持各 workspace 的正常产物目录。

六项半径筛选专项对照原始 Nivo ArcLabelsLayer 的 12 组过滤参考，覆盖薄环、实心圆饼、等值边界、活动半径、pad/corners、隐藏及全部返回入口。新增五项类型化绑定专项核对 Pie/RadialBar 的真实极坐标中点、Path 跟随锚点、原始值、无墨迹过滤、活动外扩拾取和被手改的元数据拒绝。Radial 保留私有源状态供校验，使用布局函数构造返回值。

Sunburst 的九项新专项覆盖 Plot、ClipToArc、RequireFullInkBounds、实际 pad/corner/inner-hole 路径、优先级与位移、隐藏恢复、绘制顺序、旧测量与工作预算。ABeeZee 的实际旋转墨迹、CPU 像素和默认行为分别核对。完整扇环外轮廓按填充语义补显式 Close，原数据 Path 不改写；严格 AABB 的额外省略有专门回归。

COLR v0/CPAL 的六项测试核对真实层序、两个 palette、透明与半透明前景、孔洞、竖排替换和混排旋转、矩形/路径裁剪、坏 palette、超预算及 v1 拒绝。六项纯 Scene 导出测试另核对原业务标记、series 归属、绘制顺序、剩余文字索引、ID 冲突、资源预检及允许超过单个语义 Graphic 256 命令的合法路径。四项 painted Widget 测试验证同路径 CPU 像素、稳定层身份、安全原文、整帧拒绝重试和字体字节的 owner 清理。自创字体和 FontTools 独立回读参考的最终再生成检查通过。

四项位图插槽测试覆盖真实 PNG、保留的本地 Button 状态、相机与 DPR、矩形和路径 Full/Outside、partial inert、发布失败、拒绝重试及捕获释放。原生 Apace 文档实际绘制两个 palette，核对孔洞、透明前景保留 palette 层和 RGBA 叠层像素；报告增加 `semanticColrV0PaletteLayers`。MSRV 1.95、WASM rustdoc、双语 README 可移植性及九段一致 Rust 示例通过。

227 个画廊场景已生成。15 个相关示例独立执行 CPU PNG 与 Glory vector/image 挂载 smoke；新增的两个 Sunburst 和两个彩色字体图已查看实际 PNG。四引擎阶段验收均通过 14 组 SVG/Canvas 配置、彩色字体及真实位图 Widget 插槽。Sunburst 的黑色文字像素须留在真实扇环 mask 内，移除 mask 的对照图须显著增加文字墨迹。颜色专项对比 CPU、语义 Glyph SVG、独立 SVG、Rust Canvas 与原生 FontFace 默认 palette；其他 palette/透明前景使用独立 CPAL 记录和 RGBA 算术。

最终 Chrome 五组 CLI 全通过。Firefox、Playwright WebKit 和 Edge 在同一轮新运行中各通过 52 套，合计 156/156，覆盖 227 幅画廊和 72 个配置示例。汇总程序核对 574 份冻结源码与产物；范围扩展到四个消费者的 manifests、locks、构建脚本、HTML 与生成的 JS/snippets。22 个实际 HTTP 服务文件（HTML、WASM、JS）的响应哈希与冻结文件一致。统一结果为 `target/acceptance-summary-radial-color-fonts.json`，完整矩阵为 `target/gallery/cross-browser-radial-color-fonts-report.json`，未拼接旧版结果。

## Stream 自定义点、层级标签、多列混排与类型化位图交互

本轮统一 `scripts/check.ps1 -BuildBrowser -RunNative` 已通过。全功能 1,032 项 Rust 测试与 9 项文档示例合计 1,041 项，最小功能 884 项。严格 Clippy、WASM 检查、四个浏览器消费者的格式/Clippy/发布构建，以及原生 Apace 文档的实际 CPU 图片解码和事件检查均通过。MSRV 1.95 的全功能库与最小功能全部目标检查、WASM rustdoc 和中英文 README 可移植性检查分别通过。README 的九段 Rust 示例一致。

Stream 新专项覆盖 14 类共享符号与三个堆叠锚点、nullable/hidden 工厂调用范围、偏移空心路径的精确/最近/切片拾取、过渡锚点、累计命令预算与 CPU 裁剪像素。层级标签的八项专项覆盖原始位置失效后的位移、优先级、隐藏与恢复、标题/绘图区/圆盘裁剪、受管槽及旧测量拒绝，并用 ABeeZee 的真实旋转墨迹对三种层级布局核对避碰、CPU 和轮廓输出。

多列混排使用 Unicode 16.0 的方向与 enclosing-mark 数据，并对全部 Unicode scalar 核对生成表。八组原生 HarfBuzz 12.1.0 参考覆盖实际字体的竖排替换、无替换回退、侧转字形及左右列推进。新增测试核对 CRLF、显式空列、列预算、二维测量、锚点与旋转、CPU/outline 像素及失败帧保留。两个固定版本字体与 Unicode 数据生成器的 `--check` 通过。

类型化 `RasterSurface` 的专项核对原始路径锚点、宿主坐标换算、nullable 提示与控件、绑定/绘图区/图片发布的原子拒绝。额外回归确认只改业务标签或格式化值时，当前提示即时刷新，同时保留同一个 PNG 与发布次数；缺失值改为真实零值时更新控件，保留其节点身份。九个图型示例的独立 CPU PNG 与 Glory 挂载 smoke 通过。

四引擎阶段检查均已通过 10 组 SVG/Canvas 布局配置和实际 PNG 消费者。混排 PNG 的二维墨迹与独立 HarfBuzz 边界比较，位图控件通过真实键盘输入和应用显式宿主坐标映射核对原始数据。初轮 WebKit/Edge 在键盘事件处理前取到 revision 0；脚本改为等待实际选择状态后，四引擎重新通过，保留原始数据和像素断言。

最终 Chrome 五组 CLI 全通过。Firefox、Playwright WebKit 和 Edge 在同一轮新运行中各通过 50 套，合计 150/150，覆盖 221 幅画廊和 72 个配置示例。汇总程序验证了 513 份冻结源码与产物、四个发布 WASM 与画廊页面的实际 HTTP 响应哈希、阶段检查、完整矩阵、Rust 流水线、MSRV、文档和原生 smoke，再写入 `target/acceptance-summary-labels-dots-vertical.json` 与当前汇总。完整矩阵为 `target/gallery/cross-browser-labels-dots-vertical-report.json`，未混用初轮失败或旧版结果。

## Chord、Bump、Stream 与缺失值数据控件

Chord 的六项测试核对五组原始 Nivo/D3 矩阵、11 个分组和九条双向连接，覆盖来源方向翻转、自环、二次曲线控制点、独立半径、双端隐藏、双行提示和 CPU 墨迹。Bump 的六组参考包含 17 个类别位置、13 个排名位置、24 个数据点、79 个肩部或缺失点及 80 条路径命令。

Stream 的 15 组原始 Nivo/D3 参考核对 45 条面积路径和 180 个 nullable 采样。测试覆盖共享堆叠顺序与偏移、三种点位置、曲线、独立填充透明度、缺失值和显隐。切片顺序回归将全部 25 种顺序与偏移组合及反向 Y 轴的实际 SurfaceModel 提示行与布局顺序对比。

数据标签的三项回归检查类型化缺失值与实际零值、标签和场景共同拒绝、重试、稳定控件身份及运行时所有权。浏览器数据控件采用显式缺失标签。新增的七种保留式配置分别检查 Chord 弦带和环、Bump 精确与最近点、Stream 面积、无点切片和有符号面积。四个浏览器的阶段专项均通过，报告为 `target/gallery/flow-layouts-focused-stage-report.json`。

统一流水线通过 1008 项全功能 Rust 测试、872 项无默认功能测试和九项 crate 文档测试。两个 README 的九段 Rust 代码一致，且各有十个可用相对链接。Rust 1.95 的库和最小功能检查、WASM 文档、四个浏览器消费者及 Apace headless 均通过。七幅新场景还通过 Glory 内存宿主挂载和 CPU PNG 检查。构建使用当前 Cargo 产物目录，以离线模式和两个并发任务执行，日志为 `target/check-flow-layouts.log`。

Chrome 五组 CLI 通过。Firefox、Playwright WebKit 和 Edge 在同一轮最终运行中各通过 50 套，合计 150/150，覆盖 219 幅画廊和 72 个配置示例。阶段和最终宿主的 WASM 哈希相同，四个发布 WASM 与画廊页面的 HTTP 响应均与冻结的源戳一致。汇总为 `target/acceptance-summary-flow-layouts.json`，矩阵为 `target/gallery/cross-browser-flow-layouts-report.json`。

## Funnel、Waffle、Sunburst 与平行坐标

Funnel 的 34 行参考数据由 Nivo 0.99.0 和 D3 原函数重算。测试对比三种配置的阶段边缘、描边路径、分隔线、中心、尺度与退化域。Waffle 的 Nivo 对照检查 48 个单元格位置、96 个区域以及 14 个正常、隐藏和溢出分配区间。Sunburst 的 D3 对照检查 8 个分区及 6 个圆角切点，并验证真实 CPU 墨迹和精确拾取。

平行坐标测试对比 D3 的整化线性轴、反向以 2 为底的对数轴和闰日 UTC 轴。测试还检查原始行值、隐藏域、缺失槽、夹取后的精确拾取，以及固定日历间隔生成的实际刻度总量。

Chrome、Edge、Firefox、Playwright WebKit 的独立专项分别运行同一份阶段 WASM。四种保留式实例验证 SVG 与 Canvas 的墨迹、鼠标拾取、键盘激活、更新后的标记 ID 和释放。Waffle 同时验证单元格间距留白和连续区域的覆盖。阶段报告为 `target/gallery/layout-models-focused-stage-report.json`。

统一检查通过 987 项全功能 Rust 测试、852 项无默认功能测试和 9 项 crate 文档测试。新增的五幅画廊场景均可挂载到 Glory 内存宿主，并通过 CPU PNG 的实际墨迹检查。四个浏览器消费者构建和 Apace headless 实际运行通过。Rust 1.95 原生检查、WASM 文档构建和两份 README 的示例及相对链接校验通过。

最终 Chrome 五组 CLI 通过。Firefox、Playwright WebKit 和 Edge 对同一批冻结产物各通过 49 套，合计 147/147，覆盖 212 幅画廊和 72 个配置示例。全部新增布局的阶段宿主 WASM 与最终宿主哈希相同。四个发布 WASM 和画廊页面的实际 HTTP 响应均与源戳记录一致。汇总为 `target/acceptance-summary-layout-models.json`，完整矩阵为 `target/gallery/cross-browser-layout-models-report.json`。

## 径向柱圆角、层级焦点动画、竖排与桌面窗口

径向堆叠柱的 D3 参考验收覆盖正反方向、自动和固定间距半径、弧形塌缩与整圆。图集验证圆角柱段、轨道、留白和实际 Canvas 像素。层级专项检查矩形树图、圆堆积和冰柱图的中间帧、打断回退、原始数据、拾取和减弱动画偏好。

直立竖排的四组 HarfBuzz 12.1.0 参考核对字形、字符簇和垂直坐标。测试覆盖真实轮廓、物理锚点、旋转和字体回退。浏览器解码 Glory 控件发布的 PNG，检查两个日文字形的墨迹和竖排句号的右上角位置。

Windows 上运行同一桌面示例。Apace 显示矢量图，Wry 显示 CPU 位图，两者均验证 Windows 辅助功能树、Tab 与 Enter、数据更新、用图例隐藏系列和无效帧保留。两个实际窗口均记录选中南区 `54`。Apace 的自动化鼠标点击被其他窗口覆盖，硬件输入与屏幕阅读器播报未列入通过范围。桌面事件在 `examples/native-raster/target/apace-window-events.json` 与 `examples/native-raster/target/wry-window-events.json`。

统一检查通过 967 项全功能 Rust 测试、9 项 crate 文档测试、832 项无默认功能测试。四个浏览器消费者构建和 Apace headless 实际运行通过。Rust 1.95 对库和桌面示例的检查通过。中英文 README 各通过 9 项文档测试及 10 个相对链接校验。日志为 `target/check-radial-focus-vertical-native.log`。

Chrome 五组 CLI 均通过。Firefox、WebKit 和 Edge 在同一轮运行中各通过 48 套，合计 144/144。图集覆盖 207 幅，保留式配置示例为 72 个。发布的四个 WASM 和图集页面与冻结的 SHA-256 一致。汇总为 `target/acceptance-summary-radial-focus-vertical-native.json`。

## Line 点、BoxPlot 值轴、AreaBump 标签与路径裁剪

Line／PointLine 自定义点有 6 项全功能和 5 项无默认功能专项测试。配置值轴、类别配色、原始类型化键、偏移轮廓、裁剪及动画锚点均有覆盖。PointLine 格式器现读取原始类别键。BoxPlot 有 6 项全功能和 5 项无默认功能新测试，6 组原始 Nivo／D3 参考覆盖分位数、须、Log／SymLog 和 UTC。AreaBump 实测标签布局有 5 项全功能和 4 项无默认功能新测试，包含真实旋转字体轮廓、障碍、引线和失败回退。

203 幅阶段画廊在 Chrome、Firefox、WebKit 和 Edge 通过。新增示例核对反向对数箱线图、UTC 小数分位数和折线偏移空心点。最大平均通道误差为 0.12581。报告为 `target/gallery-line-boxplot-preview/full-gallery-report.json`。

保留式 Line、AreaBump 端标签和浏览器路径裁剪在四个引擎使用同一 host WASM `f331191698f02bf17172961a391129b9bc983131568eafc601aca894785416d0` 通过。Line 的两个 SVG／Canvas 帧整帧差异均为零。AreaBump 每阶段测量六个真实文字边界，检查原侧位置、间距、障碍、引线保留、焦点和拒绝。控件专项验证两个渲染器的偶奇孔洞像素、缩放、实例隔离、完全覆盖时的原生点击和完全落在孔洞里的插槽身份。Firefox 的观测器提前驱逐了大 WASM 响应，专项运行器改用已核验的同一份响应字节交付给浏览器。报告为 `target/gallery/line-custom-points-browser-report.json`、`target/gallery/area-bump-labels-browser-report.json` 和 `target/gallery/browser-plot-widget-path-final-report.json`。

统一流水线通过 954 项 Rust 测试、9 项 crate 文档测试和 824 项无默认功能测试。四个浏览器消费者与 Apace headless 实际运行通过。Rust 1.95 的全功能和无默认功能原生编译检查通过。中英文 README 各通过 9 项文档测试。构建日志为 `target/check-line-boxplot-area-mask.log`。

Chrome 五组 CLI 通过。Firefox、WebKit 和 Edge 对同一冻结产物各通过 48 套，合计 144/144。三家均验证了 203 幅画廊、72 个配置入口及本轮保留式专项。矩阵为 `target/gallery/cross-browser-line-boxplot-area-mask-report.json`，汇总为 `target/acceptance-summary-line-boxplot-area-mask.json`。

## 类别配色、雷达点与子弹图尺度

类别色标的 5 项测试在全功能与无默认功能下均通过。独立对照读取原始 Nivo 色盘目录和 D3，覆盖 11 种类别色盘、27 种 ColorBrewer 默认色板和 30 次固定或隐式键域查询。另有 4 项图表颜色测试覆盖 Bar、Line 和 Pie 的图形与元数据、隐藏域、显式覆盖、RGB 修饰顺序、透明背景与零值切片对比色。

Radar 自定义点专项通过 5 项全功能及 4 项无默认功能测试。覆盖偏移路径、原始锚点、SVG 与 CPU 输出、精确与最近点拾取、顺逆时针以及配置值轴。Bullet 的 7 项全功能及 6 项无默认功能新测试通过，并保留 6 项旧全功能测试。7 组原始 Nivo 与 D3 参考对照包含线性、Log、SymLog、UTC、反向和各行坐标。

200 幅阶段画廊在 Chrome、Firefox、WebKit 和 Edge 全部通过。新 5 幅验证固定类别色盘、Pie 标签对比色、Radar 偏移空心点，以及 Bullet 对数区间与刻度。SVG 几何和实际 Canvas 像素均已核对，新增场景最大平均通道误差为 0.02604。报告为 `target/gallery-ordinal-layout-preview/full-gallery-report.json`。

保留式 Radar 专项在四个浏览器使用同一 host WASM `40151c1fc328a544249e8ecd2bf57ef6b54dddb6bfdb05738448d43b4f2daeb5` 通过。两阶段的整帧 SVG／Canvas 平均通道误差均为零，验证了空心轮廓、实际裁剪、原始数据锚点、极角切片及相机缩放。拒绝测试在同一浏览器调用中核对提交前后的帧版本与队列，并在后续绘制后检查保留路径。初次验收把主动绘制后的版本也计入拒绝判断，现已校准取样时序。报告为 `target/gallery/radar-custom-points-browser-report.json`。

统一流水线通过 936 项 Rust 测试、9 项 crate 文档测试和 809 项无默认功能测试。四个浏览器消费者、Apace headless 和 Rust 1.95 原生编译检查通过。中英文 README 各 9 个文档测试通过。正式 host WASM 与专项一致，构建日志为 `target/check-ordinal-radar-bullet-final.log`。

Chrome 五组 CLI 通过。Firefox、WebKit 和 Edge 对同一冻结产物各通过 46 套，合计 138/138。三家完整矩阵均覆盖 200 幅画廊、72 个配置入口和新增 Radar 双阶段交互。矩阵为 `target/gallery/cross-browser-ordinal-radar-bullet-report.json`，汇总为 `target/acceptance-summary-ordinal-radar-bullet.json`。

## 当前推进：雷达值轴、颜色配置与浏览器图内控件

颜色与雷达专项通过 29 项全功能和 25 项无默认功能测试。雷达的 8 组独立 D3 配置覆盖 Log、SymLog、负值域、网格及真实绘制半径，另验证顺逆时针轴顺序。颜色对照覆盖 288 次插值查询、38 条命名色带的 5690 次精确取样和 207 组 ColorBrewer 离散色板。新增 CPU 像素回归验证连续图例的半像素采样位置与同值色块颜色一致。反向查表色带保留 512 个停点，端点为发散中心时也保持边界语义。

195 幅阶段画廊在 Chrome、Firefox、WebKit 和 Edge 通过。新 5 幅验证对数刻度环、对称对数零环、两种 HCL 中点色和 Viridis 查表边界。它们的最大场景平均通道误差为 0.06144。Viridis 停点颜色逐值匹配，浏览器渐变像素允许一个通道单位。报告为 `target/gallery-radar-color-preview/full-gallery-report.json`。

BrowserChart 混色动画与图内控件均使用隔离 host WASM `dde0663264461963325ec44b1f898a8df8988188a9bf288819c4c39b044c2141` 在四个浏览器通过。混色覆盖两种动画模式、重定向、缩放、域重排、减少动效与缓冲失败回滚，共 64 次 SVG/Canvas 对比，保持 0.5 阈值。图内控件验证两种渲染器、相机与域重排、实际裁剪、鼠标与键盘、失败替换及重入卸载。报告为 `target/blend-motion-stage/final-summary.json` 和 `target/gallery/browser-plot-widgets-report.json`。

首次正式矩阵通过 133/135。DPR 2 下独立截图的混色过渡误差在 Firefox 为 0.51684，在 WebKit 为 0.77886。Firefox 的同帧序列化 SVG 与 Canvas 缓冲逐像素一致，实际截图显示边缘偏移。验收将示例放在视口原点，从同一张实际截图裁取两个完整图面。四引擎的实时混色过渡差值均为零，64 次对比中的最大平均通道误差为 0.05479。绘制代码和 WASM 保持原样，0.5 阈值保持原样。原始观测保存在 `target/gallery/radar-color-browser-initial-diagnostic`，校准后的专项为 `target/blend-motion-stage/canonical-dpr2-final-proof.json`。

统一流水线通过 915 项 Rust 测试、9 项 crate 文档测试和 790 项无默认功能测试。四个浏览器消费者及 Apace headless 通过，正式 host WASM 与专项一致。中英文 README 各通过 9 项文档测试。Rust 1.95 的两种原生配置编译检查及 WASM rustdoc 严格检查也通过。构建日志为 `target/check-radar-color-browser-final.log`。

Chrome 五组 CLI 复验通过。Firefox、WebKit 和 Edge 各通过 45 套，合计 135/135。复验使用同一份 195 幅画廊和四个 WASM 包，涵盖 72 个浏览器配置入口和两个新增保留式专项。完整报告为 `target/gallery/cross-browser-radar-color-browser-final-report.json`，构建及浏览器验收汇总为 `target/acceptance-summary-radar-color-browser-final.json`。

## 已验收：自定义 Scatter、双向文字、混色和语义图内 Widget

自定义 Scatter 专项通过 9 项新增测试，全功能相关专项共 38 项、精简相关专项共 35 项。覆盖偏移空心几何、零尺寸端帽、保留作者路径、裁剪、精确与最近点、分数坐标的矩形锚点及拒绝无效帧。

189 幅阶段画廊和两个新的 SVG 定量检查在 Chrome、Firefox、WebKit 和 Edge 通过。自定义节点示例的最大平均通道误差为 0.02370。真实浏览器专项使用隔离 WASM `b4411a50cf2d39b798b5618b738e59bb3c8b8a56c58696229e5ae93b76ca03a3`，四家均通过双阶段空心轮廓、绘图区裁剪、稳定节点、当前原始值、拒绝帧和原点最近拾取。最大平均通道误差为 0.02660。两份报告为 `target/gallery-custom-scatter-preview/gallery-report.json` 和 `target/gallery-custom-scatter-preview/live-report.json`。

加入 Multiply 面积图后，190 幅阶段画廊在四家通过集成验证。新混色示例平均通道误差最高 0.35159，重叠像素的 SVG 与 Canvas 结果均在独立公式的一个通道单位以内。背景像素和 Canvas 状态恢复通过。报告为 `target/gallery-blend-preview/full-gallery-report.json`。

逐标记混色的 7 项全功能、2 项无默认功能专项通过。CPU 与独立 Chrome Canvas 的 30 个像素样本对照涵盖 15 种模式和两种背景透明度，误差至多两个通道单位。真实保留式 SVG/Canvas 在四个浏览器验证了 Multiply 的整层透明度、缩放、裁剪孔洞及超预算拒绝。该检查点的普通替换和混色替换各有独立配置。混色帧使用旧动画入口时在入队前拒绝。

语义图内 Widget 的 5 项全功能、4 项无默认功能及 14 项既有 Widget 回归通过。正式 Widget WASM `1a3bb2fd8079f14bcdceac096e5f2f6c04c7531550df8553d2b8bd812c93e6dc` 在 Chrome、Firefox、WebKit 和 Edge 同次专项全过。实际裁剪内像素为 `[19,98,81,255]`，裁剪外为 `[255,255,255,255]`。验证了真实点击、Space/Enter、局部可见控件的 inert、节点状态保留、失败回退、卸载和重挂。报告为 `target/gallery/plot-widgets-browser-report.json`。

`raster-bidi` 的专项包含两个独立对照测试和四个集成测试。对照覆盖 Unicode 16 双向视觉顺序 96 例与原生 HarfBuzz 12.1.0 的字形、簇和位置 10 例。集成验证 CPU RGBA/PNG、物理文字锚点、旋转轮廓、Glory 的源文本和拒绝后保留旧帧。原 CPU、RasterSurface 和轮廓 24 项回归通过。与 Apace 的分词依赖兼容后，六个新测试以 `unicode-segmentation` 1.13.3 重跑通过。Apace headless 实际混合方向标签有 275 个指定颜色轮廓像素，旧轮廓仍有 521 个墨迹像素。报告为 `examples/native-raster/target/native-bidi-apace.json`。

统一流水线通过 893 项 Rust 测试、9 项 crate 文档测试及 771 项无默认功能测试，覆盖四个浏览器消费者与 Apace headless。冻结 host WASM 为 `0c153f0472866acbddc8e329f8e00bc0abff41a2a8e676ad2e8cf442b5a0a736`。Chrome 五组 CLI 全部通过。Firefox、WebKit 和 Edge 各通过 43 套，共 129/129，包含 190 幅画廊、72 个配置入口以及自定义 Scatter、15 种混色模式和语义图内控件。报告为 `target/gallery/cross-browser-custom-blend-widgets-report.json`，汇总为 `target/acceptance-summary-custom-blend-widgets.json`。

## 共享尺度与符号阶段：Bar、Swarm、Network、Combo

已新增 `bar_layout_with_scale_options`、`swarm_layout`、`ForceSimulation::scene_with_glyphs` 和 `NetworkDragController::begin_with_glyphs`。柱状图的 4 项新全功能测试覆盖实际刻度像素、反向与整化域、正负堆叠合计、真实裁剪及拾取，原 Bar 与 Cartesian 14 项通过。蜂群图 9 项全功能、8 项无默认功能通过，覆盖固定值轴与保守包络、UTC 和对数轴、分组以及 CPU 墨迹。网络符号 6 项全功能、5 项无默认功能通过，原模拟、拖动和关系 29 项通过。它们覆盖模拟轨迹和连线端点、真实轮廓与空心、边缘裁剪、隐藏、尺寸变更和拖动重定位。

`combo_layout_with_scale_options` 的 6 项新测试在全功能与无默认功能下均通过，另有 26 项旧 Combo 回归通过。新增测试覆盖独立连续 X、Linear/SymLog 与 Log 值轴、UTC 闰日、分类窗、正负百分比堆叠、圆角及箱体裁剪、真实绘制与最近点、无效尺度拒绝。

188 幅阶段画廊已在 Chrome 157、Firefox 150、WebKit 26 和 Edge 157 完整通过，6 幅新增示例均通过定量几何检查。对数蜂群核对 Wye 面积、固定值轴、节点间距与分组顺序，UTC 蜂群核对日期格式、闰日、反向轴与真实空心描边。柱图核对反向取整、SymLog 投影和正负合计，网络图核对符号原点和连线端点，Combo 核对双轴、分类裁剪与堆叠。报告为 `target/gallery-combo-preview/gallery-report.json`，187 幅中间检查另存于 `target/gallery-feature-gap-preview/full-gallery-report.json`。

隔离 host WASM `cac5fef33e5e2314d64b1373775208e42b88c761c5b316eaf158e86d796a2dca` 的新 Swarm、Network 专项在四家均通过。Swarm 覆盖双阶段描边命中、空心中心、图例、稳定节点和错误帧拒绝，最大平均通道误差 0.01672。Network 覆盖实际轮廓、缩放、resize 重定位、拖放、隐藏、拓扑换图和保留节点，最大平均通道误差 0.05743。两套原 0.5 阈值保持。报告为 `target/gallery-combo-preview/swarm-report.json` 和 `target/gallery-combo-preview/network-glyph-report.json`。

同批统一流水线 `check.ps1 -BuildBrowser -RunNative` 已通过 866 项 Rust 测试、9 项 crate 文档测试及 756 项无默认功能测试，四个浏览器消费者和 Apace headless 消费者均通过。Rust 1.95 的全功能和精简配置编译检查也已通过。中英文 README 各 9 项文档测试通过。统一 host WASM 为 `044be16c71069d6858eb5919a68ec62b8c359f63bab5a3393c1453ee4ab15b18`，Chrome 五组 CLI 全部通过，其中 host 验证 72 个配置入口、Swarm、Network glyph、平行坐标悬停修复、Scatter 六组双阶段和 12 组端帽。Firefox、WebKit 和 Edge 各通过 40 套，合计 120/120。报告 `target/gallery/cross-browser-extended-layouts-report.json` 与汇总 `target/acceptance-summary-extended-layouts.json` 对应同一冻结产物。

## 此前推进：独立 typed Scatter

已实现稳定series/node业务身份、双轴连续/UTC/point、四侧轴、动态直径/显式glyph单位和typed拾取/导航；`scatter_layout`直接计算节点，旧 `scatter()` 半径5保持。默认新Circle直径9、Linear min0/maxauto/niceDefault；Point Y从上到下。尺寸线性外推、constant中点、missing策略、零尺寸logical sites/真实Exact、域外中心部分ink、全逻辑Nearest的plot查询范围、raw元数据与stroked实际ink颜色均有独立验证。

| 当前实际通过的专项 | all-features | no-default | 证据 |
| --- | ---: | ---: | --- |
| `scatter_layout` | 8 | 8 | 旧半径兼容、新直径、稳定ID/原源索引、zero/off-plot中心、反向PointY/偏心anchor、单类别Y独立cap、negativeLog全hiddenfallback、stale geometry拒绝、gradient/underflow/domain预算及stroke色片 |
| `scatter_profiles`（独立） | 9（CPU1） | 8 | actual Nivo9profiles/76points/18domains、四侧轴、constant/descending动态尺寸、typed身份/ownedlabels、Filled面积/实际origin、hollow Exact与Nearest、真实clip像素、坏输入/绑定事务、20k Star路径预算 |
| `viewport_ordinal_y` | 3 | 3 | 独立Y上限不限制X；保center/reset/min_zoom、late cap及无Y拒绝；原viewport/directions回归亦通过 |
| `scatter_zero_circle`（共享修复） | 7 | 6 | 单点源几何准入、SVG2 dash端点、真实Round/Square/Butt墨迹与typed Exact、alpha一次填充及CPU回归 |
| `widget_zero_caps` | 1 | 1 | 真实`App::memory`两cap×五dash cases，alpha/Square角/业务controls保持；原semantic Widget pure-dash展开已正确，无生产Widget改动 |

共享修复后 `check.ps1 -BuildBrowser -RunNative` 通过。全功能 841 项 Rust 测试加 9 项 crate doctest，共 850 项，精简配置另有 733 项通过。四个浏览器消费者均通过格式、严格检查、release WASM 和绑定生成。Apace headless 原生消费者已实际运行。Rust 1.95 的全功能和精简配置原生编译检查均已刷新通过。

英文 [README](../README.md) 与[中文 README](../README.zh-CN.md) 各含 9 个对应 Rust 示例，两份均已执行文档测试。crate 使用英文 README 生成 API 文档，因此正式总数计入英文的 9 项。WASM rustdoc 以警告视为错误的配置通过。

修复前checkpoint曾通过833项Rust+9doctest=842、no-default726；随后独立审计确认共享Circle0描边缺陷：diameter0/stroke10时，CPU/SVG的Butt墨迹为0却被typed Exact命中；Round/Square的CPU/SVG分别有88/100个ink像素，ScriptCanvas却为0。`target/scatter-circle-zero-audit.rs`及`scatter-circle-zero-audit-browser.json`记录实际最小复现。这是已修复的生产渲染/拾取差异，不能作为测量错误或放宽阈值处理；原始诊断保留，不能把842/726预修复checkpoint当最终新源结果。

进一步原始观测 `target/scatter-zero-dash-oracle.json`：Chrome157.0.8103.39与Firefox150.0.2各228row，包含Circle/open/triangle/plus、caps、dash/offset。Chrome闭合零Circle遇非空dash均无墨迹，Firefox零长度phase边界不同，native Canvas对这些cap也漏绘；这些是引擎差异观测，不能全部同时作为统一golden，也不修改该原始产物。

修复采用 [SVG2理想stroke的dash positions规则](https://www.w3.org/TR/SVG2/painting.html#StrokeDashing)，其步骤10–12在长度为0时仍可保留dash端点cap；这不是半开区间简化。`point_stroke_footprint`只接零半径Circle、宽高同时为0的Rect，或所有Move/Line/Cubic支持点重合且含Line/Cubic或Close的Path（含`M Z`，仅Move不算）；不能仅从bbox面积0推定。空/全零dash为solid；`[3,3]` offset0/1/3/6为on、4/-1为off，`[0,3]`仅offset模3为0时为on。Butt、透明/零宽无墨迹，原stroke-width准入仍保留。static/retained SVG、Script/typed Canvas、CPU和typed Exact使用同一结果并只填一次实际cap，原Scene/业务ID/value/anchor/clip/opacity不改；SVG `d`可变为paint footprint，非退化正长路径和bareScene旧hit不变，不把规范化结果表述为原生各家默认相同。

独立 static SVG 与 ScriptCanvas 对照在 Chrome 和 Firefox 均通过。每个引擎实测 60 组，每组比较三种端帽，全部平均像素差为零。原始引擎观测与规范化后对照分别记录在 `target/scatter-zero-dash-oracle.json` 和 `target/gallery/scatter-zero-circle-script-browser-report.json`。

首个修复包 `a54c7c1b…610d` 通过 Chrome 五组 CLI、72 个配置入口、Scatter 六组双阶段交互和 12 组真实端帽检查。首轮三引擎矩阵中，Firefox 的双 UTC 示例平均通道误差为 1.7410。截图确认完整时间戳在四轴上重叠，独立探针将几何差异与文字差异分离。示例改用每日刻度和紧凑日期，点标签保留时分，原始时间戳仍用于域、坐标和交互。

新版统一包的 host WASM SHA256 为 `c480d462744924126761c327a7466cd8c40bab854f83f63fa2e42f2f2e8783dc`。同 SHA 的独立包在 Firefox、WebKit 和 Chrome 均通过六组双阶段和 12 组端帽，最大平均通道误差分别为 0.27573、0.00459 和 0.01546。统一包再次通过 Chrome 五组完整回归、72 个配置入口、Scatter 专项及全部 12 组端帽。随后的完整矩阵为 Firefox 38/38、WebKit 37/38、Edge 38/38，三家均通过 Scatter。

WebKit 唯一失败为平行坐标的 Canvas 悬停。实测数据按钮 B 离开后，图面命中已绘制的虚线 A，Glory 迟到的按钮离开事件又清除了 A。Browser 现按实际指针区域接收按钮进入，并在图面拥有悬停时保留图面拾取状态。隔离构建 `16aad349e68c4ce79912ba1ce955c3faa5db94126acc7e6412c9938f5f034202` 在四个浏览器均通过原平行坐标专项和新增的 SVG/Canvas 事件顺序回归。旧矩阵和修复专项分别归档为 `target/gallery/cross-browser-scatter-before-hover-owner-report.json` 和 `target/gallery/parallel-hover-ownership-after-report.json`。

`scripts/scatter-reference.mjs --check`执行本地Nivo0.99.0实际`scatterplot/src/compute.ts`的`computePoints/getNodeSizeGenerator`及`scales`实际pipeline/factories，只适配宿主/lodash；D3 runtime为scale4.0.2/array3.2.5/time3.1.1。冻结`scatter-nivo.csv`的76点中66个可绘输出逐raw/indices/pixel/diameter核对，另10个负尺寸外推要求Rust明确Err；`scatter-domains.csv`有18个域记录。Nivo reverse记录domain、Rust记录range的差别明确，但实际映射坐标一致。

182静态Scene已生成并全部validate，Chrome/Firefox独立完整静态验收实际通过，保留旧176与全部原阈值；新6项`scatterGeometry`验证默认9直径/自动nice域、动态外推8..40、filled900面积与origin、hollow/真实clip、UTC leap day/反向Y、typed身份与PointY方向。报告`target/gallery/scatter-gallery-chrome-report.json`的mean为0.03981625362484736、max0.33926888020833335；Firefox150.0.2报告`scatter-gallery-firefox-report.json`为0.17183551897321434、max0.6044194878472222，保留gallery独立3.5阈值。三组`gallery182-scatter-{sizes,glyphs,axes}.png`为静态状态快照，不冒充动态测量/交互证据。

可独立复核（不构建浏览器pkg）：

```powershell
node scripts/scatter-reference.mjs --check
cargo test --locked --all-features --test scatter_layout --test scatter_profiles --test viewport_ordinal_y
cargo test --locked --no-default-features --test scatter_layout --test scatter_profiles --test viewport_ordinal_y
```

最近完成的基线已保存为 `target/acceptance-summary-previous822.json`、`target/check-before-scatter.log` 和 `target/gallery/cross-browser-scatter-previous111-report.json`。以下结果描述该已完成基线。

## 前轮完成：共享尺度与真实 glyph（正式完整通过，2026-11-10）

本轮新增真实Filled/Stroked glyph、typed Path anchor、Line/Radar/Geo接线与共享连续/类别point尺度。正式`check.ps1 -BuildBrowser -RunNative`已通过：813项all-features Rust+9项README doctest=822，另707项no-default，不重复累计；相对769新增44项Rust。四consumer严格/release/bindgen与native执行均由正式流水线确认。176画廊正式Chrome及Scale/Glyph/Geo新专项、66配置入口/32host专项/Chrome五CLI组合通过；37×3=111在同一次fresh完整运行全部通过，2026-11-10 15:33:25.063→15:46:34.500 UTC；Firefox150.0.2/WebKit26.0/Edge157.0.4341.46各37/37，每家176画廊/66导航；不拼接初轮诊断。上一完整检查点778/no-default668/164/102在下节作为历史保留。

MSRV另有实际native证据：`rustc 1.95.0 (59807616e 2026-04-14)`运行`cargo +1.95.0 check --locked --all-features --all-targets`，exit0，日志`target/msrv-1.95-latest.log`；同工具链的`cargo +1.95.0 check --locked --no-default-features --all-targets`也exit0，日志`target/msrv-1.95-minimal-latest.log`。该工具链仅已安装native target，未运行1.95 WASM构建或测试，不将这一check扩称完整MSRV跨平台验收。下文历史轮次的“MSRV未另测”仅描述其当时状态。

已实际通过：glyph6项all-features/4项no-default、geo_glyphs7/6（合计13/10，含CPU3），native/WASM lib严格Clippy；既有Geo62项回归通过。`scripts/glyph-reference.mjs --check`读取实际workspace Cargo metadata target_directory中的d3-shape3.2.0，252组/1440记录包含14种符号、6种size（含0）、3种rotation；冻结CSV逐source SHA记录来源。CPU独立验证filled面积、stroke感知size不等墨迹面积、零长Butt/Round/Square端帽，以及Geo真实mask外无墨迹。可只读复核：

```powershell
node scripts/glyph-reference.mjs --check
cargo test --locked --all-features --test glyph --test geo_glyphs
```

共享scale专项已实际7项all-features/7项no-default通过，1132条实际d3-scale4.0.2、d3-array3.2.5、d3-time3.1.1冻结queries，包含continuous/point、round负半值/负零、log base/minor与负域、原值SymLog、UTC calendar nice/TimeClip。新增Line wrapper与viewport独立12项已通过；这些数量是专项证据，不与旧全量数字手工相加预测统一计数。

glyph_interaction当前12项all-features/10项no-default（含CPU2）实际通过，验证Path真实anchor/Exact、零尺寸Filled与Butt无墨迹、Round/Square有墨迹、整数round anchor、裁剪边界、默认跨shape Fade、enter0逻辑focus/selection及ghost不可交互。新的Browser glyph fixture为4case×2phase，另有Circle/Wye/Triangle/Star跨shape链；这些unit与实际浏览器专项分开记录；同源最终pkg的4×2及跨shape链已实测通过，见下表。

Chrome157.0.8103.39同源host WASM SHA256为`d9cf74470ea2d245633c234e54bb969eda68f1367ba673efd76a045569808c8b`，三家新专项已实测通过，均保持原SVG/Canvas平均通道误差0.5阈值，pageErrors为空：

| 专项 | 真实浏览器验收范围 | 最高meanChannelError | 报告 |
| --- | --- | ---: | --- |
| Shared scales | 四case×两phase，typed类别/数值值、偏心wheel/pan/Shift brush、reverse方向、no double inverse、resize域稳定、图例重布局、retainedID/0Mutation/原子与清理 | 0.0115869140625 | `target/gallery/scale-profiles-browser-report.json` |
| Factory glyphs | 四case×两phase；Exact真实ink/空隙、Nearest真实origin非BBox、物理Path支持点同步动画；Star→Circle→Wye→Triangle→Star Fade链保focus/selection且无ghost controls；Radar Exact/Polar真切换 | 0.03239013671875 | `target/gallery/glyph-browser-report.json` |
| Geo glyphs | EqualEarth Star→Wye、Orthographic Triangle→Plus各两phase；同形图例、partial真实mask/backface、nullable/zero、pointer/keyboard、Exact/Nearest、hidden owned labels、typed resize、retainedID/0Mutation/拒绝保旧与Stop | 0.03136691623263889 | `target/gallery/geo-glyph-browser-report.json` |

Geo专项有三处验收定位修正，未改库或阈值：missing颜色图例是实际额外glyph，必须计入；hide真正删除节点，no-op身份参照在restore之后保存，跨phase身份另行严格比较；非法候选的完整snapshot/revision立即比对（Rust也逐项比较），额外显式flush会正常增加revision，因此flush后检查保留DOM而非要求时钟revision不动。四phase means为0.00748860677/0.00727864583/0.00333794488/0.03136691623，resize为0.00201273148/0.00222800926。实际mask外探针在移除mask后才命中，原场景无hit且Canvas无墨迹；accepted投影anchor仍在disk内，背面raw源明确剔除，不伪造mask外anchor。

176场景非canonical独立Chrome完整报告为`target/gallery-scale-glyph-check/browser-report.json`，mean0.03909166247435289、max0.33926888020833335，保持原3.5阈值；最终canonical画廊及其余四CLI已fresh通过，host有66导航/32extras且pageErrors空；最终完整111跨引擎矩阵也已同一次fresh全绿，不拼接专项或非canonical成绩。

组合导航另发现新Glyph/Geo fixture的空owner snapshot把“未挂载/已Stop”当Err；这是fixture合同，不是生产库渲染失败。两入口现返回`{"active":false}`，最终Geo增量consumer严格检查与release/bindgen日志为`examples/browser-host/target/final-inactive-contract-clippy.log`、`examples/browser-host/target/final-inactive-contract-build.log`。最终d9cf包的三家独立Chrome专项重新完整跑过，Geo报告额外明确beforeMount/afterStop均true；`target/gallery/glyph-composition-browser-report.json`实际同页依次验证inactive三family→configured66→scale→glyph→旧Radar，均绿。原生产库未因该fixture修正改变。

首次新跨引擎矩阵中的Firefox gallery几何检查暴露`defs`测量差异：原clipPath内path的`getBBox()`返回[0,0,0,0]，实际字节相同的path克隆到原outer SVG后得到[225,70,270,270]；主triangle origin、partial越界几何、hollow与所有其余子条件均正确。统一比较器现临时挂载不可见clone测量并在finally删除；独立probe另外确认child count恢复；保留四坐标0.01及全部像素/几何阈值，不改clip source或渲染。证据`target/gallery/globe-gallery-firefox-debug.json`，修正后Firefox176与Chrome176完整报告为`target/gallery/gallery176-cap-firefox-report.json`、`target/gallery/gallery176-cap-chrome-report.json`；另独立Firefox Geo2×2/resize/输入全通过，`target/gallery/geo-glyph-firefox-report.json`最高mean0.12013165509259259<0.5。初轮被明确保存为incomplete：57条捕获状态中56过、1个已定位的defs测量失败（Firefox36/37、WebKit20/20，Edge未跑），不是110/111完整成绩；归档`target/gallery/cross-browser-glyphs-initial-report.json`。最终矩阵已从零完整跑完111/111，不拼接诊断或专项。

最终canonical `target/gallery/cross-browser-report.json`已直接核对三个`tests`集合各37项全部passed、各176个charts和66个styles。下面均为该同一次fresh报告中的最大平均通道误差（含相应resize/animation采样），不是合并独立报告：

| 0..255通道mean | Firefox150.0.2 | WebKit26.0 | Edge157.0.4341.46 |
| --- | ---: | ---: | ---: |
| Scale profiles | 0.171341796875 | 0 | 0.0115869140625 |
| Factory glyphs | 0.029816080729166666 | 0 | 0.03239013671875 |
| Geo glyphs | 0.12013165509259259 | 0 | 0.03136691623263889 |
| 176画廊平均 | 0.16521190913036615 | 0.024997208412247476 | 0.0390916624743529 |
| 单幅画廊最大 | 0.6044194878472222 | 0.40265559895833336 | 0.33926888020833335 |

三个host专项保留0.5，画廊保留3.5，不混称全部场景小于0.5；Fade四段三引擎的stableFocus/stableSelection/currentDatum均true、ghostControls均false。最终WASM SHA为上述d9cf；中央画廊verifier SHA256为`04b948484e2a0f21b674fdb0120f3b266be27c6ed5cba88def47ff625dea91a7`，实际文件hash已再次核对。旧102全绿raw独立归档`target/gallery/cross-browser-glyphs-previous102-report.json`，其历史summary为`target/acceptance-summary-previous778.json`，不把当前canonical当成旧轮原始结果。

## 上轮：Pie、Line/Radar定位与关系集合（正式完整通过，2026-11-10）

最新正式日志已确认 **769项all-features Rust测试+9项README doctest，共778项；另668项no-default检查**，较前轮726项Rust新增43项，不把no-default重复累计。完整`check.ps1 -BuildBrowser -RunNative`已exit0，含fmt/native严格Clippy、全部Rust/doctest、WASM检查、四consumer严格检查/release/bindgen及native Apace真实PNG/Graphic/input/cleanup；根严格WASM Clippy/rustdoc也通过。最终源码包含结构union与完整decorated dash准入。Chrome最终五CLI全部exit0，综合host含29extras/56实际style入口、motionAdmissionRejected以及motionBudget/dashAdmission实际断言全true、pageErrors空；四家新专项均在同源最终pkg通过。最终三引擎各34套、共**102/102**在同一次fresh完整run全部通过，运行2026-11-10 13:47:57.677→13:59:19.090 UTC；Firefox150.0.2、WebKit26.0、Edge157.0.4341.46各自的164场景和56导航均通过。保留旧151并新增13幅，每幅Scene验证/严格检查、实际像素/输入和13项新几何不变量均通过。

两个实际预算回归都已修：old/new各10000个互异ID的合法目标形成20000-mark动画union，必须再为crosshair留结构预算；而dash `[1e-8,1e-8]`本身有限合法，却会让完整decorated Scene超200k展开命令。目标与真实union在排队前联合准入，完整decorated Scene在Model/Cage提交前预检。目标/union拒绝保持旧running motion/clock/model；末段逐帧失败回上一接受Scene/hover/binding/focus，取消animation和live gesture/captures，避免无穷RAF及终止contact复活。Geometry Transform允许边界min1/max100未变。3项新`crosshair` unit在all/default均过，涵盖tiny dash拒绝、合法mixed order保留、约199960个业务dash命令与单独合法overlay组合超额；相关Browser新pkg报告见下。

当前Chrome157.0.8103.39画廊正式CLI已通过164幅：meanChannelError均值0.03916217024474932、最大0.33926888020833335（0..255），保留3.5阈值；全部旧几何/实际gallery拾取/移动布局检查及新增13项`interactionGeometry`通过。`target/gallery/gallery-browser-report.json`记录真实结果，zero Pie的两条padding-only border保留，跳过零面积arc labels以免文字重叠；截图`gallery164-zero-pie.png`已人工查看。其余三组状态快照也已查看。共享动画准入修正已进入最终统一pkg，四专项已完整同源复验，102套通过由独立完整报告确认，不从静态结果外推。

新增13项几何检查分别验证donut孔洞/pad gap与active半径、partial实际放大fit、真实MLZ零面积border、point/SliceX/SliceY真实crosshair、顺逆/zero-hidden polar sites及spoke、四种关系的raw角色opacity和真实拓扑端点、Sankey parallel相邻ports与2:1流宽。SVG的`isPointInFill`可把退化闭线上的边界点算作inside；因此零值border检查明确MLZ/真实弧长和中线stroke，再在法线外2px检查无fill且无stroke，不把边界API结果误作有面积墨迹。26坐标完全同点+原生总长0的透明site检查继续保持，不使用BBox容差代替退化几何。

| 当前独立专项 | 已确认Rust项数（all-features / no-default） | 独立算法/协议证据 |
| --- | --- | --- |
| `pie_layout` | 8 / 7 | 264条实际D3 arc/pie几何oracle；asin内外pad/圆角/partial exact-cardinal fit、zero与border、测量签名/leader、事务/CPU |
| `line_interaction` | 14 / 13 | 11种crosshair、X/Y slices、raw/format/source index、无点透明sites、当前数据/hidden/缩放、typed Scene/plot原子更新与旧Scene清binding；typed viewport真实hook、contour motion准入事务 |
| `radar_interaction` | 8 / 7 | 实际Nivo源码提取636条角分区、15条排序；CW/CCW/旋转/seam/half-open、全0中心sites远处角拾取、真实Surface键盘/hidden/zoom/事务、CPU无新增墨迹 |
| `relations` | 10 / 9 | 134条实际Nivo predicate oracle；显式图身份、Tree/Network/Chord/Sankey集合、同向parallel、hidden/current替换与authored paint恢复、Network edge Surface可达、CPU |
| `crosshair` unit | 3 / 3 | full-frame decorated Scene、tiny/正常/zero dash、混排与原帧不变、累计dash资源拒绝 |

Radar参考脚本为`scripts/radar-slices-reference.mjs`，执行本地Nivo0.99.0 `RadarSlices.tsx`中实际分区语句和`RadarSlice.tsx`中sort/reverse语句，不重写一份JS算法作自我对照。冻结CSV分别记录源SHA256 `25e9f38d515176e41ac72a33a039106b0489cc39d98fa5b1c2a25b3019fde974`与`6daf0c9f679eadf0c220a402b030cbea0535726a66a00483646535974131fe96`；再生成命令`node scripts/radar-slices-reference.mjs E:/Repos/nivo/packages/radar/src/`会更新fixture，并非只读检查。它们是开发参考，不进入Rust或浏览器运行依赖。其余源码归属和冻结参考见[third-party notices](third-party-notices.md)。

Pie开发参考使用`scripts/pie-layout-reference.mjs`，执行d3-shape3.2.0实际pie/arc和Nivo hooks/boundingBox；`--check`逐字对照冻结264行。Relations的`scripts/relations-reference.mjs`执行实际Nivo tree hooks、chord hooks、Sankey选择predicate，React适配不重写选择算法，134行含独立祖先/后代开关及平行边/孤点。冻结CSV记录完整源SHA；可只读复核：

```powershell
node scripts/pie-layout-reference.mjs E:/Repos/nivo --check
node scripts/relations-reference.mjs E:/Repos/nivo --check
```

Pie脚本按其workspace Cargo metadata读取已安装的本地d3-shape3.2.0参考，不重定向target或自动安装依赖。

本轮实际浏览器专项入口为`scripts/{pie-layout,line-crosshair,radar-slices,relations}-browser-acceptance.js`。四组专项的当前实际Chrome157.0.8103.39结果如下（完整组合与矩阵仍按最后统一报告收尾），真实SVG/Canvas均保持0.5阈值：

| 当前Chrome专项 | 实际证据 | 最高meanChannelError | 报告 |
| --- | --- | ---: | --- |
| Pie layout | 4case×2phase、D3/Nivo独立pad、zero/border、partial fit、实际Canvas字体测量的Hide/ShiftY及真实leader重连、当前输入/hidden重布局/retained ID/原子与清理 | 0.0133154296875 | `target/gallery/pie-layout-browser-report.json` |
| Line crosshair | 3case×2phase、typed resize/viewport、实际RAF弹簧中目标业务值与当前几何同步、native wheel固定轴/no double inverse、失败保Scene/plot/domain/binding、callback stop；旧/typed setter和set_layout拒绝不兼容contour；双solver union超额拒绝/19998+2合法边界/instant换图；tiny dash挂载拒绝无claim、150k业务dash累计超额typedsetter/resize拒绝保PNG/hover/binding/queue；输入/键盘/legend/cleanup | 0.0124560546875 | `target/gallery/line-crosshair-browser-report.json` |
| Radar slices | 3case×2phase、零值无点/旋转Triangle/逆时针Square、排序多行tooltip/spoke、当前0→85更新、轴键盘、legend、实际wheel与typed resize、Scene-only清binding、0Mutation/原子拒绝/Stop | 0.005960205078125 | `target/gallery/radar-slices-browser-report.json` |
| 图关系 | 5case×2phase（Tree/Network/Chord/DAG及循环Sankey）、节点/真实边pointer与keyboard、独立树开关、双端hidden、动态换图、同向parallel排除反向、重复focus不入队、受控paint不累乘/0Mutation/原子与清理 | 0.17021019345238095 | `target/gallery/relations-browser-report.json` |

Radar sixphase means为0.001798828125/0.001798828125、0.00359912109375/0.00281689453125、0.005960205078125/0.005577392578125；report保存core/fixture/verifier/WASM SHA。Path glyph（含Square）保留独立decorative墨迹，标准point ID是零半径站点，verifier验证15 glyph+15sites而非误当Rect。其根SVG包含clip viewport嵌套SVG，retained markup断言明确选择root `.first()`；均是fixture测量定位修正，不修改拾取、渲染或阈值。最终decorated pkg的四专项与完整Chrome五CLI均已通过；完整102套矩阵也已同源全部通过。新静态13幅明确为交互状态快照；Pie测量只在真实宿主用字体度量，不将静态caption冒充已测量结果。

最终raw为`target/gallery/cross-browser-report.json`，三个引擎均34/34，gallery164/configured56。四家新suite最高mean按Firefox/WebKit/Edge分别为：PieLayout 0.09843970/0/0.01331543，Line 0.03180382/0.00448032/0.01245605，Radar 0.07650317/0/0.00596021，Relations 0.14205450/0.36352237/0.17021019，均保持各自原0.5阈值；gallery最大为0.60441949/0.40265560/0.33926888，仍用独立3.5阈值。三个引擎的motionBudget/dashAdmission以及旧Hierarchy四case均通过，无page errors；版本/矩阵见[跨浏览器记录](cross-browser-verification.md)。最终host WASM SHA256为`3f24ef7e1cc03d5159529c937e7a74d6513ff4dd299a8ce1f8c947447980877a`，Radar独立报告sourceStamp与之相同。未以本轮结果推导GPU/worker、OS物理设备或全部Nivo props等价。

本轮首个完整跨引擎run为2026-11-10 12:58:47.396→13:08:18.499 UTC，结果**101/102**，单独保存`target/gallery/cross-browser-interactions-initial-report.json`：Firefox150.0.2为33/34，WebKit26.0/Edge157.0.4341.46各34/34。四个新增suite在三引擎均通过；唯一失败是旧`hierarchyFocus`的CirclePacking Canvas parent pointer检查。诊断与fixture修正见下；其后最终102/102从零完整重跑，不拼接已有101项。

独立Firefox诊断`target/gallery/hierarchy-firefox-pointer-debug.json`确认其原probe落在父circle填充外、仅在1px border+旧2px拾取slop内：父radius63.2377，请求dist64.8658，trusted输入因该次坐标量化偏(-0.78248,-0.67362)px后dist65.7836，正确超出radius+2.5=65.7377，hover背景root；scroll timing/box保持不变，排除该次滚动漂移。此指针诊断未改生产拾取core、slop、旧verifier或像素阈值，仅修Rust fixture：原21×21候选中要求真实有fill的Rect/Circle、center fillInset≥1.5，再检查±1.5九点Scene.hit同owner。`hierarchy-firefox-newprobe.json`记录新候选fillInset1.81860和九个真实Rust命中；新pkg的原Firefox四case全阶段重跑通过，maxmean0.17852077908，`hierarchy-focus-firefox-report.json`带SHA。Line/Radar/Pie/Relations亦复验当前pkg；其后已完成最终fresh102，而非把专项拼成整套。

历史产物说明：151场景/90套的最终raw已另存`target/gallery/cross-browser-previous90-report.json`。更早记录中未另存的canonical报告路径是当轮使用位置，会被后续运行覆盖；不能把当前同名JSON当作所有历史轮次的原始快照。

## 独立Marimekko、Delaunay、地理线流与nice（前轮正式历史，2026-11-10）

`scripts/check.ps1 -BuildBrowser -RunNative` 最终新源原生阶段已确认：**726项全功能Rust单元/集成测试+9项README doctest，共735项；另629项no-default检查**，较上轮689项Rust新增37项。native/WASM严格Clippy、WASM检查及严格WASM rustdoc均通过；该源已包含Geo Data fit冗余mask资源修正及Delaunay装饰数据语义修正。最终完整`check.ps1 -BuildBrowser -RunNative`已exit0，包含四consumer的严格检查/release WASM/wasm-bindgen与Apace实际运行；最终Chrome五CLI也已全部exit0（host含25个extras和41项导航）；最终三引擎各30套、共90/90也已在同一次fresh完整重跑通过，原阈值不变。历史698/595、140场景和81套保持下文原记录，no-default不重复累计入全功能总数。

| 当前已通过的独立专项 | Rust项数 | 独立期望与实际范围 |
| --- | ---: | --- |
| `linear_nice` | 4 | 644条实际d3-scale4.0.2 / d3-array3.2.5 nice查询；往返域、常量、极值及decimal非严格幂等 |
| `marimekko_layout` | 8（含CPU1）；no-default7 | 实际Nivo hooks328个cell几何/端点/displayed值；独立厚度、五offset/nice/横纵/padding、hidden与原索引、zero/None、原始裁剪矩形、样式/轴、预算和失败原子 |
| `delaunay` | 11（含CPU1）；no-default10 | 20组非退化实际D3拓扑oracle；独立i128空圆/不交叉/Euler/CCW断言、重复/共线/共圆、MAX/subnormal/20000sites、实际点与边像素 |
| `geo_stream` 与独立 `geo_stream_audit` | 主9（含CPU1）+peer5；no-default合计13 | 314个TM点、2573个默认graticule点、61个stream点的实际D3 oracle；dense2001采样、4万scale误差、fit/overlay、微小弧/horizon/tiny seam保真及预算事务 |

Marimekko generator为`scripts/marimekko-reference.mjs`，读取本地Nivo0.99.0实际`hooks.ts`与`scales/src/linearScale.ts`，只替换React/color/accessor等宿主适配；冻结CSV记录源SHA256和D3scale4.0.2/array3.2.5/shape3.2.0版本。Nice generator为`scripts/linear-nice-reference.mjs`。Delaunay参考为`scripts/generate-delaunay-reference.mjs`与`tests/fixtures/delaunay_d3.rs`，实际参考版本d3-delaunay6.0.4、delaunator5.1.0、robust-predicates3.0.3；其20组输入用精确整数检查排除共线三点和共圆四点，避免把歧义对角线当作唯一golden。Geo使用`scripts/geo-stream-reference.mjs`及d3-geo3.1.1。四个脚本均按自身所属workspace metadata解析reference target；Delaunay的`--check`只读重生成对比已从父cwd实际逐字通过20组，不写冻结fixture。

从kartis workspace复核/再生成开发参考（不属于图表运行依赖）：

```powershell
$referenceTarget = (cargo metadata --no-deps --format-version 1 | ConvertFrom-Json).target_directory
npm install --prefix (Join-Path $referenceTarget 'd3-delaunay-reference') --no-audit --no-fund d3-delaunay@6.0.4 delaunator@5.1.0 robust-predicates@3.0.3
node scripts/generate-delaunay-reference.mjs --check
node scripts/linear-nice-reference.mjs
node scripts/marimekko-reference.mjs E:/Repos/nivo
node scripts/geo-stream-reference.mjs
```

后面三个再生成命令会写对应CSV，需要已有指定版本的本地参考包；不是一次只读验收，也不会由本库自动下载/执行Node。

本轮核心修正包括：Marimekko默认None、两套nice与零厚度计数，raw/interval/displayed分离；nice稳定迭代修复`[.01,.71]→[0,.8]`、`[0,14.1]→[0,16]`等域过早停步；Geo独立审计实际复现并修复tiny arc被删、数学horizon丢弧、tiny反经线跨越画假长线。审计继续要求原非零seam分段和端点精度，World绝对误差1e-9px、nearSeam/nearPole Data fit放大后1e-8px，maxPoints2失败保留旧Scene；没有通过放宽阈值遮蔽错误。极短弧Data fit可能产生超Scene坐标预算的冗余巨大cap mask；仅在原mask超预算、启用plot clip且以浮点余量和至少2逻辑px余量证明disk完整包含plot四角时省略，普通有效mask保留，horizon附近或不确定情况继续mask或拒绝。默认Orthographic/Gnomonic短弧完整铺满与20px round stroke真实CPU裁剪通过。

Delaunay首轮Chrome还发现装饰links/triangles沿用`Mark::new`默认label，5个site误生成17个datum控件。已清除两层装饰label并保留几何ID/series，原native11项扩充了SurfaceModel面内不伪拾取与canonical-only数据计数；verifier强化精确控件数及新键盘事件序列。最终统一pkg的Delaunay Chrome157.0.8103.39正式3×2专项全部通过，最大平均通道误差0.0007492404513888889，原阈值0.5未变，报告`target/gallery/delaunay-browser-report.json`。canonical-only控件、current hit/tooltip/keyboard、duplicate owner、共线→非共线、site/triangle raw-ID保留、零Mutation、坏输入原子及cleanup均通过。拒绝更新的revision/pending在显式flush前比较；flush本身会推进revision，因此flush后只比较实际markup，未削弱拒绝事务检查。

默认Rust图已增加**Spade2.15.1/robust1.2.0**，不是“只有Glory”；开发oracle不进入图表运行时。下文历史记录中的“当时默认只有Glory”保留为对应旧版本事实，不适用于本轮。算法归属见[third-party notices](third-party-notices.md)。旧Marimekko面积守恒入口和旧地图两度采样仍保留，新profile不宣称所有Nivo props、全球歧义polygon或任意投影形式误差已覆盖。

最终Chrome正式验收覆盖**151场景**、**41个实际style入口**（新增10个）和三个新专项，五CLI全部通过。画廊SVG/Canvas平均通道误差均值0.03783144057、最大0.33926888021，11个新增geometry断言全true；三组新场景截图`gallery151-{marimekko,delaunay,geo}.png`已人工检查。该结果来自最后新源fresh生成及统一pkg，而非`configured-preflight.json`预验。**30×3=90套**Firefox150.0.2/WebKit26.0/Edge157.0.4341.46组合最终全部通过，单次完整run为2026-11-10 11:36:21.414→11:43:30.117 UTC，raw报告`target/gallery/cross-browser-previous90-report.json`；151画廊及41导航在每家均通过。新增Mekko最大mean为0.02543677/0.26335327/0.01006567，Geo为0.00010579/0.00001085/0.01922173，Delaunay为0.02834961/0/0.00074924，均保原0.5阈值；详表见[跨浏览器记录](cross-browser-verification.md)。

最终新源Marimekko的Chrome157.0.8103.39正式4×2专项全部通过，报告为`target/gallery/marimekko-browser-report.json`；SVG/Canvas平均通道误差最大0.01006567，原阈值0.5不变。current raw/tooltip/keyboard、独立厚度、hidden重排、保留身份、zero无墨迹、零Mutation、坏输入事务、resize和Stop/stale-listener cleanup均通过，无page errors。已检查`marimekko-vertical.png`和`marimekko-diverging.png`截图，几何/标签排版正常。

Geo最终同一pkg的3×2 Chrome专项也通过，报告`target/gallery/geo-stream-browser-report.json`包含source/fixture/JS/WASM SHA256 stamp；最大平均通道误差0.01922173（hidden重布局），原阈值0.5不变。实际shared fit/symbol、seam两分段、完整stroke mask、graticule精确17/17/6条、当前raw值10→25输入/键盘、hidden保共享层、零Mutation/身份/事务/Stop均通过，无page errors。最终Chrome专项与三引擎汇总均以当前产物为准。

首轮完整90矩阵实跑为88/90，单独保留`target/gallery/cross-browser-p2-initial-report.json`：Firefox150.0.2与WebKit26.0各29/30，Edge157.0.4341.46为30/30；两处失败均为geoStreams pointer probe前景遮挡歧义。Delaunay/Marimekko、151画廊与41导航在三引擎均已通过，但不拼接作最终90结果。

case0/phase0请求点距前景symbol中心7.56898px，而r5+stroke0.5+既有2px slop合计7.5px，仅出界0.069px；Firefox实际坐标量化偏差(-0.33704,+0.12920)后距中心7.22373px，正确命中最上层symbol，route stroke在原请求与实际点也都命中。实证`target/gallery/geo-firefox-pointer-debug.json`保留actual coordinates、bounds和hover；不能据此判定route渲染或拾取错误。

fixture已只改选真实route中点，并要求±1逻辑px九点均由`Scene.hit`独立确认归属该route；保留原plot/paint条件，不改JS、库或像素阈值。唯一host workspace严格Clippy/release/wasm-bindgen重建已exit0（`examples/browser-host/target/probe-fix-{clippy,build}.log`），其余三consumer字节不变。新host pkg的Marimekko4×2 Chrome已再次全部通过，最大均值仍0.01006567。Geo也在Firefox与Chrome各3×2全部通过，Firefox报告`target/gallery/geo-stream-firefox-report.json`保留12个SVG/Canvas组合的实际hover/selected route、current10/25、trusted coordinates与sourceStamp；FF像素最大均值0.00010579427，Chrome仍0.01922173394。新case0的实际x量化偏差约-0.8084px仍处于安全九点区域；先前-0.337px仅是一次观测，不是通用量化上界。

新host完整CLI也已exit0，包含25个extras与41导航；三家新增Chrome专项已在新pkg全部复测通过，其余四Chrome CLI产物未变且已通过。随后完整90矩阵fresh重跑90/90通过，不拼接首轮29/30或旧pkg结果。735/629的核心原生结果继续有效；最终浏览器日期与报告均来自11:36:21.414→11:43:30.117 UTC的单次完整重跑。

## P1 AreaBump、Hierarchy、Calendar 与统计配置（历史结果，2026-11-10）

`scripts/check.ps1 -BuildBrowser -RunNative` 最终正式完整通过：**689个全功能Rust单元/集成测试 + 9个README doctest，共698项**，较上一轮新增49项Rust测试；另**595项no-default-feature测试**验证条件编译，不重复累计。格式、native与全features WASM严格Clippy、WASM检查、四个独立consumer的严格WASM/release/wasm-bindgen，以及最新Apace headless真实PNG/Graphic/UI输入/清理全部通过；WASM rustdoc在`-D warnings`下通过。日志为`target/check-latest.log`、`target/rustdoc-latest.log`。正式Chrome五CLI、140场景画廊及Firefox/WebKit/Edge各27套、共81组完整重跑也全部通过，未拼接历史69组结果。汇总为`target/acceptance-summary.json`。

四组新增专项为AreaBump8、Hierarchy12（layout8/focus4）、Calendar16、BoxPlot7/Bullet6，包含真实CPU像素；默认依赖图未增加图形/时区/浏览器依赖。旧radial26/Resquarify10回归也通过，不重复计作新增测试。GPU/worker本轮不实现，也未把历史memory测量当作本轮新测。

- AreaBump：逐slice排序/tie、共同厚度/spacing/align、数字与字符串x、显式空列、缺列Connect/Gap、零值/单点不造ink、Linear/Basis闭带、当前source index/rank/y bounds、active失败原子与CPU交叉。冻结oracle为396个actual Nivo compute点和180个actual Nivo areaPoints+D3完整路径，源`compute.ts` SHA256为`a158d7852f5042647b0d260ad2ac314011d7afb2fea1f0055df083470ed95a83`；脚本`area-bump-reference.mjs`读取实际本地TS，仅剥类型/cast后执行，引用d3-scale4.0.2/d3-shape3.2.0。CSV为`tests/fixtures/area-bump-{nivo,paths}.csv`，再生成按所属workspace Cargo metadata查target，不把Node/reference包当运行依赖。全零scale明确给0厚度，不与D3 constant-domain半高强行等价。
- Hierarchy：treemap五tilers面积守恒、四侧真实header/inner/outer预留及label mask/归属；padding后Resquarify复用原rows，改值/尺寸/focus保留、拓扑局部失效与失败cache回滚；packing真实viewport焦点圆/半径比/包含不重叠、icicle四向镜像/重排、raw ID与输入排列稳定；focus/back/parent/root、有界history、换树删焦点最近旧祖先/新根/空树恢复、坏输入/坏布局不改变已接受数据/历史/cache。额外发现先投screen再缩放会把1e-300子树的两叶重叠，已改同一packing算法的稳定局部归一化和parent-relative offsets；两/三叶×两算法×1e-300或最小subnormal(+MAX兄弟)×零/微小正/普通padding共24profiles真实成功，padding0两等叶golden为半径97.5/中心±97.5，旧普通padding比例回归和有界投影不变量通过。CPU验证focus真正扩大ink及parent header保留；旧API保原计算路径。
- Calendar：Compact实际周数、跨年/闰日/firstWeekday、FullYears面板、横纵方/长方日格、nullable/zero/provided区分、visible色域和year归属；每月真实union边界含凹折、跨周分离和padding后合并。CPU验证圆角/边界，显式纽约DST的当地日聚合通过；有界日期/配置/重复/坏区间保持纯事务输出。拾取保留旧Scene undashed border的width/2+2px容差，明确测试near-stroke角可hit、超过stroke/slop角miss，以及无day_border时fill外角miss，不声称strict painted hit。
- BoxPlot/Bullet：R7/group-subgroup/缺样本/Tukey/Extrema/离群点，横纵/round/fill与独立border/median/whisker/cap及CPU；canonical series/raw ID和隐藏域/slot/palette；全隐藏Log保留显式ticks或原数据一致的非零sign，负域fallback而非伪造正域，mixed/zero明确Err。主体/median标量是中位数，whisker/cap/border为不可交互装饰，mean保留metadata。Bullet Shared/PerRow/min/max/reverse、相邻端点区间和synthetic max/source index、域外裁剪与marker不伪命中、title/axis位置与完整row归属。两图旧specialized与Combo五数入口保持。

最终冻结产物的独立Chrome157.0.8103.39专项及正式完整host组合均通过，原平均通道差阈值0.5保持：

| 专项 | 实际覆盖 | 最大平均通道差（0..255） | 报告 |
| --- | --- | ---: | --- |
| AreaBump | 3case×2phase及hidden/allhidden，当前raw endpoint/controlled hover-focus、legend重布局、retained ID/零Mutation/拒绝/Stop | 0.00441455（含hidden） | `target/gallery/area-bump-browser-report.json` |
| Calendar | 4case×2phase及hidden：紧凑/完整年份、横纵/NULL/zero、真实圆角与旧stroke拾取政策、当前输入/键盘、共享装饰保留/失败保旧/Stop | 0.09887323 | `target/gallery/calendar-layout-browser-report.json` |
| Hierarchy | 4case×root/group/leaf/update/delete-recovery/hidden×SVG/Canvas，真实pointer/键盘导航、identity camera、祖先返回/当前值9、owned labels及原子拒绝/Stop | 0.05197130 | `target/gallery/hierarchy-focus-browser-report.json` |
| BoxPlot/Bullet | 4case×2phase：两图横纵与Bullet reverse，真实legend重排/全隐藏恢复、current tooltip/keyboard、owned axes/labels、retained ID/零Mutation/失败保旧/Stop | 0.00418701 | `target/gallery/statistics-browser-report.json` |

Hierarchy在最终精度修复后的统一pkg上，于2026-11-10 10:23:42.950 UTC重新完整跑四case、1440×1000/DPR2；报告附source/fixture/JS及实际`host_bg.wasm`的SHA256 sourceStamp，均无page errors。宿主revision按显式flush计帧；拒绝测试在拒绝调用后同步比较revision/pending/controller状态，再flush比较下一帧DOM，避免把正常flush计数误作坏帧提交。像素阈值与控制器原子断言未放宽。

**Chrome157.0.8103.39正式五套CLI全部通过，无page errors**：gallery、Rust/WASM Canvas、Glory Widget、CPU Raster Widget及完整BrowserChart host。Host包含四个P1新family、31个真实style入口、Hierarchy原生键盘focus/back/root与identity camera，并继续验证旧touch/CDP、RAF/队列/清理等。画廊**140场景**，较P0新增14场，平均通道差均值**0.03701396**、最大**0.33926888**（0..255），原3.5阈值不变；切换、实际拾取/13项新增几何不变量与移动布局通过。报告为`target/gallery/{gallery-browser-report,wasm-browser-report,widget-browser-report,raster-browser-report,host-browser-report}.json`，专项画廊另存`gallery140-browser-report.json`。四组新场景截图已实际人工查看。新P1像素专项使用独立0.5阈值，不把gallery的3.5阈值套到host上。

**Firefox150.0.2、WebKit26.0、Edge157.0.4341.46各27套，共81/81组全部通过，无page errors**。最终同套verifier在2026-11-10 10:29:52.439–10:35:47.079 UTC完整执行，每引擎gallery140、配置style入口31；包括Hierarchy四种视图的原生Enter自动focus/back/root，而非只由测试显式flush导航。新P1 suite最大平均通道差按FF/WebKit/Edge为：AreaBump **0.02708960/0.00015674/0.00441455**，Hierarchy **0.17852078/0.19609131/0.05197130**，Calendar **0.06323451/0.00064755/0.09887323**，BoxPlot/Bullet **0.05724316/0.03271045/0.00418701**，均保持原0.5阈值。Gallery另按原3.5阈值，最大值为0.60441949/0.31094206/0.33926888；不混称所有画面均低于0.5。原始完整报告`target/gallery/cross-browser-report.json`，矩阵与测量边界见 [跨浏览器记录](cross-browser-verification.md)。

该轮结束时最初10项优先任务已交付8项typed范围；随后Marimekko独立厚度与地理/Delaunay实现见本页首。更广props/关系/共享协议/平台仍继续，不以该优先队列完成宣称全部目标完成。当前API/profile见 [architecture.md](architecture.md) / [feature-gaps.md](feature-gaps.md)。

## P0 颜色尺度、HeatMap、Bar 与极坐标（历史结果，2026-11-10）

`scripts/check.ps1 -BuildBrowser -RunNative` 最终正式重跑完整通过：**640个全功能Rust单元/集成测试 + 9个README doctest，共649项**，较上一轮新增49个Rust测试；另552个无默认feature测试检查条件编译，不重复累计。格式、native与全features WASM严格Clippy、WASM检查、四个独立browser consumer的严格WASM Clippy/release/wasm-bindgen及Apace headless的真实PNG/Graphic/输入/清理全部通过；WASM rustdoc在`-D warnings`下通过。日志为 `target/check-latest.log`、`target/rustdoc-latest.log`。本轮未重跑内存采样，不把历史memory结果当作新增测量。正式126场景画廊、Chrome五套CLI及三引擎各23套共69组也全部通过；均使用最新冻结源码与最终同套脚本完整重跑。

当前专项为颜色尺度7项、色图例4项、独立颜色审计2项、HeatMap12项、Bar8项、公共polar axes6项、stacked RadialBar8项，以及静态SVG namespace2项。图表专项包含CPU真像素检查，namespace另有静态defs/refs断言及live实例截图对照。默认依赖不新增图形/浏览器/时区库，GPU/worker仍按用户指示留到最后。

- 颜色：Sequential/Diverging/Quantize、NULL/zero、正反向/常值/空域、clamp与域外unknown；300条独立D3 oracle含原threshold及±1ulp探针。普通有限Quantize保留D3原表达式舍入；极窄域/极大域若阈值越域或非单调，整组回退稳定插值，另验证正反向6量级×4ULP跨度×7色数的域内/单调不变量，不宣称病态域全部bit等价。独立审计先红后绿修复相邻f64域自动legend ticks重复叠绘，另用CPU验证半透明连续legend与同值datum颜色一致。
- 公共极坐标/RadialBar：numeric/category ticks、径向圈/圆周辐条、真实tick/文字旋转、事务grid背景/混排/slot保存，`polar_bar_with_axes`实际面积半径；新ring沿角堆叠保守恒/原身份、缺失与零、hidden严格等于Scene过滤、逆向/跨零/部分/整圈、8192cell预算和CPU墨迹/track。大端点整圈按端点浮点误差上界识别，±100000/±999990与两方向golden通过，超过该界的额外1e-7弧仍拒绝，首尾去重不误删内部近邻ticks。
- HeatMap：NULL与zero、稳定pair ID与重排、Rect/Circle/圆角/size/force_square/两轴padding、四侧axis/文字、visible自动色域、prefix不混淆的五种精确关联、事务active/结构缓存损坏拒绝；CPU形状/间隙与真实关联opacity。稀疏缺pair不补NULL，nullable metadata与Scene有限标量兼容值分开验证。
- Bar：分类inner/outer ratio与group pixel gap、两方向分组/正负堆叠/百分比、raw/displayed label/totals、补偿净和/正负合计、类别/系列颜色及稳定ID、独立raw tooltip、真实圆角端点/裁剪、隐藏重布局与label归属；CPU实际位置/颜色像素。
- SVG namespace：本轮发现并实际复现渲染库缺陷，两个inline SVG有5份重复defs ID；首owner隐藏后，第二SVG的linear像素变白，而Canvas仍为`[155,60,128]`。修复为显式 `to_svg_with_namespace` 和retained每owner稳定作用域，覆盖gradient/pattern/clip的定义与typed引用，原业务ID及包含`url(#...)`的安全文字不改写。两个静态native回归与严格WASM编译通过。最新产物的Chrome动态双SVG+Canvas专项也通过：首owner的hide/drop/remount/stop期间，第二linear实像素始终为`[155,60,128,255]`并与Canvas相同；radial/pattern/mark+label clip的固定平滑内部样本RGBA差≤2、文字clip外ink为0、本地defs/ref唯一、定义退休/节点身份/零Mutation、当前输入/拒绝和清理均验证。该用例采live composited SVG截图，未用SVG序列化成图片来掩盖文档内资源解析问题。Firefox的径向中心曾因CSS/device采样位置出现3级差异，最终保留原±2阈值，在离中心20px的平滑内部采样，并继续记录原center颜色、surface/svg CSS bounds和device-pixel诊断；不声称任意径向像素精确等价。报告为 `target/gallery/svg-isolation-browser-report.json`。修复前证据在 `target/gallery/svg-isolation-before-fix.json`、`svg-isolation-before.png`、`svg-isolation-hidden.png`。

四个新增独立Chrome157.0.8103.39专项均通过，原平均通道差阈值0.5未放宽：

| 专项 | 实际覆盖 | 本次最大平均通道差（0..255） | 报告 |
| --- | --- | ---: | --- |
| 共享颜色 | Geo/Calendar真实消费者、连续/离散legend、missing swatch、当前输入/键盘、defs退休/零Mutation/拒绝/Stop | 0.02290454 | `target/gallery/color-browser-report.json` |
| HeatMap | 4 shape/size/color case×2phase×5关联目标×SVG/Canvas；Controlled peer RGBA、NULL/zero、空隙/圆外、隐藏重排与清理 | 0.25178595 | `target/gallery/heatmap-browser-report.json` |
| Bar | 3 profile×2phase×SVG/Canvas；真实hidden重布局、raw totals/tooltip与percent labels、当前输入/保留DOM/拒绝/Stop | 0.00302222 | `target/gallery/bar-style-browser-report.json` |
| Stacked RadialBar | 3phase：部分顺向、改值逆向/旋转、整圈跨零；9段/3track、公用轴、seam4→3tick、category ownedlabels、defs1→0/身份/拒绝/Stop | 0.06799341 | `target/gallery/stacked-radial-browser-report.json` |

**Chrome157.0.8103.39正式五个CLI组合全部通过，无page errors**：gallery、Rust/WASM Canvas、Glory Widget、CPU Raster Widget及完整BrowserChart host。Host包含本轮颜色/HeatMap/Bar/Stacked RadialBar、SVG实例隔离及实际配置导航16入口、HeatMap五种真实hover选择；旧touch/CDP、viewport、RAF/队列/清理等用例继续通过。画廊为**126场景**，平均通道误差均值**0.03428053**、最大**0.33926888**（0..255），原3.5阈值保留；切换、实际命中及移动布局通过。正式报告为 `target/gallery/{gallery-browser-report,wasm-browser-report,widget-browser-report,raster-browser-report,host-browser-report}.json`。**Firefox150.0.2、WebKit26.0、Edge157.0.4341.46各23套，共69/69组可移植验收全部通过**，各gallery均126场景、配置导航均16入口，无page errors。最终完整运行时间为2026-11-10 09:17:49.052–09:22:09.333 UTC，不拼接初轮68/69结果。新四个P0 host suite均保持0.5阈值，gallery保持独立3.5阈值；live SVG隔离保留±2，实际最大通道差FF/WebKit/Edge为1/0/1。完整版本、各suite像素表和测量边界见 [跨浏览器记录](cross-browser-verification.md)，原始报告为 `target/gallery/cross-browser-report.json`。

Typed API、兼容入口和剩余范围见 [architecture.md](architecture.md) 与 [feature-gaps.md](feature-gaps.md)。Bar totals隐藏须重布局，HeatMap关系由应用接事件，RadialBar隐藏保留完整域/栈槽位；这些契约不等同所有Nivo props。P0已落实但整体目标继续进行，P1/P2变体、更多配置与平台验收仍保留，GPU/worker继续最后处理。

独立颜色oracle的可选再生成命令如下；参考包仅放在所属workspace正常Cargo产物目录，不是运行依赖，Rust测试读取已冻结CSV而不调用Node：

```powershell
$taskTarget = (cargo metadata --no-deps --format-version 1 | ConvertFrom-Json).target_directory
npm install --prefix (Join-Path $taskTarget 'd3-color-reference') --no-save --package-lock=false d3-scale@4.0.2 d3-interpolate@3.0.1 d3-color@3.1.0
node scripts/color-scale-reference.mjs
```

生成脚本同样在实际workspace读取metadata，并动态导入该目录下参考包，不硬编码target或重定向Cargo/TEMP/cache。

## Pie、Radar、Sankey 与 Line 配置（历史记录，2026-11-10）

`scripts/check.ps1 -BuildBrowser -RunNative` 正式完整通过：**591个全功能Rust单元/集成测试 + 9个README doctest，共600项**，比上一轮新增31个Rust测试；另509个无默认feature测试检查条件编译，不重复累计。格式、native/WASM严格Clippy、WASM检查、四个独立browser consumer的严格WASM Clippy/release/wasm-bindgen，以及Apace headless消费者的真实PNG/provider/Graphic/输入/清理全部通过。WASM rustdoc在`-D warnings`下通过，日志为 `target/check-latest.log`、`target/rustdoc-latest.log`。本轮未重新运行内存采样，不把历史memory结果计作新增实测。

四组全功能专项为 **Pie 5、Radar 6、Sankey 11、Line 9 项**，各组包含实际 CPU 像素测试。Pie/Radar 另与既有26项 radial 回归一起验证；Sankey 另与既有9项 circular Sankey 回归验证。以下浏览器结果不等同 OS 窗口、物理设备或全 Nivo 功能验收。

- Pie：解析切圆起点/切线、真实角落切除、内孔、顺逆时针、小角/薄环/完整无接缝 annulus；fill alpha 不影响 border，active 半径、Label/Id/Value/Percent、skip angle/radius、真实引线/归属、ID碰撞和坏输入预算。Chrome157.0.8103.39三phase实际SVG/Canvas、角落/孔洞像素、改值/active/百分比/逆向、不可选引线、hover/click/keyboard、legend隐藏slice+leader+文字、定义退休1→0、相同帧零Mutation、path身份、坏输入不入队及Stop全部通过。三phase meanChannelError 为 **0.00417046 / 0 / 0.02014404**，隐藏后0.00653429，保留0.75阈值。
- Radar：独立域、圆/多边形网格和辐条、格式化轴/刻度、真实 closed curves、RGBA8独立填充/边框、五种等面积点/owned labels、隐藏域稳定、业务ID和精确命令预算（199,684命令可准入）。Chrome双phase及Firefox150.0.2、WebKit26.0、Edge157.0.4341.46独立同套验收均通过，包含当前点/tooltip、键盘、legend归属标签、更新原子性、零Mutation与清理。Chrome两phase平均通道误差 **0.02003003 / 0.02221143**；三个跨引擎的最高值0.04062524，原像素阈值未放宽。
- Sankey：DAG与循环、横纵方向、stable link IDs、并行边流宽/端口、正向 curvature、回流真实side/centerline、节点圆角/边框、渐变和标签；隐藏不动完整拓扑/流宽、正流投影下溢拒绝，CPU实际渐变与圆角像素。Chrome及Firefox/WebKit/Edge均通过四case×SVG/Canvas：更新/当前命中/tooltip/keyboard、source图例与two-ended重布局、owned labels、几何身份、梯度退休、坏输入保旧与Stop。最高平均通道误差分别为 **0.05648577 / 0.09931431 / 0.27871605 / 0.05648577**，全部低于原0.5阈值。
- Line：系列 stroke/area/point 独立颜色/透明度与真实渐变、dash/点边框、无可见点时的标签/透明mesh站点、共享曲线/裁剪、connect_missing保留原gap x域和源索引、隐藏图例身份、文字格式/归属/clip及失败原子。独立只读审计复查生成角色前缀与合法业务ID、隐藏palette、源index和中间/最终预算，未发现已证实语义缺陷。Chrome157.0.8103.39实际三phase及hidden通过：SVG/Canvas更新、当前点/tooltip/键盘、透明mesh站点、owned labels/legend、渐变退休、零Mutation、path身份、坏输入保旧与Stop；三phase平均通道误差0.00372179 / 0.00486816 / 0.00492486，hidden为0.06602403，保留0.5阈值。

**Chrome157.0.8103.39正式五个CLI组合全部通过，无page errors**：gallery、Rust/WASM Canvas、Glory Web Widget、CPU Raster Widget及BrowserChart host。Host包含本轮Pie/Radar/Sankey/Line、实际配置导航及Change styles，并继续通过touch/CDP、viewport、RAF/队列/清理等既有用例；Widget包含outline/dashes。画廊为**116场景**，新增11个配置对照场景，逐通道误差均值**0.03150934**、最大**0.14879384**（0..255），保留3.5阈值；切换、实际拾取及移动布局通过。正式报告为 `target/gallery/{gallery-browser-report,wasm-browser-report,widget-browser-report,raster-browser-report,host-browser-report}.json`，独立画廊报告另存 `gallery116-browser-report.json`。**Firefox150.0.2、WebKit26.0、Edge157.0.4341.46各18套，共54/54组可移植验收全部通过**，每份gallery均为116场景，无page errors。最终报告在2026-11-10 08:14:14.469–08:16:21.524 UTC完整重跑，不拼接前轮通过项；路径为 `target/gallery/cross-browser-report.json`，详见 [跨浏览器记录](cross-browser-verification.md)。Pie保留0.75、Radar/Sankey/Line保留0.5的独立阈值，画廊仍为3.5，没有统一放宽阈值。

专项原始报告为 `target/gallery/pie-browser-report.json`、`radar-browser-report.json`、`radar-cross-report.json`、`sankey-styles-browser-report.json`、`sankey-styles-cross-browser-report.json` 和 `styled-line-browser-report.json`；均为所属workspace正常Cargo产物目录下的可再生产物。SVG转图片时复制live继承语言，保持实际文字环境；像素指标为0..255的平均通道绝对误差，不是帧率或性能保证。

本轮跨引擎修正只调整验收测量与输入上下文：WebKit对完全同点的零长4C路径仍报告1.5258789e-5高的bbox，现要求原26坐标严格同点、native总长0、透明fill/noStroke，并检查Scene半径0；没有增加bbox容差。Firefox的Escape现直接发送到surface执行真实Clear，避免外部datum control焦点保留selection高亮后检查错误状态。受影响Chrome gallery/host也已重跑通过，最终54组含同一严格检查。

当时未覆盖的Pie asin/显式测量放置已在本轮新增入口补上；当前真实差距见 [roadmap.md](roadmap.md)：点文字自动避碰/全局leader最优仍保留、active显式控制；Sankey回流保留正交清障，source-owned Scene过滤与两端hidden重布局不同。GPU/worker按用户指示留到最后，本轮未实现。

## 树图、平行坐标、网络拖动与内存（历史记录，2026-11-10）

`scripts/check.ps1 -BuildBrowser -RunNative -RunMemory` 完整通过：**560个全功能Rust单元/集成测试 + 9个README doctest，共569项**；比上一轮新增28个Rust测试、3个实际API用例doctest。另482个无默认feature测试检查条件编译，不重复累计。格式、native/WASM严格Clippy、四个独立browser consumer的release WASM/wasm-bindgen、Apace headless实际PNG/provider/Graphic/输入/清理，以及两种原生内存profile全部通过。WASM rustdoc继续在`-D warnings`下通过。实际工具链1.99.1，声明MSRV1.95未另测；日志`target/check-latest.log`、`target/rustdoc-latest.log`。

- Tree：五方向、Bump/Straight/Step/StepBefore/StepAfter、link宽/颜色/透明度/dash、Circle/Square/Diamond、leaf/internal分类样式、标签位置/显隐/旋转与plot clip。默认旧几何和旧tree API保留。新6项覆盖真实几何、CPU像素、极值及预算；Chrome三phase实际SVG/Canvas像素、节点角落、hover/click/current tooltip/keyboard、hidden ownedlabel、同帧零Mutation、keyed path与原子拒绝通过。
- Parallel：独立Linear/Log/SymLog/反向轴、横纵布局、四种missing策略、共享曲线、轴/刻度/格式、按series样式/虚线/opacity、group legend和owned label mask；旧函数保持原样。13项新增测试及1项独立审计；审计先红后绿修复合法业务ID含`-grid-`时轴/网格角色、位置和前后景顺序错乱，改为真实grid索引。四引擎实际painted-dash命中/当前格式化tooltip/键盘/legend-ownedmask/坏输入保旧/Stop通过。
- Network：`NetworkDragController`世界/场景/client映射、抓取偏移、camera/resize rebase、RestoreFixedness/Free/Pin和cancel恢复；`BrowserNetwork`真实SVG/Canvas Pointer/capture/RAF、64项有序手势FIFO、latest simulation slot和动态拓扑替换。节点更换保留common DOM/选择、修剪移除hidden、更新标签；Scene/RAF拒绝不覆盖已有候选。独立第2tick积分失败测试验证整批nodes/heat/ticks/session不提交，cancel恢复prior状态。
- Network审计修复了焦点切换blur误取消、隐藏0尺寸永久卸载、同task隐藏→flush→相同尺寸恢复不唤醒、快速Up/Down丢新gesture，以及恢复回调的重入边界。实际测试覆盖热度恢复/冷却后idle RAF、foreign pointers、lostcapture/非有限Up/窗口blur/隐藏节点、late integration failure保留physics与完整DOM、Error回调内replace/select/nested flush、Stop/Drop/pagehide及stale listener。另实测pending resize和绝对view同帧调用，最终宽200、camera scale2/translation0保持请求值。
- 画廊由98增至**105场景**，新增3个Tree、2个Parallel、Network拖动前后两图；已检查实际截图，并为外侧标签预留margin、区分组颜色。修复独立画廊JS检查遗漏dash/cap及路径mask的问题，SVG/Canvas真实鼠标验证墨迹命中、虚线间隙、孔洞与mask外侧均通过。实际示例导航增加Tree、两向Parallel、Network，切换/恢复会清理旧owner。
- 内存：default原生34+raster50个同步存活checkpoint、浏览器自然回收49+显式GC诊断65个snapshot，全通过。采实际OS工作集/生命周期峰值/private commit与真实WASM导出容量；报告含可执行文件/源码hash与实际WASM response SHA256，确认加载模块匹配当前构建。数字、命令、原始数据及限制见[memory-measurement.md](memory-measurement.md)。WASM本例最大52.31MiB为容量，未宣称活分配或无泄漏。

Chrome157.0.8103.39的gallery/WASM/widget/raster/host五个CLI组合全部通过；host新增Tree/Parallel/Network与示例按钮验收。105场景逐通道误差均值**0.0285145**、最大**0.148794**（0..255），原3.5阈值保留。**Firefox150.0.2、WebKit26.0、Edge157.0.4341.46各14套，共42组可移植验收全部通过**，原阈值未放宽。Firefox输入坐标检查采用独立记录的真实trusted PointerEvent而非请求给driver的坐标，保留0.01/0.02世界坐标容差。实际版本/明细在`target/gallery/cross-browser-report.json`及[cross-browser-verification.md](cross-browser-verification.md)。

构建使用每个workspace的正常metadata target，默认依赖仍只有Glory，Glory源码未改；未重定向任何缓存，也未实现GPU/worker。全部Nivo props、自动业务语义、任意球面拓扑、完整活分配/显存/compositor、OS窗口/物理设备/辅助技术仍保留为差距，详见[roadmap.md](roadmap.md)。

## Pattern、虚线、完整mask与区间组合（历史记录，2026-11-10）

`scripts/check.ps1 -BuildBrowser -RunNative` 完整通过：**532个全功能Rust单元/集成测试 + 6个README doctest，共538项**，比上一轮新增71个Rust测试。另457个无默认feature测试独立验证条件编译，不重复累计。格式、native/WASM严格Clippy、四个独立browser consumer的release WASM与wasm-bindgen，以及Apace headless consumer的真实PNG解码/绘制/输入/清理全部通过；全功能WASM rustdoc在`-D warnings`下通过。实际工具链1.99.1，声明MSRV1.95未另测。日志`target/check-latest.log`、`target/rustdoc-latest.log`。

- Pattern：Dots/Squares/Lines、错列/旋转/偏移/透明色，SVG定义及typed Canvas/ScriptCanvas/CPU瓦片缓存。单瓦片4M、整帧16M像素，累计操作及unique预算；DPR/view矩阵和ceil backing校正。真实BrowserChart定义共享、零Mutation、样式替换/退休、实际motif像素通过。强制createPattern异常时旧画布changedBytes=0、尺寸/变换不变；恢复后两个mark只创建一次Pattern（`pattern-failure-audit.json`）。
- 虚线：奇偶数组、负offset、零长Round/Square、源切线、closed seam、全零实线、有界曲线/弧长误差。修复Butt间隙误中、Square角漏中，并用CPU真像素对照；Canvas统一miterlimit4。CPU先合并整条ink再单次着色，透明重叠修复前`[255,64,64]`、修复后`[255,127,127]`。原生SVG/Canvas先normalize并拒绝超f32范围，避免宿主饱和；纯几何/CPU保留f64 profile。
- 路径mask：NonZero/EvenOdd、孔洞、矩形交集、空mask、完整stroke及旋转文字裁切；保留SVG path/text身份、切换四种wrapper组合及定义4→0→4退休通过。Scene/motion/hidden/mesh/slices/brush和label槽位映射接入。自适应curve拾取边界不确定带≤0.05logical，source/flatten/work均有累计准入。Geo五种带stroke符号与地点文字使用投影disk；真实CPU验证外侧ink清零、内侧保留、legend逐RGBA不变。
- Combo：ErrorBars/BoxPlot/RangeBand与既有柱/线/面积/散点共享x及左右y；缺段、裁剪、区间extents、分组slot、预计算五数、系列线/点/边框/alpha/标签。修复median与box同色不可见，默认theme.text并支持median_color。真实浏览器三组组合图覆盖phase变化、命中/tooltip/键盘、owned labels/legend hide、坏输入保旧和Stop。
- 语义Widget：实际多Graphic虚线和整mark组opacity，零dot只绘一次，Square保持源切线，完整subpath按256命令分块。mask/shader及多ink Graphic的部分透明stroke颜色明确拒绝；plot(false)/raster可用，拒绝不改旧控件和图形。Chrome/Firefox/WebKit/Edge实际截图验证重叠、间隙、方帽、响应式变化与清理。

Chrome157.0.8103.39的gallery/WASM/widget/raster/host五个CLI组合通过，host含paint/contours/clips/combo/pattern、widget含outline/dash。画廊为**98场景**，新增6个样式/组合图场景；已实际查看pattern、mask、箱线图、区间带截图。逐通道误差均值**0.0285741**、最大值**0.148794**（0..255），保存于`target/gallery/gallery-browser-report.json`。旧捆绑Chromium141的完整host亦通过。

**Firefox150.0.2、WebKit26.0、Edge157.0.4341.46各10套，共30组可移植验收全部通过**。新加pattern/clip/combo/semantic dash，保持既有阈值。审计修正两项测量上下文：SVG序列化复制原继承语言，避免generic sans字体回退变化；语义组opacity逐通道严格等于独立CSS对照（Firefox/WebKit为126，Chrome/Edge为127），保留重叠相等、gap白、Square切线及交互断言。原先141 host误差1.24435→0.01263、Firefox clip0.90248→0.12283，未修改字体或live DOM，也未放宽阈值。详情见`target/clip-engine-audit`与`target/gallery/cross-browser-report.json`。

全部构建使用各workspace正常metadata target_directory；未重定向缓存、未改Glory仓库。GPU/worker仍暂缓。全部Nivo props、任意球面拓扑/自动业务语义匹配、OS窗口/物理设备和辅助技术等仍有差距，保留在[roadmap.md](roadmap.md)。

## 多轮廓、地理裁剪、字形与跨引擎验收（历史记录，2026-11-10）

`scripts/check.ps1 -BuildBrowser -RunNative` 通过：**461个全功能Rust单元/集成测试 + 6个README doctest，共467项**；另401个无默认feature测试验证条件编译，不重复累计。格式、native严格Clippy、WASM检查、四个独立browser consumer的严格Clippy/release WASM/wasm-bindgen，以及Apace headless consumer的实际执行通过。全功能WASM rustdoc在`-D warnings`下通过。声明MSRV1.95仍未用该版本另测；实际工具链1.99.1。日志`target/check-latest.log`、`target/rustdoc-latest.log`。

- 多轮廓：8项新增core测试及既有46项动画/时钟回归；显式key/Move映射、outer/hole顺序、缺槽collapse、出现/消失/重入、物理速度/原始精确终态、32槽/4m工作量与失败原子。真实BrowserChart SVG/Canvas Properties RAF各约82变化帧，实际两组96点，精确孔洞/ring像素、settle后重启、reduced、坏映射及active Drop通过。
- 地理：新增11项polygon裁剪与7项symbol clip测试，相关43项；非凸可见多岛、outer+holes整体重接、跨界hole成真实notch、cap全包围/全被hole遮蔽、外→外双交点、反向winding/旋转membership grid、小角/近90/Gnomonic和CPU像素。符号五种shape共享投影disk并实际裁填充；stroke须None，label不球面裁剪，均为明确能力边界。
- 绘制：7项paint及5项独立审计，线性/径向SVG/Canvas/CPU渐变、透明异色stop与Chromium像素oracle、规范化负零ID/定义复用退休、stop预算、backend准入、失败保旧；真实圆角/border覆盖bar/combo。审计修复裁剪边假圆角，包含横纵/正负/堆叠/分组/±MAX与独立membership oracle。
- 拾取/极值审计：3项回归验证仅fill_paint有值的Circle参与Mesh/Brush/Surface nearest/slices，hidden/clip仍过滤；Grouped不做无用合计，合法巨大并列柱不再因总和overflow拒绝。原溢出拒绝测试明确保留在Stacked。
- 完整Chrome host组合验收发现并修复媒体偏好丢失：256动作满队列可能丢ReducedMotion(false)，导致权威设置已恢复但后续动画永久snap。新独立合并槽在动作前处理，纳入排程/snapshot/stop；两后端真实false/true/false回归保持pending257且0新增drop，drain后各206变化帧/207 revisions正常动画。原NaN wheel保持动画断言未放宽，完整host及paints/contours通过，零page errors。
- 旋转文字：6项outline测试，真实FontRegistry shaping、kerning/ligature/combining、完整glyph/holes、slot/owner/draworder、反应式变化与缺字/预算保旧。真实Glory语义SVG glyph像素、明确安全文本、浏览器卸载通过；Apace直接Graphic有521个字形ink像素，确认无PNG图像替代该label分支。

Chrome画廊现为 **92场景**，新增多轮廓孔洞、非凸/小圆/Gnomonic、投影轮廓内五种符号、渐变/圆角。全部切换、几何断言、像素、移动布局和真实Canvas tooltip通过；逐通道误差mean **0.02817**、max **0.14879**（0..255）。已查看四个新增截图；原生轮廓/clip另有实际像素测试。Chrome widget子报告含outline，host子报告含paints/contours，新增独立CLI `--outline` / `--density` 可复跑。

**Firefox150.0.2、WebKit26.0、Edge157.0.4341.42 × 六套可移植suite，共18组通过**，均复用原断言与阈值。三引擎分别覆盖92场景gallery、widget、outline、paints、contours、raster PNG；gallery最大误差分别1.65179/0.31094/0.14879，保持原3.5上限。修正两处verifier输入问题：画廊选完整位于clip内的真实数据点，SVG图像像素采样显式采用场景尺寸，避免Firefox默认300px intrinsic大小。完整版本、现存bundle路径/driver差异、CDP与设备排除项见[cross-browser-verification.md](cross-browser-verification.md)，报告`target/gallery/cross-browser-report.json`。

**浏览器高密度八组通过**：SVG/Canvas各1k/5k/10k/20k，64个真实变化帧warm样本、8个cold、实际DOM/backing/pixels、hover/click和精确回到DOM基线。验收促成修复f64 client坐标：旧i32 getter会令1.882px高mark漏选，现实际fractional pointer命中通过。记录的20k sync median为SVG181.8ms/Canvas43.45ms；Canvas固定7个DOM后代、1280×720 backing，SVG40012个DOM后代。JS heap只作为可选观测，不代表进程RSS/全部WASM内存，未声称60FPS。见[browser-density-benchmark.md](browser-density-benchmark.md)；原始`density-browser-report.json`与公共CLI复跑`density-cli-report.json`分别保留。

所有构建仍使用各workspace正常metadata target_directory；默认依赖仍只有Glory，Glory仓库未改。GPU/worker按用户指示暂缓，未引入依赖。全球歧义球面拓扑、带stroke的完整mask、文字球面clip、自动业务语义匹配、OS窗口/设备与辅助技术仍保持明确差距，见[roadmap.md](roadmap.md)。

## 功能差距补全与统一验收（历史记录，2026-11-10）

`./scripts/check.ps1 -BuildBrowser -RunNative` 完整通过：**414 个全功能 Rust 单元/集成测试 + 5 个 README doctest，共419项**。另运行366个无默认feature测试验证条件编译路径，不将重复用例累加。格式、无默认feature全目标检查、全功能严格Clippy、WASM检查、四个独立浏览器consumer的格式/严格Clippy/release WASM/wasm-bindgen、独立native-raster consumer的格式/check/严格Clippy和实际执行全部通过。全功能WASM rustdoc在`-D warnings`下通过；修正了新增源码文档中两处裸URL。统一日志`target/check-latest.log`，文档检查`target/rustdoc-latest.log`。

- 日期：严格parse/parse_with、冲突/ISO周/epoch/offset/精度、IANA gap/fold；官方d3-time-format4.1.0全部34locale，918条独立D3输出与parse对照；当地日期与Sum/Count/Mean/Min/Max聚合，涵盖DST及极值均值。
- 循环Sankey：环/自环/平行边、四对齐、输入重排稳定、流宽和端口守恒、回流通道碰撞、交叉目标不增、subnormal/大图预算、真实CPU ribbon像素。
- 地理：3种新增真实方位投影的18个D3 raw golden、等积Jacobian/距离性质、horizon/rotation/backface、5种等面积符号与同形状图例/实际hit/CPU面积像素。方位投影明确为前半球90°profile。
- 展示：多列legend、测量布局、tooltip禁区与viewport、label优先级/隐藏、自定义Rust Widget与当前Bond、controlled提案/坏测量保旧帧、回调及owner清理。独立审计先复现后修复透明label错误阻挡可见label。
- 导航：分类窗口、分数Band映射、bar/combo稳定ID/颜色/堆叠、可见刻度/边缘裁剪；独立X/Y绝对变换及增量手势，分类Xcap与值轴独立。审计修复虚拟范围准入、动态plot过期错误、旧plot像素解释、单轴饱和锚点漂移、pinch重复点1ulp漂移。显式bar值域可排除零，先堆叠后裁剪并移除不可见命中。
- 动画：按ID显式单轮廓采样morph、closed对齐/方向、enter/exit几何、retarget速度和exact target；坏配置/工作量预算原子拒绝，默认保留Fade。
- RasterSurface：9项owner/weak/单帧controller测试，包含offset/DPI、控件桥、resize/zoom、publication失败与budget保旧帧、回调重入/stop、初始及更新中owner退休、capture清理。

**五套真实Chrome CLI全部通过，零page errors**，报告为`target/gallery/{browser,wasm-browser,widget-browser,host-browser,raster-browser}-report.json`。Host包含既有touch/domainViewport/retainedSvg，以及新增navigation/presentation/geometryMotion：

- numeric/category × SVG/Canvas四种profile验证非等比X4/Y2、当前像素hit、wheel保留比例、pan、动态insets、坏plot/布局失败保旧域与geometry、Reset。
- 两种renderer各验证旧plot回调替换后返回Err不泄漏、同task动态margin+偏心wheel仍按已接受plot像素解释、单分类的Y请求和真实wheel可继续缩放。
- 真实DOM测量的八方tooltip、边角与resize clamp、禁区避让、完整安全多行文本与超大内容滚动。
- 真实RAF morph产生多帧；SVG中实际48点单轮廓、无ghost替身；中途retarget与reduced motion后恢复精确circle目标。Canvas验证实际目标像素。页面提供分类、独立轴、morph和tooltip布局模式入口。

画廊现在为 **84个场景**，含循环Sankey、分类bar/combo、新投影/非圆符号、morph初始/中间/终态和测量label避让。全部切换/逐像素/移动布局通过；SVG/Canvas平均逐通道误差均值 **0.02880**、最大 **0.14879**（0..255）。已查看新增Sankey、morph中间帧及原生输出图片。

**实际Apace headless消费者通过**：独立应用内bounded PNG/font provider，原始Chart.raster链及RasterSurface链均通过真实解码/CPU绘制；733个字体outline ink像素、显式Apace字体注册1、offset image hit、实际UiEvent图例/键盘选择、tooltip、resize/DPR、invalidscene原子保留、weak/guard/provider close清理。默认产物为`examples/native-raster/target/native-raster-apace.{png,json}`（使用该workspace metadata返回路径）。此结果不包含OS窗口、物理设备或辅助技术验收。

高密度benchmark两套release已实际运行，20个样本+2次预热+首次调用；128次mesh暴力oracle核对与预算越界拒绝通过。本机10k scatter布局/SVG median约35.384/28.611ms，20k Scene SVG约78.927ms；20k sites的2000次nearest批次median约0.944ms。真实RGBA的1k/5k/10k/20k median为7.124/15.240/44.414/75.110ms。完整first/p95/min/max、工具链、图幅、资源预算与范围见[density-benchmark.md](density-benchmark.md)，不宣称RSS、浏览器FPS或跨机器Nivo比较。

默认依赖图仍只有Glory core/render/style/ui，无Glory仓库改动；所有workspace遵循正常Cargo配置并读取自己的target_directory，无Cargo/TEMP/cache重定向。完整Nivo props、多轮廓morph、复杂非凸跨horizon与精确球面轮廓clip、GPU/worker、OS窗口/辅助技术/物理触屏/其他浏览器仍有真实差距，见[roadmap.md](roadmap.md)。

## CPU栅格、当地刻度与增量SVG验收（历史记录，2026-11-10）

`./scripts/check.ps1 -BuildBrowser` 完成：**333 个全功能 Rust 单元/集成测试 + 5 个 README doctest，共338项通过**；另独立运行 **298 个无默认feature测试** 验证条件编译路径（不把两套重复测试相加为独立项目）。无默认feature全目标编译、格式、严格全目标Clippy、全功能WASM与四个独立consumer的严格Clippy/release WASM/wasm-bindgen构建通过。全功能WASM rustdoc在`-D warnings`下通过。日志为`target/check-latest.log`。

- CPU Raster 9项：真实RGBA source-over、winding孔洞、曲线/stroke cap/clip、DPR/view、PNG解码逐字节roundtrip、字体锚点/旋转、AV字距/fi连字/组合字符、显式回退和缺字策略，以及输入/预算/primitive失败保留旧帧。OFL字体来自官方google/fonts，原文件和许可证均保留。原生Rust `raster_export` 实际输出1440×800 PNG，已查看其图形和旋转标签。
- Raster Widget 11项及独立审计1项：真实PNG源、Cage尺寸/DPR、图例像素与控件一起提交、失败保留整帧、601指令路径、控件变化不重复PNG、publisher owner清理和stale回调。审计发现并修复首次publisher销毁construction owner后继续创建Cage的panic；cleanup现于首次回调前注册，回调后检查owner。
- 当地日历10项时区测试（无timezone为4项）：Day2跨月重启、Hour5/Minute7/Month5字段对齐、fixedoffset/负epoch、NY23/25小时日及狭小fold范围、LordHowe30分钟回拨、Apia跳日、午夜gap/fold、显式策略、自动ticks、line/dual_axis/combo真实轴像素与失败原子性。不是把UTC ticks换标签；先计算civil边界再反解真实instant。

**五套真实Chrome CLI均通过，无page errors**。新增 `--raster` 对实际Glory `<img>` PNG解码并采样：图形颜色正确，alpha/beta/shared文字ink像素分别305/254/448；图例重绘、尺寸/DPR、保留img身份、hover/focus/keyboard/currentdatum、发布失败与无效DPR原子保留、清理/重挂载均通过。图像sampling canvas仅用于验收。未据此声明原生窗口provider或原生pointer实机通过。

Host新增`retainedSvg`验收：动画206个不同path帧期间保留SVG根、mark路径、文字slot；重复同帧MutationObserver为零；clip增删、几何类型变化、stroke属性清除、混合画序、安全文本、退场/重进、Stop/重挂载均验证实际DOM。与实时Canvas像素对照本次平均通道误差约0.01262/255。Host的实际WASM `calendarTime` 同时验证NY春季午夜间隔24/23/24小时、秋季两份01:00的不同offset和严格有序instant。

画廊 **74个场景**，新增当地day/month边界；全部切换/像素/tooltip/移动布局通过，SVG/Canvas平均通道误差均值0.03043、最大0.14879（0..255）。CPU导出与浏览器PNG截图均已查看。报告为`target/gallery/{browser-report,wasm-browser-report,widget-browser-report,host-browser-report,raster-browser-report}.json`；Host包含touch/domainViewport/retainedSvg三个子报告。

构建保持每个workspace的实际Cargo target_directory；默认依赖图仍只有Glory，未引入栅格/字体/PNG/时区依赖。Glory仓库无改动。当地calendar日期数据模型、parse、bidi/跨字体shaping、原生provider/窗口/辅助技术验收等真实差距仍列在[roadmap.md](roadmap.md)，目标继续进行。

## 日期时区、固定轴与展示配置验收（历史记录，2026-11-10）

`./scripts/check.ps1 -BuildBrowser` 完成：**302 个 Rust 单元/集成测试 + 5 个 README doctest，共 307 项通过**。无默认 feature 全目标编译、格式、全功能严格 Clippy、WASM 编译、三个独立 consumer 的严格 Clippy/release WASM/wasm-bindgen 均通过；全功能 WASM rustdoc 在 `-D warnings` 下通过。日志为 `target/check-latest.log`。

- DateTimeFormat 默认 9 项、启用 timezone 后 13 项测试：locale 模板/字段、ISO 周、负 epoch、Gregorian 扩展年份、偏移、循环/预算，以及纽约/上海历史 DST、巴黎历史561秒偏移、2101年纽约与悉尼季节规则。静态 IANA 数据为2026e，直接 TZif/POSIX 解析；默认依赖图仍只有 Glory，Named 越界明确拒绝。
- DomainViewport 10 项、Surface relayout 3 项、独立审计 4 项：Linear/Log/SymLog/UTC 逆映射、左右域、缩放/平移/刷选、resize域不变、时间1ms、极值拒绝原子性、当前像素命中、不重复 inverse、连续 pan/pinch 基线和重布局后的 hover repick。
- Presentation 9 项及既有 Widget/render 回归：真实图例布局/符号、文本模板、受控多 ID、缺失值策略、安全 SSR、动态组合标签预算、格式失败清空和恢复、回调卸载清理。浏览器发现并修复倍数 line-height 到 Glory px 单位的转换。

四套真实 Chrome CLI 全部通过，无 page errors。画廊 **72 个场景**，新增中文闰日/星期、UTC+08跨午夜及固定轴域变化前后对照；同 plot、Y域、轴路径和datum半径保持，X域0..100→25..75后可见点11→5及位置转换已断言。全画廊SVG/Canvas平均通道绝对误差均值0.03108、最大0.14879（0..255）。

Host 报告新增 `domainViewport`：SVG/Canvas 均通过 wheel/pan/真实触摸 pinch、固定轴路径、当前像素命中、resize保持域、数据布局优先于旧 on_resize、拒绝布局保持域/view/geometry。独立审计促成并验证：替换回调的过期成功/错误都丢弃；回调 stop 后 flush 成功；Up/TouchEnd 最后坐标触发布局但被拒绝时，不复活已结束触点；stationary hover 随新几何重新命中。回调替换使用 `viewport_controller()` 的精确副本，保留严格数值相等断言，避免像素 transform 往返的末位舍入。

非默认 legend/tooltip 的实际 CSS、纯文本模板、Fixed2、过滤均通过，实测 line-height 为 legend28.9px、tooltip28.8px。浏览器 consumer 显式启用 timezone，实际 WASM 输出纽约2024春跳 `01:59 -0500`→`03:00 -0400`、2101夏季 `08:00 -0400` 及中文 locale；并非仅编译时区代码。

浏览器页面也提供可点击的固定轴/自定义展示模式、动画和重置按钮，以及实际 WASM 时区样本；已通过模式按钮验收并检查固定轴截图。完整报告在 `target/gallery/host-browser-report.json`（包含 touch 与 domainViewport）和其余三套既有 JSON。默认依赖图已复核；构建使用所属 workspace 的实际 Cargo 产物路径，Glory 源码无改动。剩余类别/非等比导航、当地日历刻度/parse、更完整locale目录、原生Canvas及平台验收等仍见 [roadmap.md](roadmap.md)，目标继续进行。

## 双指、曲线、地图符号与格式化验收（历史记录，2026-11-10）

`./scripts/check.ps1 -BuildBrowser` 已完成：**263 个 Rust 单元/集成测试 + 4 个 README doctest，共 267 项通过**。检查新增无默认 feature 的全目标编译；格式、全功能严格 Clippy、WASM、三个独立 consumer workspace 的严格 Clippy/release WASM/wasm-bindgen 均通过。全功能 WASM rustdoc 在 `-D warnings` 下通过。日志为 `target/check-latest.log`。

- 曲线增加 14 个枚举变体及 Cardinal tension 构造器；126 组独立 d3-shape 3.2.0 context golden 覆盖不同点数。另验证长 run 连续分块、周期轮廓、缺失段、open area 真实端点/基线、MonotoneX/Y 独立轴单调性。700 点 closed area 保留完整单路径：直接 SVG/Canvas 成功，语义 Widget 按自身预算明确拒绝；没有把节点限制强加给直接后端。
- 数值格式扩展通过 741 组冻结的发布版 d3-format 3.1.0 Node golden，覆盖精确十进制舍入、极小/极大数、符号/对齐/基数/locale、固定 SI 与显式字符串 c。另做 1,200 组确定性随机 IEEE 位模式 oracle 对照；测试运行本身不需要 Node 或 D3。资源与类型 profile 见 format 模块文档。
- 地图原 5 项及新增 13 项测试验证五投影的 World/Data 共用映射、view/roll/scale/translate、反经线与背面、面积/半径、zero/missing、数值色阶、真实图例输出、身份/画序/归属、极值、10k 点与联合预算。覆盖层不重新拟合底图。
- Surface 的 6 项双指测试及 7 项既有回归全部通过：锚点、中点平移、缩放上下界、重合触点、单指恢复、点击抑制、取消/resize、Touch/Mouse 互斥、第三指和坏输入原子拒绝。

四套真实 Chrome CLI 均通过、无 page errors。画廊 **68 个场景**，新增长闭合面积图、open/closed/tension/Bump/MonotoneY 对照，以及实际大小/颜色图例的地图。SVG/Canvas 通道绝对误差均值0.02997、最大0.14879（0..255）；700 点完整闭合图0.01287，两个新地图分别0.01560与0.02136。已检查实际截图，移动布局、切换、tooltip 均通过。

BrowserChart 的新增触摸脚本通过 [Chrome DevTools Input 协议](https://raw.githubusercontent.com/ChromeDevTools/devtools-protocol/master/pdl/domains/Input.pdl) 注入真实浏览器触摸流，并观测由浏览器产生的 primary/nonprimary PointerEvents 与实际 capture。SVG、Canvas 均验证双指锚点、抬一指后的连续平移、无误点击、单指点击、第三触点忽略和 cancel。无 resize 回调、回调拒绝、隐藏、活动触点中的 Stop/Drop 也验证了捕获释放及重挂载。该测试不等同物理触屏、Wry/Apace 或其他浏览器验收。

报告为 `target/gallery/{browser-report,wasm-browser-report,widget-browser-report,host-browser-report}.json`；触摸明细在 host 报告 `touch` 中。默认依赖仍不引入 serde/DOM；产物使用各 workspace 的实际 Cargo target_directory，Glory 源码无改动。目标保持进行，剩余范围见 [roadmap.md](roadmap.md)。

## 混合图、稳定 treemap 与属性动画验收（历史记录，2026-11-10）

`./scripts/check.ps1 -BuildBrowser` 完成：**231 个 Rust 单元/集成测试 + 3 个 README doctest，共 234 项通过**。格式检查、严格全目标/全功能 Clippy、WASM 编译及三个独立 consumer workspace 的严格 Clippy/release WASM/wasm-bindgen 构建全部通过。全功能 WASM rustdoc 在 `-D warnings` 下通过。日志为 `target/check-latest.log`。

- 混合图 8 项测试与独立审计 6 项通过，验证柱/线/面积/散点的共享 x、左右值轴、正负堆叠/百分比、点身份、排列不变性和分类缺失段；审计修复分组求和溢出、散点影响单柱宽度、域外中心可见柱裁剪、隐藏负 Log x/y 系列的自动域问题。
- Resquarify 10 项测试通过，证明复用后的布局与重新 squarify 确有不同，行成员和方向保持不变，支持局部结构失效、零值复活、目标比率、极值面积与 2048 叶节点的连续 12 帧缓存复用。
- PropertyMotion 17 项测试与属性帧时钟集成测试通过：逐属性速度、反向重定向、目标为当前绘制值仍保留动量、同 ID 拓扑和用户 ghost ID 冲突、标签归属、退出复活、约束前真实状态、最短旋转、混排快照、原子预算拒绝与 reduced motion。
- 旋转轴/文字的真实锚点、SVG/Canvas 脚本、后端能力准入、语义 Widget 拒绝及原场景弹簧角度插值均有回归；通用 ExplicitClock 另验证已活跃轨道首步不丢时间。

四套真实 Chrome 验收全部通过，无 page errors。画廊现在 **60 个场景**，包含旋转分类标签、四种类型混合双轴图和 retained treemap 前后帧；SVG/Canvas 逐通道绝对误差均值0.03012、最大0.14879（0..255），移动单列、交互和切换通过。已检查新增示例的实际截图。

BrowserChart 两个后端均在采样中明确观测到 `animation_kind = Properties`，自动推进至精确目标并连续 12 帧稳定，旋转文字到达 30°；另外重挂载为 `BrowserAnimation::Scene` 验证原共享进度模式及 reduced-motion 完成。旧的输入队列、捕获、RAF 失败回滚、回调重入、销毁/重挂载用例也全部通过。帧采样数量属于本次运行记录，不作为 FPS 或性能保证。报告仍为 `target/gallery/{browser-report,wasm-browser-report,widget-browser-report,host-browser-report}.json`。

本轮仍未实机验收 Glory 语义文字旋转或原生 Canvas；语义 Chart 对旋转场景明确报错，BrowserChart 的直接 SVG/Canvas 可正常使用。所有构建使用所属 workspace 的实际 Cargo `target_directory`，Glory 源仓库保持无改动。剩余范围见 [roadmap.md](roadmap.md)，目标继续进行。

## 浏览器宿主与算法补全验收（历史记录，2026-11-10）

`./scripts/check.ps1 -BuildBrowser` 已完成：**185 个 Rust 单元/集成测试 + 3 个 README doctest，共 188 项通过**。包格式、严格全目标/全功能 Clippy、全功能 WASM 编译通过；三个独立 consumer workspace（browser-canvas、browser-widget、browser-host）的格式、严格 WASM Clippy、release WASM 和 wasm-bindgen 构建通过。另以 `RUSTDOCFLAGS='-D warnings'` 完成全功能 WASM 文档构建。统一日志为 `target/check-latest.log`。

新增数值和状态验证涵盖：Top/Right 轴与独立双值轴，Basis/CatmullRom/Bundle 的 D3 控制点对照，locale 格式解析与舍入，四种 Sankey 对齐，支撑集外包圆及退化数值输入，显式帧调度与速度 retarget，事务式 SurfaceModel、零面积 clip/填充命中，以及 Widget 外部 hover 与 plot 隐藏模式。

四套真实 Chrome CLI 已复跑：56 图画廊、Rust/WASM Canvas、Glory Web Widget 和新增 BrowserChart 宿主，全部通过且无 page errors。画廊覆盖本轮新增曲线、侧轴、locale、双轴和四种 Sankey 对齐，SVG/Canvas 逐通道绝对误差均值0.03088、最大0.14879（0..255），切换、tooltip 和移动单列无溢出均通过。

BrowserChart 的 SVG/Canvas 两种后端均验证实际 pointer、keyboard、滚轮锚点、平移、Shift 框选、图例归属标签、ResizeObserver 重排和 RAF 动画；601 条指令的路径也完成绘制，未受语义 Graphic 单路径预算限制。每种后端采集到 170 次受控时间采样、159 个不同动画帧，最后 12 帧稳定；这不是实测帧率指标。

宿主还验证 RAF 请求失败时入队回滚、回调替换重入、满队列 resize 通知、lostpointercapture、隐藏宿主取消、多种 primary pointer 的互斥、非法滚轮保持原动画、类型化输入打断动画、缩放后视口外框选裁剪、256 项队列准入/溢出、动态 reduced-motion、回调中 stop、旧监听器清理与 Drop/重挂载。报告为 `target/gallery/host-browser-report.json`，其余报告沿用下方路径。

```powershell
$env:PLAYWRIGHT_CHANNEL = 'chrome'
npm run test:browser -- --host http://127.0.0.1:8814 target/gallery/host-browser-report.json
```

服务根目录为 `examples/browser-host/`；构建与启动见该示例 README。所有构建均读取所属 workspace 的 Cargo metadata，产物分别位于该 workspace 自身的 `target/`；没有重定向全局缓存、Cargo home 或临时目录。Glory 仓库保持无改动。

## 上一轮继续完善验收（历史记录，2026-11-10）

`./scripts/check.ps1 -BuildBrowser` 已完成：**143 个 Rust 单元/集成测试 + 3 个 README doctest，共 146 项通过**；包格式检查、严格全目标/全功能 Clippy、全功能 WASM 编译通过。两个独立浏览器 consumer workspace 的格式/严格 WASM Clippy、release WASM 与 wasm-bindgen 构建通过。日志为 `target/check-latest.log`。

最终源码另做了极值审计：两轴量级悬殊的裁剪、精确单位对角交线、异构站点 clip 的 Voronoi 明确拒绝、微小站点间距、正负零刻度、UTC 纵轴 tooltip、显式 heatmap domain 和非默认字号 bump；对应 `axes_clip_mesh` 15 项均通过。

三套真实 Chrome CLI 验收均通过，DPR=2，无 page errors：

- 画廊 **46 个示例**，包括实际 custom layers、Log/UTC 轴、域裁剪、Wiggle、Tidy/Squarify、Voronoi、球面/反经线地图、多年日历、Barnes–Hut。SVG 与 Canvas 逐通道绝对误差均值0.02572、最大0.14879（0..255），另验证 inline SVG 裁剪 viewport、键盘切换、指针和移动单列布局。
- Rust/WASM Canvas：实际绘制、命中、点击、键盘、图例、移动重排与监听器清理，backing store2020×960、移动逻辑尺寸344×300。
- 真实 Glory Web Widget：受控图例拒绝/接受、标签归属过滤、几何/文字混排与原 DOM 节点保留、实际 hover/focus tooltip、真实 ArrowUp 焦点、当前数据激活、旧帧保留、退休监听器清理、卸载/重挂载。CSS clip 的 left30/width560、内部 Graphic left0/width640 符合逻辑坐标。

报告为 `target/gallery/{browser-report,wasm-browser-report,widget-browser-report}.json`。浏览器运行命令：

```powershell
$env:PLAYWRIGHT_CHANNEL = 'chrome'
npm run test:browser -- http://127.0.0.1:8811 target/gallery/browser-report.json
npm run test:browser -- --wasm http://127.0.0.1:8812 target/gallery/wasm-browser-report.json
npm run test:browser -- --widget http://127.0.0.1:8813 target/gallery/widget-browser-report.json
```

### 数值与性能基准

可再生产例：`cargo run --release --example mesh_bench`、`cargo run --release --example network_bench`。当前一次本机测量（不作为 CI 速度门槛）：

| 基准 | 输入 | 本次结果 |
| --- | --- | --- |
| KD-tree |20,000点，2,000查询|构建11.857ms，查询2.389ms，暴力对照316.471ms，0不一致|
| Barnes–Hut |4096节点|精确149.810ms、近似4.721ms；相对L2误差0.004583|
| 完整力模拟 |1024节点、20tick|38.404ms|

Barnes–Hut 此样例直接相互作用54,484次、聚合单元180,875次，对照精确16,773,120次。theta=0、精确重合桶等仍存在二次最坏情况。基准衡量计算本身，不能代表浏览器帧率或端到端原生 UI 性能。原始产物为 `target/mesh-bench.json`、`target/network-bench.csv`。

默认依赖图通过 `cargo tree -e normal --no-default-features` 检查，只有 Glory core/render/style/ui，不引入 serde、DOM 或图形库；`geojson` 显式选择 JSON 解析。Glory 仓库没有因本轮任务产生改动。

剩余工作以 [roadmap.md](roadmap.md) 为准；目标仍在进行中，没有据此宣称完全达到 nivo 功能等价。

## 初始实现基线（历史记录）

日期：2026-11-10。运行环境为 Windows / rustc 1.99.1、cargo 1.99.1（声明 MSRV 1.95，本次未单独验收 MSRV），浏览器验收使用本机 Chrome，Playwright 1.56.1。记录的是本次本地执行结果，不代表所有浏览器、操作系统或原生 host 的验收。

## Rust

`scripts/check.ps1` 全部通过：

- 55 个单元/集成测试，包括 UTC 日历刻度、数值比例尺、布局守恒、坏输入、几何/动画、命中、框选/缩放、注释准入、SVG/Canvas 回放、Glory SSR、响应式旧帧保留/恢复、身份与卸载清理。
- 3 个 README doctest。
- `cargo fmt --package kartis --check`。
- `cargo clippy --locked --all-features --all-targets -- -D warnings`。
- `cargo check --locked --target wasm32-unknown-unknown --features canvas --lib`。
- 独立 browser-canvas workspace 的 fmt 与 wasm32 严格 Clippy。

`scripts/check.ps1 -BuildBrowser` 另外完成浏览器示例 release WASM 编译和 wasm-bindgen 0.2.130 绑定生成。每个 workspace 先读取自身 `cargo metadata` 的 `target_directory`，没有更改 Cargo/TEMP/cache 环境。

## 实际浏览器

通过 npm 提供的独立 CLI 运行，而不仅是编译浏览器代码：

```powershell
$env:PLAYWRIGHT_CHANNEL = 'chrome' # 可选：使用本机 Chrome；否则用 Playwright Chromium
npm run test:browser -- http://127.0.0.1:8811 target/gallery/browser-report.json
npm run test:browser -- --wasm http://127.0.0.1:8812 target/gallery/wasm-browser-report.json
```

两个服务分别以 `target/gallery/` 和 `examples/browser-canvas/` 为根目录。

画廊测试验证 32 个示例：Canvas 非空绘制、SVG/Canvas 模式切换、SVG 栅格化与 Canvas 的逐通道像素差异、390px 单列无横向溢出、键盘切换、Canvas 指针数值。DPR=2，Canvas backing store 为 1440×800；32 图的平均逐通道绝对误差均值为 0.0322、最大为 0.1488（0..255）。这证明当前样例在该浏览器中的一致性，不保证任意字体/浏览器逐像素相等。

Rust/WASM 测试验证直接调用 `draw_canvas` 的真实绘制，以及 Rust 侧 pointer/click/keyboard、图例隐藏、resize 重排和 `stop()` 后监听器清理。DPR=2 时 backing store 为 2020×960；移动布局为 344×300 逻辑像素，无横向溢出；无 page errors。

详细 JSON 报告与截图位于 `target/gallery/`，属于可再生产物，不纳入版本控制。

## 未涵盖

尚未实机验收 Glory Wry/Apace 图形显示、操作系统屏幕阅读器、触屏手势、其他浏览器、复杂字体及性能规模。当前没有对上述平台作验收通过声明。算法与 nivo 的具体差异见 [architecture.md](architecture.md)。
## 球面工具：新增实现与独立反例

距离、流式长度、有向面积、采样圆、重心与包含判断已提供公共入口，见[球面指南](geo-spherical.zh-CN.md)。首次正式检查通过最小特性 1092 项、全部特性 1300 项、15 个 doctest、native/WASM 严格 Clippy 和 Rust 1.95；四个浏览器消费者重建和两段完整指南程序运行成功。248 幅画廊保留旧 246 幅独立 SVG 字节不变；四幅 CPU PNG 经独立解码、填充计数和 72 个源位置像素核验通过。

独立新增随机与简单矩形样本发现极小多边形重心的数值消去，以及部分包含判断与原 D3 分歧；首次失败源码和反例完整保留。随后已合入局部面积一阶矩、稳定经界叉积和近直角边系数修复，普通原参考门槛及 EPS 不变。第二轮正式检查通过最小特性 1095 项、全部特性 1304 项、15 个 doctest，19 项球面专项、四消费者和两个指南程序通过。独立真实 WASM 对照覆盖原 51/467 样本、281 个高精度面积重心及 12000 次矩形内外判断。80/120 位精确 binary64 见证保留原 D3 错值；舍入敏感度选择属于工程估计，不声称任意平台的认证区间界。该数值修复未改变 248 幅独立 SVG 和四幅 CPU PNG 的字节。

整批功能验收仍未完成。新画廊首轮 Chrome DPR1/2 通过，Firefox 的完整 SVG-image/Canvas 平均差未通过既有 0.5 门槛。为 SVG 图像匹配文档语言虽减少差异，仍未达标，不能计作修复。进一步最小复现发现，相同字体字节与字号下 HTMLCanvas 和 SVG 的实际文字 advance 不同；主线程 Offscreen 的显式字体配置对照不能冒充默认 Sans 已修复。尚未运行的上下文不计入通过项，原门槛、文字和失败记录均保留。



## Development Funnel and Parallel filtering

Unreleased development APIs, not part of the published Kartis 0.1.0 package.

ParallelFilterController provides transactional raw-value AND filters and actual
scale inversion for pixel/Brush intervals. It renders from the full source,
preserving domain/palette/group legend/source indices while removing rejected
row ink, owned labels and interaction sites. This is a Rust extension that can
connect to D3 Brush, not an existing Nivo Parallel filter prop. Automatic live
host gesture wiring and categorical dimensions remain outside this acceptance.

Funnel stage factories replace one bounded body and explicitly preserve or
remove its original open-sided border. Exact picking follows custom ink;
Nearest keeps the original center. Default gradient inheritance preserves the
complete layout. Independent audit reproduced and fixed both default metadata
drift and a cumulative-budget false rejection; the final 16,384 gradient stops
and 200,000 command boundaries remain admitted with no raised limits.

Native acceptance: 1,440 minimal tests, 1,707 all-feature tests and 17 doctests;
native all-target Clippy, Rust 1.95 WASM Clippy and pinned nightly formatting
passed. Records: `target/development-funnel-parallel-validation/`.

The executable `funnel_custom` exports four directional/active frames, SVG,
Canvas scripts and CPU PNG. Chrome, Firefox, WebKit and Edge at DPR 1/2 compare
48 SVG-image/Canvas samples with CPU RGBA per context: 8 contexts and 384
comparisons. These samples verify the tested offset geometry and controlled
active expansion; they are not full-frame equality or retained-host/device
acceptance. Final artifacts: `target/funnel-custom-sealed/`. The first browser
run failed for a missing pinned Firefox executable; its log is retained, the
pinned browser installation completed, and the complete cohort was rerun.

Guides: [Funnel](funnel-custom.md), [Parallel filtering](parallel-filtering.md).
Arbitrary React/Widget components, automatic host Brush mounting, GPU/worker
and the remaining broader gap list are not closed by these results.

## Development measured text wrapping

Unreleased Rust extension, not a Nivo automatic-wrap prop and not included in
crates.io 0.1.0. Full-candidate real measurements drive word/grapheme breaking;
Scene replacement atomically remaps typography, ownership, clips and draw order.
True FontRegistry advances/ink and font-defined line metrics supply the CPU
wrapper. The primary face's line metrics are not HTML/CSS line-box metrics.

Core 6, independent audit 6 and raster 6 tests cover combining/emoji graphemes,
CRLF, ellipsis, non-monotonic advance, rotation, stale measurements, budgets,
family/weight, empty registry rejection and transactional metadata. The audit
reproduced and fixed discarded trailing-space mismeasurement and moved repeated
clip-cost admission before copying. Raster-only 5 tests also passed.

Complete regression: 1,452 minimal and 1,725 all-feature tests, 17 doctests;
native all-target Clippy, Rust 1.95 WASM Clippy and fixed-nightly format checks.
The first native Clippy run caught a new test's chunks_exact style; the corrected
test and strict Clippy were rerun, preserving the failed log.

The executable `text_wrapping` exports ordinary editable labels and separately
outlined text plus a CPU PNG. Four browsers at DPR1/2 (8 contexts) verify all
11 line strings, native column fit and SVG-path/Canvas-path ink bounding boxes
against CPU (within the declared 1.5 logical-pixel bound). This does not compare
entire RGBA images or establish ordinary SVG/Canvas text pixel equivalence.
The original exact-advance probe (0.1 px) failed on Firefox: maximum difference
0.38590502929687887 px. That failure is retained at
`target/text-wrapping-browser-first.log`; its cause was not isolated. The later
profile report separately records native column fit, advance differences and
outline placement; it does not relabel that exact-advance criterion as passed.
Final exports/reports: `target/text-wrapping-sealed/`.

The initial wrapping acceptance preceded an explicit host whitespace policy.
The subsequent development acceptance below supersedes that whitespace limitation;
it does not resolve the retained Firefox exact-advance failure. Full CSS/UAX14,
paragraph-wide bidi followed by breaking, automatic vertical wrapping and
automatic host reflow remain open gaps. See [guide](text-wrapping.md).

## Development explicit whitespace acceptance (2026-11-14)

`LabelWhitespace::Legacy` preserves existing behavior. Explicit `Preserve`
normalizes single-line control separators and retains repeated/leading/trailing
spaces in standalone SVG, retained SVG, browser Canvas, CPU raster, and actual
Glory ordinary Text. Scene wrapping marks every generated line Preserve.
Measurement identity includes the mode; mode-only changes request fresh metrics.
Filtering, painted-label conversion, motion, and managed-label placement preserve
or remap the sparse policy with the labels.

Complete regression passed: 1,468 minimal tests, 1,742 all-feature tests and
17 doctests; native all-target strict Clippy and Rust 1.95 WASM strict Clippy.
The public `SceneTypography` and `SemanticTextRequest` fields require updates to
complete struct literals. This remains Unreleased and is not in published 0.1.0.

The actual browser-widget fixture runs Legacy/Preserve/Legacy/Preserve with
identical source text in Chrome, Firefox, WebKit and Edge at DPR1/2: 8 contexts,
32 phases, unchanged retained text-node identity, restored Legacy styling,
fresh mode-dependent measurements, and real Glory Text content/styles.
Preserve SVG/HTML advance and DOM Range glyph widths agree within the declared
1 logical-pixel bound. Sixteen Preserve phases compare actual Canvas ink bounds
with serialized SVG-image ink bounds within 1.5 logical pixels. This is not
whole-image RGBA equality, arbitrary-font/device certification, or a solution to
the earlier CPU/native Firefox advance difference.

Reports: `target/whitespace-browser-after-clippy.json` and
`target/whitespace-final-validation/acceptance.json`. Initial fixture setup
failures are retained: missing bootstrap mount root, an incorrectly hidden
measurement host, and default Canvas selection where the retained-SVG fixture
required explicit SVG. The final fixture uses a normally sized offscreen
bootstrap host and explicit renderer selection; thresholds were not relaxed.
A later fixture-only strict Clippy check rejected a nested conditional; its
first failure is retained, the conditional was corrected, and the actual WASM
fixture and all 8 browser contexts were rebuilt and rerun successfully.
See the [policy guide](text-whitespace.md) and [中文指南](text-whitespace.zh-CN.md).


## Development Waffle cells and Calendar year labels

Unreleased additions, not included in the published 0.1.0 package. The borrowed
Waffle cell factory preserves physical slot identity and default whole-layout
output, styles real empty cells without creating typed data sites, and separates
custom ink picking from the original logical slot. In Areas mode only empty
cell marks invoke it; retained solid group unions count toward the final budget.
Seven core tests and five independent audit tests cover identity, traversal,
paint/site/source-color separation, dashed ink, extreme Rect round-off, animated
Path anchors, rejection recovery, and exact 200,000-command/16,384-stop budgets.

The Calendar entry composes FullYears year-label Before/After placement, signed
finite offsets and a borrowed text factory with existing day colors. Old entries,
panel allocation, dates/months and Scene.legend remain unchanged. Compact rejects
the new entry. Six minimal/seven raster tests cover negative offsets, rotation,
month spacing, text budgets, empty output and actual Lato raster ink.

Full regression passed 1,486 minimal tests, 1,761 all-feature tests and 17
doctests; Rust 1.95 WASM strict Clippy passed. Native strict Clippy first caught
new test-expression issues (fixed-size chunks and cloned singleton slices).
Those first failures and one failed test-expression migration compile are kept.
After correction, the affected Calendar 7 and Waffle audit 5 tests and native
all-target strict Clippy passed. Production library code did not change during
these test-only corrections.

Four engines (Chrome, Firefox, WebKit, Edge), each at DPR1/2, passed these fresh
export profiles:

- Waffle: 16 frames cover four directions, Cells/Areas and a hidden group.
  Every slot's logical origin and offset ink position is queried. Each context
  compares 384 SVG-image/Canvas samples against CPU RGBA within 1 channel unit;
  3,072 comparisons total. These samples are not whole-image equality.
- Calendar: 24 frames cover two orientations, Before/After, offsets -12/0/20 and
  month spacing 0/3, with two years per frame. The exact pinned original Nivo
  computeYearLegendPositions function independently verifies 48 placement
  anchors/rotations. The existing rotated size/3 alphabetic shift is removed
  only for this anchor comparison: Nivo Canvas uses middle baseline, which is
  not claimed equivalent. Supplied-font outlined SVG-image/Canvas ink bounding
  boxes agree with CPU within 1.5 logical pixels (48 comparisons/context, 384
  total). Ordinary editable chart SVGs are exported separately; outline checks
  do not prove ordinary native-font pixels or full Nivo layout equality.

Both scripts also support the bundled Chromium-only CI profile at DPR1/2;
the gallery workflow now generates these examples and retains their reports and
exports. The two-context CI profile is distinct from the eight-context manual
matrix. Guides: [Waffle](waffle-custom.md), [Calendar](calendar-year-legend.md).
Fresh reports live under `target/waffle-custom-v2/` and
`target/calendar-year-legend-v2/`; aggregate acceptance is
`target/waffle-calendar-final-validation/acceptance.json`. Initial 16-frame
Calendar results are retained separately and not reused for the expanded matrix.
GPU/worker, arbitrary components, broader platform input and other open gaps
remain outside this acceptance.


## Development Bullet part factories

Unreleased API, not included in the published 0.1.0 package. One borrowed typed
factory handles actual Range/Measure/Marker marks, including synthetic domain
ranges, while preserving raw intervals, source indices, row domains and track
clips. Rank includes omitted endpoints; source-row and visible-row indices stay
separate. Metadata geometry is updated after callbacks so every callback sees
original row snapshots. Default inheritance preserves the entire inner layout.

Ten minimal/eleven all-feature core tests and five independent audit tests cover
source/order identity, omitted and synthetic parts, actual offset ink versus
original logical centers, scale composition, track clipping, paint/geometry
rejection, late callback errors and exact 200,000-command/16,384-stop budgets.
Inspection found a transparent-fill/visible-stroke representative-color issue;
identity and custom ink now share the existing visibility-aware paint rule,
without rewriting original identity marks. A regression verifies the red stroke
site color and unchanged original transparent-fill layout.

Actual Glory Memory mounting initially rejected the legacy interaction binding.
The retained failed logs identify the initial host rejection; source inspection
confirmed that chart_layout requires explicit PaintedGeometry. The new
into_painted_scene_interaction conversion preflights that solid-ink profile.
The ordinary conversion retains Legacy Geometry/Nearest behavior. Tests mount a
real strict Glory chart and verify that strict Nearest and eligible gradient
fills reject, while legacy gradient SVG/Canvas exports remain valid. These
profiles are distinct; semantic Graphic/Text limits still apply.

Full regression passed: 1,501 minimal tests, 1,777 all-feature tests, 17 doctests,
native all-target strict Clippy and Rust 1.95 WASM strict Clippy. Formatting,
hidden-file spelling and diff checks passed. Initial development failures remain
retained: a new test counter's borrow lifetime, the initial host profile mismatch,
and a test-only incorrect FillPaint variant spelling. The full final regression
ran after all corrections, rather than reusing pre-correction suite results.

The executable bullet_custom checks 16 profiles: horizontal/vertical, reverse,
legacy/Linear/SymLog/Log, shared/per-row domains and a hidden row. Every profile
mounts its full Scene in real Glory Memory, verifies semantic Graphic and typed
control counts, and exercises Surface Exact/Nearest at custom ink and original
centers. There are 144 actual controls and callback invocations across 16 mounts.
Ink-only export clones remove ordinary labels before CPU/SVG/Canvas comparisons;
this does not test font rendering. Each of 8 browser contexts (Chrome, Firefox,
WebKit, Edge at DPR1/2) compares 432 SVG-image/Canvas samples with CPU RGBA within
1 channel unit, 3,456 comparisons total. Samples are not whole-image equivalence.
The separate bundled Chromium CI profile passed both DPRs (864 comparisons).

CI now generates and retains this export profile alongside Waffle and Calendar.
Original Nivo rangeComponent/measureComponent/markerComponent source contracts
were checked against commit 0dae2c32052a573f9f1d66ec1b453b572119b265; this is not a
React render oracle. Nivo removes zero endpoints and uses filtered sorted ranks;
Kartis's original zero/negative/source-index contract remains. Arbitrary React
components, animatedProps lifecycle, physical devices and broader parity remain
outside this acceptance. See [guide](bullet-custom.md) and [中文指南](bullet-custom.zh-CN.md).
Reports: target/bullet-custom-final/browser-report.json and
 target/bullet-custom-final-validation/acceptance.json.


## Development Bump endpoint text

Unreleased addition, absent from published 0.1.0. Independent Start/End text
factories compose with the existing point factory through BumpFactories. The
source series/label, legend, tooltip, geometry, style and logical identity stay
unchanged; only actual owned endpoint Label text is replaced. Default wrappers
preserve the complete layout. Contexts expose original source indices, completed
rows and both line and padded label positions. Source traversal remains explicit
with reversed X. Hidden/disabled/missing ends skip callbacks; empty and zero-ink
owned slots remain, with the shared 1,024-byte UTF-8 budget.

Core 6 and independent audit 6 tests passed in both minimal and all-feature
profiles, with the existing Bump 6 tests passing in both profiles. They cover
source borrowing, missing and nonmatching source order, reverse, actual Memory
Props::Text, owner/whitespace remapping, zero opacity/point size, callback order,
failure stops and byte boundaries. The audit's initial helper missed an Ok
wrapper required by the old point-factory signature; the compile failure was
retained and corrected. No production-source correction was required.

Full regression passed: 1,513 minimal tests, 1,789 all-feature tests and 17
doctests; native all-target strict Clippy and Rust 1.95 WASM strict Clippy.
Afterward, the export fixture was extended to execute ordinary Canvas text as
well as outlined Canvas; it was rebuilt and its example-specific strict Clippy
passed. Production library/tests did not change during this fixture extension.
Formatting, hidden-file spelling and diff checks passed.

The real-font fixture exercises 16 reverse/hidden/start/end/disabled profiles,
combined rectangular point geometry, missing endpoints and an empty start label.
It mounts the full Scene in 16 actual Glory Memory apps and checks 24 ordinary
Text nodes, including eight empty slots, alongside 88 point and 24 endpoint
callback invocations. Text-only export clones retain the accepted label data.
Lato font bytes are explicitly bound for CPU and loaded in the real browser.

Chrome, Firefox, WebKit and Edge at DPR1/2 passed eight contexts. Each checks all
24 editable SVG labels/anchors and all 24 actual ordinary Canvas fillText calls
(text, transformed anchor, alignment, alphabetic baseline and requested font),
including eight empty calls. Native Canvas blank/nonblank states also match the
expected six blank frames. No native-text advance or pixel equality is claimed.
Separately, outlined SVG-image and Canvas bounds match actual CPU text ink within
1.5 logical pixels: 32 checks/context, 256 total, including exact blank-state
checks. Whole-image RGBA and ordinary native-font pixel equivalence remain outside
this result. Bundled Chromium at both DPRs also passed the separate CI profile.

The original Nivo startLabel/endLabel source contract was checked at commit
0dae2c32052a573f9f1d66ec1b453b572119b265. Its position-separated hooks call the
accessor before excluding missing endpoints; Rust's actual-label, series-first
point→Start→End order differs deliberately. This is not a React execution or
rendering oracle. Automatic label collision, arbitrary component lifecycle,
physical-device acceptance and the broader gap list remain open.

Guides: [English](bump-endpoint-labels.md), [中文](bump-endpoint-labels.zh-CN.md).
Fresh exports and reports: target/bump-endpoint-labels-v2/;
aggregate receipt: target/bump-endpoint-final-validation/acceptance.json.
The first outline-only fixture profile remains separately retained. CI now
creates the extended profile and uploads its exports/reports; its bundled
Chromium profile was exercised directly, without claiming a new remote CI run.


## Development Albers USA primitives and planar rectangle postclip

**Foundation-stage acceptance only; full Albers USA integration remains open.**
The Unreleased `GeoAlbersUsaProjection` point/inverse kernel and planar rectangle
postclip APIs are implemented. They do not yet supply an Albers USA option to
`GeoMapOptions`, fitted polygon maps, geographic line/graticule streams,
GeoSymbol/glyph/label consumers or geographic host-coordinate adapters. This
record does not close that feature gap or establish a complete USA map renderer.
Published 0.1.0 does not contain these additions.

The pinned original d3-geo 3.1.1 reference covers 9 scale/translation states,
5,778 forward source-point records, 792 inverse records and 414 region-specific
inverse records, including each child's window and visibility. The final kernel
suite passed 6 minimal/7 all-feature tests. Forward clipping uses the original
1e-6-final-pixel inset and exact window comparisons; no forward clip tolerance
was added. Official serde_json float_roundtrip preserves the source binary64
values; no custom JSON lexer remains.

Inverse admission changed explicitly during development. The final contract
checks the requested pixel's exact selected-region rectangle, checked inverse
admission, the native child representative and an unclipped same-region residual.
A recovered source may round one ULP outside strict forward visibility at an
exact edge; forward point-stream visibility is therefore not a second inverse
domain check. The original D3 inverse remains an unrestricted algebraic result;
Rust's input-domain admission is a derived policy, not a claim that arbitrary
far-away native inverse pixels are accepted. Schema 2 revises 44 derived strict
expectations from None to a finite inverse whose candidate forward stream is
not visible. Native forward/native inverse/region-inverse records, scales,
translations and clip windows remain unchanged against the retained v1 data.
The policy change is recorded in target/geo-usa-inverse-policy-change.json;
target/geo-usa-kernel-v1-reference/ and the failed intermediate kernel logs are
retained. Old-policy failures are not relabelled as passes of that old contract.

The rectangle clipper uses native stream buffering, planar line clipping,
boundary ordering and contour rejoining. It preserves native winding, closing
duplicates and degenerate contacts, without assigning exterior/hole roles from
list position. Seven minimal/eight all-feature tests cover concave splitting,
holes through the window, whole-window coverage/removal, independent reentry,
coincident edges, point contacts, finite arithmetic and resource limits. Its
383-case original D3 differential includes the initial 63 cases plus 128 simple
star-shaped rings, 128 folding open lines and 64 two-hole profiles. Contour and
point counts/order agree, with each coordinate difference below 1e-8. The initial
63-case reference and acceptance remain separately retained; expanding the
reference required no production or comparison-threshold changes.

An independent audit reproduced a work-budget bypass: empty input line headers
previously consumed no work. The failing test is retained at
target/geo-rect-clip-empty-header-before.log. Per-line/per-ring and winding-ring
headers now consume work, while point-count semantics remain unchanged. Final
native output points are bounded by max_points, intermediate buffering by twice
that allowance, and scanning/copy/sort/traversal by max_work. Finite crossing
endpoints follow D3's +/-1e9 numerical clamp; unrepresentable polygon winding
arithmetic returns an error. Expanded acceptance is recorded in
target/geo-rect-clip-expanded-acceptance.json.

The final combined regression passed 1,526 minimal tests, 1,804 all-feature
tests and 17 doctests. All five commands in
 target/geo-usa-primitives-final-validation/state.json exited zero, including
native all-target strict Clippy and Rust 1.95 WASM strict Clippy. The focused kernel and rectangle suites also passed their owned-source
formatting checks.

The executable geo_usa_primitives exports 10 diagnostic frames: six polygon
clip cases, one open-line reentry case and three manually authored regional
circle/rectangle-clip cases. Its 37 authored probes check Scene hit results and
CPU samples. Chrome, Firefox, WebKit and Edge at DPR1/2 passed 8 contexts, each
comparing 74 actual SVG-image/Canvas samples against CPU RGBA within one channel
unit: 592 comparisons total. A distinct bundled Chromium profile passed 2
contexts and 148 comparisons. These are sample checks, not whole-image equality.
The three regional circles are manual Mark.clip diagnostics using kernel
roundtrip coordinates; they are **not GeoSymbol/glyph/label pipeline acceptance**.
There is no ordinary text, physical-device or integrated geographic host-input
claim in this browser result.

Fresh browser reports and exports: target/geo-usa-primitives-final/.
Full regression logs: target/geo-usa-primitives-final-validation/.
See [English integration guide](geo-albers-usa.md) and
[中文说明](geo-albers-usa.zh-CN.md) for the remaining consumers and completion
conditions. GPU/worker and the broader feature-gap list remain deferred/open.


## Development Albers USA fitted maps and region overlays

This batch connects the earlier primitives to dedicated `GeoUsaMapOptions` map
entries and the shared fitted context. It remains Unreleased. The full Albers
USA feature is **not complete**: component line/graticule ownership and browser
host runtime/editable-label acceptance remain open. Older foundation-only
receipts above retain their original scope and are not retroactively relabeled.

Every source polygon streams through three continuous conic children and actual
rectangle postclipping. World/Data fitting retains one canonical `map:{id}` mark
per feature and one color domain. Data fitting uses native temporary scale 150
and rebuilds final pixel-epsilon windows. Aggregate sampling/output/work budgets
span all components and both fitting/final passes. Five minimal/six all-feature
map tests include exact aggregate budget admission and one-less rejection, holes,
three-region ownership and actual CPU polygon coverage.

The independent original d3-geo 3.1.1 producer records 24 configurations, 46
feature states and 60 contours, including same-feature three-region geometry,
Alaska antimeridian geometry, concave splits, boundary-crossing holes and
invisible source values. All 32,912 coverage queries remain in the fixture:
32,890 actual Scene hit assertions pass, with 22 queries excluded by the fixed
0.05-pixel native-boundary band established before the first Rust run. Another
192 point projection assertions pass. World affine fitting, Data temporary/final
fit, material contour signs, signed areas and bounds are checked independently.
Adaptive algorithms differ; tests do not compare sampled vertex count/order.
Measured maximum relative scale difference is 0.00003915420443079132, raw-center
difference 0.00004069229700653931, and normalized native-frame bounds difference
0.023803136888318477 pixels, within the predefined two-sampler precision bound.
The first Rust harness omitted 3,366 invisible-feature empty-Scene queries from
its cumulative count. The retained first log fails that count; the corrected
harness executes those queries. No production implementation, native reference
or tolerance changed to make this result pass. Original producer hashes,
source inventory and ISC notice accompany `tests/fixtures/geo-usa-map`.

Six overlay tests cover all 14 real glyph shapes, independent plot/region and
location-label masks, original geometry/strokes/anchors, nullable values, hidden
rows, stable palettes, shared choropleth domains and atomic failures. A selected
component rectangle masks full ink; a union footprint is not used. The old
line/graticule entry explicitly rejects composite contexts, and a single conic
footprint rejects rather than silently masking them as Albers.

Three raster-host tests include four real Glory memory mounts with 12 data
controls, World/Data fits, DPR1/2 and identity/1.15 zoom cameras. They check 24
actual CPU glyph pixels and 48 host-coordinate cases in offset logical/physical
bounds, including general, default-region and known-region inverses plus typed
anchors/raw values. Stop removes the controls. A separately registered Lato
fixture proves that Hawaii label clipping removes actual red CPU text at both
DPRs even though the broad Lower48 rectangle contains it. Browser region-query
methods compile for WASM; these native observations are not browser-host runtime
acceptance.

The `geo_usa_maps` exporter produces 20 actual World/Data polygon, symbol and
glyph frames with 168 authored probes and actual Scene hit assertions. Chrome,
Firefox, WebKit and Edge at DPR1/2 pass eight contexts with 336 SVG-image/Canvas
versus CPU RGBA comparisons each (2,688 total), retaining a one-channel-unit
threshold. A distinct bundled Chromium profile passes two contexts/672
comparisons. These are flat-interior sample checks, not complete image equality,
editable text or physical-device acceptance. A fixed-radius first fixture had
no visible Data-fit spill; later fixtures scale the radius with the fitted
projection. Earlier probes near overlapping fill/stroke antialiasing produced a
two-channel cross-engine difference and remain failed evidence. Final probes
require 12-pixel fill-interior clearance and four-pixel clip-edge clearance;
exported geometry/strokes and comparison thresholds are unchanged. The tighter
clearance changes the recorded sample count from 180 to 168 per frame set; the
CI checker requires the new exact count. No claim is made that the earlier edge
samples now match.

Exports/reports: `target/geo-usa-maps-v5/`. First fixture, browser, harness and
Clippy failures are retained under distinct log names. The gallery CI definition
includes this export and Chromium profile; no remote run or release is claimed.
The prior published 0.1.0 and its release tag remain unchanged.


Final combined regression: **1,537 minimal tests, 1,821 all-feature tests and
17 doctests**, all passed. Native all-target strict Clippy, Rust 1.95 WASM
strict Clippy, pinned-nightly package formatting, typos and diff checks pass.
Fresh command logs and the scope-specific sealed acceptance are in
`target/geo-usa-maps-final-validation/`. The first package-format check required
splitting an oracle test import according to the repository configuration;
the import-only correction changes no native values or test semantics.


## Development Albers USA streams and host integration

This Unreleased batch implements the previously pending dedicated USA line and
graticule entries and verifies the browser input/editable-label consumers. The
bounded composite profile is implemented; arbitrary spherical topology and
bit-identical degenerate D3 contacts are not claimed. Published 0.1.0 and its tag
remain unchanged. Earlier stage receipts keep their original narrower scope.

Lines reuse the accepted polygon fit and emit independent continuous child
paths followed by pixel rectangle clipping. Each visible region has real ink;
one source has one canonical real control and direct physical aliases. The
anchor is its first visible Move endpoint. Source indexes, nullable values and
raw source_series are retained; empty grouping/label strings have documented ID
fallbacks. Shared source/coarse/adaptive/postclip budgets, explicit order,
noninteractive graticules and atomic rejection have dedicated tests. The old
single-projection line signatures still reject composite contexts.

A real Memory-host audit found that generic Mark.clip_path masks were rejected
by Glory Graphic, even when rectangular. Default USA lines and glyphs now use
native Mark.clip, with the exact region/plot rectangle intersection when both
are requested. Empty intersections remain empty. The original full geometry,
stroke and caps are retained. Default-mask Memory tests pass; turning projection
clipping off is not used as a substitute. Fourteen real glyph shapes mount with
42 actual Glory controls and independently clipped editable labels.

The same audit found duplicate numeric line labels and a missing-value tooltip
that read an unrelated live scalar (-913). Line names and formatted values are
now separate. Bound Glory default tooltips read ActiveInteraction source rows.
Bound layout metadata changes invalidate the readout even when Scene and the
public controls_scene projection are completely unchanged: a stationary-pointer
None to Some(0)/accepted-zero test proves this. Custom finite-Mark formatters
keep their existing API. Default-mask Memory World/Data and Raster World/Data
at DPR1/2 pass actual plot input, three-region alias ownership, nullable source
updates and stop cleanup. Four native publisher rejection transactions retain
the original PNG Rc, source/binding, revision, controls and tree; 24 real native
line-pixel assertions pass. The first failures for these production bugs are
retained, including the temporary mask-disabled diagnostic run; that run is not
the final acceptance profile.

The original d3-geo 3.1.1 line/graticule producer preserves six fitted map frames,
132 source states and 396 region states. All 156 material components match;
6,520 bidirectional vertex/midpoint-to-segment probes and 18,002 fixed-tube
coverage queries pass. Three fixed-band queries remain recorded exclusions.
Maximum geometric distance is 0.021113654106 pixels and bounds difference
0.017248094840 pixels. Comparison limits were fixed before the first Rust run,
including native endpoint/clip-angle amplification (maximum limit
0.081709729631 pixels). Sampled vertex count/order is not an equality contract.
Fifteen original major/minor graticule source lines are compared separately.

The original composite really interleaves child lineStart events in six
cross-inset cases (maxActive 2); full events, source cases, child records and
native/child point multiset equality are retained. Region geometry is compared
to the original independent child streams, not a fictitious flattened no-bridge
composite. Three roughly 1e-13-pixel corner-contact differences remain explicit
diagnostics: native has five short contacts, Rust four. This is not a claim of
identical degenerate cap paint at those discontinuous boundaries. The initial
native producer could not honestly parse interleaved events as a flat stream;
its first failure remains. Before the first Rust run, source gaps were expressed
as separate valid components rather than an invalid empty component; the
pre-domain native fixture is also retained. No native geometry tolerance was
changed to make the Rust comparison pass.

The geo_usa_streams exporter passes 37 actual frames: three regions, three cap
styles, solid/dashed paint, mask on/off, plus graticules. Its 240 probes verify
physical ink, including original cap spill, at CPU DPR1/2. Chrome, Firefox,
WebKit and Edge at DPR1/2 pass 480 SVG-image/Canvas comparisons each: 3,840
comparisons, one channel-unit tolerance. A separate bundled Chromium profile
passes 960 comparisons. Probes have three-pixel physical-ink interior and
four-pixel clip-edge clearance; each graticule part must independently be
uniform inside/outside the probe neighborhood. Initial legacy hit-testing
admitted invisible butt-cap space; final picking uses PaintedGeometry. An early
crossing-stroke AA sample differed in Firefox; it remains failed evidence.
Probe clearance was strengthened, not the RGBA threshold. Fresh native-rect
exports and both reports are in target/geo-usa-streams-native-rect/.

The actual WASM BrowserChart suite passes Chrome/Firefox/WebKit/Edge at DPR1/2:
224 successful native-pointer inverse observations, 64 wrong-region/outside
observations, 16 wheel/pan gesture pairs, 16 CSS-transformed observations, and
80 trusted keyboard activations. D3 comparisons use the actual received event
coordinates and actual paint-root CSS bounds; API/Scene/native-inverse checks
remain at 1e-7. Requested pixels are rounded explicitly; the installed Firefox
driver's flooring and native coordinate representation are separately bounded
and recorded, rather than treated as exact subpixel pointer delivery. Known
region and default inverse disagree in a deliberately overlapping interior
case, and both match the original child/default D3 inverse. None behavior does
not fall back to another component. Source controls and raw values survive
accepted line-layout replacement, and invalid layout proposals retain the
accepted snapshot, paint and control node in the same event-loop observation.
Ninety-six actual browser line-part probes resolve three physical parts to one
canonical control, including nullable to 42 updates; unbound graticules create
no controls.

Thirty-two editable-label observations retain the actual SVG text node and
record actual Canvas fillText calls. Unmasked text has real magenta pixels;
selected-region clipping removes them at both DPRs in each engine. No outlined
text surrogate is used for this browser claim. Stop removes the roots and the
inverse fixture releases its weak callbacks/handles. First harness failures are
retained: exact intended-subpixel input was too strong, a prefix selector also
counted the oracle iframe, an ambiguous point lay too close to an inset edge,
protocol bounding boxes had different rounding from native DOMRect, and a
native rectangle appeared as nested overflow-hidden SVG rather than clip-path.
Corrected checks use the authoritative native event/DOM and still require exact
API behavior. Browser reports: target/geo-usa-host-final/report.json; separate
bundled Chromium: target/geo-usa-host-ci-profile-final/report.json. Pixel checks
remain samples, not whole-image equality or physical-device acceptance.

The earlier 20 map/symbol/glyph frames were rebuilt after the native-rectangle
fix: all eight browser contexts and 2,688 comparisons pass again in
 target/geo-usa-maps-native-rect/. The library regression passes **1,549 minimal,
1,838 all-feature and 17 doctests**, with strict native and Rust 1.95 WASM Clippy.
The independent browser-host workspace also passes Rust 1.95 WASM Clippy and
pinned package formatting. Root pinned formatting, typos and diff checks pass.
A format-only split of an oracle test import follows repository configuration;
no oracle values or test semantics change. Full logs and the sealed batch
receipt are in target/geo-usa-streams-final-validation/.

CI now builds the host WASM with pinned matching wasm-bindgen CLI and runs the
bundled Chromium host/label/control suite, alongside the route/graticule export
suite. Both CI profiles were executed locally; no remote run is claimed. The
broader chart feature-gap list and deferred GPU/worker work remain open.

## Development repeated-domain scales and consumers

This Unreleased increment accepts non-strictly monotonic numeric/UTC domains,
including two-stop and multi-stop constant domains, and keeps the checked
rejection of true turns, distinct transformed collapse and inverse ambiguity.
The original d3-scale 4.0.2/d3-zoom 3.0.0 producer preserves 9,496 scale profiles:
8,960 accepted, 536 checked rejections, 120,176 map queries, 109,024 global inverse
queries and 113,264 explicit branch inverse queries. Original IEEE special
values, signed zero, near-knot neighbors, full licenses and source hashes are
retained in tests/fixtures/piecewise-repeated/.

The independent native viewport test executes all 4,128 original records through
both real Line and Scatter layouts, full-stop helper reflow and original-controller
reset. Each rejects 1,216 constant displayed baselines and eight native true-turn
proposals. Line accepts 2,904 reflows; Scatter accepts 2,848 and rejects 56 native
point-center proposals beyond Scene coordinate budgets. Both preserve 872
fully constant windows and reset through their original controller. Four extra
unbounded requests check the authored inverse clamp. Actual Glory Memory pen
events, semantic controls, invalid-binding rollback and CPU pixels at DPR 1/2
are covered separately. The earlier 2,400 strict profiles and 432 original
viewport records remain passing.

First failures are preserved: target/piecewise-repeated-rust-first.log records
the SymLog transformed-neighbor branch jump; the all-portable-pow candidate
failed separately and was not adopted. Consumer failures exposed projected
edge-site rejection, constant-line splitting and the wrong area baseline.
The first and corrected-contract viewport runs retained 420 and 228 diagnostic
differences; local pixel inverse arithmetic reduced these to 32, then removing
the second raw-domain clamp preserved the native weighted endpoint values and
resolved the remaining cases. No native values or numeric tolerance were
weakened. The 192 test-contract corrections preserved the pre-existing raw-domain
admission rule for nice-expanded profiles that no longer have repeated stops.

The complete main numerical matrix also executes through actual compiled WASM.
The first Chromium run retained 108 quarter-power differences in 72 profiles,
including a 26.5-pixel branch jump; the native-Windows pass did not mask them.
Web general signed powers now use the actual JavaScript host's Math.pow. The
unchanged reference and comparison policy then pass all 9,496 cases, 106,208
checked map queries, 102,656 global inverses, 106,240 branch inverses and 43,392
ticks, with no excluded operations or one-ULP rounding exceptions. Both full
reports and 75 actual result packets per run are retained in
target/piecewise-repeated-wasm-chromium-first/ and
target/piecewise-repeated-wasm-browser-power/. This numerical acceptance covers
one bundled Chromium context; it does not establish chart paint in all engines.

Browser paint remains a separate acceptance boundary while this section is
being completed. The first complete sequence exceeded the UTC SVG/Canvas paint
gate. An initial image-preview interpretation suggested missing data ink;
direct native readback and PNG pixel comparison corrected that diagnosis:
the data ink is present and equal, while differences concentrate in axis text.
PNGs, exact SVG and accepted states are preserved in
target/piecewise-repeated-host-utc-state/, with independent native-call traces.
The missing inherited language in the detached SVG sample caused different
font selection: the live date text measured 61.390625 pixels, while the detached
sample measured 66.6796875. Preserving the actual inherited lang/xml:lang alone
reduced that frame's mean from 0.56980957 to zero. No Canvas production path,
date label, reference value or paint threshold was changed to resolve this.
The diagnostic receipt is target/piecewise-language-capture-review.json.

The complete final chart suite passes Chrome, Firefox, WebKit and Edge at DPR
1/2: eight contexts, 288 paired SVG/Canvas frames, 256 trusted pointer/keyboard
pick pairs, 96 paired publisher rejection cases, and 16 constant-window/reset
cases. Every returned site is compared to original D3 geometry; actual paint
readback remains below the unchanged whole-frame mean gate of 0.5, with observed
maximum 0.05610546875. Rejected callbacks preserve accepted domains, sites,
selection, controller and exact SVG/Canvas output; explicit-flush revision
telemetry matches a real idle-flush control. Reports preserve those deltas,
rather than claiming an unchanged paint counter. See
target/piecewise-repeated-host-final/report.json. The separate bundled Chromium
CI profile passes both DPR contexts with the same script and WASM hashes in
target/piecewise-repeated-host-language/report.json.

Full regression passes 1,561 minimal tests, 1,854 all-feature tests and 17
doctests, plus strict native and Rust 1.95 WASM Clippy. Final logs are retained
in target/piecewise-repeated-final-validation/. The independent browser-host
and direct Canvas example workspaces also pass Rust 1.95 WASM Clippy. Root
formatting, bilingual documentation spelling, release-gate unit tests and
workflow YAML checks pass. CI runs the frozen WASM numeric matrix and paired
SVG/Canvas host suite using locked root npm reference dependencies; these CI
profiles were executed locally, with no new remote run claimed.

These results do not turn earlier strict compositor or ordinary text-wrap
failures into passes. Signed-power transcendental behavior retains a platform
boundary rather than an all-engine bit identity claim. Unreleased browser
builds with canvas (including web) use host Math.pow; native and standalone
WASM core builds retain platform powf. GPU/worker and the wider feature-gap
list remain deferred or open as already recorded.

## Development custom Swarm nodes and Network links

This Unreleased increment implements checked Scene geometry hooks for Nivo
0.99's Swarm SVG circleComponent/Canvas renderCircle and Network
linkComponent/renderLink roles. Borrowed source data, original physical layout
and actual host lifetimes remain distinct. It does not execute arbitrary React
components, live Glory child Widgets or an equivalent force solver trajectory.

The independent reference retains 16 complete Nivo source/package files and
the original MIT notice in tests/fixtures/node-link-custom/vendor/nivo/.
The producer executes original renderer bodies, layer loops, transitions and
source-preparation fragments on actual Chromium Canvas2D: ten Network cases,
six Swarm cases, 34 link calls, 14 node calls, six original ordinal-scale cases
and 42 pixel witnesses. Requested versus applied Canvas paint is recorded;
the native zero-lineWidth ignored-assignment behavior and Canvas/SVG line-cap
difference are preserved as explicit profiles rather than copied into Rust's
existing round-cap defaults. Four independent Rust reference tests pass with
minimal+geojson and all features. No Nivo force positions are used as a fake
oracle for Kartis's deterministic packing or application-owned simulation.

Swarm custom factories preserve full default Scenes, metadata and bindings.
Callbacks follow packed drawing order and preserve hidden/cull/zero/transparent
eligibility, original values, packing discs and logical origins. Geometry
Exact uses replacement contours; Nearest retains original centers and the
host's existing eligibility policy. Real Glory Memory tests exercise pen
events, source replacement, stable controls and invalid input rollback. Native
RasterSurface tests add actual gradient pixels at DPR 1/2, failed image
publication retaining the prior PNG Rc, model, controls and tree, then accepted
replacement and stop. Original zero-size semantic controls are retained; no
fake ink or extra source is introduced.

The final Swarm browser report is
target/swarm-custom-browser-final-artifact/report.json. Chrome, Firefox, WebKit
and Edge at DPR 1/2 pass all eight contexts: 1,088 exported SVG/Canvas/CPU and
2,688 live-host RGBA samples within the original one-channel limit, 160 host
paint frames, 64 trusted anchor/offset/zero pick groups, 64 keyboard activations
and 128 per-renderer factory-error/invalid-geometry rejection receipts. Every
failed synchronous candidate retains revision, pending state, source, actual
SVG/Canvas pixels, controls and root style in the same event loop. Later RAF
paint counters are recorded separately, with no-op controls, while accepted
source/paint/controls remain strict. Earlier fixed-delta assumptions and their
failed reports are preserved. Source updates add nodes and resize to 480 pixels.

Network borrowed link callbacks see current source/target force states and
Scene endpoints. Self/parallel/zero-weight links retain independent source
indices; explicit relation IDs retain graph ownership and authored paint.
All custom links and node glyphs share final Scene command/gradient budgets.
The retained BrowserNetwork factory rebuilds for tick, physical dragging,
resize and source replacement. It supplies typed node data, while links remain
decoration and are excluded from force-node dragging and fake node controls.
The explicit painted adapter is verified through actual Glory pen input and
CPU raster pixels at both DPRs.

An independent audit reproduced a real stop-during-replacement-preflight
panic_already_borrowed failure. The factory had executed while core.state was
borrowed. The fix releases state before calling user code, checks liveness
afterward and re-acquires state for admission. The original failure is retained
in target/network-link-stop-preflight-audit.{json,log}; six independent fresh
Chromium scenarios at DPR 1/2 pass in
target/network-link-stop-preflight-fixed-audit/. Stopping the target drops its
callback once and leaves the other renderer active. Nested replacement returns
a checked error; accepted-state reads and callback-owned stop do not panic.

The final Network report is target/network-link-host-final-artifact/report.json.
All eight named-browser/DPR contexts pass 40 paired SVG/Canvas paint frames,
16 trusted node drag/keyboard cases under nonuniform CSS scale, 24 paired
factory-error/invalid-geometry/late-replacement rejections and 32 reentry/stop
checks. Actual received client events, captured before the driver's Window
listeners consume them, determine expected world displacements. Serialized SVG
geometry is independently checked against current accepted node positions.
Whole-frame mean remains below the unchanged 0.5 gate, with observed maximum
0.05151258680555556. Source ticks move nodes, accepted replacement changes
positions/labels/weights, resize rebuilds at 520 pixels, and stop releases the
retained factory both during ticks and replacement preflight. Earlier document
capture missed Window-consumed events; that harness failure was preserved and
corrected without modifying input or product behavior.

Full regression passes 1,577 minimal tests, 1,876 all-feature tests and 17
doctests, plus strict native and Rust 1.95 WASM Clippy. The browser-host workspace
also passes Rust 1.95 WASM Clippy. Existing Network glyph/drag browser suites
pass independently in Chromium. Fresh locked Swarm exports reproduce all 26
prior SVG/PNG/HTML/metadata files byte for byte. CI now checks the original
renderer reference, generates exports and runs both bundled-Chromium host
suites; their separate two-DPR CI profiles pass with the same final scripts
and WASM artifact. Reports are in target/node-link-custom-final-validation/,
target/swarm-custom-ci-profile/ and target/network-link-host-ci-profile/.
No new remote CI run or published-version change is claimed. Wider Widget,
text/compositor, device and deferred GPU/worker gaps remain open.

## Development typed Bullet Widget overlays

This is an Unreleased API checkpoint. `PlotWidgetsData<T>` carries measured
slots and owned metadata together; `TypedPlotWidgetFrame<T>` derives from the
same accepted row. Existing public untyped struct literals and APIs remain
unchanged. Metadata value equality can trigger an update without remounting a
child. Typed browser mounts retain their metadata TypeId, including empty
packets. Missing/extra metadata keys and duplicate slots are rejected.

`bullet_part_widgets` adapts actual range, measure and marker marks into these
packets. Original source IDs, raw and visible intervals, synthetic-domain
indices, zero/cropped rank accounting and current row/domain/track snapshots
remain explicit. Constant-domain marker-only layouts are included. Widgets are
overlays above the original ink; Scene exports omit their content. Nivo Bullet
is an SVG React component; this checkpoint does not claim React spring props,
ink replacement, Nivo component identity or a Bullet Canvas port.

Independent Memory tests pass two minimal / three all-feature cases, covering
metadata-only updates, current Scene/data, retained NodeId and input state,
reorder, invalid Scene/placement/metadata packets, factory errors, a real
mid-build Widget error, cleanup and raster PNG publication failure at DPR 1/2.
Rejection retains the accepted image, tree and typed data. Failed child revisions
retain the old child while it reads the new accepted frame and metadata, as the
existing local child protocol specifies. No whole-Scene rollback is claimed for
child-construction errors. Typed Bonds are owned by the candidate revision's
child scope so failed and replaced revisions retire those Bonds.

Chrome, Firefox, WebKit and Edge at DPR 1/2 pass eight contexts with 48 paired
SVG/Canvas frames and 96 actual renderer-specific rejected candidates (48 per
renderer). The fixtures use real Glory buttons for all three Bullet roles and
exercise trusted activation/keyboard input, retained child state and focus,
row reorder/current values, metadata-only updates, hidden state, resize,
nonuniform CSS scaling, explicit failed/successful revisions, typed empty
packets, cleanup and remount. Actual known range and measure RGBA patches are
checked in every Canvas frame; 96 slot rectangles have x/y/width/height checked
against an independent source-layout formula after CSS scaling. Paired paint
means remain below the unchanged 0.5 gate. These checks are not an independent
Nivo pixel or React lifecycle oracle.

Earlier harness reports remain available: the first used a fixed CSS width
instead of the responsive Scene-to-box ratio; the second sampled text before a
Glory flush. An independent audit then found that a shared invalid-update loop
returned after the SVG error and had not reached the Canvas API. The final
driver addresses each renderer separately, verifies the corresponding WASM
export exists and excludes JS TypeError/WASM traps from expected rejections.
The final scripts and fixtures pass in the four-browser report and in an
independently rerun bundled-Chromium CI profile. The final records are
target/typed-bullet-browser-final-checked/report.json and
target/typed-bullet-browser-independent-final-checked-ci/report.json.

Full regression passes 1,583 minimal tests, 1,883 all-feature tests and 18
doctests, including the executable Bullet API example. Strict native and Rust
1.95 WASM Clippy pass for the library, and the independent browser-host workspace
passes Rust 1.95 WASM Clippy. The original untyped browser slot suite passes
against the final artifact. Both typed-slot guides have their complete Rust
examples extracted, compiled and run in a real Memory App. Pinned formatting,
hidden-file spelling checks, workflow YAML and eleven release helper tests
also pass. CI adds the bundled-Chromium typed Bullet suite and preserves its
reports; no new remote CI run or publication is claimed.

The combined bounded-feature receipt is
target/typed-bullet-final-validation/acceptance.json. More chart-specific roles,
Widget ink replacement/export, text/compositor and device gaps remain open;
GPU/worker work remains deferred.

## Development transactional Widget composition

This Unreleased checkpoint adds `PlotWidgetsComposition<T>` and the terminal
`Chart::plot_widgets_composed` builder, plus composed BrowserChart mount/set
methods. `PlotWidgetCompositionContext<T>` explicitly describes tentative child
construction; its Bond becomes live only after commit. Existing overlay APIs
retain their separate local revision-failure behavior. No published tag changes.

Candidate children build in hidden, inert shells before paint publication.
Retained children keep their own preceding accepted snapshots during staging.
Factory/build, source, placement or publication rejection drops the candidate
scopes and preserves the preceding accepted output/data. Success adopts the
staged children and updates retained Cages. Epoch and reentry checks prevent
stopped or stale stages from reviving children. Up to 128 active measured slots
and 256 pending new children are admitted; shared stage leases release their
reservation once on commit/drop. Merely reusing an accepted revision does not
consume pending-child budget.

Only requested physical Mark ink is omitted from the host paint projection.
Draw indices and opacity-group membership are remapped; all original labels,
typography, axis/title text, source data and canonical controls remain. Physical
pointer/brush/touch picking excludes the replaced ink, while other visible ink
aliases and explicit logical selection/crosshair remain available. A zero source
opacity group hides and disables the slot while retaining state. Widget styling
does not automatically inherit every Scene opacity/blend operation. Composition
stays above the Scene, so it does not reproduce Nivo Bullet's component ordering.
Authored Scene exports retain authored marks and omit live Widget content;
filtered browser/raster base images omit replaced ink but also omit Widgets.

Seven core tests cover actual Scene draw-order/group/label/font/clip retention,
bound/unbound geometry/nearest/slice cohorts with camera transforms, physical
aliases, source token invalidation, rejected unsupported source profiles, both
brush controllers, touch, real Radar polar slices and source retirement.
Native composition tests cover actual semantic Graphic suppression and restored
ink, retained state/data, finite PlotPointer positive/negative controls,
factory/build rejection with exact live-tree identity, PNG refusal at DPR 1/2,
initial child failure before any PNG publisher call and group-zero visibility.
The staging-budget unit verifies that quota rejection precedes another factory
call and that dropping/committing shared leases restores capacity.

The final artifact passes Chrome, Firefox, WebKit and Edge at DPR 1/2: eight
contexts, 88 paired SVG/Canvas frames, 112 renderer-specific candidate rejections
and eight injected Canvas publication rejections. Every frame checks known
Canvas colors and exact source SVG ID omission/restoration, with paired mean
below the unchanged 0.5 gate (observed maximum 0.050012731481481484). Actual
trusted activation/keyboard input retains child state and DOM focus across
source reorder. Restored original ink is a positive pointer control before
checking the replaced position has no ghost hit. CSS and camera rectangles use
independent source-layout formulas; hole masks make actual controls inert and
ignore trusted clicks. New-key factory/build rejection cannot swallow existing
ink. Each renderer queues 42 six-child revisions before the 256-child quota
rejects further staging, then drains, cleans up while reading weak handles and
successfully retries. Initial rejection, stop-during-build and remount also pass;
JS TypeError/WASM traps are excluded from expected rejection handling.

The final four-browser report is
target/widget-composition-browser-final-artifact/report.json. The independently
rerun bundled-Chromium CI profile and mutated-harness negative controls are
linked from target/widget-composition-final-validation/acceptance.json.
The independent final CI profile passes 22 frames and 30 rejection records.
Three intentionally broken copies of the final harness fail as expected:
retaining a transparent source SVG ID, disabling all pointer movement before
the positive hit control, and throwing a WASM RuntimeError during the stop
case. These are validator negative controls, not product failures; their
original scripts/logs remain in
target/widget-composition-harness-negative-controls-final/receipt.json.
Original ordinary slot and typed overlay browser suites pass on the final
artifact. Full regression passes 1,595 minimal tests, 1,897 all-feature tests and
18 doctests, with strict native and Rust 1.95 WASM Clippy; the browser-host
workspace also passes Rust 1.95 WASM Clippy. Complete examples extracted from
both composition guides compile and run in a real Memory App. Formatting,
hidden-file spelling, workflow YAML and release-helper checks are recorded in
the combined receipt. CI adds the composed host suite and uploads its reports;
no new remote CI run or release is claimed.

Earlier failures remain recorded: a fixture omitted the stopped-handle snapshot
guard, native tests compared historical Memory debug counters instead of the
live semantic tree, and a group-opacity fixture named nonconsecutive members
which the source validator correctly rejected. The last fixture now uses a
valid consecutive source group. Host Clippy also caught duplicate fixture-module
loading and an eight-argument WASM helper; both were corrected without lint
allowances. Broader in-layer composition, live Widget Scene export, other chart
roles, text/compositor and device work remain open; GPU/worker stays deferred.


## Development Scene-order live Widgets

The browser-only `mount_with_plot_widgets_in_scene` and packet/layout setters
interleave actual Glory component roots with transparent SVG/Canvas source
runs. The shared checked `widget_paint_plan` preserves authored Mark/Label
order, required font/clip declarations and once-only enclosing group opacity.
Non-Normal source runs use CSS blending against earlier source and Widget
content. Retained components move across registered parents before old parents
are disposed. The native semantic/CPU top-layer API keeps its existing scope;
precise native same-layer input is not claimed.

The frozen browser artifact is tested in Chrome, Firefox, WebKit and Edge at
DPR 1/2: eight contexts, 256 actual composited screenshots, 320 trusted native
pointer/hover checks, 128 rejection records and 80 lifecycle records. Source
order reversals, enclosing group alpha, semi/zero alpha, EvenOdd holes, dash
gaps, source clips, actual text, CSS stretching, camera changes, hidden/group0
state, clear/restore and replacement picking are included. Same-ID/revision
actual DOM identity and local state survive order/group moves; real focus and
Enter activation also survive. Surfaces remain accessible groups with two
actual buttons in the normal fixture; visual source planes are aria-hidden
rather than repeating an image title for each run.

The input contract uses Canvas alpha-positive pixels, including for SVG output;
it does not promise exact SVG subpixel coverage. Label probes use independent
native Canvas alpha-positive interiors intersected with actual solid SVG
screenshot interiors. Every final native input record retains requested and
trusted received coordinates. Independent native DOM probes reproduced
Firefox transport quantization: requested (127.25, 88.25) arrived near
(126.85, 88.5), outside a path whose x interval began at 127. The same requested
`elementFromPoint` correctly hit the path. Original first failures and native
probe reports remain in target/ordered-label-native-hit-probe/ and
target/ordered-label-native-input-metrics.json. The final harness verifies the
received point against both the actual alpha mask and the independent glyph
footprint; it does not expand production masks or retry until a hit passes.

Independent native HTML/SVG/Canvas primitive controls establish each engine's
group and semi-alpha screen colors. Chrome and WebKit can round an SVG
composition differently from a Canvas composition. The original mathematical
color failures are retained. Final comparisons keep a one-channel tolerance,
exact source opacity 0.5 and once-only group alpha; expected screen colors come
from separately authored native primitives, not Kartis output.

Factory/build/model failures reject complete candidates. A one-shot exception
from the third native `getContext` rejects a partially prepared renderer tree
while preserving actual screen PNG bytes, backing stores, canonical controls,
accepted source revision, typed data, child identity and focus. A separate idle
flush control accounts for unrelated renderer revision increments. Nested
proposals, stop during build, failed initial mounts, scoped cleanup and pending
child exhaustion/recovery are exercised. JS TypeError and WASM RuntimeError
are never accepted as ordinary business rejection. Dynamic child-binding errors
observed after commit remain ordinary UI errors, as documented.

Pure plan tests pass nine cases in both feature profiles; exact alpha rectangle
partition tests pass six, including randomized union/disjointness, budgets and
overflow. Five private placement tests cover retained owners, third-move
rollback, suspended pending siblings and quotas. Full regression passes 1,614
minimal tests, 1,916 all-feature tests and 18 doctests. Native and Rust 1.95 WASM
Clippy, the independent browser-host WASM workspace, pinned formatting,
hidden-file spelling, workflow YAML and 11 release-helper checks pass. The
previous top-layer composition and typed Bullet Widget Chromium suites also
pass on the final artifact.

The final four-browser report is
 target/ordered-widgets-browser-final-provenance/report.json.
Startup/end hashes bind the test script, WASM and JS glue without modification
during a run. The combined source inventory and independent CI/harness-negative
receipts are linked from target/ordered-widgets-final-validation/acceptance.json.
The three negative controls disable the hit companion, incorrectly place all
Widget anchors on top, and throw a RuntimeError after a real stop. Each must
fail its relevant gate; earlier ineffective namespace-based sentinel injections
remain recorded as harness mistakes, not successful negative controls.
CI runs the focused Chromium profile and uploads its reports. No new remote CI
run, tag or package publication is claimed. These development APIs are absent
from the published 0.1.0 package. Native same-layer input, arbitrary live Widget
Scene export, automatic springs, broader text/device work and GPU/worker remain
open. See the [English guide](ordered-widgets.md) and
[中文指南](ordered-widgets.zh-CN.md).


## Development cell Widgets and complete Chrome host CLI

Unreleased `heatmap_cell_widgets` and `waffle_cell_widgets` supply real Glory
components for Nivo's cellComponent roles. These adapters do not change their
layout algorithms or promise React animatedProps, automatic springs or arbitrary
live-component export. Original scalar/null values, original HeatMap input
indices, Waffle physical slot identity and owned current metadata stay distinct
from formatted captions and actual decorated Mark paint. Same-group Waffle
snapshots share an Arc. Both adapters preflight their complete layout before any
placement callback. Explicit typed packets can attach to existing decorative
marks, including empty Waffle cells, without fabricating source labels, sites
or canonical controls; plain slot eligibility is unchanged.

Native tests cover authentic nullable source, missing sparse pairs, input/order
changes, current custom paint and geometry, late invalid metadata before any
callback, hidden/zero-ink cells and the selected-slot budget. Actual Memory
buttons preserve local state and typed data across accepted updates and reject
whole failed revisions. Native semantic HeatMap input uses the explicit,
preflighted solid PaintedGeometry conversion. Real DPR1/2 CPU image publication
failures preserve earlier PNG pixels and child data. This raster_chart path is
Scene-only and does not claim a new bound pointer API; nullable image examples
use actual child controls rather than inventing values for old scalar controls.
Both complete examples run with no optional features.

The final HeatMap four-engine DPR1/2 run passes 208 frames, 1,616 RGBA samples
(max channel delta 0), 656 actual DOM XYWH comparisons (fixed 1/32 CSS pixel
bound), 48 trusted input groups and 96 candidate rejection records. All 192
before/after rejection PNG files are reread and byte-identical in their pairs;
full raw witnesses retain controls, typed text, native backing stores and focus.
Sixteen actual None source hovers hide the default tooltip; sixteen same-ID
Some(0) hovers display zero. Visible sparse gaps remain blank. Clearing selected
Widget ink restores actual source picking and Canvas material. Layout removal
cleans exactly the removed children; re-adding them creates new DOM, while
remaining North cells keep identity through custom Circle changes. Stop cleans
every current child exactly once. Source-ready mounting uses BrowserOptions'
initial interaction binding and plot before any first child factory executes.
The final report is
 target/heatmap-widgets-browser-four-engines-lf-final/report.json,
with full sources and preserved first failures in
 target/heatmap-widgets-browser-acceptance.json.

The final Waffle run passes eight contexts, 208 frames, 544 nine-pixel witnesses
(4,896 RGBA pixels, max channel delta 1), 144 trusted pointer checks,
112 rejection records and 96 lifecycle records. It verifies real data and empty
cell buttons, no synthetic empty source controls, raw 17.25 separately from
formatted 17.3, ownership reallocation under physical keys, full-slot direction
and ordering changes, controlled metadata, true Node/focus retention,
zero-opacity retention, occupied Areas unions versus actual empty cell marks,
custom Circles, CSS/camera changes, source-order occlusion and exact source
caption text. Factory/build/queue/stop and third-native-getContext failures
retain accepted pixels, backing stores, data and focus. The report is
 target/waffle-widgets-browser-four-engines-first/report.json;
 target/waffle-widgets-acceptance.json binds source and primary Nivo files.

Separate Chromium CI profiles pass for both roles on the same final WASM/glue.
Four intentionally broken harness copies must fail: fabricate an empty-cell
source control, make live Widget content invisible, read null as zero, and
throw a WASM RuntimeError after a real proposal. Their first preparation failed
to generate files due an overstrict one-mount assumption; those MODULE_NOT_FOUND
exits are explicitly excluded from valid negative controls. All four actual
mutants are executed and checked at their intended gates. Final drivers bind
startup/end script/WASM/glue hashes. One HeatMap script normalization removes
exactly six CRLF bytes; snapshots/proof and a fresh final eight-context plus CI
rerun bind the final LF script. Pixel/geometry/business thresholds are unchanged.

Full Rust regression passes 1,627 minimal tests, 1,931 all-feature tests and
18 doctests, with native and Rust 1.95 WASM strict Clippy, independent host WASM
Clippy, pinned formatting, hidden-file typos, workflow YAML, diff checks and
11 release-helper checks. CI includes both new focused Chromium gates and
uploads raw reports. README files remain separate English/Chinese and contain
portable wording. No new tag, publication or remote CI run is claimed.

The old advanced-scale failure is also rechecked without reclassifying history.
A read-only original focused Chrome run on the previous 2dd1 artifact passes
DPR1/2 with the original cases and raw1/paint1/median1/4 gates; source fingerprints
and limits are in target/advanced-scales-current-audit/readonly-review.json.
The current 247a artifact subsequently passes the **complete original Chrome
host CLI**, including every preceding suite, piecewiseScales and all later
suites. Its 96 strict rollback pairs have both screen and paint raw maximum 0.
The full report and executed-input hashes are in
 target/cell-widgets-full-host-final/{report,execution}.json.
The CLI's typography/baseline calls now honor its supplied URL; its first run
failed because those calls ignored that URL and navigated to an unavailable
default port. The piecewise verifier now imports the root's pinned development
D3 modules rather than requiring an old target cache. Existing official source
hash and version gates remain exact. After terminal success, only CRLF-to-LF
normalization is applied to those two tracked scripts for the scripts/** -text
policy. Executed-byte snapshots and an exhaustive no-other-byte-change proof
are in target/cell-widgets-final-validation/cli-eol-normalization.json. Syntax
and Git whitespace checks then pass on final LF sources.

This complete current Chrome result does not identify a unique cause of the
old raw36 compositor difference, replace old 173/174 matrix evidence, or claim
that the wider catalog has been rerun on every engine in this version. Native
same-layer input, raster bound-pointer/component composition, Waffle area
components, arbitrary Widget exports and springs remain open. GPU/worker stays
deferred. Combined sources, artifact fingerprints and all gates are recorded in
 target/cell-widgets-final-validation/acceptance.json.
See the [English guide](cell-widgets.md) and [中文指南](cell-widgets.zh-CN.md).

## Development CPU image Chart bindings

The later `chart_layout(frame).raster(...)` implementation closes the image
Chart binding/composition wiring gap recorded at the preceding checkpoint.
Twelve focused Memory/CPU tests cover the actual Image/View port contract,
nullable canonical sites and physical aliases, independent PNG pixels at two
DPRs, nonzero plots, absolute cameras, visual click slop, stationary repicking,
physical frame retirement and publication rollback with live Widget state.
Artificial Memory dispatch is not operating-system input evidence.

The complete native runs pass 1,627 minimal-feature tests and 1,943 all-feature
tests, plus 18 doctests. Rust 1.95 WASM and strict native/consumer Clippy checks
pass. Browser input is checked separately by
`scripts/raster-chart-bindings-acceptance.mjs`, which uses actual Glory web
input on the Image's parent View and does not install an application pointer
bridge. Its Chromium DPR1/2 gate is included in CI with report artifacts.

The image Chart camera remains separate from Surface's plot-relative gesture
view. Widget input stays on real sibling children. Native OS input,
arbitrary live Widget exports, Waffle area components and springs remain open;
GPU/worker remains deferred. Guides:
[English](raster-chart-bindings.md), [中文](raster-chart-bindings.zh-CN.md).

The final browser matrix passes Chrome, Firefox, WebKit and Edge at DPR1/2:
72 accepted frames, 120 trusted pointer groups, 72 exact received-coordinate
checks with maximum error zero, and eight PNG/typed/control/focus rollback pairs.
Four negative controls fail at their intended pixel, nullability, physical-ID
and publication assertions. Report:
`target/raster-chart-bindings-browser-four-engines-final/report.json`.
Combined source fingerprints and gates:
`target/raster-chart-bindings-final-validation/acceptance.json`.

Earlier browser attempts remain failed or explicitly aborted. Independent
blank-page and image-fixture probes reproduce a Firefox/Playwright viewport
origin `(0,0)` input acknowledgement wait on the tested Windows host; `(8,8)`
is verified as real content outside the chart and retains the leave/reentry
test. All browser operations now have Node wall-clock bounds. Exact inverse
checks use actual trusted event coordinates and the DOM viewport rectangle,
with the original `1e-6` gate. Click state and explicit DOM focus are checked
separately because WebKit button clicks do not establish focus by default.
These harness corrections do not change source paint or picking thresholds.

## Development Stream dot components and bound Chart anchors

`stream_dot_widgets` closes the actual enabled non-null Stream dot-component
role gap. It owns shared original series, resolved layer color, sample/stack
metadata and stable mark identity. None/disabled placeholders remain distinct
from zero-sized or transparent actual dots. Complete source, scale options and
actual Stream-mark admission occurs before placement callbacks. Bound Charts
now resolve accepted typed Path anchors for overlay and strict Widgets, also
before CPU image camera projection. Source-only Paths still require explicit
rectangles. Guides: [English](stream-widgets.md), [中文](stream-widgets.zh-CN.md).

Final native gates pass 1,643 minimal-feature tests, 1,961 all-feature tests and
18 doctests. Strict Rust 1.95 native/consumer WASM Clippy, MSRV WASM, pinned
nightly formatting, spelling, workflow YAML/executable-step validation and the
complete Memory example pass. Focused tests include actual PNG publication
rollback, None-to-zero insertion, layer-color versus custom-paint separation,
public source/scale forgery rejection and off-ink logical Path anchors.

The browser suite passes Chrome, Firefox, WebKit and Edge at DPR1/2 using one
unchanged artifact: 208 frame records, 64 trusted child/source input groups and
112 rejected candidates with complete accepted DOM/revision/focus/control and
before/after PNG equality. All 1,248 gated RGBA samples keep the channel limit
of one. Zero-valued children receive real native clicks; clearing components
restores actual source hover and click. Four negative controls fail at the
intended pixels, invented roles, displaced Path anchors and transaction checks.
An independent Chromium DPR1/2 gate uses the same artifact and is wired into CI.
Raw report: `target/stream-widgets-browser-role-contract-final/report.json`.
Combined evidence: `target/stream-widgets-final-validation/acceptance.json`.

The original Nivo producer executes getDotY, the actual data.map component
calls, enable gates and default circle-spring inputs: nine profiles, 27 calls,
12 gates and three circle inputs. Read-only `--frozen` reruns complete licensed
sources without a checkout or Git. Eight intentional source/reference changes
are rejected. This is numeric component-input evidence, not React rendering,
React lifecycle equivalence or an official Nivo nullable-data contract.

Sixteen disabled zero-height area-boundary samples remain separate diagnostics;
one fails the old strict RGBA limit, with `[1,3,3,0]` differences in Firefox DPR2.
An independent audit proves 10/10 paths, paint and clip attributes match the
standalone source and that no dot component exists there. Gated source checks
continue to cover non-degenerate area interiors and restored real dots,
including zero-valued dots. No source geometry, alpha or pixel limit was changed.
This component acceptance does not establish identical area-edge antialiasing.
Evidence: `target/stream-widgets-disabled-edge-audit/report.json`.

Earlier failed browser attempts remain archived. The first rollback witness
failure was independently reproduced without any candidate: the test called
an extra explicit flush, which requests a new paint revision. The synchronous
rejected-setter check now observes its accepted result directly, preserving all
original DOM/revision/PNG equality gates. Separate SVG-image, native SVG and
independent Canvas-primitive carriers document raster-origin/edge differences;
the collapsed boundary remains diagnostic rather than silently reclassified.
Automatic springs, other component roles, arbitrary live Widget exports, native
OS input and the broader existing catalog remain outside this result.
GPU/worker remains deferred.


## Development: Waffle occupied-group event regions

`waffle_area_interaction` now projects the existing checked Waffle layout into
one real canonical site per occupied visible group, full-slot logical rectangles
and physical cell aliases. Scene and exported SVG remain identical. Region
selection stays independent of opacity and actual paint; padding creates no
physical ink. Camera inversion and whole-frame publication use the shared Surface
model across semantic Chart, CPU image Chart and browser SVG/Canvas.

Final native gates: **1,658 minimal**, **1,977 all-features/all-targets**,
**18 doctests**, and **16 focused raster-only** tests. MSRV 1.95 native/WASM
strict Clippy, fmt, typos, frozen Stream reference regression, release-helper
checks, workflow schema/executable steps and portable bilingual READMEs pass.
Independent native tests cover exact rectangle budgets, half-open/overlap rules,
14 invalid candidate types, nullable typed values, camera/stationary repicking,
active gestures and physical token rollback. A review found stale physical state
on explicit clear after region-owned ink overflow; six clear paths now have
regression coverage. Memory protocol injection does not establish OS transport.

Chrome, Firefox, WebKit and Edge each passed DPR 1/2: **128 frame records**,
**160 trusted input groups**, **64 rejected candidates** and **400 RGBA samples**.
All sampled channels matched exactly (allowed tolerance remains ±1). Samples
cover actual solid interiors, padding, transparent cells, empty paint, Areas
unions and the actual offset custom Path interior. Each click must add exactly
one activation for its group; keyboard activation checks the exact second group.
A separate Chromium DPR 1/2 run executes the CI subset against the same artifact.
Driver/helper/WASM/glue fingerprints are unchanged within each accepted run.

Firefox native input request quantization was independently reproduced on a
blank page, at both DPRs, with errors up to about 0.817 CSS pixels. Requested
screen coordinates allow at most one CSS pixel of native delivery error; this
is separate from geometry and pixel gates. Business selection is recomputed
from actual received client coordinates, and physical Scene points still require
1/32 logical pixel agreement. Pixel tolerance remains ±1. Earlier failed input
assertions remain failed; no production coordinate or geometry change was made.
The first color-carrier parser expected hex while the exporter emits RGBA;
that harness parser was corrected. An earlier rollback witness changed focus
between its two before snapshots; focus now settles before either witness.
Exact raw state, DOM, focus identity and screenshot rollback checks remain.

Evidence is in `target/waffle-regions-final-validation/acceptance.json`,
`target/waffle-regions-browser-accepted-final/report.json`,
`target/waffle-regions-browser-ci-final/report.json`,
`target/waffle-regions-native-acceptance.json` and
`target/waffle-region-independent-final-audit.json`.
Three independent mutated drivers also failed at their intended gates: wrong
RGBA, invented physical ink in a logical-only gap, and a successful wrong layout
in the rejected-candidate position. Their scripts, logs and reports remain in
`target/waffle-regions-driver-negatives/negative-acceptance.json`; the original
driver/helper/package hashes stayed unchanged.

Nivo source/commit hashes and its inclusive-first shared-edge difference are
recorded in the independent audit. Kartis intentionally uses half-open bounds
and reverse declaration priority. There is no Nivo areaComponent prop.

The API remains Unreleased. Group Widget factories, automatic springs, arbitrary
live Widget exports, native same-layer input and GPU/worker remain open. Prior
Stream disabled zero-height area edge diagnostics and broader historical catalog
reports are not reclassified by this batch.


## Development: Scatter live node components

`scatter_node_widgets` adds measured owned node roles while retaining the public
ScatterLayout structure. An explicit original series input supplies complete raw
datums; selected nodes share one Arc per selected series. The adapter validates
all source metadata, exact declared/resolved axes, site/mark indices, projections,
actual bounds and typed anchors before invoking any placement callback. Public
coherent edits remain supported; the original size accessor and format options
are not retained or reconstructed. Raw XY/size values preserve original sign
bits; compatible Scene scalars retain their existing numeric semantics.

Final native gates pass: **1,673 minimal**, **1,994 all-features/all-targets**,
**18 doctests**, **17 focused raster-only** tests, MSRV 1.95 native/WASM strict
Clippy, fmt, typos, the runnable Memory example and release-helper checks.
Workflow schema/executable steps and portable separate bilingual READMEs pass.
Independent audit exercises 50 actual source/layout conflicts and eight partial
signed-zero conflicts before callbacks, complete Arc source data, role eligibility,
coherent edits and offset Path anchors. Real Memory tests retain state/focus,
mount an inert off-plot child and reject typed candidates. Real DPR1/2 PNG
publication failures preserve accepted bindings, camera, source and children.
These checks do not establish native OS event transport.

Original Nivo 0.99 sources and their MIT license are frozen in
`tests/fixtures/scatter-node-widgets`. The read-only verifier executes actual
computePoints, regular/leave/key functions, transition option expressions, the
real Nodes createElement callback and the default Node radius expression.
Independent offline controls block checkout/Git/network/writes; two verification
runs pass and ten fixture/source/recipe tamper cases fail at intended assertions.
This verifies original argument recipes, not React reconciliation, SpringValues
or transition timing. Nivo serieIndex is the datum index within its series;
Rust source_index and series_index remain separate. Rust's explicit omissions
are not claimed to reproduce a Nivo filter that does not exist.

Chrome, Firefox, WebKit and Edge each passed DPR1/2 with SVG and Canvas:
**176 frames**, **48 trusted input groups**, **112 rejected candidates**,
**10,368 Widget interior RGBA samples** plus **48 restored source-ink samples**.
Maximum channel error is **1**, retaining the original ±1 gate. Actual button
position/size are independently measured from DOM and compared to logical slots
within 1/32. Original Number/String XY and complete series metadata are checked
against fresh source packets. Native clicks must increment this exact child's
count once; source selection remains separate. Legitimate revision changes retire
old children, and stop disposes each still-active child exactly once. Transparent,
zero-size, hidden, missing-size and omitted-domain profiles keep distinct roles.
Restored Circle and offset Path paint have positive pixels and trusted source
picking; zero-size source geometry creates no fake ink.

A separate Chromium DPR1/2 CI subset passes against the identical protected
WASM/glue/driver/helper. Four independent mutated drivers each fail at the exact
intended gate: wrong RGBA, a raw Number-to-String substitution in both site and
owned datum, actual buttons shrunk while their center samples still lie inside,
and an accepted committed layout disguised as a rejected setter. The last case
passes the return-status check and is then caught by the full frame witness.

An earlier complete matrix failed the first WebKit rollback witness with a
revision increment and identical PNG. A complete-prefix no-candidate control
reproduced the same increment: the raw DOM changed only data-revision. The
observation order was corrected by settling the pointer outside both surfaces
before focus/baselines and capturing screenshots before reading accepted state
and DOM. The corrected no-candidate control passes both WebKit DPRs. Exact
revision, raw DOM, focus identity and PNG rollback checks remain; production
code and pixel/geometry gates were not changed. Old reports and the earlier
negative-control round remain archived, not relabeled as current acceptance.
Screen request quantization still allows at most 1 CSS px, separate from the
1/32 DOM geometry and ±1 color gates, as independently measured in the preceding
Firefox blank-page control.

Current evidence:
`target/scatter-widgets-final-validation/acceptance.json`,
`target/scatter-widgets-browser-accepted-final/report.json`,
`target/scatter-widgets-browser-ci-final/report.json`,
`target/scatter-widgets-native-acceptance.json`,
`target/scatter-widgets-independent-final-audit.json`,
`target/scatter-widget-frozen-audit/report.json`,
`target/scatter-widgets-driver-negatives-final/negative-acceptance.json` and
`target/scatter-widgets-no-candidate-control/summary.json`.

The API remains Unreleased. Automatic springs, other component roles, arbitrary
live Widget exports, native same-layer input and GPU/worker remain open. Prior
whole-catalog results and Stream zero-height area edge diagnostics are unchanged.


## Development: Bar live component roles

`bar_widgets` and `bar_widget_frame` add owned measured roles to existing checked
CustomBarLayout factory entries. Default factories inherit ordinary Bar output,
including grouped/stacked/percent, both orientations and configured scales.
Private captured sources retain complete nullable series/categories, exact raw
values and indices, ordinary cropped and unclipped geometry, original endpoint
projections and typed Path anchors. Current labels/clips include factory text,
empty text and accepted controlled dim colors. Scene labels/totals retain their
existing ownership; components do not automatically duplicate or replace them.
Empty layouts retain an accurate plot, and callback preflight rejects changed
managed frames before any placement side effect.

Final native gates pass: **1,689 minimal**, **2,012 all-features/all-targets**,
**18 doctests**, **18 focused raster-only** tests, MSRV1.95 native/WASM strict
Clippy, fmt, typos, the complete Memory application and release-helper checks.
This includes existing Bar geometry/label factory regressions, default inherited
Scene/SVG equality, empty/hidden/cropped variants and exact source snapshots.
Independent tests cover 30 public frame mutations, raw negative zero, Arc source
isolation, source rectangles/endpoints and label/clip association. Real Memory
children preserve state/focus and live label data; real DPR1/2 PNG publisher
failure preserves accepted source, data, camera, controls and children. Memory
injection does not establish native OS transport.

The original Nivo0.99 source/complete MIT notices are frozen in
`tests/fixtures/bar-widgets`. Its read-only verifier executes actual null-value
eligibility, bars-layer gates, common props, component creation, transition
recipes, label layout and default rectangle expressions: 128 createElement calls
across eight profiles and four style phases. Two read-only runs pass under
checkout/Git/network/write tripwires, and 15 tamper controls reject at intended
assertions. React/spring runtime, complete D3 numeric layout equivalence and
Rust-specific omission/zero-label policies are outside that reference result.

The final explicit solid-color Button fixture passes Chrome, Firefox, WebKit
and Edge at DPR1/2 in SVG and Canvas: **240 paint frames**, **48 label-metadata
frames**, **48 trusted input groups**, **112 rejected candidates**,
**14,688 Widget interior RGBA samples** and **48 restored source-ink samples**.
All gated sample channels match exactly; the allowed tolerance remains ±1.
Real DOM Button bounds retain a separate 1/32 logical-pixel gate. Label stages
check actual factory text/empty text/clip, current Mark opacity and disabled
Widget input. Legitimate revisions replace the correct child scopes, and stop
cleans each of 14 still-active children exactly once. Raw/displayed/stacked,
percent, horizontal, cropped-category and cropped-SymLog data all reach retained
children. Restored ordinary and offset Path source paint have positive samples
and trusted source picking, while a zero-area bar creates no fake physical ink.
A separate Chromium DPR1/2 CI subset passes the same protected artifact.
Four independent mutated-driver controls reject wrong RGB, jointly forged raw
bar/series values, actual smaller Buttons whose centers and nine sampled points
still lie inside, and a real accepted layout disguised as rejection. The last
case passes both return-status guards and then fails the full state/DOM/PNG
witness. Evidence: `target/bar-widgets-driver-negatives-final/negative-acceptance.json`.

Earlier failed attempts remain failed. The first small-button fixture had a
7px inline baseline offset; its application CSS now explicitly uses display:block.
No library geometry adjustment was needed. A later font8 same-color text sample
on a blue Button was [0,98,255,255] instead of [0,100,255,255] in the actual Canvas
compositor. Pillow independently confirms those bytes. A reversible CSS font8/0/8
probe gives 98/100/98 in that channel with all bounds, complete accepted state
and revision44 unchanged. An independent blank native Button does not reproduce
it; this is not generalized as a universal font error or a resolved compositor
issue. The final Widget probe explicitly draws solid color with font-size0,
retaining actual DOM JSON, source data, click state, focus and labels. Its color
samples do not establish strict arbitrary Widget text raster equivalence.
The original glyph report remains featureAccepted=false. Pixel tolerance stays
±1, DOM geometry stays 1/32, and no production painting or input code changed.

Current evidence:
`target/bar-widgets-final-validation/acceptance.json`,
`target/bar-widgets-browser-accepted-final/report.json`,
`target/bar-widgets-browser-ci-final/report.json`,
`target/bar-widgets-native-acceptance.json`,
`target/bar-widgets-independent-audit.json`,
`target/bar-widget-frozen-audit/report.json`,
`target/bar-widgets-driver-review/static-review.json` and
`target/bar-widgets-glyph-control/summary.json`.
New work remains Unreleased. Automatic springs, other component roles, arbitrary
Widget export, native same-layer input and GPU/worker remain open. Earlier
Stream boundaries, font8 diagnostics and broader historical reports are not
reclassified by this focused matrix.

## Development: exact viewport sampling and native text diagnosis

The read-only `acceptance-viewport.mjs` helper captures the complete configured
viewport, checks before/after geometry, and projects accepted logical surface
points through independent X/Y DOM scales and DPR with one physical floor.
The native-only matrix passes Chrome, Firefox, WebKit and Edge at DPR1/2:
48 frames, 80 exact gated RGBA samples, 24 independently laid-out native-marker
coordinate/index checks, 120 rejected queries/captures and eight legacy padded
crop counterexamples. The separate Chromium CI run passes both DPRs. Four
independent mutant copies reject a one-row index error, a wrong Y scale,
a removed geometry guard and an offscreen clamp at their intended gates.
No old fitted-image helper or historical report was rewritten.

Hard palette gates cover unscrolled physical tags, broad cyan CSS panels and
unscaled fractional-origin CSS rows at DPR2. Other fine CSS rows and scrolled
fixed-Canvas palettes are explicitly diagnostic.
In the Firefox fractional-scroll case, the latter palette differs by one/two
physical pixels while the independent native-marker index remains correct.
This is not counted as a passing palette comparison. Geometry stability does
not prove stable paint; callers still settle and verify their accepted frame.
CI executes the Chromium sampling matrix and preserves its reports.

The original Bar font8 comparison remains **featureAccepted=false**. Correct
full-viewport sampling of the current package still reads [0,98,255,255] at
physical index [282,985]. An independent read-only Pillow check confirms the
bytes; the reproduced element PNG matches the historical failure byte for byte.
Removing or hiding the actual Canvas eliminates the glyph difference; merely
setting its opacity to zero does not. A scriptless native DOM carrier with no
Glory/WASM also reproduces it after initializing and clearing a transparent
Canvas2D context. Simpler native controls do not universally reproduce it.

A separate portable native-only probe fixes document language to English and
reports default and diagnostic `--disable-lcd-text` profiles, each with font8,
font0 and black-text controls. Default same-color text reads G100, diagnostic
non-LCD text G98, and font0 G100 in both. Independent experiments match all 275
pixels of the local font8 and black-text regions between the actual carrier and
the diagnostic non-LCD control. This supports a text raster/compositing-path
explanation for that carrier; it does not identify a unique Skia branch or
establish arbitrary text equivalence. Production flags, source rendering and
strict ±1/1/32 gates are unchanged. No glyph fix is claimed.

Evidence: `target/viewport-sampling-final-validation/acceptance.json`,
`target/viewport-capture-final/report.json`,
`target/viewport-capture-ci-final/report.json`,
`target/viewport-helper-negatives-final/review.json`,
`target/bar-widget-glyph-independent-audit.json`,
`target/bar-widgets-font-compositor/review.json` and
`target/native-text-compositor-final/report.json`.
See the [sampling guide](viewport-sampling.md) and
[中文指南](viewport-sampling.zh-CN.md). The 575 Rust files fingerprinted by the preceding Bar verification and its
accepted browser package remain byte-for-byte unchanged. Those Rust test counts
are inherited evidence, not tests rerun for this JavaScript/documentation change. This work
remains Unreleased; the broader feature goal and arbitrary text acceptance remain
open.

## Development: Bump live point components

`bump_point_widgets` builds measured typed roles from actual Bump point Marks.
Each selected series shares one owned original-series snapshot; raw source index,
Number/String category identity, nullable rank and numeric sign bits remain
separate from current computed geometry/paint. Full source, scale, point,
line-run, site and anchor validation precedes callbacks. The public layout
structure is unchanged; consistent current edits are supported without claiming
private original-style provenance. Zero-size and transparent actual points retain
roles, while gaps and hidden series do not. `bump_widget_frame` preserves the
actual plot, including empty frames, and uses explicit painted picking.

Native checks pass **1,698 minimal**, **2,022 all-features/all-targets**,
**18 doctests** and **8 focused raster-profile tests**. Rust1.95 strict native
and WASM Clippy, normal consumer formatting/strict Clippy/release build, the
complete portable application, release-helper tests, spelling and formatting
also pass. A fresh normal workspace release build plus bindgen reproduces the
browser package byte for byte. Focused tests include 24 reverse/round/curve/
padding profiles, 31 corrupted-frame rejections before callbacks, source snapshot
sharing/isolation, budget/error behavior and actual custom-Path original anchors.
Memory children preserve state/focus; real CPU PNG publisher rejection retains
the entire tree, image and source data at DPR1/2.

Chrome, Firefox, WebKit and Edge pass DPR1/2 in SVG/Canvas: **192 frames**,
**48 trusted input groups**, **112 rejected candidates**, **11,808 Widget RGBA
samples** and **48 restored source-ink samples**. Maximum channel error is zero,
with the existing ±1 allowance; real Button geometry retains its independent
1/32 logical-pixel gate. Captures use the read-only full-viewport helper and
single physical floor. Raw full series/ranks/categories are independently
hardcoded in the driver before comparison with host-generated metadata.
The separate two-context Chromium run passes the same artifact. Legitimate
revisions rebuild children; stop cleans exactly the currently active children.
Ordinary circles, true zero-size ink absence and offset Path source picking have
positive restored-ink checks. Source lines and endpoint labels remain Scene-owned.
Four independently mutated drivers reject wrong expected RGB, jointly forged
rank/source metadata, real smaller Buttons whose nine probes still lie inside,
and a real accepted update disguised as a rejection. The last control passes
return guards and fails the complete accepted-state/DOM/PNG witness.

The upstream frozen verifier executes Nivo0.99's original computation and actual
point-scale adapter, style branches, component creation, spring-input recipes,
radius clamp and layer gates. It confirms gap filtering before point creation,
original source indices, zero-value/size0 eligibility and separate active-series
versus active-point axes. Default Nivo Point has pointer-events:none and does
not apply computed-series opacity as a circle attribute. Rust category-based
IDs, child-owned input and controlled Mark opacity are explicit model choices.
Expression verification does not execute React/Spring lifecycles or prove full
D3 line-drawing equivalence. Two offline read-only passes under checkout/Git/
network/write tripwires and eight tamper controls pass. Complete source and MIT
notices are preserved. The separately hand-authored null-computed-hook diagnostic
is excluded from the 30 normal-pipeline component calls.

The first browser attempt failed because its independent expectation used +0
for original -0; the expectation was corrected to -0, with the explicit source
bit assertion retained. That failed report remains archived. No production
rendering/input rule or pixel/geometry threshold was changed for that correction.
The final fixture explicitly uses solid-color font0 Buttons while retaining JSON
text for data/state checks. The older Bar font8 diagnostic remains unaccepted;
this result does not claim arbitrary Widget text equivalence or reclassify old
whole-catalog/Stream edge reports.

Evidence: `target/bump-widgets-final-validation/acceptance.json`,
`target/bump-widgets-browser-full/report.json`,
`target/bump-widgets-browser-signed/report.json`,
`target/bump-widgets-native-acceptance.json` and
`target/bump-widgets-driver-negatives/review.json`.
See [the component guide](bump-widgets.md) and [中文指南](bump-widgets.zh-CN.md).
This work is Unreleased. Automatic springs, arbitrary component export, native
same-layer input, further chart roles and GPU/worker remain open.

## Development: live hierarchy node components

`hierarchy_node_widgets` supports Treemap, CirclePacking and Icicle actual node
Marks through one typed measured protocol. A packet shares one owned original
flat tree and distinguishes datum self-weight from computed subtree aggregate;
original indices, family, focus/ancestors and current labels/clips remain available.
Complete source/topology/aggregate/focus/eligibility/index/geometry/managed-label
preflight precedes callbacks. `hierarchy_widget_frame` retains the exact plot and
adds typed sites for actual unpainted roles while PaintedGeometry picking remains
physical. Zero-weight, hidden, outside-focus or configured exclusions do not
invent components. Actual transparent and no-paint Marks still retain roles.

Current Rust checks pass **1,706 minimal**, **2,031 all-features/all-targets**,
**18 doctests** and **9 focused raster-profile tests**. Strict native and WASM
Clippy on Rust1.95, normal consumer formatting/Clippy/release build, the complete
three-view portable application and release-helper tests pass. A fresh normal
workspace release build and bindgen candidate reproduce the tested browser
package byte for byte. Tests cover parents' raw0 versus positive aggregates,
shared source ownership, managed measured-label hiding/restoration, 36 three-view
eligibility profiles, 28 pre-callback conflict rejections, budgets, empty frames
and the explicit motion-target boundary. Real Memory children preserve local
state/focus; DPR1/2 CPU image publication failure retains tree, PNG, source and
camera for each of the three views.

Adding private raw provenance initially broke the old input-permutation layout
equality test. That real failed run is preserved; explicit presentation equality
now retains every previous field comparison and excludes the new private raw
snapshot. Payload equality separately compares complete source weights by bits:
a -0/+0-only update refreshes actual owned data even when Scene is equal. The
native parent-component regression and all 24 final browser cases preserve child
identity/count/focus and geometry while confirming that exact raw-bit change.

Chrome, Firefox, WebKit and Edge pass DPR1/2 separately for each of the three
families: **24 contexts**, **624 frames**, **144 trusted input groups**,
**336 rejected candidates**, **20,736 Widget RGBA samples** and **96 restored
source-ink samples**. Maximum channel error is zero, with the unchanged ±1
allowance; actual Button geometry retains its separate 1/32 logical-pixel gate.
Captures use full-viewport coordinates and one physical floor. Original full-tree
rows, source bits, node roles, depth, parent and focus paths are independently
hardcoded before comparison with host-generated data. Legal revision rebuilds
children; stop cleans all currently active children exactly once. The independent
Chromium subset passes six contexts and 156 frames using the same frozen package.
Four mutated-driver controls reject wrong RGB, jointly forged raw/aggregate/full
tree data, smaller actual Buttons containing every sampled point, and a real
accepted focus update disguised as rejection. The last passes both return guards
before the complete accepted-state/DOM/PNG witness rejects it.

The reference preserves 29 complete Nivo0.99 sources and five MIT notices. It
executes the distinct original Treemap node, CirclePacking circle and Icicle
shared RectNodes/Wrapper recipes: 16/4/18 profiles and 280/14/90 creation-argument
calls respectively, with scalar transitions, handlers and label conditions.
Two offline read-only runs under checkout/Git/network/write tripwires and ten
tamper controls pass. Computed geometry is authored; no D3 hierarchy sum/layout,
React reconciliation, actual Spring scheduling or browser focus transport is
claimed. Upstream zero-area/invisible component eligibility, nested/path IDs,
nonzero internal self-weights, opacity and label ownership differ from Rust's
flat globally unique-ID, actual-Mark and zero-internal-weight model.

The first portable Memory example hit the existing circular label-clip capability
boundary; the final complete example explicitly uses plot-rectangle clipping
for CirclePacking labels, preserving their text. No production clipping gate was
relaxed. An early matrix process was deliberately stopped to correct a stale
Path-anchor scope description to node anchors; it is not counted as passing.
Final results bind the corrected driver. Solid-color font0 Buttons retain JSON
for data/state checks; arbitrary glyph equivalence and the earlier Bar font8
failure remain unaccepted. Historical catalog and Stream edge results remain
separate.

Evidence: `target/hierarchy-widgets-final-validation/acceptance.json`,
`target/hierarchy-widgets-browser-accepted/report.json`,
`target/hierarchy-widgets-browser-ci-final/report.json`,
`target/hierarchy-widgets-native-acceptance.json`,
`target/hierarchy-widgets-driver-negatives/review.json` and
`target/hierarchy-node-widgets-independent-audit.json`.
See [the guide](hierarchy-widgets.md) and [中文指南](hierarchy-widgets.zh-CN.md).
The work is Unreleased. Automatic sampled-motion/exit-node components, arbitrary
Widget export, native same-layer input, other chart roles and GPU/worker remain
open. Only static accepted layouts or explicit motion targets are supplied here.

## Development: original values in retained Widgets

Unreleased. Typed Bump, Scatter, Bar, Stream, Heatmap and Bullet payload equality
now observes original signed-zero changes independently of ordinary Scene/key/
geometry equality. Shared owned series use pointer identity or exact original
content; current raw fields are also compared. Bullet original interval endpoints
include an omitted preceding zero part, while visible projected coordinates keep
numeric equality. Stream category and Heatmap value provenance reject tampering
before placement callbacks. The public payload shapes remain unchanged.

Actual Memory RED/GREEN evidence covers retained identity, local count/focus,
metadata-only updates and invalid-candidate retry. The initial Bullet test placed
a child outside plot clipping and failed to activate it; that is fixture-error
evidence only. The corrected fixture then reproduced three genuine stale-data
updates before the endpoint fix. Both original endpoints, positive range/measure,
synthetic source-index-none range and a marker control are tested. Waffle's
unowned zero group is not presented as a reachable cell payload.

Final native checks pass **1744 minimal-profile tests**, **2070 all-feature tests**
and **18 doctests**, plus strict Rust1.95 native/WASM Clippy. The normal consumer
workspace passes formatting, strict Clippy and a release WASM build. The complete
portable Memory example is executed; both guides reproduce its exact Rust code.
These counts bind the final endpoint fix, not only the earlier six-role pass.

The distinct browser functionality oracle requires real SVG/Canvas source planes,
independent exact bit strings, actual retained Buttons and expected business
rejection reasons. Actual DOM geometry keeps the 1/32 logical-pixel gate and
foreground pixels keep ±1 per-channel tolerance. Rollback compares accepted state,
complete root DOM, computed SVG paint styles, native Canvas backing bytes and
viewport geometry exactly. TypeError/WASM traps fail, rather than count as a
business rejection. The fixture authors zero compatibility Mark scalars as +0
in both phases; original typed metadata is untouched. A fixed reference rectangle
keeps a source plane present even for the single-mark Scatter replacement.

The final Chrome/Firefox/WebKit/Edge matrix passes **48 contexts, 480 frames,
96 trusted input groups, 480 rejected candidates and 4320 foreground RGBA
samples**. The independent Chromium CI subset passes **12 contexts, 120 frames,
24 trusted input groups, 120 rejected candidates and 1080 foreground samples**
on the same frozen package. Five mutated controls plus a separately disclosed
deep-witness variant are rejected by the intended gates. A hidden one-pixel
Canvas backing edit leaves foreground colors, full PNG, state, DOM and geometry
identical but is still rejected by the native-buffer witness. Sixteen protected
source/helper/package files remain byte-identical across the negative controls.

Two full-matrix viewport hash comparisons differ; the CI subset has zero.
These are preserved diagnostics and do not weaken the separate functional gates.

The earlier strict whole-viewport run remains failed. Independent no-candidate
and script-free native carriers reproduce one changing SVG antialias pixel with
identical DOM, geometry and Canvas backing. The prescribed integer alignment also
fails to eliminate it. Whole-viewport hashes in the new driver are diagnostic;
functional acceptance does not establish strict screenshot stability. The original
failed driver/report and native diagnosis are preserved under separate identities.
Arbitrary glyph parity, the Bar font8 compositor failure, original upstream
React/Spring/D3 runtime parity and full-catalog acceptance remain unclaimed.

See [the guide](widget-source-values.md), [中文指南](widget-source-values.zh-CN.md)
and the complete [example](../examples/widget_source_values.rs). Automatic sampled
motion, arbitrary Widget export, native same-layer input and GPU/worker remain
open. This work does not modify the published 0.1.0 artifacts.

Evidence: `target/widget-raw-bits-final-validation/acceptance.json`,
`target/widget-source-values-browser-final/report.json`,
`target/widget-source-values-browser-ci-final/report.json`,
`target/widget-source-values-browser-negatives/review.json`, the point/role
native receipts and `target/widget-raw-bits-bullet-endpoints-acceptance.json`.
The previous failed strict report and native script-free diagnosis remain
`target/widget-raw-bits-browser-accepted/report.json` and
`target/widget-raw-bits-witness-independent-audit.json` respectively.

## Development: live Swarm node components

Unreleased. `swarm_node_widgets` pairs actual packed nodes with a shared owned
complete original `SwarmPoint` input, original/packed indices, current group,
plot, collision disc and resolved value scale. `swarm_widget_frame` opts into
explicit PaintedGeometry. Complete source/group/metadata/current projection/
packed-order/Mark/anchor validation precedes the first placement callback.
Public SwarmLayout fields and ordinary rendering remain unchanged. The layout
does not retain original packing/shape/orientation/fit/format configuration;
coherent public edits are legal and original force trajectories are unclaimed.

Final full regression passes **1759 minimal-profile tests**, **2086 all-feature
tests** and **18 doctests**. Rust1.95 native/WASM strict Clippy and the normal
consumer formatting/Clippy/release build pass. The full standalone portable
Memory application runs, retaining an actual Button's identity and click count
across raw-zero-only source changes and source reordering. Both guides reproduce
its exact Rust code.

The core focused suite covers 32 orientation/reverse/round/fit combinations,
whole-frame conflicts before callbacks, actual offset Path anchors, a shared
source copy, hidden/unselected raw value/size bits, zero/transparent/no-ink roles,
skip/error/128-slot limits and empty plots. Independent tests add 30 source/public
conflicts, coherent public-edit controls and actual Memory count/focus/identity.
DPR1/2 CPU publisher refusals prove reachable publication, old PNG/source/data/
node-count retention and recovery with the same child. Early reference harness,
test API/compile and lint authoring failures are preserved and classified;
they are not reported as production regressions.

Chrome, Firefox, WebKit and Edge pass DPR1/2 SVG/Canvas: **8 contexts, 272 frames,
48 trusted input groups, 112 rejected candidates, 10,800 foreground samples
and 32 restored-source samples**. Maximum channel error is zero; the allowance
remains ±1. Actual Button bounds retain a separate 1/32 logical-pixel gate.
The same-artifact Chromium subset passes **2 contexts, 68 frames, 2700 foreground
samples and 8 restored-source samples**. Independent Pillow decoding checks
all **380 saved frame/source PNGs and 13,540 samples**, recomputing physical
viewport indices from actual DOM geometry and DPR with one floor.

Independent input/role oracles precede source/live comparison. Measured placements,
source metadata, source reorder, metadata-only resize, hidden groups, all-zero
sizes, transparent nodes, skipped nodes, clipping, reversed vertical layout,
empty source, legal revision replacement, source restoration and actual offset
Path picking are tested. Actual trusted Widget clicks and source pointer events
are checked separately. Stop cleans all active children exactly once.

Five mutated controls plus a separately disclosed deep-witness variant fail the
intended data, color, actual-DOM and rollback gates. A joint source/live source-ID
and raw-bit forgery fails the independent source oracle. Smaller Buttons contain
all color probes but fail the actual DOM gate. A real accepted source change
disguised as rejection passes return/error-reason checks and then fails source or
complete-state witnesses. A hidden one-pixel Canvas backing edit keeps the full
PNG, accepted data, root DOM, SVG styles, geometry and nine foreground pixels
identical, yet fails the exact backing-byte witness. Ten protected source/helper/
package files remain unchanged during those negative controls.

Rejection witnesses compare accepted state, complete root DOM, SVG computed
paint styles, native Canvas backing bytes and viewport geometry exactly. Expected
business reasons are checked; TypeError and WASM traps fail validation. Whole
viewport PNG hashes are diagnostic in this distinct functionality driver; this
matrix has zero diagnostic mismatches. It does not establish arbitrary glyph
or strict compositor equivalence or resolve the historical Bar font8/native SVG
variance. Font-zero solid-color Buttons and raw JSON remain separate from text
rasterization claims.

The frozen original Nivo0.99 reference preserves eight complete source files,
package metadata and one full MIT notice. It executes source preparation,
20 SVG creation/transition/handler profiles (80 creation calls), default circle
expressions and eight record-only Canvas command cases. Positions and simulation
indices are authored; React, actual Spring/force scheduling and pixel output are
not executed. Two offline read-only tripwire runs, six tamper controls and
explicit UTC/Shanghai formatter-probe checks pass. The formatter probe uses ISO
Date text so reference verification does not depend on machine timezone.

Evidence: `target/swarm-widgets-final-validation/acceptance.json`,
`target/swarm-widgets-browser-final/report.json`,
`target/swarm-widgets-browser-ci-final/report.json`,
`target/swarm-widgets-native-acceptance.json`,
`target/swarm-widgets-independent-acceptance.json` and
`target/swarm-widgets-browser-negatives/review.json`.
See [the guide](swarm-widgets.md) and [中文指南](swarm-widgets.zh-CN.md).
Automatic sampled-motion/retiring components, arbitrary Widget export, native
same-layer input and GPU/worker remain open. Published 0.1.0 artifacts are unchanged.

## Development: live Network node components

Unreleased. `network_node_widgets` retains complete current force-node state,
original float bits, source indices and real glyph origins without inventing
canonical source controls. BrowserNetwork accepts force, Scene and typed child
data together across real ticks, source-node dragging, resize, graph replacement
and revision/heat requests. Retained child Bond reads agree with the adopted
owner snapshot. Stop interrupts later placements; cold hover avoids identical
Scene/hover-token RAF feedback.

The final frozen cohort passes Chrome/Firefox/WebKit/Edge at DPR1/2, with four
source-ordered/top-layer and replace/keep profiles: **32 contexts,
1216 SVG/Canvas frames, 128 trusted input groups,
448 rejected candidates and 64 rejected CSS-resize
candidates**. There are **30,528 Widget foreground,
1,088 unreplaced-source and 64 decorative-link
samples**; maximum channel error is 0, with the allowance unchanged at ±1.
Actual DOM dimensions and mapped world positions keep their independent 1/32
logical-pixel gate. The same-artifact Chromium subset passes **8
contexts, 304 frames and 7,920 samples**. Independent Pillow decoding
checks **1,520 saved PNGs and 39,600 samples**,
recomputing expected probe points/colors and physical viewport indices from
accepted state, actual DOM bounds and DPR with one floor.

Original RED production evidence is retained for transformed CSS dimensions,
owner/live-Bond adoption, callback-stop boundaries, monotonic reheat coalescing
and idle source-hover RAF feedback. Corrected behavior is exercised by the final
complete cohort. A standalone BrowserChart control separates independent CSS
repaint from explicit snapshot flush; external CSS itself cannot be rolled back.
The first control accidentally reused its initial JSON path; that artifact loss
is explicitly recorded rather than presented as a preserved first-run report.

Two additional fixture/oracle failures remain distinct. The original 2px link
profile's failed color sample is preserved; the final fixture explicitly authors
a 6px link for a solid interior sample. Production thin-line rendering is not
reported as fixed. Active-drag fault injection and its accepted baseline are
read atomically. Firefox delivered trusted down/up coordinates differ from the
requested automation delta; a script-free, resource-free DOM control reproduces
the delivered coordinates. Expectations use actual trusted endpoints, measured
stable client bounds and grab offset. The one-client-pixel input qualification
does not relax the independent 1/32 world-position gate.

Six target-only negative controls fail their intended independent data/color/
DOM/rollback gates. A real accepted graph update disguised as rejection fails
both the original-source guard and the separately exercised deep witness.
A one-pixel hidden Canvas-backing mutation leaves full viewport PNG bytes,
accepted force state, DOM, SVG styles and geometry unchanged, but fails the
exact backing witness. All 13 protected files remain unchanged during controls.

Full accepted state, DOM, SVG paint styles, Canvas backing bytes and geometry
rollback are strict; whole-viewport PNG equality is diagnostic. The full matrix
records 19 diagnostic mismatches, and the CI subset
records 0. These results do not reclassify historical strict
compositor, Bar font8 or native scriptless-SVG failures. Font-zero solid Buttons
do not establish arbitrary text/glyph or OS input equivalence.

Native validation passes 1769 minimal-feature and 2097 all-feature tests, plus
18 doctests, native/WASM strict Clippy and the consumer WASM release build.
The frozen Nivo0.99 reference preserves eight original source files, package
metadata and one MIT notice, and checks 20 component profiles/80 creation calls,
four style profiles and eight record-only Canvas cases. Authored force positions
are not actual React/Spring/D3 scheduling or pixel equivalence.

Evidence: `target/network-widgets-final-validation/acceptance.json`,
`target/network-widgets-browser-pointer-accepted/*/report.json`,
`target/network-widgets-browser-ci-pointer-accepted/report.json`,
`target/network-widgets-independent-acceptance.json` and
`target/network-widgets-browser-negatives-pointer-final/review.json`.
See [the guide](network-widgets.md) and [中文指南](network-widgets.zh-CN.md).
Live linkComponent, automatic retiring-node components, arbitrary Widget export,
native same-layer input and GPU/worker remain open. Published 0.1.0 is unchanged;
the new CI step is locally exercised and remains an uncommitted workflow edit.

## Development: live Network link and mixed node components

Unreleased. `network_link_widgets` adds complete shared current node/edge data,
original float bits, edge indices and checked endpoint/midpoint metadata.
`network_widgets` combines both roles in one strict packet and one 128-slot
budget; the BrowserNetwork driver accepts matching force/Scene/children together.
The original node-only payload type is preserved. Link Widgets add no canonical
node controls or force-drag targets. Their source identity is `force-link:index`:
edge reordering keeps index-local state; explicit revision rebuilds it rather
than inferring business-edge identity.

The final strict script passes all four browsers at DPR1/2 and four ordered/top,
replace/keep profiles: **32 contexts, 832 SVG/Canvas frames,
256 trusted input groups, 384 rejected candidates,
64 exact business-stop receipts and 32,448 RGBA samples**.
The same-artifact Chromium subset passes **8 contexts,
208 frames and 8,112 samples**. Independent Python/Pillow decoding
reconstructs authored primitives, source order and selected replacements, then
checks all **1,040 frame PNGs and 40,560 samples**.
Maximum channel difference is 0; allowance stays ±1.
Measured DOM and mapped world positions keep the separate 1/32 logical-pixel gate.

The bounded profile explicitly authors a solid offset Path, a red 12×16 parallel
link rectangle and a 14px square B node. Later source ink covers earlier link
Widgets in ordered mode; top Widgets paint above it. The oracle independently
applies complete source/Widget order, including source coverage and later Widget
coverage. Exposed side pixels, covered centers and native activation/blocking
are all checked. It does not infer expected colors from actual DOM or PNG colors.
Real ticks, source dragging, CSS scale/resize, camera changes, replacement,
reorder, empty source and revision are included. Retained Bond/owner snapshots
stay coherent; source-role placement/factory/build failures preserve accepted
state, full DOM, SVG styles, native Canvas buffers and geometry exactly.

Eight negative controls fail intended gates: jointly forged raw-source readouts,
wrong actual Button paint, smaller measured DOM, an actual accepted graph update
disguised as rejection, its separate deep-witness variant, hidden Canvas backing
corruption, and native TypeError/RuntimeError after a real callback stop.
The hidden edit leaves full viewport PNG bytes and all other accepted witnesses
unchanged. The native traps are rethrown by the original class classifier,
never recorded as business-stop success. Sixteen protected files remain identical.

Authoring failures remain archived: offset Rect was correctly rejected (Path is
required); initial foreground expectations ignored real later source ink;
positive camera translation exceeded existing constraints; and empty ordered
graphs legitimately lack a Canvas plane. Nonempty graphs retain real nonzero
SVG/Canvas qualification. After an earlier complete cohort passed, endpoint
delivery and stop-exception guards were strengthened, and the entire final
cohort was rerun. No prior partial results or old negative receipts are counted
as final-cohort passes.

Whole-viewport equality remains diagnostic: **29** mismatches in the
full matrix, **0** in CI. Mismatch PNG pairs are preserved and independently
decoded. Source order, local paint, geometry and strict rollback acceptance do
not resolve historical arbitrary compositor/glyph, thin-line AA, Bar font8 or
native scriptless-SVG failures.

Native regression passes **1783 minimal-feature tests, 2112 all-feature tests,
18 doctests**, native/WASM strict Clippy and a consumer WASM release build.
The standalone complete Memory example executes actual ticks and retained counts.
The unchanged node-only driver regression passes 8 contexts/304 frames on the
same new WASM; separate Pillow readback checks its 7920 samples with max error 0.
The new original Nivo reference executes 32 profiles, 224 creation calls and
28 record-only Canvas commands; two offline runs and seven tamper controls pass.
These source recipes do not execute actual React/Spring or D3 force trajectories.

Evidence: `target/network-link-widgets-validation/acceptance.json`,
`target/network-link-widgets-browser-strict-final/*/report.json`,
`target/network-link-widgets-browser-ci-strict-final/report.json`,
`target/network-link-widgets-final-review/readback.json` and
`target/network-link-widgets-browser-negatives-strict-final/review.json`.
See [the guide](network-link-widgets.md) and [中文指南](network-link-widgets.zh-CN.md).
Automatic retiring components, arbitrary Widget export, explicit business-edge
keys, native same-layer/OS input, more style profiles and GPU/worker remain open.
Published 0.1.0 is unchanged; new CI steps are locally exercised workflow edits.

## Development: Funnel function and live layer roles

Unreleased Funnel now accepts borrowed raw-datum color, value-format,
computed-part border-color and label-color functions. They compose with custom
stage geometry without replaying an explicit label-color function. Original
profile body and open-side geometry helpers require an explicit interpolation
mode; their profiles remain distinct from custom body ink and bounds.

Typed live roles attach only to actual body, border and before/after separator
Marks. They share the complete owned source, computed parts, separators, labels,
clips and label ownership. Source/profile/role validation precedes placement;
zero pixel bodies create no fake role, and closing separators have no invented
source datum. Original stage centers and separator midpoints determine anchors.
Supplied source and public layout must be coherent, without pretending the
original constructor configuration is an immutable authenticated snapshot.

Original Nivo Funnel has custom function layers, not partComponent,
labelComponent or separatorComponent. The frozen reference executes 16 style,
4 handler, 8 layer-creation and 2 default JSX profiles from 19 full source and
package files, with three complete MIT notices. Two offline checks and eight
tamper controls pass. This executes source recipes with explicit dependency
probes, not React/Spring lifecycle or D3 path numerical equivalence. Rust keeps
batch size evaluation and completed profiles in style contexts; upstream style
functions observe temporary parts before profile population.

Native integration passes **1811 minimal-feature tests, 2141 all-feature tests,
18 doctests**, native and WASM strict Clippy, and a release WASM consumer build.
The complete standalone Memory example executes successfully and is reproduced
verbatim in both guides. Thirty-two old compiled default profiles retain
byte-identical full layout metadata, Scene and SVG output in the final build.

Two production regressions discovered during implementation were corrected:
preflight now compares full Scene coordinate equations instead of rejecting
valid tiny reconstructed offsets, and an unused disabled Contrast border keeps
the old transparent-surface admission rule. Original REDs and actual old-binary
proof remain archived. Explicit style callbacks still request a resolvable
default context, and external callback side effects are not rolled back.

The final four-engine/DPR1/2/four-profile matrix passes **32 contexts/1280 frames,
384 trusted input groups and 704 rejected candidates**. A separate Chromium
CI subset passes **8 contexts/320 frames, 96 input groups and 176 rejections**
on the same frozen package and driver. Source-ordered/top and keep/replace
composition covers actual SVG/Canvas, signed-zero raw data, reorder, custom
offset body, horizontal/smooth/current modes, zero pixel bodies, absent labels,
empty/skip, CSS scale, resize, camera, hidden state and explicit revision.
Actual retained Node and activeElement identity, Enter activation, selected
canonical sources, and exact native backing PNG rollback are runtime gates.

Independent Python/Pillow reconstruction checks all **1600 frame PNGs,
54720 foreground/source-body samples, 3200 source-color censuses and 2960
actual source-color witnesses**, including raw bit sidecars, full parts,
separator paths, labels, owned role eligibility, original anchor and camera
mapping. Sample max channel difference is **0**; source-color census max is
**1** under the declared per-channel tolerance. Maximum actual DOM geometry
error is **0.012500000000045475**, below `1/32` Scene pixel. Source labels remain
20px and Widget content 0px; the census proves globally visible unique source
paint, not every glyph contour or every stage's stroke coverage.

All six target-only negative controls fail at the intended gates: jointly
forged full original source/part raw bits, wrong actual Widget paint, shrunken
actual DOM despite all nine sample points remaining inside, a real bit-changing
commit disguised as rejection, the deeper complete rollback witness, and one
hidden native Canvas pixel. The last changes only backing pixel `(460,59)`;
full viewport PNG bytes remain identical and the foreground remains blue.
Fourteen protected files retain their exact hashes throughout the controls.

The first camera oracle omitted plot-origin translation. A separate WebKit
failure sampled real tooltip white text covering the tail Widget after CSS
scale. These REDs remain preserved: the final camera equation is independently
authored, and actual pointer movement outside surfaces precedes unobstructed
paint probes without changing tooltip CSS or native focus. The initial exact
source-color census failure also stays RED; final census explicitly records
both exact counts and counts under the already declared one-channel tolerance.
The final entire browser cohort and negative controls were rerun after the
pointer correction; earlier partial results are not reused as final passes.

The first independent reader had 360 index-only failures because it floored an
ideal camera coordinate of `120` instead of the validated actual floating-point
coordinate `119.99999999999993`. Both pixels are independently decoded blue.
The corrected reader retains independent ideal geometry checks, then applies
one exact floor to the validated actual sample point. No pixel-index allowance
or color tolerance was added; the entire 1600-frame readback was rerun and the
first reader/script/log plus focused coordinate proof are preserved.

Whole-viewport equality remains diagnostic: **48** mismatch pairs in the full
matrix (Chrome 22, Edge 26), **0** in CI. Independent decoding finds 478 changed
pixels per pair, 22944 in total, maximum channel difference 99. All mismatch
PNG pairs remain archived. This is not a strict whole-image pass and does not
resolve earlier compositor/glyph, Bar font8, scriptless SVG or thin-line AA
failures.

Evidence: `target/funnel-widgets-validation/acceptance.json`,
`target/funnel-widgets-browser-strict-final/*/report.json`,
`target/funnel-widgets-browser-ci-strict-final/report.json`,
`target/funnel-widgets-final-review/readback.json`,
`target/funnel-widgets-browser-negatives-strict-final/review.json` and
`target/funnel-layer-widget-frozen-audit/report.json`.
See [the guide](funnel-widgets.md) and [中文指南](funnel-widgets.zh-CN.md).
Arbitrary custom-layer positions, generic RawDatum extra fields, automatic
retiring/Spring components, arbitrary Widget export and native same-layer/OS
input remain open; GPU/worker stay deferred. Published Glory 0.8.0 and Kartis
0.1.0 are unchanged. New CI commands are locally exercised workflow edits;
they are not claimed as remotely executed against this dirty development tree.

## Development: Tree node and link components

Unreleased `tree_layout` captures the original flat input, source-order nodes,
breadth-first links, parent/children, depth/height, raw and aggregate weights,
true logical centers and actual link endpoints/midpoints in the same existing
Tree calculation. `tree_with_options` delegates to that calculation and retains
its Scene. **200** real old/new compiled profile executions preserve complete
Scene Debug and SVG bytes across five orientations, two modes, five curves,
empty/single/root-not-first sources, styles, clips and label configurations.

`tree_node_widgets`, `tree_link_widgets` and mixed `tree_widgets` share owned
source, node/link and current label/clip/ownership pools. They validate captured
metadata and actual managed Marks before the first placement callback, and use
one total 128-slot budget. Current legal paint, measured labels and offset Paths
may differ from constructor ink; original node centers and true link midpoints
remain the placement anchors. Exact zero radius is newly supported; existing
positive-radius validation ranges remain. Disabled links retain metadata with
no invented Mark/role; zero, transparent and no-ink actual nodes retain roles.
Only labeled nodes provide canonical controls; links and empty-label nodes do
not create fake business sources or physical ink.

Nivo's real `nodeComponent`/`linkComponent` and Canvas `renderNode`/`renderLink`
are the reference use cases. The frozen source executes 32 computed, 16 creation
profiles (80 node and 64 link calls), 8 handler, 36 actual Canvas-function cases
and 8 actual Canvas-layer-loop profiles, from 14 complete source/package files
and the full Tree MIT notice. Two offline runs and eight tamper controls pass.
Authored traversal/coordinates/scales/hooks are explicit dependency probes,
not D3 numeric or React/Spring lifecycle equivalence. Nivo nested raw objects
and path UIDs differ from Rust flat globally unique IDs. Nivo does not supply
the Rust aggregate-weight model; Rust internal raw weights require zero.
Tree continues to emit its root regardless of the shared `include_root` field.
The radial fifth orientation is a Rust extension.

Native integration passes **1822 minimal-feature tests, 2154 all-feature tests,
18 doctests**, native/WASM strict Clippy and consumer release WASM build.
Owned and independent tests include conflicts rejected before callbacks,
shared pools, exact raw signed-zero updates, actual Memory Node/count/focus
retention and CPU DPR1/2 rejected-publisher PNG rollback and recovery.
The complete standalone Memory application executes successfully; both guides
reproduce its source exactly.

The complete final four-engine/DPR1/2/four-profile matrix passes **32 contexts,
2048 frames, 384 trusted input receipts and 640 rejected candidates**; a separate
Chromium CI subset passes **8 contexts, 512 frames, 96 inputs and 160 rejections**.
Profiles cover source-order/top and keep/replace, all directions, mode/curve,
raw-bit-only and nonzero source changes, real source selection, actual retained
DOM identity/count/focus, Enter activation, custom offset Paths, disabled links,
absent/empty labels, zero radius, no fill, zero opacity, empty/single sources,
skip/revision, CSS scale, resize, camera and hidden state. A declared overlap
phase puts the link Widget under a real solid node: source-order profiles show
source paint and trusted clicks select that node without incrementing the
Widget; top profiles show Widget paint and route the trusted click to it.

Independent Python/Pillow reconstruction reads all **2560 frame PNGs** and
**65680** foreground/source-interior samples, **5120** exact/tolerant source
color censuses and **4720** actual color witnesses. Raw bits, full computed
topology/style/labels, original Rect/offset Path/straight/Step commands,
anchor/camera mapping, actual DOM geometry, canonical text and input/rejection
receipts are independently checked. Foreground max channel difference is
**0**, source-census max is
**1**, and actual DOM max error is
**0.016668701171866473** under the existing `1/32` Scene-pixel threshold.
Source labels remain 20px and Widget content 0px. The census establishes globally
visible unique source paint, not every glyph contour or every link coverage.

Eight target-only negative controls each fail at the intended gate: jointly
forged source/raw-node bits, forged typed style, a forged geometry readout,
actual wrong DOM paint, shrunken actual DOM despite nine probes still inside,
real phase1 commit disguised as rejection, its deeper complete rollback
witness, and a hidden native Canvas pixel. Geometry readout forgery is explicitly
not an actual renderer mutation. Independent Pillow decoding confirms only
backing pixel `(430,137)` changes from transparent to red; full viewport PNG
bytes, accepted state/DOM and the blue foreground remain identical. Seventeen
protected files retain exact hashes throughout these controls.

Implementation-time REDs are preserved. An original binary rejected an invalid
positive internal-weight fixture; a Memory slot crossed the plot edge; a
snapshot ActiveInteraction was mistaken for the full canonical binding; source
probes hit real Widget/link/glyph ink; and zero-opacity host suppression was
mistaken for visible DOM geometry. These were fixture/oracle corrections,
without renderer changes or relaxed tolerances. Exact source/raw bit updates
are separate from compatible numeric Mark equality: canonical scalar zero may
retain `-0` or show `0`, while nonzero changes must update exactly. The final
driver enforces that narrow zero display set and the exact original bit sidecars.

All three pre-freeze gaps (typed style, actual source geometry and empty-label
Nodeb identity/count) were closed in the final C182 driver. The earlier readonly
receipt mixed content from an earlier script with a later hash and is preserved
as unreliable for that script; its replacement reads an exact saved snapshot.
The first full matrix lost three running processes during application permission
revocation. Missing handles and OS process inventory confirmed termination;
complete/partial outputs remain archived. All five final reports were rerun in
fresh directories on the same frozen source/package/driver, with no partial
results reused. A copied static check initially ran Funnel's reference; final
quality explicitly executes the frozen Tree reference, preserving the old record.

Whole-viewport equality stays diagnostic: **44** mismatched PNG
pairs in the full matrix and **0** in CI. All differing pairs are
independently decoded and retained; they do not become a strict framebuffer
pass or resolve historical compositor/glyph, Bar font8, scriptless SVG or
thin-line AA failures.

Evidence: `target/tree-widgets-validation/acceptance.json`,
`target/tree-widgets-browser-complete-final/*/report.json`,
`target/tree-widgets-browser-ci-complete-final/report.json`,
`target/tree-widgets-final-review/readback.json`,
`target/tree-widgets-browser-negatives-strict-final/review.json` and
`target/tree-widget-independent/acceptance.json`.
See [English guide](tree-widgets.md) and [中文指南](tree-widgets.zh-CN.md).
Tree label components, arbitrary original layer positions, automatic
Spring/retiring components, Widget export and native same-layer/OS input remain
open. GPU/worker stay deferred. These development APIs are Unreleased;
published Glory 0.8.0/Kartis 0.1.0 are unchanged. New CI commands and workflow
syntax are validated locally, without claiming this dirty tree ran remotely.

## Development: Stream function configuration

Unreleased `StreamFactories` supplies eight synchronous borrowed roles: layer
label/color, per-value formatter, area border color and dot size/color/border
width/border color. `stream_layout_with_factories` and its dot-geometry-factory
variant use the authoritative build. Full original validation/projection/path
admission precedes callbacks; each layer formats every Some sample once, then
resolves label/color, complete-layer area border and complete-layer dot styles.
The area summary reuses the final sample's formatting, including final None.
Explicit functions are never retained. Empty labels, nullable samples, hidden
layers, zero size/width and existing source anchors keep documented eligibility.

Functions propagate through actual Mark ink, legends, bindings, slices,
tooltips and typed dot data. A custom geometry callback has final shape authority;
its declared style remains distinct from resolved fill/stroke. Dot-size functions
cannot silently mix circle diameter with configured glyph area/nominal size.
Other styles combine with glyphs; stroke-only glyphs require actual positive
width before custom geometry is called. Static ordinal metadata remains its
declared mapping, not an invented cache of arbitrary function results.

The original Nivo reference executes actual passthrough, enhanced-layer,
area-stroke and dot creation/default-circle recipes: **24 profiles, 72 dot calls**,
four disabled-layer profiles and sixteen gates, from **19** complete source/package
files and **three** complete MIT notices. Two offline checks and nine tamper
controls pass. Stack/scale/path/hook dependencies are explicit authored probes,
not D3 numeric paths, React/Spring lifecycle or arbitrary JavaScript getter
execution. Rust Geometry and nullable flat series retain their distinct contracts.

Three real implementation regressions were found and corrected with preserved
REDs: an explicit source color was rejected by an unused static Missing lookup;
hidden1024-byte labels were wrongly charged for nonexistent visible sample text;
and hidden Natural areas generated unused out-of-budget control hulls despite
valid sample projections. Actual old compiled binaries independently prove the
two hidden cases were previously accepted. Final code validates only required
paint/path work while explicit path functions still require bounded Geometry.
No coordinate, text or paint budget was relaxed.

Final integration passes **1842 minimal-feature tests, 2175 all-feature tests,
18 doctests**, native/WASM strict Clippy and a release consumer WASM build.
The standalone complete Glory Memory application runs successfully and its final
source additionally passes Rust1.95.0 strict Clippy. Both portable guides reproduce
the complete program verbatim. Its explicit PaintedGeometry binding and actual
SurfaceModel preflight satisfy the existing bound-Chart contract. The initial
missing-profile example failure remains preserved. **96** actual old/new compiled
Stream profiles retain byte-identical full layout metadata, Scene and SVG output.

The final four-engine/DPR1/2 matrix passes **8 contexts/304 frames, 16 real inputs
and 192 rejected candidates**. The separate Chromium CI subset passes
**2 contexts/76 frames, 4 inputs and 48 rejections** on the same frozen package,
source and driver. Cases cover all eight functions, reverse source order, hidden
layer, disabled/zero dots, empty labels, zero borders, offset Path geometry, last
None, glyph combination, transparent area, empty source, CSS scale, resize and
camera/reset. Actual legend controls, independently authored complete function
logs/metadata and precise Rect/Circle/Path commands are checked. Four additional
size-sensitive interior axis probes per painted dot distinguish callback sizes
from the old static diameter. Native delivered click coordinates/counts are gated,
and the original runtime checks the clicked sample's formatted interaction row.

Independent Python/Pillow reconstruction checks all **380 captured PNGs and
21120 samples**, maximum RGBA channel difference **0**: 13680 ordinary dot probes,
6080 size-sensitive interior probes, 700 area-interior and 660 area-border probes.
It independently reconstructs original source/category bit sidecars, Zero stacks,
full layers/slices, function invocation order, formatting, actual geometry/paint,
bindings and actual legend text. Twenty no-action witness pairs are stable;
240 rejected-candidate witnesses retain exact accepted model/source/log, DOM,
native SVG computed paint, Canvas backing PNG bytes and capture geometry.
Additional independently computed Orange/Purple ROI histograms are ungated
diagnostics; the driver has no invented per-frame census fields to compare.

All **eight** target-only negative controls fail at the intended gates: forged
function order, jointly forged raw/sample values and bits, forged reported radius,
actual SVG red fill, actual SVG twenty-pixel translation, a real accepted phase1
commit disguised as a label rejection, a hidden Canvas pixel, and forged actual
legend text. Radius readout forgery is explicitly distinct from the actual SVG
geometry mutation. The hidden Canvas control establishes an extra opaque white
DOM overlay before its baseline; only underlying pixel `(700,310)` changes white
to red. Independent Pillow confirms exactly one backing pixel changed while
viewport PNG bytes, accepted state, DOM and geometry remain identical. This is
a target-only overlay probe, not a claim of natural production Widget occlusion.
All seventeen protected file hashes remain unchanged through the controls.

Authoring REDs are retained. Numeric category -0 and the two-byte n0 identity,
filled Square Path geometry and real legend action prefixes were corrected in
independent expectations. A read helper incorrectly called explicit plain
BrowserChart.flush; a no-candidate experiment advanced revision8 to9 because
flush performs a repaint. Final rollback witnesses issue no repaint and retain
exact revision checks plus their own no-action stability gate.

The two-pixel border/CSS Canvas color failure stays RED; a separate authored
six-pixel solid-border carrier verifies representative callback color. Another
original Firefox sample overlapped actual Square glyph paint; independent
decoding of the same original PNG proves a farther point on that same border
is exact Orange. Later size-sensitive r6 probes decoded mixed colors despite
correct geometry metadata; this does not establish a general size bug or the
unique underlying coverage cause. Those REDs stay RED. The final authored
diameter16+4index carrier uses deeper solid fill, retaining size-sensitive gates
and the same RGBA1 tolerance. Whole final cohorts were rerun after changes;
previous complete or partial results are not reused as final passes.

The first independent reader incorrectly expected capture-time ActiveInteraction
to retain the immediate click row after the real pointer had moved outside.
It crashed, and its script/log/record remain preserved. The final reader checks
the persistent selected ID and accepted binding independently; the immediate
formatted interaction assertion remains original runtime evidence. The entire
380-frame readback was rerun. A negative-control finalizer also initially expected
SVG paint styles to change with source values even though only geometry changed;
that receipt-authoring mistake is preserved separately from the unchanged eight
executed negative scripts/reports, with actual source/revision/DOM changes checked.

Whole-viewport equality remains diagnostic: **22** mismatched pairs in the full
matrix (Chrome10, Edge12), **0** in CI. All pairs are independently decoded,
497 or498 changed pixels each, **10955** in total, maximum channel difference99.
No strict whole-image acceptance or arbitrary glyph/AA/OS equivalence is claimed,
and previous thin-line, compositor, scriptless SVG and Bar font8 failures remain.

Evidence: `target/stream-functions-validation/acceptance.json`,
`target/stream-functions-browser-solid-final/*/report.json`,
`target/stream-functions-browser-ci-solid-final/report.json`,
`target/stream-functions-final-review/readback.json`,
`target/stream-functions-browser-negatives-solid-final/review.json` and
`target/stream-function-independent/final-admission-acceptance.json`.
See [English guide](stream-functions.md) and [中文指南](stream-functions.zh-CN.md).
Arbitrary Stream custom-layer placement/lifecycle, automatic Spring/retiring
components, arbitrary Widget export and native same-layer/OS input remain open;
GPU/worker stay deferred. These additions remain Unreleased, with published
Glory0.8.0/Kartis0.1.0 unchanged. New CI commands are locally exercised workflow
edits, not claimed as remotely executed against this dirty development tree.

## Development: Tree function configuration

`TreeFactories` adds five borrowed base functions for node diameter/fill,
computed-link thickness/color and drawing-label text. Intermediate node contexts
have complete source/topology/position; link contexts have both fully resolved
nodes. Breadth-first nodes run size then color, followed by all computed links
and eligible labels. Disabled links still compute styles, zero-width enabled
links keep decorative Marks without strokes, and explicit colors override None
fill. Label results do not rewrite raw datum or canonical business labels.

The existing static-style projection inset remains authoritative. Function
diameters change actual geometry and label spacing without reprojecting nodes.
Rust's shared visibility-gated label calculation differs from original Nivo
Canvas's unconditional calculation. Active/inactive modifiers and fixed-depth
color inheritance are not implemented by this five-function bundle.

Root checks pass **1855 minimal / 2189 all-feature tests and 18 doctests**, plus
exact Rust 1.95.0 native/WASM strict Clippy. Owned tests pass 8/8 and independent
tests pass 5/6, including real retained Memory children with original bit changes,
state/count/focus preservation and rejected candidates. CPU DPR 1/2 publisher
failure preserves accepted PNG, typed data and controls. The standalone complete
Memory application runs successfully and is reproduced exactly in both guides.
Two actual compiled binaries compare **200 profiles** of full TreeLayout metadata
and SVG byte for byte, including five orientations, two modes and five curves.

The frozen Nivo 0.99.0 reference retains 19 full source/package files and 3 complete
MIT notices. It executes 48 function/active/fixed-depth profiles,384 label-position
profiles and separate 6 SVG/6 Canvas label gates. Two offline runs pass and nine
valid tampered copies fail. Hierarchy coordinates/scales/hooks/Spring are explicit
dependency probes; these results do not establish D 3 or React runtime equivalence.

Final four-engine Chrome/Firefox/WebKit/Edge DPR 1/2 acceptance passes
**8 contexts/368 frames,16 native inputs and 160 rejected candidates**. Separate
Chromium CI passes **2 contexts/92 frames,4 inputs and 40 rejections**, on the same
frozen source, driver and consumer package. Cases cover all five functions,
child-before-parent source, source reversal, zero and transparent nodes, zero and
disabled links, empty function labels, both label gates, all five orientations,
Square geometry, empty/single source, CSS scale, resize, camera and reset.
Accepted source bits, complete computed metadata, function logs, actual SVG text,
business controls, geometry, paint and original-label picking are checked.
Twenty no-action witnesses are stable, and 200 rejected-candidate witnesses retain
exact accepted source/log/revision, DOM, computed SVG paint, Canvas backing PNG
and capture geometry. Runtime errors are not relabeled as expected rejection.

There are **16160 gated viewport samples**, maximum RGBA channel difference 0.
Canvas stroke width additionally uses **420 native cross-section ROIs / 72720
pixel comparisons**, maximum channel difference 0 against independent browser
Canvas line primitives. Each reference canvas remains detached from DOM and
never enters Kartis. Source endpoints, widths and requested camera come from
the independently authored oracle, not accepted geometry/paint metadata.
Nine physical scanlines per straight-segment ROI compare every RGBA channel
within 1; Orange-over-white coverage integral bounds follow directly from that
per-pixel byte bound. There is no separately fitted integral tolerance.
Actual backing PNG bytes are stable across each viewport screenshot. Reference
and actual PNGs are retained; these finite ROIs avoid nodes, labels, endpoints
and clips, and do not establish arbitrary stroke caps or full-image equivalence.

The first strict viewport run failed at a Canvas CSS-scaled oblique-line sample.
Independent decoding preserves that RED. A separate diagnostic matrix records
12 such failures across four engines; native backing PNG bytes before/after the
CSS-only step are identical, with nearby AA pixels in the sampling neighborhood.
This bounds the difference to composition of the same bitmap without identifying
a unique kernel or claiming a renderer repair. That pilot matrix is not final
package evidence: WebKit's end fence correctly detected the package promotion.
Its original processes were followed to terminal, and a new frozen final cohort
was run. No old runs were relabeled as final passes.

In the final cohort, the **760 original Canvas width-interior viewport points
remain ungated diagnostics**, with 15 failures of the pure-source-color assumption
(12 full-matrix and 3 CI). They are not claimed green. Exact center/outside and all
other viewport gates remain, as do SVG's original width-interior gates. The new
complete native contour comparison supplies a stronger actual-width measurement
without changing the fixture's line widths, source geometry or RGBA tolerance.
Whole-viewport equality also remains diagnostic:13 mismatched pairs (Chrome 6,
Edge 7),0 in CI. Independent Python/Pillow reconstruction decodes 946 PNG files
(460 viewport,460 actual/reference native and 26 full-image diagnostic files),
recomputes complete source/geometry/function/camera expectations and all gated
samples/ROIs, and confirms the same maxima. The 13 full-image pairs contain 9515
changed pixels with maximum channel difference 99. Previous compositor, thin-line, font and scriptless-SVG failures
remain outside this bounded acceptance.

Eight target-only negative controls fail at their intended gates: forged callback
order, jointly forged raw/node bits, actual Canvas stroke thickness, actual SVG
fill, SVG translation, a real accepted commit disguised as rejection, a hidden
Canvas backing pixel, and actual drawn SVG label text. The thickness control
changes only actual host painting from 6 to 1.5 while accepted metadata remains 6;
two real source strokes are changed, while two detached reference strokes stay 6.
Its original failed ROI decodes white at `(325,131)` against unchanged baseline
and reference `[248,213,186,255]`. The first wrapper copy had an authoring
ReferenceError and is preserved separately; it is not counted as a successful
width negative. The corrected copy reaches the actual contour gate.

Independent decoding confirms that the hidden Canvas control changes exactly
one backing pixel `(700,310)` from white to red, while accepted snapshot, DOM,
capture geometry and viewport PNG bytes stay identical. A target-only opaque
white overlay is established before the baseline; this is not a claim of natural
production Widget occlusion. All 17 protected files remain unchanged. Read-only
independent negative review and full PNG reconstruction accompany the executed
reports; no target mutation enters the library or its actual driver.

Evidence: `target/tree-functions-validation/quality-tree-functions-final.json`,
`target/tree-functions-native/acceptance.json`,
`target/tree-function-independent/acceptance.json`,
`target/tree-function-reference-audit/report.json`,
`target/tree-functions-browser-final/summary.json`,
`target/tree-functions-browser-ci-final/report.json`,
`target/tree-functions-browser-negatives/review.json` and
`target/tree-functions-final-review/acceptance.json`.
See [English guide](tree-functions.md) and [中文指南](tree-functions.zh-CN.md).
New workflow commands are locally exercised edits, not remote CI proof for the
uncommitted development tree. These APIs remain Unreleased; published
Glory 0.8.0/Kartis 0.1.0 and their tags are unchanged. Activity modifiers and
fixed-depth inheritance are covered by the later section below. Label components,
arbitrary layers/retirement/Widget export and native same-layer input remain open;
GPU/worker remain deferred.

## Development: Tree activity, inherited colors and browser owner

`tree_layout_with_activity` adds four borrowed active/inactive size/width
modifiers, four independent ancestor/descendant node/link flags and fixed-depth
parent-color inheritance. No current node means neutral classifications and no
modifier calls. Final node sizes feed links, labels and typed components. Color
inheritance skips deeper color callbacks, including when the parent has no fill.
The result binds activity indices to its original captured layout. An actual
first RED accepted a whole-layout graft and invoked placement nine times; the
corrected preflight rejects equal-length reordered and different-topology grafts
before any placement call, while preserving legal same-capture Scene edits.

Root checks pass **1870 minimal / 2205 all-feature tests and 18 doctests**.
Exact Rust 1.95.0 native and final-owner WASM strict Clippy pass. Owned tests pass
8/8; independent tests pass 7 minimal/8 all-feature, including retained Memory
identity/count/focus, raw signed-zero updates, explicit application activity
publication after canonical Focus/PlotPointer proposals and CPU DPR1/2 publisher
failure preserving the accepted PNG, typed data and controls. The complete
standalone Memory application runs and appears byte for byte in both guides.
Actual compiled old/new binaries preserve **200 default full TreeLayout/SVG
profiles and 34 five-function profiles**, including complete callback logs.

The frozen Nivo reference retains 24 full source/package files and four complete
MIT notices. It executes 576 core profiles, 16 zero cases, 6 numeric boundary
probes, 8 node-handler profiles and 4 mesh profiles. Two offline runs pass;
12 valid tampered copies fail. Hook, hierarchy, scale, Delaunay-query and
event-coordinate dependencies are explicit probes, not D3/React/OS runtime proof.

The public BrowserTree reserves native primary non-touch pointer work and builds
it in a separate owner RAF. A successful paint and strict Widget-stage adoption
move the accepted snapshot together. Geometry uses canonical painted node hits;
nearest activity separately indexes logical centers, including zero-size,
transparent and empty-label nodes, without inventing chart hits or tooltips.
Stationary or publishing-generated repicks do not replay factories. Actual first
REDs exposed non-primary/touch leave clearing activity and programmatic Focus
clearing Geometry current without a native move. Corrected protocol probes retain
current in both cases and still accept genuine mouse move/leave. Synthetic touch
and programmatic DOM Focus probes do not establish OS touch/keyboard acceptance.

Seven focused owner suites pass **23 cases** on the final consumer package.
They include ordered/overlay Widget adoption, 88 accepted Scope-effect observations
and 176 accepted two-row comparisons, plus eight explicitly tentative construction
observations. The projection covers raw source, topology, geometry, color and
activity; it does not serialize every TreeNode style or the full label pool.
Four eligibility cases exercise transparent and empty-label nodes with actual
mouse input and independent PNG decoding of 36 samples. These bounded probes do
not replace the full browser matrix.

Original authoring REDs are preserved. Inline button baselines placed the actual
component six pixels below its independently expected rectangle; the fixture now
declares block layout. No production positioning tolerance changed. Native
Canvas `getTransform` quantizes some label translations beyond the old f64-only
assumption; actual label call/style/CTM is instead compared exactly with a
detached standard Canvas receiving independent source-derived coordinates and
the same declared transform operations. This checks native calls, not every
glyph's pixel outline or cross-browser font parity.

The original mathematical pure-Orange quarter-width assumption also fails on
an actual inactive 4px SVG diagonal. An independently declared standard SVG
reproduces the original failed pixel exactly. Stronger nine-scanline reference
experiments, however, do not establish equivalent antialiasing: both distant
integer translations and same-origin opaque reference constructions retain
edge differences, including maxima 24..35 after public viewport/CTM declarations
match. Original DOM, accepted state and source-region hashes restore exactly
after removing each test carrier. These experiments remain RED and are not
accepted as a pixel oracle or a renderer repair. Width-interior pure-color
samples are diagnostics. Actual SVG path endpoints, computed width/color,
opacity/camera and bounded center/outside/node-size samples remain hard gates;
native Canvas line cross sections retain the independent primitive gate.

A pixel's source cannot be inferred from an ideal geometric point alone. A
later SVG exterior sample is Orange/White despite lying beyond the direct
pixel-square/half-diagonal capsule bound. This counterexample is retained;
that bound is not expanded or accepted as a spatial oracle.

For a node probe independently outside its own visible circle, with an ideal
Scene-order winner of White background or an Orange line and an actually
paintable Orange stroke somewhere in the scene, a separate chroma-only witness
requires one value in 0..1 to explain all RGB channels as declared Orange over
White within one byte, with opaque alpha. Eligibility never reads actual RGBA.
The original point-color comparison remains diagnostic. This tests the permitted
foreground color family, not pixel/stroke intersection, exact source location,
coverage, line width, exterior spatial whiteness or absence of extra same-Orange
ink. Blue/Purple overpaint is rejected, but a same-Orange ghost need not be.
Actual geometry and other gates supply separate evidence. True node interiors,
Widget/other source roles, White when no Orange stroke is paintable, and the
complete empty surface keep their original RGBA gates. Nonzero node ink retains
center, half-radius and radius-minus-four probes; native Canvas line contours
and actual SVG paths/sizes/computed styles remain independently gated.

Pointer-current and keyboard activation are tested separately. Native Tab
experiments show two platform effects: Safari mouse clicks need not focus a
button, and keyboard scrolling can move the surface away from a stationary
pointer and emit a genuine primary mouse leave. That leave may correctly clear
current. Reusing a sequential-navigation anchor after retained DOM migration
also differs between engines. The original REDs and successful bounded Tab
controls remain diagnostics. Formal keyboard activation explicitly establishes
focus on the actual retained button with `focus({preventScroll:true})`, records
this as programmatic setup, checks unchanged NodeID/count and accepted owner,
then delivers Enter through the actual browser keyboard transport. Same-child
activation is checked, but a separate serialized `keydown.isTrusted` receipt is
not retained. Subsequent updates and rejected candidates
must preserve actual document focus. This does not establish universal Tab
navigation or OS-mediated focus acquisition.
Widget clicks first deliver a real hover and settle the owner's coalesced RAF,
check unchanged identity/count, then deliver the real click. Immediate move/down
stress races are outside this finite input sequence.

A full-prefix WebKit trace confirms a real unresolved lifecycle risk, rather
than just a test assumption: move/down target DOM80, an activity commit replaces
six source-plane/hit-mask nodes, and up targets Surface DOM19. No fresh click is
delivered and selection remains None. Evidence is retained at
`target/tree-activity-v4-source-click-transport-diagnostic/run/report.json`.
Surface up-retargeting is also normal pointer-capture behavior, so that target
alone does not establish causation. Scene replacement cancels the shared gesture
and is another concrete lifecycle candidate; a unique cause is not isolated.
Settled-hover click acceptance does not repair or cancel this RED. Rapid native
click lifecycle acceptance remains false and is the next priority before more
Tree configuration work.

The final frozen execution passes **20 contexts / 3300 frames / 720 rejected
candidates**: Chrome, Firefox, WebKit and Edge each run DPR1/2 with Geometry/keep
ink and Nearest/replace ink, followed by the independent Chromium CI subset.
All five processes reach exit 0 on the same driver, source and consumer hashes;
earlier failed cohorts remain separate. Cases include all 16 flag combinations
at three inheritance depths, actual branch descendants, all nine functions,
zero/hidden/disabled/empty sources, signed-zero-only replacement, source reorder,
successful Widget revision, cleanup and real picking after combined CSS scaling,
resize and camera changes. There are 160 stationary witnesses and 40 no-action
witness pairs, plus complete accepted DOM/native paint/data rollback checks.

The 740 input records comprise 380 actual moves, 80 actual Widget hovers, 40 Widget
clicks, 40 source clicks, 80 programmatic focus setups, 40 chart-only dispatches
and 80 synthetic ignored-leave probes. They are not 740 native actions. Forty
additional browser-keyboard Enter activations use the protected executing
script and retained-child count checks. Final Scope evidence contains 6354
accepted observations and 200 construction observations: 120 independently
identified new-role constructors and 80 revision constructors. Original boolean
fields remain visible; membership, rather than an inconsistent-data result,
determines the constructor classification. Accepted selected rows still compare
their entire serialized source/topology/activity projection with their owner.

There are 229640 exact-color viewport samples and 6440 native Canvas cross-section
ROIs covering 1291230 pixels; both recorded maximum channel differences are 0.
The separate 35660 chroma-only witnesses do not count as spatial RGBA successes.
All 61420 ideal-point diagnostics remain ungated, including 5563 pure-point color
failures. Empty-source interiors check 24309840 physical pixels for White.
Actual SVG geometry/style is inspected in 1650 frames, and 8010 native Canvas
label calls retain their text/style/independent native CTM evidence. Whole-page
equality is still diagnostic: 31 rejection-witness pairs have different PNG bytes.
Independent Python/Pillow reconstruction reaches exit 0 after decoding 9882 PNG
files and 2220 embedded witness PNGs, recomputing the source/function/input/DOM
and pixel expectations. The 31 whole-page pairs contain 17135 changed pixels
with maximum channel difference 99; these remain ungated. The first full reader
run had 3340 plot-only errors caused by its own local stroke-width variable
shadowing chart width. Its original script, oracle, log and failed report remain
preserved; four local identifier corrections fix the authoring error, followed
by a complete five-report reread without changing product code or tolerances.

Eight target-only negative controls reach their intended checks: forged reported
callback order, actual Canvas stroke width, actual SVG node radius/translation,
a real accepted source commit disguised as rejection, one hidden backing pixel,
actual SVG label text and a mixed other-row observation with forged consistency
booleans. Within the other-row donor's raw source array, only original b
signed-zero bits change; numeric raw values and the owner/own-child remain
unchanged. The complete donor row also differs in activity and node radius.
Its first actual failure is the exact raw-source guard; this is not a full-row
bit-only mutation. It is a readout forgery,
not a claim of actual mixed Scope publication. The radius control mutates the
actual connected path after the original attributes check and decodes White at
the new radius-minus-four probe against expected Blue; metadata still says 20
while actual radius is 14. Attributes checked before mutation are not evidence
that the subsequently mutated geometry is correct.

The final stroke control uses public width 721→720 to force a real paint and
ends with complete accepted metadata exactly equal to the original 720 baseline.
Only the four 720 source strokes change 8/8/10/10→1.5; 721 strokes and the four
detached references stay unarmed. Independent PNG decoding finds baseline/ref
ROI error 0 and mutated/ref error 255 with reduced alpha coverage. The hidden
control changes exactly one source-plane pixel (700,310), transparent to Red,
while accepted DOM/data and the entire foreground PNG stay unchanged. Ten
actual processes include an original wrapper ReferenceError and an early thin
attempt whose activity differed from its baseline; neither counts among the
eight final controls. Historical runner-label corrections preserve both archived
and reconstructed first-executed script bytes with explicit provenance.

Execution evidence: `target/tree-activity-browser-final-v5/summary.json`,
`target/tree-activity-browser-ci-final-v5/report.json`,
`target/tree-activity-validation/browser-counts.json` and
`target/tree-activity-browser-negatives-v5/review-final.json`.
Independent evidence is in `target/tree-activity-final-review/readback.json`,
`target/tree-activity-final-review/negative-readback.json` and
`target/tree-activity-final-review/authoring-correction.json`.
The bounded combined receipt is `target/tree-activity-validation/acceptance.json`;
it keeps rapid native click lifecycle and strict whole-page acceptance false.
New workflow commands are locally exercised edits, not remote CI proof for this
uncommitted development tree. Published Glory 0.8.0/Kartis 0.1.0 and their tags
remain unchanged.

See [English guide](tree-activity.md) and [中文指南](tree-activity.zh-CN.md).
New APIs remain Unreleased. Label components, arbitrary layers, automatic
Spring/retirement and Widget export remain open. Nearest-node tooltip parity,
native same-layer input and OS touch are outside this activity owner; GPU/worker
remain deferred.

## Tree native pointer lifecycle

The following development work is separate from the original Tree activity
execution above. Its recorded rapid-click limitation does not become evidence
for the subsequent fix.

Later manual cleanup removed historical `target` artifacts, including that
original receipt and some intermediate pointer runs. Earlier paths in this
document remain execution-history indexes; missing originals are not presented
as recovered evidence. Fresh final verification uses the project-local
`artifacts/verification` directory for reports while Cargo continues to use its
normal metadata target directory. Surviving copies have a byte/hash manifest;
new runs and newly authored readers are identified separately.

The old-package comparison executes 64 fixed rapid input trials: four engines,
two picking policies, two composition modes, two renderers and source/Widget
targets. Movement is immediately followed by Down, then a held interval and a
single Up, without a pre-press hover wait or retry. Chrome, Firefox and Edge
originally passed their 16 cases each; WebKit passed six and failed ten. With
the native-press fix, all 64 matching trials pass. Original package/source and
first failures remain in `target/tree-input-lifecycle-baseline`; the matching
fixed-package runs are in `target/tree-input-lifecycle-pilot3/baseline`.

Root validation passes 1879 minimal tests, 2214 all-feature/all-target tests,
18 doctests, and strict Rust 1.95 native/all-target and WASM Clippy. The nine
additional tests include five pure press-lifetime cases and four independently
authored public Surface gesture contracts. The latter check matching Up,
explicit replacement cancellation, rejection preservation, movement threshold
and touch cancellation; they do not pretend to be browser capture tests.
Logs and counts are in
`artifacts/verification/tree-input-native-current/report.json`, from a fresh
full execution after cleanup.

Native Up can normally target the capture Surface. That target alone does not
identify the cause of a lost semantic click. The new lifecycle preserves the
pending model gesture and native click target while automatic activity waits;
accepted explicit changes still cancel the old press. See the paired
[English](tree-input-lifecycle.md) / [中文](tree-input-lifecycle.zh-CN.md) guides
for precommit failure and postcommit UI-error boundaries.

The headless two-page activation probe did not deliver a real OS window blur in
any of the four engines. DOM-injected blur and pointer cancellation remain
untrusted protocol tests. They do not establish OS delivery or touch behavior.
Whole-image equality, automatic Spring/retirement, live Widget export and
GPU/worker remain outside this input-lifecycle work. The dedicated CI job is
a local workflow edit until a remote run exercises its exact commit.

The final package also passes the unchanged original Tree activity driver on
Chrome DPR1/profile0: 164 frames and 36 rejections, with identical start/end
source/package fences. This bounded regression is recorded in
`target/tree-input-lifecycle-activity-regression-focus/execution.json`; it does not
replace the historical full activity receipt.

`scripts/tree-ui-errors-acceptance.mjs` adds four retained dynamic-UI error
cases per browser: SVG/Canvas crossed with source-order/overlay composition.
The final script passes all eight actual Chromium/WebKit cases. A real Glory
Style validation error occurs only after same-revision data adoption. The
public request succeeds, the owner and both actual Frame Bonds contain the new
source, child identities and factory counts stay unchanged, and the old native
Up cannot activate the new payload. Disarming and a subsequent hover update
change the same live child's actual color, then a fresh native click activates
exactly once. Independent readback reconstructs 80 full owner/Bond observations
and decodes 16 PNGs with 144 hard RGBA samples, maximum error zero. These finite
samples do not establish full-image equality. Scope cleanup after stop is not
claimed from counters that the fixture does not expose after retirement.

Execution evidence for the focus-corrected package is
`target/tree-ui-errors-control/focus-acceptance.json`; independent readback is
`target/tree-input-lifecycle-final-review/ui-focus-readback.json`. The script
runs alongside native pointer acceptance in the dedicated CI job, using the
bundled Chromium browser. Its first target-only launch failed at a CommonJS
import before executing any browser cases; that authoring failure remains
preserved and is excluded from accepted counts.

The first complete pointer matrix remains a failed historical run: nine actual
processes, ten contexts and 5280 cases, with 116 failures. Chrome, Edge and the
independent Chromium run passed; Firefox reported 52 failures at each DPR and
WebKit six at each DPR. `target/tree-input-lifecycle-browser-final/first-execution.json`
originally preserved terminal results, all 326 executed Rust sources and the
executed driver. Those files were removed by the later manual cleanup. The
recorded result remains a failed historical execution, not final acceptance.

Separate diagnostics read actual Bonds synchronously in six Firefox focus
windows. Both source-update roles, the remaining options-update role, and both
coalesced-activity roles match the owner in every window. Their same-identity
DOM text still shows the exact earlier frame. The original six trials retain
their failed DOM-cohort gate. This evidence supports distinguishing ordinary
DOM presentation from actual typed data adoption.

Two Firefox coalesced-input traces also exposed a product defect: in-frame
Widget focus restoration erased the latest pointer hover. The private input
packet now records the synchronous Engine-frame boundary at delivery and
preserves that pointer state when processed later. It retains outside-frame
focus handling. With the corrected package the same two traces end at current
`a`/hover `node:a`; their old DOM-text guard is still recorded as failed, so
that diagnostic is not counted as a complete passing case.

WebKit's rejected Canvas candidate can repaint the old Scene using the current
shared Series decoration. An independent fresh old Scene, public Series policy
and programmatic shared-chart Move reproduce all 244800 backing pixels exactly
(maximum error zero). The earlier held neutral backing differs in 2995 pixels,
maximum error 226. The first reference author attempt compared a serialized
numeric zero as though it retained negative-zero identity; its failure remains
preserved. The corrected comparison keeps original raw-bit checks exact and
uses the existing numeric compatibility contract for derived Mark scalars.
Neither the Canvas change nor a redisplayed old-source tooltip permits a
rejected candidate's geometry or typed data to become accepted.

Evidence and independent reconstruction are in
`target/tree-input-lifecycle-presentation-diagnostics/diagnostic-summary.json`
and `target/tree-input-lifecycle-final-review/presentation-diagnostics-readonly.json`.
Firefox input also demonstrates native fractional-coordinate quantization;
pan expectations must use independently observed trusted Down/Move coordinates
and actual bounds, while separately bounding delivery against the requested
coordinates. A qualified later native plot Leave clears current. These input
observations do not authorize larger data or pixel tolerances.

The final uniform pointer matrix uses one frozen driver and package throughout:
four engines at DPR1/2 plus the independent bundled Chromium CI group, ten
contexts and 5280 fixed cases. All nine actual processes exit 0. Every process
keeps the same 331 protected source/package/script hashes at start and end.
The final driver records the deliberate stop-to-remount registry gap as
`UNAVAILABLE`: exact successful-stop epoch, matching renderer, absent root,
empty rows, exact business errors, and closure after successful remount.
These observations do not count as successful accepted-data reads. JavaScript
TypeError and WASM RuntimeError remain failures.

Current execution reports are in
`artifacts/verification/tree-input-browser-final-uniform`; independently decoded
readback is in `artifacts/verification/tree-input-final-review`. The preceding
db1 run remains failed with all 5280 cases preserved: its 4960 non-stop cases
passed, while 131 stop cases rejected that expected registry gap. The final
stop-only change updates the observer qualification; production source, input
parameters and other hard checks remain unchanged.

Fresh native validation is in `artifacts/verification/tree-input-native-current`;
consumer compilation and exact regenerated package equality are in
`artifacts/verification/tree-input-consumer-current`. New UI fault executions
are in `artifacts/verification/tree-ui-errors-current`. The source archive and
current fence are in `artifacts/verification/tree-input-source-current`.
Matched outside-frame programmatic focus controls retain shared hover clearing
and preserve Tree activity until real movement. Their original surviving bytes
are included in `artifacts/verification/input-lifecycle-survivors` with the
manifest; they do not claim OS keyboard delivery or a private keyboard flag
that the fixture does not expose.

Independent final readback passes all nine reports and 5280 case artifacts:
81686 actual Bond reads, 81694 independently checked owner reads, and 163300
role/cohort checks. It keeps 192 synchronous and 115 asynchronous stopped-gap
observations explicitly unavailable. The 96 retiring blur observations and 72
same-revision old-DOM focus observations retain their exact identity and prior
cohort restrictions. All 10560 Down/Up records are trusted browser events;
4181 native click records include cancelled and moved gestures and are not
reported as 4181 successful activations.

The reader decodes 328 PNG files and 968 embedded backing PNGs. All 160 pan
cases pass 4320 hard RGBA samples with maximum error zero. Complete old-Scene
Series references cover eight planes and 4896000 pixels, independently decoded
with zero changed pixels and maximum error zero. Source input coordinates and
10880 actual Widget rectangles are independently reconstructed; the largest
DOM rectangle difference is 0.01562500000001421, below the unchanged 1/32 bound.
There are 160 real precommit paint-error cases and 28 qualified redisplays of
the exact old `source-c: 30` tooltip. These finite checks do not establish OS
window-blur/pointer-cancel delivery or strict whole-page equality.


## 开发版 Tree 标签组件：统一验收

本轮增加 `tree_components`、`tree_label_widgets`、`tree_activity_components`、
`SceneWidgetsComposition` 以及 BrowserTree 三角色挂载入口。标签绑定真实 Scene Label，
使用稳定 source key；节点／连线／标签共享同一事务。仅替换标签绘制，不制造 Mark 或
canonical 数据控件。使用说明见[中文指南](tree-label-components.zh-CN.md)与
[English guide](tree-label-components.md)。新增接口保持 Unreleased。

当前执行包的 WASM SHA256 为
`2fff235355a2282deb44f78ee2ada324713ede5fd4ea69780b0915725f16d924`，
JS glue 为 `3ed5bf8ce1c7d118eb39059f6c75d5a2f208550cfffb3d7e7a7489dc3de77e54`。
`artifacts/verification/tree-label-consumer-final5` 保存确切源码 ZIP、源码／manifest／lock
起止哈希、实际 MSRV Clippy/release/bindgen 日志、生成包及前包副本；执行前后源码相同。
Cargo 使用各实际 workspace 的 metadata `target_directory`，没有缓存目录或环境重定向。

| 本轮实际检查 | 结果与范围 |
| --- | --- |
| Rust 1.95.0 最小特性 | 1904 项通过，完整 lib/tests |
| Rust 1.95.0 全部特性 | 2240 项通过，完整 all-targets；另 18 项 doctest 通过 |
| 最终原生宿主目标 | minimal 6 / all-features 7 通过，真实 Memory 状态／焦点和 CPU DPR1/2 发布回滚 |
| 原生／WASM 严格检查 | 全部特性 native all-targets 与 WASM lib Clippy `-D warnings` 通过；独立 consumer strict/release/bindgen 实际终态 0 |
| 主浏览器矩阵 | Chrome、Firefox、WebKit、Edge 与独立 Chromium，各 DPR1/2，四个 picking/order 配置，SVG/Canvas 共 2400 帧通过 |
| 固定 Scene 仿射专项 | 同五种浏览器／CI 配置，320×240 源尺寸保持；320×240、480×192、288×360 三个实际 CSS 盒，共 120 帧通过 |
| 统一进程收据 | 七个独立进程全部实际 exit 0；主矩阵和专项共 2520 帧，全部起止 source/package/driver fence 一致 |

全量 Rust 日志在 `tree-label-native-final2`；其最终严格检查首次发现测试文件一个冗余
`..Default::default()`，该单行删除的精确哈希重建桥接与完整宿主目标复验保存在
`tree-label-semantic-index/test-lint-bridge.json` 和 `tree-label-native-final-gates`。
后续独立 consumer 样例新增／样式修正不属于根 native workspace；最终 Label carrier
修正位于明确 `cfg(web && wasm32)` 的 browser 模块。最终 source bridge 逐项记录差异，
没有把旧字节的全量运行伪称为重新执行。最终 native strict 又以当前代码实际通过。

`tree-label-browser-final-consolidated/summary.json` 保存最后统一矩阵的七个进程终态；
每个报告都有完整 frame JSON、viewport PNG、原生 Canvas backing、独立 reference PNG、
数据／实际 DOM、Scope 观察及拒绝前后见证。检查覆盖真实活动 hover、键盘 Enter、保留
本地状态与焦点、原始值和 label pool 更新、索引重排、正负旋转、空／透明文字、源节点
透明、标签门禁、裁剪、真实尺寸变化、祖先 CSS 缩放、相机变化及 revision 重建。
400 次真实候选拒绝分别检查 builder、placement、非法测量、factory、child build，
接受元数据、全部 live rows、源绘制、DOM、本地状态与焦点都保留旧帧。

额外的独立 Python/Pillow reader 使用自己的输入／BFS／活动／标签／矩阵模型，
不导入 JS oracle。最终独立读回记录在 `tree-label-final-review`；它逐张核对 PNG 哈希、
原采样索引与原始 RGBA、原生字形 ROI、空 backing、完整 typed rows、Scope 同帧观察及
拒绝见证。位置门禁维持 `1/32 CSS px`，原生／前景 RGBA 门禁维持每通道最多 1。
最终 reader 收据为 `tree-label-final-review/acceptance.json`，SHA256
`295c9a008ed60fd73d1636d0986f428f58a03f7c0573ce6e0f4269a20caf1e7a`。
实际共解码 6110 张 PNG、16000 个前景样本（main max1 / micro max0）；main DOM 最大
误差 `0.01666259765625 px`。独立核对 13440 个主 typed rows、9680 次已接受 Scope
观察和 880 次候选观察，候选不计成功业务观察；400 次实际拒绝、80 个真实指针 opt-in
见证及全部源绘制顺序另有硬核。原生 fallback ROI 为 2918400 像素（max1），源标签遗漏
区域 4132500 像素，空 backing 为 12240000 像素；micro 另核 5760000 个白色原生像素。
这不是跨浏览器整页 AA、完整控件 Canvas/SVG 导出或 OS 触控协议等价的声明。

独立审查确实发现并复现了原生投影标签索引错读（font 400/700 与测量签名 panic）和
修复期间 tracked packet 自订阅的 RunawayEffect；两者的真实 RED、修复和最终 GREEN
分开保存。原测试作者 Preserve 空白预期错误也单独分类。WebKit 未旋转部分裁剪标签的
双 percentage 尺寸量化误差先以单变量控制复现，随后采用精确局部 px 盒和浏览器矩阵
修复；公开 `frame.transform == None` 的 Scene 语义保留，旧 Mark carrier 路径不变。

两轮未通过的主浏览器矩阵始终保留 failed：第一轮包含应用 inline opt-in 被响应式
Style 更新替换、WebKit 尺寸问题及 reference 字形环境错误；第二轮只余 Firefox overlay
参考环境不一致。持久应用 stylesheet 经过真实鼠标对照；reference 明确采用作者文档
`lang=en`，并匹配后端实际绘制时的 DOM 连接状态：ordered detached、overlay hidden
connected。Firefox 两 DPR 的原生控制证实这些环境差异，不是放宽像素阈值或修改生产字体。
原单像素与批量 readback API 的低 alpha 1-byte 差异也保留为失败诊断，不声称两种 API
bit-equivalent；冻结 buffer 的索引运算另经逐像素精确比较，最终业务 ROI／样本和 RGBA1
门禁不变，并有独立 PNG 解码复核。

先前同批生产改动的旧 Mark 指针流程在 3e4 包上另实际通过 Chromium DPR1/2 的 1056 项，
收据在 `tree-label-pointer-regression`。后续仅 Label browser carrier 和新增独立样例变更
没有被冒称为以 2fff 包重新执行了那份旧指针报告；两份完整源码和差异桥接均保留。
更早一轮用户清理 target 的记录仍见前节，这批所有新验收证据均位于项目独立 verification
目录，没有重造已丢失的旧日志。

CI 已接入 Chromium 两 DPR 的主标签矩阵及固定 Scene 非均匀 CSS 专项，构建本提交的
WASM 并上传报告、PNG 和实际日志。当前工作树未推送，本节不冒称远端 Actions 已执行。
Canvas 原始 `renderLabel`、自动退出动画、任意层插入与控件导出继续单列差距；GPU/worker
按用户要求后置。

### Tree 原生 Canvas 回调、所有者和联合组件

本轮补齐原始节点、连线、标签和自定义 Canvas 层。它们接收真实的同一原生上下文；
重复层真实执行，绘制状态在用户回调之间传递。新增完整计算标签／连线样式捕获，
候选绘制在 Engine 状态借用之外执行，封存位图从未交给回调。原始模式与严格顶层
Glory 组件联合提交，不把 HTML 组件混入共享 Canvas 背景。API 见[双语指南](tree-canvas.zh-CN.md)。
本节补充上一节仍待实现的原始 Canvas 回调，不代表其他退出动画、SVG 函数层或控件导出完成。

实际编译使用 MSRV 1.95.0、项目正常 workspace target，无环境或全局缓存重定向。
完整原生检查 minimal **1925**、all-features **2261**、doc **18** 全部通过；原生和
WASM all-features 严格 Clippy、项目及独立 browser-host 格式检查通过。旧实际编译二进制
与新编译二进制的 200 组普通／pretty Debug、SVG 和活动包装输出，完整 5,912,919 字节一致。
后续仅浏览器所有者与独立样例变更由最终消费者及 WASM 严格检查覆盖，不冒称旧原生日志
包含这些后续 WASM-only 字节。源码压缩包及最终生成包首尾指纹保存于
`artifacts/verification/tree-canvas-consumer-fourth`。

同一最终包的 WASM SHA 为
`ebb07bcc1a9050e52761e6748e997e33ac131501030ec5b60a2fb33d25a78d1f`，
glue SHA 为 `e52d5f29dbdcd89c5b0d020374c42706eec79baee866325880d9f9e1f48c1778`。
以下正式矩阵均使用这个包，分别核验各自实际执行脚本和源码指纹：

| 专项 | 正式范围 | 独立验收 |
| --- | --- | --- |
| 原生回调和丰富默认图元 | Chrome、Edge、Firefox、WebKit、bundled Chromium，各 DPR1/2；30 个完整场景、70 次拒绝、30 次保留 context 隔离 | Pillow 完整解码实际／参考 PNG，5,760,000 像素最大 RGBA 差 0，门槛仍为每通道 ≤1 |
| 跨源污染 | 同 10 个配置，两个真实 HTTP origin、无 CORS 图片绘入真实候选 | 每次 22 个真实回调完成后，原生 origin-clean 导出实际拒绝，旧封存像素和数据保持 |
| BrowserTree 所有者 | 10 个配置、两种源命中／最近节点活动策略；360 帧 | 720 PNG、6,500 RGBA 最大差 0；3,300 次回调、20 次晚拒绝及20次停止控制，真实指针记录120份 |
| Canvas 与真实 Glory 组件 | 10 个配置，Node／Link／Label 三种真实组件；130 帧、40 次应用拒绝 | 9,360 个有界颜色样本、210 个真实 effect 中的 owner／全部行／当刻 Canvas 联合观察；110 个 scope 全部清理 |

独立 late-DOM 控制在 Chrome／WebKit 中记录了新 Green 候选已经出现在实际 Canvas，随后
对确切所有者 mask-root 的 computed-style API 注入一次 `InvalidStateError` DOMException。
完整接受数据、行、DOM、Canvas、封存导出、焦点和 revision 均精确恢复为原 Cyan；随后正常
恢复也通过。公开可移植脚本另在 bundled Chromium DPR1 实际通过。此为模拟原生 DOM API
拒绝，不能称真实 CSS 解析故障、OS 故障或用户绘制函数报错。

负控只改变真实 Rust 回调的一次节点 `fillRect`，元数据和调用顺序保持正确、公共调用无错误；
全图像素门禁以实际 maxΔ176／449 个超限通道拒绝，并由 Pillow 重新读回确认。
它不是改写 snapshot 字段后自行报错。

原始失败原件保持 failed，包括测试样本违反源锚点约束、原生 CTM／alpha getter 的
f32 与数学常量不一致，以及 single-quad 与既有 Square pointcap 两半轮廓的抗锯齿差异。
偏移图元样本仅改成同几何的合法 Path；原生状态由固定输入的独立原生 Canvas 控制严格比较，
Scene 数值门槛未变。Square 的参考按公开 incoming／outgoing 两半轮廓合同独立构造；
单变量控制及另一审阅者完整解码确认，两半参考与实际 PNG 字节一致，single-quad 的原始
125 像素／235 通道、maxΔ13 失败仍保留。没有声称不同路径编码或 Nivo 原生零长度 stroke
像素等价，也没有扩大 RGBA1 门槛。原函数编译作者错误和读图 CSS／设备像素作者错误分别保留。

最终包另重新执行旧标签组件 Chromium DPR1/2 的 480 帧回归，1192 PNG 全部保留并通过。
更早 `tree-canvas-legacy-labels-first` 属于先前 d6d 包，不被替代或错标为最终 ebb 包。

主收据保存在 `artifacts/verification/tree-canvas-acceptance.json`，分项收据分别位于
`tree-canvas-native-final-readback`、`tree-canvas-owner-final-review` 和
`tree-canvas-widgets-final-review`。CI 接入原生回调、所有者、联合组件、晚期 DOM 拒绝和
跨源污染专项，使用本提交实际构建的 WASM 并上传报告、PNG 及日志。工作树没有推送，
不冒称远端 Actions 已执行；本轮功能保持 Unreleased，不改已发布的 0.1.0。

### Tree 类型化节点提示与真实 Glory 内容

新增显式启用的 `TreeTooltipData`、固定／光标定位及九种自然尺寸对齐，支持安全文字与真实
保留式 Glory 提示组件。提示目标与 activity、canonical 控件和物理命中分离；最近节点模式
可以呈现零尺寸、透明或空标签节点。完整原始输入、raw 位型、聚合值、最终布局和活动数据
来自同一接受阶段，不重跑工厂重建元数据。用法与限制见[中文](tree-tooltip.zh-CN.md)／
[英文指南](tree-tooltip.md)。新增 API 保持 Unreleased，不属于已发布的 0.1.0。

MSRV 1.95.0 的原生 minimal **1931**、all-features **2267**、doc **18** 全部通过，
包含五项纯提示数据测试和提交协调测试；原生全部目标严格 Clippy 通过。独立消费者严格检查、
release 构建和 bindgen 均实际退出 0，使用各 workspace 的正常 target，无环境重定向。
最终 WASM SHA 为 `8e81f00722a6ef18dc130564e61e4ae9369c0e87ec4179152644cdb9cdf91d18`，
glue SHA 为 `7967794c724ff7a208723e7d4275e427579030c4b505a8e5949f8e6901764a42`。
完整编译源与生成包记录保存在 `artifacts/verification/tree-tooltip-consumer-fourth`。

最终正式矩阵统一执行脚本 SHA
`48dd91c5d8af847dc34ba0e0d827da285fa7d6c28f3d2d7fc38b8b3f5a34e7d5`，
Chrome、Edge、Firefox、WebKit 和独立 bundled Chromium 各 DPR1/2，全部原进程终态为 0。
八种 profile 覆盖 SVG／普通 Canvas 的 Geometry、源顺序及顶层组件的 Nearest，以及有无
源组件的原始 Canvas 程序，共 **80 profiles／4180 帧**。包括固定／光标位置、九种锚点、
局部状态与键盘激活、源位型更新、隐藏、相机／尺寸／CSS 缩放、按压延后、拒绝恢复及停止。
程序 focus／共享图表 dispatch 不被当成原生提示意图，同节点移动不重跑源／绘制工厂。

全矩阵实际通过 **230** 次提交前拒绝、**80** 次接纳后的动态视图错误及恢复、**80** 次
停止清理断言。提交后视图错误保留新源和新提示数据并隐藏失败视图，不把旧文字继续显示为
新源；修复后恢复同一组件和局部计数。display:none 下，三个无源组件 profile 接受源更新，
五个带源 Label 的 profile 明确拒绝未解析的 CSS 尺寸并保留旧帧，恢复可见后显式 refresh
通过；不将这 **50** 次严格拒绝冒称隐藏宿主下的源 Label 支持。持续源构建／放置／原始
绘制错误仍保持共同接受的源和提示，不通过 tooltip-only 更新绕过错误。

独立 Python/Pillow 从五份正式报告重新核对全部 **4180 PNG**、原始位型／聚合／拓扑／活动、
位置与自然尺寸，**12080** 个有资格的颜色点保持每通道误差 ≤1。另读取 **4320** 张源 Canvas
PNG，验证 **11376** 次 Scope 观察、**30420** 个接受源行、**10486** 个提示行及 **2800** 次
组件身份／局部状态检查。限定作者源、活动、相机、隐藏与物理 hover／selection 均相同的
50 组锚点／光标变化中，550 对帧的 880 对原生 PNG、538560000 像素全部字节一致；这不是
所有帧或任意原始绘制程序的整体像素等价。独立结果为
`artifacts/verification/tree-tooltip-final-review/readback-uniform-corrected.json`。

两项负控分别只移动真实提示 host 的 left 8 像素、只把真实 Glory 内容背景从 Purple 改为
Red。完整公开数据、Source／Tooltip Bond、源 SVG／Canvas 和控件保持不变，原位置门禁和
原 RGBA 门禁分别以实际 exit 1 拒绝，Pillow 独立确认实际画面变化。见
`artifacts/verification/tree-tooltip-negatives-independent/uniform-48dd/acceptance.json`。

修复前包还实际复现了两个生命周期问题：失败的新组件工厂会毒化后续真实 Leave／重入，
reset 被 DOM 拒绝后提前丢失目标。现在区分 desired／accepted 配置，自动输入使用接受组件，
显式源请求／refresh 才重试待应用配置；只有 Unmanaged 真正接纳后才清除目标。精确 host
transform 的一次 DOMException 在新旧包都完整回滚，随后真实 Leave 在旧包仍显示 c，
修复包清空 frame 并隐藏。八张原件由 Pillow 读回，故障前后整幅回滚差 0；不同故障的
旧包 RED、修复 GREEN 及源码绑定分别保留，没有混用静态推断和实测证据。
reset 差分的修复包为 42f4，后续 8e81 只将独立夹具的子色块从 4 改为 8；生产 Rust 源码
逐文件相同。该历史差分没有被重新标为在 8e81 上执行。

原作者错误保持 failed，包括夹具编译、跨 WebApp 误用 tracked Bond 读取、原生事件过渡
遗漏、装饰连线的空 series 隐藏规则，以及未解析 Label 宿主尺寸的错误正向预期。合法的
foreign Bond 观察改用公共 `try_with_untracked`，实时读取实际值并保留存活／借用／类型错误；
组件自己的 effect 仍跟踪原 Bond。退休行只在真实清理与失活证据下接受确切的 Disposed／
RuntimeGone，不跳过存活行错误。

4 像素子色块在 .8 缩放后的边缘混色原件与独立纯 DOM 参考均保留，不主张微小图元的
RGBA 等价。正式夹具使用固定 8×8 子色块，独立检查实际 DOM 几何，再仅按作者矩形选择
两个边界净距 ≥2 物理像素的内部点，不按颜色挑点，也不扩大 RGBA1 门槛。Firefox 的
Playwright protocol 高度 339.999954 与原生 DOM 高度 340 的差异经同帧诊断确认；最终脚本
读取原生 DOMRect，Scene 数值 1e-9 门槛未变。独立 reader 首轮将 BCR 比值误当精确 CSS
缩放而失败，原件保留；修正只让正向绘制使用作者固定 CSS scale，逆向定位仍用原生 BCR，
随后从头读回整个五报告矩阵，未拼接部分成绩。

同一最终包另通过旧 Canvas 所有者 72 帧、联合组件 26 帧、标签组件 480 帧回归，共 578 帧；
先前包的结果没有被重新标为当前包。CI 接入新的 Chromium 两 DPR 提示矩阵并上传报告与 PNG，
拼写、格式、工作流及发布门禁另有最终收据。工作树未推送，不冒称远端 Actions 已执行。
总收据为 `artifacts/verification/tree-tooltip-acceptance.json`。该历史提示专项不覆盖任意 SVG
函数层，后续 Glory 适配见下一节。原生同层输入、自动 Spring／退出和实时 Widget 导出仍单列；
GPU/worker 继续后置。本节不代表整个 Nivo／D3 差距清单完成。

## Development: Tree SVG function layers

本节记录 Unreleased 的真实 Glory 适配，不更改已发布的 0.1.0，也不表示整个 Nivo／D3
差距清单已完成。`TreeSvgFrame` / `TreeSvgPlan` 和 `BrowserTreeSvgProgram` 提供字面省略、
重复、顺序、完整已计算 Tree 数据及每个 occurrence 独立保留的 Glory 子树。原始业务
布局、Mark 与 canonical 控件保持单份。接口、限制及完整示例见[English](tree-svg-layers.md)
与[中文指南](tree-svg-layers.zh-CN.md)。

最终源码封存于 `artifacts/verification/tree-svg-consumer-eleventh/executed-source.zip`；
严格 WASM Clippy、release 编译与 bindgen 均以真实退出码 0 完成，源码前后哈希一致。
当前 WASM SHA256 为
`08b9c5461d154e55a6a53b3f7fbf7d7656d0a8dceb5497c49a3203425b3b28c8`，
JS glue 为 `b576c6dcc75e9164cc96a5b728969b95f5a2315918a4e178ede234f11e9da39b`。
正式执行的 SVG 验收脚本 SHA256 为
`db6eb2a18e3279a4801d7089b90e5269f9baae65b3c1670617bc743ea80d56b9`。
本节全部正式浏览器结果来自这个包与该脚本，未借用旧包的部分通过项。

| 检查 | 同一最终源码／构建的结果 |
| --- | --- |
| Rust | 最小特性 1946、全特性 2282、文档示例 18 项通过，无忽略项；严格全特性全部目标 Clippy 无警告 |
| 主浏览器矩阵 | Chrome、Edge、Firefox、WebKit、独立 Chromium；各 DPR1/2、六种配置，共 10 上下文／60 配置／3690 帧，五个原进程均退出 0 |
| 旧入口回归 | Canvas 所有者 72、联合组件 26、标签组件 480 帧，共 578 帧；类型化提示两 DPR／16 配置／836 帧另通过 |
| 实际绘制负控 | 只改 connected SVG Graphic 的 fill 或 XHTML 相机 tx+10，全部元数据深等；两次原像素断言均真实退出 1，独立 Pillow 验证拒绝位置与整图变化 |

主矩阵位于 `artifacts/verification/tree-svg-browser-final-third/`，每个配置保存原 PNG、
完整 DOM、源／层／提示 frame、原始 IEEE 位串及执行脚本副本，另有原进程终态和哈希。
覆盖 Geometry／Nearest、无源组件／keepInk／replaceInk、重复 Source 三角色、实际控件
点击与焦点、Mesh 上下层遮挡、缩放后捕获拖动、隐藏、禁用交互、revision 与预算、持续
拒绝期的真实输入、采用前 carrier 拒绝恢复、响应式属性错误恢复、停止和清理。
独立 Python/Pillow 从固定作者输入重新计算原始／计算值、活动、几何、字面顺序、相机、
控件位置及安全内部像素，并读取真实 SVG 路径／标签和 retained Scope。RGBA 门槛保持 1，
元数据门槛保持 1e-9，Source DOM 位置保持 1/32；Gecko 的 CSS 60 app-unit 和其它引擎的
64 app-unit 量化分别处理，未扩大产品或像素容差。

独立完整读回与收口分别为
`artifacts/verification/tree-svg-independent-review/final-third-matrix-readback.json` 和
`artifacts/verification/tree-svg-independent-review/final-third-readback.json`。
前一构建的独立工具三次拒绝分别涉及 Firefox 合法的全零第三 transform-origin 分量、
CSSFloat 的矩阵数字序列化，以及最终相同上下文 tuple 的工具集合比较。三份工具失败
及对应 reader 均保留。新的前向模型依据 Gecko 的 f32／最短往返十进制／六有效位
序列化，只用于 CSSOM 文本字段；真实 view、位置与像素仍使用原始作者 double。
非零 Z、错误单位和矩阵平移 X/Y+1px 的 reader-only 负控均拒绝。最终上下文采用原生
精确集合比较；未扩大元数据、像素或位置容差。最终新构建从头读取全部五份报告。
真实 DOM 负控原件在 `artifacts/verification/tree-svg-negative-controls-final-second/`，其独立审核为
`tree-svg-independent-review/independent-actual-browser-negative-controls-final-second.json`；它们与只修改
已保存原件的 reader 自测分开计数。前者保持数据不变而修改实际呈现，后者只检验 reader
是否拒绝被篡改的存档，不能相互替代。

首轮完整矩阵的 WebKit 失败保持原结论，见
`artifacts/verification/tree-svg-layers-planning/matrix-first-checkpoint.json`；真实浏览器
原件、作者等待时序错误与生产修复没有合并重标。最终 full-Scene foreignObject carrier、
固定视口绘图区裁剪与 HTML 相机解决了已观测的 WebKit 层叠和缩放问题。后续 WebKit
缩放捕获拖动在 `tree-svg-webkit-fixed-second/mesh-source/` 真实失败，同参数、同正式
脚本的 `tree-svg-webkit-fixed-third/mesh-source/` 通过；捕获释放时位于 Surface 内的原生
Leave 不再覆盖该来源手势的最后 Move，真正外移继续通过窗口输入处理。最终五配置矩阵
重新从头运行，包含这些分支。

数据共同采用先于已接受 frame 的 update pump 和退休清理，但构造与初始 effect 仍在
候选准备时运行。受检拒绝点是 owned carrier/mask 的采用前实际 DOM 操作，不是受管
Source 根节点的后端移动。Glory 0.8 将受管渲染写入排队到采用后的 pump；后端失败可能
留下已执行前缀并使运行时 fault，须重新挂载。调查和发布版上游源证据见
`artifacts/verification/tree-svg-layers-planning/presentation-boundary/investigation.md`。
本轮不声称任意受管 DOM 后端故障的完整回滚，也不以数据一致性替代该呈现边界。

总收据为 `artifacts/verification/tree-svg-acceptance.json`。最终质量检查保存于
`artifacts/verification/tree-svg-quality-final-fourth/`，涵盖固定工具链 fmt、typos、actionlint、
发布脚本门禁、Node 语法、diff 空白与文档链接／README 便携性检查。首轮质量检查的
示例模块排序失败保持原记录；修正排序、将脚本局部变量改为完整名称 `surfaceNode` 后，
实际重新编译的四个包文件与先前包逐字节相同，仍采用新源码、新脚本重新运行的完整
主矩阵与回归结果，没有使用拼写豁免或旧报告替代。CI 已接入独立
Chromium 的两 DPR SVG 矩阵并上传日志和原件；本轮没有推送工作树或宣称远端 Actions
已经执行。受检内部像素与拒绝前后整图零差不代表任意自定义绘制、整页 AA 或 OS 触控／
笔／辅助技术等价；原始 React SVG/defs、节点／连线各七种事件属性、自动 Spring／退出、
实时 Widget／重复层导出与 GPU/worker 仍按独立差距推进。

## Development: Tree native mouse hooks

本轮只验收未发布的 Tree 类型化原生鼠标回调，不结束完整 Nivo/D3 差距目标。
SVG 节点、连线各七槽；Canvas 为节点七槽，Link 注册明确拒绝。
`TreeEventFrame` 共享原始来源、计算节点／连线、样式、活动与 Scene；索引分别属于原始
扁平来源和计算连线顺序。真实 MouseEvent、捕获阶段坐标／指针快照、已采用来源 generation、
occurrence key/index/publication 与显式绑定身份各自校验，不用 Core revision 代替来源身份。

最终包为 `artifacts/verification/tree-events-consumer-tenth/`，WASM SHA256 为
`5d7e4ac61d21d86424c22e97aef72b5853b9b52749daf6d2402757aab9f5fa90`。原生最终检查位于
`artifacts/verification/tree-events-native-final-second/`：最小特性 1953、
全特性 2289、文档 18 项通过，严格 Clippy 通过。
编译、bindgen、源码封存与现用包字节核对均保留原始退出码。

主矩阵为 `artifacts/verification/tree-events-browser-final-third/`。
Chrome、Edge、Firefox、WebKit 与 Chromium 各 DPR1/2 以十个独立原进程完整运行 p0..p8，
均退出 0；同一脚本、同一编译包，无重复、遗漏或历史通过项拼接。
独立读回为 `artifacts/verification/tree-events-independent-review/final-matrix-readback-capture-tail-corrected.json`，
核对 10 上下文／90 场景、8860 步、12799 条回调及 12799 条返回，
包括完整 Source 行、实际 getter、原 Event 身份、坐标、相位、像素和生命周期。

普通 SVG 的真实可用路径优先，其余位置显式使用 SourceModelNearest；字面 SVG 仅实际
Mesh 或匹配接触继承该路线。零半径 C 的中心仍可命中其连线终点；普通 SVG 的透明路径
可仍成为 DOM 目标，但只能按声明的来源 nearest 合同交付，不能据此伪造可见几何命中。
捕获 Geometry 的释放重新查询当前几何，不回落到近邻节点，也不回放旧 Down 角色。
原 nearest 优先级错误及同一冻结脚本的修前失败保留于第八包 p1，修复后重新构建并验收。

回调实际发生在同一 Event 的 Window 晚冒泡阶段，transport 已预留且 owner 借用释放。
逐次 Move 守恒、完整 raw/computed 数据与正负零、重排、拒绝来源／程序、成功来源／程序／
widget revision 退役旧按压、handler Err、替换自身注册、绑定退役、nested owner、停止与清理均有真实控制。
程序工厂拒绝前后的 DOM 身份和完整视口 PNG 逐字节比较；共享源／层数据不扩大为
Glory 0.8 任意受管 DOM 后端故障的完整回滚，原 fault／重新挂载边界保持。

菜单和采集窗口错误未归为生产缺陷。WebKit 纯 DOM 隔离对照实际证明默认菜单可阻断
后续 Up；正式测试宿主只对当前 owner 明确阻止菜单，并记录实际 contextmenu 与真实 Up。
框架默认菜单行为保持。Firefox Reset 后允许有限 r／None 已采用状态，下一次真实外移
仍须清空。WebKit 两步之间的真实 Leave 按原 ordinal 归档为 preAction，核对相邻已接受
完整 checkpoint；只用本动作记录满足正控。无单项重试、补造事件或丢弃旧回调。
此前失败、纯 DOM 原件及前两矩阵明确取消记录保留；第二矩阵的 WebKit DPR2 子进程
异常退出码 0xC0000409 保持原结论，原因未确定。另已消除采集器反复序列化完整历史的
冗余，使用严格游标增量、原始日志总数及 owner 专用 getter，并在相同同步 checkpoint
与原 full snapshot 核对全部记录，不以此推断异常根因。

独立 reader 首次正式读回的旧冗余 Mesh 门禁错误保留：真实 release-tail 按原 Mesh
Down／Up 来源核对，区域外 Move 派生 Leave 按先前真实 hover 与当前查询 miss 核对，
不要求已释放指针仍有 capture。修正工具后重新完整读回全部 90 场景，未改浏览器
原件或阈值。reader 篡改自测与实际运行时拒绝分开计数。

同一最终包的六套旧功能回归保存于 `artifacts/verification/tree-events-regressions-final-second/`，
覆盖两 DPR 的旧指针生命周期、Canvas owner／组件／标签、托管节点提示和字面 SVG，
每套均独立退出 0。最终质量检查及独立收口分别见
`artifacts/verification/tree-events-quality-final-second/` 和
`artifacts/verification/tree-events-independent-review/final-readback.json`；总收据为
`artifacts/verification/tree-events-acceptance.json`。

CI 新增独立 Chromium 两 DPR 事件矩阵并上传日志与原件，工作流静态检查与发布门禁
在本轮实际执行。本轮未推送工作树，也未把静态检查描述为远端 Actions 已运行。
受检像素与拒绝前后整图零差不表示任意整页 AA、OS 鼠标／笔／触摸或辅助技术等价。
React SyntheticEvent 身份、任意泛型 Datum、D3 平局、自动 Spring／退出、实时控件导出
与 GPU/worker 仍各有边界；SVG 连线托管提示及 anchor 继续作为独立差距推进。

## Development: Tree SVG link tooltips

本轮验收 Unreleased SVG Tree 连线提示的受检范围，不结束完整 Nivo/D3 差距目标。
`TreeLinkTooltipData` 共享完整来源及计算池，保留原始正负零、两端点、索引、样式与活动；
受管 Link frame 与原生事件的历史捕获 frame 分别验证。两类提示与 Source／SVG 层共同采用，
保持各自 Glory 组件状态；Link-only 配置保留原 generic Node 提示。

最终编译包为 `artifacts/verification/tree-link-tooltip-consumer-second/`，WASM SHA256 为
`21d0378236950ca371b9e7d9ac033a0527a953ab7b795d5a26c5ff99ae2dcc73`。
原生最小特性 1957、全特性 2293、文档 18 项通过，严格 native／WASM Clippy 通过。
源码桥接收据 `tree-link-tooltip-source-bridge.json` 核对 717 个文件与当前源码一致。

正式矩阵 `artifacts/verification/tree-link-tooltip-browser-final-second/` 以十个独立子进程
完整执行 Chrome、Edge、Firefox、WebKit 与 Chromium CI 的两 DPR、六场景，全部实际退出 0。
同一包、同一脚本共 60 场景、3470 步、1680 个截图检查点；Root 原矩阵进程也实际退出 0。
Chromium 三类上下文显式带 `--disable-gpu`，Firefox／WebKit 使用报告所列默认启动选项；
每个 context 记录与实际 launch 调用相同的 options。加速 GPU 下逐像素稳定性不由本轮证明。

检查包含无公共鼠标 handler 的真实 Link Enter/Move、Link-only 旧 Node 兼容、两角色仲裁、
真实局部计数／Scope 保留、source 更新、held Move 与 callback.flush、九种自然 shell 对齐、
Nivo Left/Right 解析、两类 CSS 缩放、相机／尺寸、隐藏节点不隐藏装饰连线、零宽／透明／
禁用连线、源重排、空业务标签／负零、字面重复／省略／Mesh／显式可交互子控件以及停止。
新应用回调观察只实际注册 Move；Enter-before-app 和全部七槽的新重复矩阵未在此扩大声称。
BoundComponent 自动 Linktip、再次设置同 revision 的新 factory、活跃提示时直接省略 occurrence、
factory 内 stop/reentry 及晚期受管后端故障也未由此新矩阵验收。
原完整 typed events 的当前包回归位于 `tree-link-tooltip-events-regression-first/`（两 DPR、
18 场景），六套旧输入／提示／SVG／Canvas owner／控件／标签回归均独立实际退出 0。

独立读回与原件核对保存于 `tree-link-tooltip-independent-review/`，总收据为
`artifacts/verification/tree-link-tooltip-acceptance.json`。最终同脚本的实际 DOM 负控位于
`tree-link-tooltip-dom-controls-final-third/`：保持完整 public metadata 不变，分别改色和偏移
真实已连接卡片，原像素／几何门各以实际子进程退出 1 拒绝。负控与 reader 的私有自测分开。

所有早期 RED 均保留。作者修正包括自然 shell 的 8px padding、真实 Surface 尺寸前置、
错误包装、未采用的 Source 初值、已隐藏待清理的旧提示 revision，以及自定义交互子控件
显式 pointer-events。已采用的当前数据、像素及几何门始终独立严格检查。
原默认 Chromium DPR2 在拒绝前后产生 56 个源圆周边缘像素差异；真实身份控制证明
35 个对象的引用、父节点、属性、BCR／CTM／样式不变，MutationObserver 为 0。
整数宿主对照仍在后续 factory 拒绝出现像素 RED；单独 Chromium 启动参数对照通过后，
正式矩阵明确固定该受检配置，未将原 RED 改写为通过，也未认定具体驱动或 AA 根因。
原 Chrome／Edge Node 退出 134 的 OOM、剩余矩阵取消和辅助 shell 资源错误另有原件。
整图比较改为线性字节相等与有界差异诊断，仍要求零字节差；RGBA 正控保持原门。

CI 新增独立 Chromium 两 DPR 连线提示任务并上传日志／原件。最终质量检查覆盖 fmt、
typos、actionlint、发布脚本门禁、Node 语法、空白与文档链接／README 便携性。
本轮未推送工作树，未将工作流静态检查描述为远端 Actions 已运行。Glory 延后 DOM
写入的 fault／重新挂载边界继续保留；自动 Spring／退出、实时控件导出与 GPU/worker 后置。

## Development: Tree spring property foundation

本轮继续完整自动 Tree Spring／组件退出目标，先落实可供宿主采用的类型化逐属性模型，
不将该阶段记为自动 Browser／Glory 功能完成。新增 `TreeMotion`、7/9/3 通道、独立业务键、
Live／Leaving 完整来源、实例复活、原始物理状态、显式时钟及事务预算；目标 Scene 不变。

12 项独立专项覆盖闭式欠阻尼数值、非零速度重定向、退出负尺寸过冲与重入、Link／Label
继续旧目标、标签换槽／直接旋转、原始正负零、父子结构键、非法时步／union 预算、
reduced／finish、未归属标签、禁用边与源工厂不重放。定向首轮实际退出 0，原件在
`artifacts/verification/tree-spring-targeted-first/`。展示直径按独立通道限制，未以坐标
假定 Circle 组成而重写尺寸；独立无阻尼共振正控验证坐标达到限制时直径轨道仍保留。

真实参考脚本 `scripts/tree-motion-reference.mjs` 从指定 Nivo 快照提取并执行 Node／Link／
Label 六个 regular／leave 函数及 keys／options 闭包，也核对原始 UID、端点引用与标签映射。
四输入、56 角色行的源码／MIT／commit、输入、输出和提取位置在 `tree-spring-reference-first/`；
独立复跑主文件逐字节一致，实际运行退出 0。该参考不执行 React Spring 时间积分。

物理预期由独立 Python 标准库闭式公式计算，未导入产品 `SpringValue`；原数值、生成器
与审计在 `tree-spring-planning/independent-oracle/`。审计核对 240 个专项源归档文件与原
完整日志，另保留最新纯模型读回和旧 Link-tooltip 收据哈希不变的证明。

当前完整原生检查在 `tree-spring-native-first/`：最小特性 1969、全特性 2305、文档 18
项通过，native 全目标及 WASM 严格 Clippy 均实际退出 0；1281 个源／测试／示例文件
在执行前后保持相同。完整可运行例子 `examples/tree_motion_data.rs` 另以真实 cargo run
执行，输出当前位置、目标、速度和原始负零位值；收据见 `tree-spring-example-first/`。
双语 README 与[模型指南](tree-motion.zh-CN.md)明确该阶段边界。

总阶段收据为 `artifacts/verification/tree-spring-foundation-verification.json`，
`foundationVerified=true`、`featureAccepted=false`、`automaticBrowserPending=true`。
后续仍需 BrowserTree RAF、实际采样绘制／拾取、Glory animated-property Bonds、退出 Scope
保留和 Source／层／事件／提示的共同采用，以及完整浏览器验收；GPU/worker 继续后置。


## Tree SVG 自动逐属性 Spring 与 Glory 退出组件（Unreleased）

本节接续上方仅基础模型的历史记录，未改写其原始收据。
`tree_motion_paint` 将实际 Node／Link／Label 属性投影为验证后的采样 Scene，保留目标
来源、私有退出 ID、标签字体／裁剪与混排映射。额外 6 项动态配置、11 项独立绘制检查
均通过；绘制主检查覆盖 3 种节点形状 × 4 Cartesian 方向 × 5 种曲线，使用独立闭式
物理预期与几何控制点。首次内部节点测试作者输入及后续测试初始化 lint 的 RED 原件
均保留；修正未放宽业务断言。

原生默认测试 **1986**、全特性 **2322**、文档 **18** 项通过。最终严格 Clippy 与
11 项绘制复验通过，精确等价的测试初始化变更由
`tree-spring-native-final-second/native-source-bridge.json` 桥接；首轮 strict 101 日志保留。
README 的后续文档段落更新另经最终文档测试与质量检查，不将旧 README 哈希冒称当前值。

最终真实浏览器记录为 `tree-spring-browser-matrix-final/`：Chromium／Firefox／WebKit
各 DPR 1／2，**6 contexts、18 配置、450 步**全部通过。主程序验证独立解析物理值、
真实 SVG 四段 Bézier 控制点、Glory 节点／链接／标签 DOM 盒、1212 次组件读回、1553 条
同 cohort effects、18 张实际 PNG 的 36 个安全内部像素及 156 条原生输入。
组件本地计数、退场 inert／历史 generation、退休前重入、退休后新实例、重复层、
`.custom_animated`、动态配置、None 后同批启用和换目标、相机／非均匀 CSS／hidden、
原生 Down 暂停／Up 恢复及 CSS 隐藏停止／显示恢复均有实际正控。采样节点与固定目标
anchor 分离，目标宽度零而实际描边可见时保留真实 Link tooltip。构建／逐帧 renderer
拒绝保持旧 DOM、数值与时钟，精确同 dt 重试通过。

最终包来自 `tree-spring-consumer-build-sixth/`，严格 WASM 编译、release 与 bindgen
均实际退出 0，源码 fence 相同；WASM SHA256 为
`26f9dbaa002c49825d84cce6b64966da8dba72a35c629bf1ad433937958144ce`。
Main 在启动前比对 producer 源码及包哈希，执行前后再次核 fence，保留确切 driver 与
PNG 解码器字节。Chromium 实际使用 `--disable-gpu`，Firefox／WebKit 使用默认启动参数。

前序真实 RED 全保留，包括错误的测试 renderer 默认值、将 Circle 误认作 DOM circle、
超出当前 plot 的测试点、Mesh 覆盖链接、紧贴裁剪边界的像素及未等待 UI 更新的读取。
第七轮在 Geometry 正控下暴露真正的 tooltip-only target／sample Scene 校验错误与
被预算错误遮盖的原始失败，已修复。首轮完整矩阵及独立 WebKit focus probe 证实
clip.scrollLeft=8 导致位姿偏移；固定裁剪改用不可滚动的 overflow:clip 后，在相同
bbox 门禁与真实键盘动作下通过全矩阵。未调整像素预期或几何阈值规避这些产品问题。

总收据为 `artifacts/verification/tree-spring-acceptance.json`；本节仅覆盖声明的 SVG
配置，不推导 React Spring 积分等价、Canvas／Radial Spring、任意 Widget 导出、所有
设备协议或 GPU/worker。原事件、Link tooltip 与基础模型历史收据继续保持原字节。

## Development: Waffle functions and owned data

本轮 Unreleased 增加逐组颜色、数值格式与边框函数，以及 `WaffleDataFrame<D>`。
函数按原始分组顺序执行 color → formatter → border，包含隐藏、零值和溢出组，
最后调用一次 Empty 样式模板。已解析结果共同用于 Cells／Areas、图例、交互和自定义
单元格。原始应用字段由同一 owned Frame 保留，实际 Glory 子组件无需 `D: Clone + PartialEq`。
完整 API、前置校验及外部回调副作用边界见[英文指南](waffle-functions.md)和
[中文指南](waffle-functions.zh-CN.md)。

原版 Nivo 的 `useWaffle` 计算闭包、网格及多边形函数经真实 AST 提取并执行，
10 个 profile、81 组、253 次回调和 10 次 Empty 模板调用通过；原文件、MIT 许可、
提取范围和独立分配／面积审计保存在 `waffle-factories-reference-first/`。
该参考只证明上游纯计算和输入序列，不替代 React 运行时或真实宿主验证。

最终源码完整原生验收为默认 2003 项、全特性 2339 项及 18 项 doctest；原生所有
targets 与 WASM 的严格 Clippy 均实际返回 0。两份指南中的四个 Rust 程序实际编译并
运行，`waffle_factories` 实际导出 Cells／Areas SVG 和原值位表。旧接口结构及 SVG
保持检查覆盖 64 组配置；原始对象地址、非 Clone／PartialEq 数据、失败候选的释放和
逐来源绑定由专项原生测试补充。

最终同包浏览器矩阵是 Chrome／Edge／Chromium／Firefox／Playwright WebKit × DPR 1／2：
10 个上下文、40 个 SVG／Canvas × Cells／Areas 场景、240 帧、160 次候选构建拒绝，
共 3040 个真实 PNG 像素检查点。原始 status 驱动样式，raw metric 单独从 `-0` 变为
`+0` 并修改额外字段时，所有实际保留 Bond 共同看到新来源，Scene／PNG 保持相等，
原 Cage 计数与 DOM 焦点保持。revision 替换则精确检查构造／Scope／清理增量、同槽位
实际 DOM 身份变化及新局部计数。原生填充、边框、自定义 Circle 和 Glory 内容均采样，
RGBA 最大差仍为 1。所有构造出的 Scope 在停止后均清理。

几何模型保持独立的 Scene 坐标公式。DOM 门禁使用由这些公式生成的不可见、独立 CSS
参考盒，逐项核对百分比声明，并将真实控件边框与参考盒逐值精确比较；局部 visible
metadata 仍严格等于数值模型。该参考处理浏览器百分比布局量化，不从实际控件的位置
反推期望，不移动绘图区或更改像素门槛。首轮完整矩阵的 Firefox 约 `1/60` CSS 像素
量化差异保留为失败；采用独立参考后，从头运行完整第二轮，不拼接旧通过项。

全部新证据位于 `artifacts/verification/`：采用
`waffle-functions-consumer-build-final/` 的包，完整矩阵在
`waffle-functions-browser-matrix-second/`，原生在 `waffle-functions-native-final/`，
示例在 `waffle-functions-examples-final/`。独立 PNG／来源读回、最终质量门及受检范围
由 `waffle-functions-acceptance.json` 汇总。早期非法测试根值、测试字段路径、缺失
启动容器、按钮行内基线偏移、误设单 Canvas 数量及严格 lint 的原始失败报告均保留。

四次拒绝只覆盖颜色函数、超长 formatter、Empty 边框函数及单元格函数在纯候选构建
阶段返回错误；不推导延迟 Browser 后端或 Widget 工厂失败的回滚。浏览器各组额外字段
刻意相同，逐组对象身份另由唯一原数据／地址的原生测试提供。Areas 仍只为实际空格
提供 cell Widget，不声称已有组区域 Widget、任意 Waffle 函数层、原生专属事件或 Spring。
Chromium 配置显式使用 `--disable-gpu`；GPU／worker 继续后置。

新增 `scripts/build-browser-acceptance.py` 在真实 consumer workspace 通过 Cargo metadata
解析 target，归档实际源码、共享模型、嵌入字体、README、两组 manifest／lock、命令日志
及最终包哈希；不重定向构建环境。GitHub CI 新增独立 Chromium 双 DPR 工作，运行该
producer 以及 Waffle、Tree spring 和普通 Tree 注释门禁，并始终上传证据。
Actionlint／发布门禁在本地验收；本轮未运行远程 GitHub Actions，未创建新 tag 或 release。

## Development: ordinary Tree motion Scene extras

普通 Tree SVG motion 的程序现在按 Links → Nodes → Labels → 单个 Scene extras → Mesh
排列。此前默认层已包含 Extras，却又追加一次 Extras；真实未归属注释会重复并被源序校验
拒绝。`ChartConfig.title` 仅为 Scene 元数据，本身不生成这个注释。
修复不改变字面 SVG 程序的默认层；真正交错的源序仍需显式字面程序，不能偷偷重排。

最终 Waffle 构建同时通过五种浏览器配置、两 DPR、60 步注释专项：真实可见中英文注释
只出现一次、精确手动采样保留它、非法交错序列拒绝后 pose／DOM／PNG 保持原样并可恢复。
同包原 Spring 主矩阵的 Chromium／Firefox／WebKit × 两 DPR、18 配置／450 步也全部通过。
新增普通程序 3 项原生测试随完整原生验收通过。

证据为 `tree-motion-extras-browser-matrix-final/`、
`waffle-functions-tree-spring-regression-final/` 和
`tree-motion-extras-acceptance.json`。原 Tree Spring、事件及 Link tooltip 总收据
保持原字节；本补充收据记录新构建，未覆盖或改写历史结果。

## Development: Funnel literal SVG layers

The Unreleased dedicated SVG `BrowserFunnel` passes five browser configurations
(Chrome, Edge, Chromium, Firefox and WebKit) at DPR1/2, with four vertical/horizontal
and linear/smooth profiles: 10 contexts, 40 profiles and 1,610 accepted frames.
The run also checks 560 rejected transactions, 20 real paint/camera negative
controls and 2,800 saved PNGs. Source-over samples allow one channel unit;
geometry has a separate 1/30 + 0.0001 CSS-pixel gate. Independent offline readback
checks the saved source, native paths, pixels, retained scopes and input evidence.

One captured business source feeds literal builtin omissions, repetitions and
ordering plus actual stateful Glory whole-layer components. Source replacement,
typed identities and signed zero, disabled-label style results, source-free
separators, camera/hidden changes, current capabilities, real keyboard/native
pointer input, resize, revision/removal, component budgets and native placement
faults are exercised. Owner and per-runtime complete Cage data adopt before any
runtime pumps; observations are compared with their actual contemporaneous owner.

Failed managed UI keeps accepted data and cumulative error diagnostics while its
whole carrier remains `display:none` and inert. Actual Glory batch controls show
that later successful refreshes keep the failed instance isolated. An explicit
front-component revision restores that fresh component while the unchanged back
component remains hidden; removal and reinsertion restore both with fresh scopes.
Postcommit UI failure is distinct from rejected precommit state.

The original Nivo Funnel reference executes 108 literal layer profiles and real
D3 generators for 48 body and 48 border paths. Its cold React/JSX/Spring adapters
are declared; this does not establish React/Spring runtime equivalence. Native
checks pass 2,018 minimal and 2,354 all-feature tests, 18 doctests, native and WASM
strict Clippy, and six bilingual Rust snippets (four native programs executed,
two browser bodies type-checked). Same-artifact Waffle and ordinary Tree extras
regressions pass Chromium at both DPRs.

A plain SVG control reproduces WebKit suppressing every observed window-exit
input after stationary hovered-path replacement. The delivered-exit test first
makes a real Move over the accepted replacement body before its trusted Out;
undelivered stationary-replacement exits remain an input limitation. No OS input
equivalence is claimed. Four public Funnel part hooks and tooltip are covered by
the later [native-event and tooltip acceptance](#development-funnel-native-events-and-tooltips).
Bound annotations, Spring/exit, generic raw business types and arbitrary mutable
D3 generators remain separate work. Existing generic Funnel Canvas APIs remain;
GPU/worker is deferred.

CI runs the source-bound Funnel Chromium suite at both DPRs and verifies workflow
syntax using a pinned checksum-verified actionlint. Release depends on the full
reusable CI workflow. These are locally verified configurations; no remote CI run,
new tag, release or publication is claimed for this Unreleased work. See the
[English guide](funnel-svg-layers.md) and [中文指南](funnel-svg-layers.zh-CN.md).


## Development: Funnel native events and tooltips

The Unreleased dedicated SVG `BrowserFunnel` now exposes four independent native
part hooks: Enter, Move, Leave and Click. The complete source-bound run passes
Chrome, Edge, Chromium, Firefox and WebKit at DPR1/2 across four direction and
interpolation profiles: 10 contexts, 40 profiles, 2,000 accepted frames, 160
rejected transactions, 20 real negative controls and 2,360 saved PNGs. Original
process exit is zero. The independent reader checks archived source, package,
frame JSON and PNGs rather than accepting the driver's summary as proof.

Callbacks receive the actual browser `MouseEvent` at Window bubble phase and an
immutable admission frame: source/SVG publication, part index, typed occurrence,
client, viewport and Scene coordinates. Six held Moves remain six independent
callbacks. Callback registration replacement, clearing, stop, authored errors,
queued mutations, rejected synchronous flush and recursive native dispatch are
exercised. The recursion test found and fixed a `FnMut` listener reentry trap;
interior state now sits behind immutable DOM listeners. Callback errors have a
separate bounded diagnostic channel and survive a successful source flush.

Native-component activity and tooltip presentation are opt-in. Enter updates
current and tooltip before its callback, Move updates only tooltip, matching Leave
clears both, and Click changes neither. The historical Owner activity policy
remains available. Same-occurrence raw Over after DOM replacement cannot overwrite
a manually assigned current with a fabricated business Enter. Actual Move remains
independent. Hover requires a primary non-touch pointer; Click excludes touch and
keeps the original event's `isTrusted` fact without adding a trust security gate.
No undelivered window-exit event is fabricated for the WebKit limitation described
in the literal-layer section.

Standard tooltips use DOM text for the raw label and cached formatted value,
including literal HTML-like labels, empty labels, signed zero and transparent color
chips. A retained custom tooltip is an actual Glory WebApp with a complete
`Bond<Option<FunnelTooltipFrame>>`; local Cage state, real keyboard activation,
focus, revision replacement and scope cleanup are checked. Nine natural-size
anchors, the default 14-pixel gap, positive nonuniform CSS scaling and camera
coordinates are checked separately from source-over pixels. No rotated/skewed
ancestor or collision-avoidance equivalence is claimed.

Owner, source layers and tooltip data adopt together before any managed runtime
pumps. Reorder updates the accepted tooltip's source index while preserving its
typed identity; a raw-only zero change updates data without changing paint. Event
payloads remain historical admission snapshots. Tooltip payloads instead rebase
to the current accepted source, which differs from an upstream React element
holding an older computed-part object. Pure tip changes reuse source/SVG
publication without rerunning the builder, layout, style functions or layer
factories. Layer and custom-tooltip pending/live/retiring instances share the
128-component budget; standard tooltips allocate no WebApp.

Rejected constructors, source functions and late placement preserve accepted
owner/data/DOM/PNG, including keyboard focus on the same retained component. The
focus rollback control exposed and fixed focus lost while relocating a retained
panel. Postcommit non-finite UI output follows a different contract: the failed
instance retains accepted data and error diagnostics but remains `display:none`
and inert. A later successful real Glory batch keeps it isolated; a new revision
restores a fresh component. Callback side effects are not rolled back.

Geometry allowance stays 1/30 + 0.0001 CSS pixels and RGBA stays one channel unit.
Before product fixtures, two fixed plain-HTML calibration boxes select one CSS or
device-border percent-translation reference model per context. Thirty-six fixed
holdouts then verify that same model at all nine anchors and nonuniform scale;
independent plain-browser controls also cover Chromium, Firefox and WebKit at both
DPRs. No per-product-frame model selection or tolerance widening is used.

Chromium-family contexts explicitly launch with `--disable-gpu`, consistent with
the existing acceptance drivers. Earlier GPU-enabled runs remain failed evidence:
one SVG antialias pixel changed during an otherwise exact rejected transaction.
The final software-capture run retains the strict whole-viewport equality gate;
it does not claim to fix a GPU compositor or establish hardware-renderer parity.
Screenshots preserve caret state and allow ongoing animations.

The same frozen sixth consumer package also passes the full literal SVG Funnel
regression (40 profiles and 1,610 frames) and Chromium DPR1/2 Tree node/link tooltip
regressions after the shared panel focus change. Native checks pass 2,018 minimal
and 2,354 all-feature tests, 18 doctests and strict native/WASM Clippy. All eight
Rust blocks in the bilingual event and tooltip guides compile as real WASM
bodies; runtime behavior is covered separately by the browser fixture.

Evidence under `artifacts/verification/` is archived in
`funnel-interactions-consumer-build-sixth/`, `funnel-interactions-browser-matrix-third/`,
`funnel-interactions-independent-final/`, `funnel-interactions-native-first/`,
`funnel-interactions-examples-final/` and `funnel-interactions-quality-final/`.
The final feature receipt is `funnel-interactions-acceptance.json`; older accepted
receipts and all failed attempts remain unchanged. The executable upstream
reference checks real Funnel handler and tooltip helper functions with declared
cold React/JSX/Spring adapters, not a live React/Spring runtime.

CI builds a source-bound consumer package and runs the new Chromium suite at both
DPRs, retaining evidence on failure. Local format, spelling, actionlint and release
gates are checked; no remote GitHub Actions run, new release or publication is
asserted for this Unreleased work. Bound annotations, Spring/exit, generic raw
business types, arbitrary mutable D3 generators, builtin item Widgets and arbitrary
UI export remain separate. GPU/worker remains deferred. See the bilingual
[event guide](funnel-events.md) / [事件指南](funnel-events.zh-CN.md) and
[tooltip guide](funnel-tooltip.md) / [提示框指南](funnel-tooltip.zh-CN.md).

## Development: Funnel bound annotations

`FunnelSvgFrame::with_annotations` binds specifications to its original computed
source and returns an immutable annotation cohort. Typed IDs retain string and
numeric identity; predicates receive complete computed parts in spec/source
order. Circle, dot and rectangle outlines, relative/absolute note positions,
connector geometry, text halos and custom Scene notes use an independent literal
Annotations layer. Replacement does not accumulate old annotation paint or
replay captured constructor functions. Annotation paint can occlude a body using
native SVG input rules, but never acquires a business part identity.

The original Nivo reference executes 144 cases with 708 bindings, eight Lodash
shorthand cases and 48 angle cases using pinned original functions and declared
cold adapters. The committed numeric table is consumed by native Rust tests.
An independent replay verifies that table and its upstream provenance.

The final five-browser, two-DPR, four-profile matrix passes all 40 profiles
and 900 frames with one frozen consumer and script. It includes 120 rejected
candidates, 20 actual DOM negative controls and 40 glyph paint-contribution
controls. The original process exits 0 with an unchanged source/package fence.
Independent readback decodes every original PNG and checks raw source, bindings,
DOM, component Bonds, native events and the complete reference provenance.
The feature receipt is `artifacts/verification/funnel-annotations-acceptance.json`;
all failed attempts and previous accepted receipts remain unchanged.

Physical sample cells are selected solely from authored geometry. Each dot
sample compares against a separate native-circle page executing the committed
original Nivo component, at the same authored pose, camera, CSS scale, layer
copy count and underlay color. The reference never receives product paths or
pixels. Its source, compiler, Node binary, HTML, DOM, PNG and worker terminal
are archived. Geometry allowance remains 1/30 + 0.0001 CSS pixels and RGBA
allowance remains one channel unit. A Firefox native control disproved the
earlier pure-fill expectation at a qualified cell; all failed runs remain
preserved. Selected-pixel agreement does not establish whole-primitive edge
rasterization or compositor equivalence.

Every profile includes actual glyph-stroke paint contribution and exact
restoration, retained layer/tooltip source adoption, original native input,
and three rejected source-function/budget candidates. Two actual DOM color and
pose mutations per browser/DPR context must fail the unchanged ordinary gates.
The glyph experiment uses its documented same-node SVG repaint cleanup;
rejected source transactions still require an exactly unchanged viewport.

Shared `LabelStroke` preserves native fill-then-stroke text across SVG, Canvas
and CPU raster renderers. Zero-size annotation shapes remain empty paint.
Inactive strokes preserve existing COLR palette fill behavior; unsupported
active color-glyph strokes fail explicitly. Native checks pass 2,046 minimal
tests, 2,391 all-feature tests, 18 doctests and strict native/WASM Clippy.
The four Rust blocks in the bilingual guides compile as actual WASM bodies.
The same frozen consumer also passes Chromium DPR1/2 regressions for Funnel
literal layers and events/tooltips, Tree Canvas and retained Canvas Widgets,
and SVG Tree motion.

CI runs the annotation suite on its source-bound producer at both DPRs and
uploads evidence on failure. Local workflow lint and release gates pass; this
work asserts no new remote CI execution, commit, tag, release or publication.
Annotation Spring/exit, per-note retained Widgets, generic application datum
and arbitrary live UI export remain separate; GPU/worker remains deferred.
See the [English guide](funnel-annotations.md) and
[中文指南](funnel-annotations.zh-CN.md).

## Development: Funnel owned application data

`FunnelDataDatum<D>` pairs opaque application data with an explicitly supplied
projection. `FunnelDataFrame<D>` owns a shared raw pool and a checked computed
frame; the typed browser facade, style/size/stage roles, annotations, legacy
Widget roles, literal layers, mouse hooks, hit snapshots and tooltips retain that
same source. No Clone, PartialEq, Debug, serialization, Send or Sync requirement
is imposed on D. Pure frames also accept borrowed nonstatic data. The existing
pure `FunnelSvgFrame` remains Send + Sync.

Native checks pass 2,056 minimal and 2,401 all-feature tests, 18 doctests and
strict native/WASM Clippy. Six real Rust bodies in the bilingual guides compile
for WASM. The same frozen consumer passes five selected legacy lanes: Funnel
native events/tooltips, bound annotations, literal SVG layers, and Tree node/link
tooltips over the shared panel implementation. These are bounded legacy checks;
the new typed owner has its own full browser matrix.

The frozen source-bound package and driver pass Chrome, Edge, bundled Chromium,
Firefox and WebKit at DPR1/2, with four vertical/horizontal and linear/smooth
profiles per context: 40 profiles, 2,010 frames, 360 rejected candidates and ten
actual DOM paint negatives. Independent offline readback decodes every one of
the 2,020 original PNGs and checks 240 synchronous presentation proofs, 1,993
original profile Window records plus 40 explicit boundary records. Both the
original matrix process and independent reader exit 0. Chromium-family contexts
explicitly launch with `--disable-gpu`; no hardware compositor stability is inferred.
The source/package fence remains unchanged. Evidence is retained under
`artifacts/verification/funnel-data-browser-matrix-third` and
`artifacts/verification/funnel-data-independent-final`; the consolidated receipt
is `artifacts/verification/funnel-data-acceptance.json`.

The fixture uses a deliberately traitless D containing nested business strings,
signed zero and Rc/Cell. Public typed frames supply all observed fields and real
raw addresses. A raw-only source replacement preserves the complete projection
while updating generation, actual Glory paint and accepted component data.
Captured events and hits retain their original frame; tooltip selection instead
rebases to an eligible newly accepted source. A late actual Window capture
listener accepts B after A was admitted and before bubble, proving the old event
is dropped rather than reinterpreted using B.

Each profile exercises six source/factory/annotation failures, two real retained
component constructor failures, and the shared live-plus-pending component
budget. Rejected candidates preserve accepted source, peers, focus, DOM and the
complete viewport exactly. Successful refresh/input then uses the accepted raw
pool. Revisions and stop retire real scopes; historical owned raw frames remain
readable. An actual Graphic fill mutation must fail the ordinary pixel gate.

Presentation-only commands carry synchronous before/after source proofs within
one JavaScript task: raw/computed data, build count and source generation remain
unchanged. Later real browser Enter/Leave input is checked separately and may
legitimately rebuild controlled current state. The earlier WebKit failure that
conflated these transactions remains archived. Initial mount effects have no
fixture handle until mount returns; their complete initial frame is reconciled
with the returned owner. All later active effects require actual owner/peer/full
capture identity agreement. Keyboard setup explicitly focuses the real retained
button before trusted activation. NativeComponent does not invent a missing
Leave when bodies are omitted.

Original Window records are drained incrementally into one complete per-profile
index. Screenshot rows retain deltas, and unadmitted boundary records are saved
explicitly. Independent readback checks context sequence conservation, all raw
fields and float bits, admission identity, component cohorts, real DOM geometry,
pixels, rejected transactions and every original PNG. Geometry allowance remains
1/30 + 0.0001 CSS pixels and RGBA allowance remains one channel unit.

Rust factory phases retain existing semantics, including computed profiles in
stage-style contexts; this does not claim Nivo's earlier pre-profile evaluation
order. D's interior mutability and external callback side effects remain the
application's responsibility. The typed dedicated owner is SVG-only; pure Scene
and legacy role adapters serve existing Canvas/raster APIs. Spring/exit, mutable
D3 generators, item mixing, arbitrary live UI export and GPU/worker remain
separate. Current work remains Unreleased, without a new remote CI run, commit,
tag, release or publication. See [中文指南](funnel-data.zh-CN.md) and
the [English guide](funnel-data.md).

## Development: Funnel property springs

The Unreleased `FunnelMotion` model keeps the accepted target Frame separate
from its actual sampled Scene. Body and border each use an independent scalar
path spring and real D3 ordinal string interpolation from the previous committed
target. Color/width, label position/color, separator coordinates and annotation
outline/link/note positions have separate tracks. Part opacity, text, custom
note Scenes and unowned extras remain target-authored. New roles appear at
target and removed roles disappear immediately, matching Nivo's keyed maps;
no upstream Funnel exit transition is invented.

The original Nivo hook/component requests, keys and D3 interpolation were
extracted and executed against Git-exact source and archived licenses. An
independent analytic oscillator and D3 helper check physical values and
velocities, interruption endpoints, changed topology, annotation geometry and
paint. This validates the declared Rust analytic spring contract, rather than
claiming execution of the React Spring scheduler. Nine focused tests cover
edited stage-factory ink, typed keys, atomic failure, independent channels,
negative physical border widths without phantom hairlines, immediate role
removal and historical target/sample ownership. Six real Rust bodies from the
bilingual guides compile for WASM; their snippets are not executed examples.

The dedicated SVG owner supports automatic RAF and exact Manual intervals,
system/explicit motion preferences, configuration changes that preserve
velocity, hidden-surface suspension and wall-clock rebasing on restore.
Source/physical presentation/layers/tooltips adopt one complete cohort. Native
body paths and part labels preserve actual DOM identity through journaled
replacement. A real injected DOM failure after native leaf placement must
restore attributes, child nodes, parents, complete viewport pixels and the
focused application button. The source factory is not replayed by physics.

The browser fixture uses traitless owned business data, historical event
certificates, retained Glory paint and local button state. A sample-only body
cell accepts trusted input, while a target-only cell grants no business input.
A late Window capture listener advances the physical publication before bubble;
the old admitted observation must be dropped. A fresh observation admits the
new pose. Exact native role membership, actual path tokens, actual text
position/color, same-Cage peer effects and original Window sequence records
are retained. Physics-only commands use their synchronous before/after result,
separately from later native Enter/Leave source transactions.

Screenshot sampling uses a geometric selector over the independently checked
public sampled Scene. It reads no product DOM, hit-test result or pixels to
choose cells. Opaque interior cells compare at RGBA1 against actual SVG and a
direct Canvas preview; cells without a geometric qualification are recorded as
ineligible. Native geometry/text gates and retained Graphic paint still apply.
Matched-series hiding is checked explicitly. The Canvas preview draws the pure
sampled Scene and is not a new dedicated Canvas owner. With motion disabled,
the authored Graphic intentionally becomes transparent over its fixed white
background while native/Canvas target paint and retained Scope state remain.

Complete component observations are saved in ordered, lossless gzip batches,
with compressed/uncompressed SHA256, byte counts and entry counts. A small
native/callback index remains redundant with the full original batches. Frame
rows retain their original entries and batch reference. Independent readback
streams every batch, checks exact conservation and publishes complete unique
owner observations for offline numerical replay; it does not expand all FX
into one huge array.

The complete five-browser, dual-DPR, four-profile matrix passes with its original
coordinator and all five original browser lane processes exiting 0: 40 profiles,
1,930 frames, 200 rejected candidates, ten actual DOM paint negatives and 2,340
original PNGs including exact before/after rejection pairs. The package, source
and authored driver fences stay unchanged. Final independent offline readback
also exits 0, decoding every original PNG and checking 2,472 complete event
batches, 13,018 original entries, 12,498 retained effects, 11,858 unique accepted
owner observations, 120 actual native-node stability observations and 40
sample-only/target-only input pairs. All 40 matched-hidden samples pass. Earlier matrices were stopped after a required
WASM library Clippy correction and a V8 single-JSON-string size failure. All
failed/partial evidence is retained and excluded from final acceptance. The
final source/package/driver epoch was checked as a whole from scratch.
Current native checks pass 2,065 default and 2,410 all-feature tests, 18 doctests,
and strict native/WASM library checks. Six bounded legacy Funnel/Tree tooltip
lanes pass on the same frozen package. Original browser evidence is retained in
`artifacts/verification/funnel-motion-browser-matrix-third`, independent evidence
in `artifacts/verification/funnel-motion-independent-final`, and the consolidated
receipt in `artifacts/verification/funnel-motion-acceptance.json`.

That receipt covers the earlier captured-Scene-role implementation. It does not
cover dormant source-role memory; the extension is recorded separately below.

Precommit rejection preserves accepted model, source, pose, peers and focus;
application callback side effects remain external. Backend errors after
adoption retain the existing UI error isolation boundary. Automatic runtime
checks use actual RAF and relative serial advancement; exact math uses Manual
time. Chromium-family contexts explicitly disable GPU. Mutable D3 generators,
item-widget mixing, arbitrary live UI export, GPU/worker and broader native OS
input equivalence remain separate. This work remains Unreleased, with no new
commit, remote CI run, tag, release or publication asserted. See the
[English guide](funnel-motion.md) and [中文指南](funnel-motion.zh-CN.md).

## Development: Funnel dormant source-role motion

Body/border tracks now survive absent paint while the source key remains. The
constructor captures default degenerate paths and actual custom targets once;
motion does not replay factories. The explicit all-source stage pass covers
custom collapsed sources. Legacy drawable-only callbacks retain their contract;
an uncalled custom target rejects motion before adoption. Actual edited Mark
geometry remains authoritative. Transparent explicit colors retain numeric
targets, while missing optional paint creates no invented color channel.

Twelve native cases verify analytic width/velocity, previous-target path resets,
custom capture and callback counts, typed raw source sharing, reduced motion,
history, budgets and invalid latent paint. Current default/all-feature/doctest
counts are **2077 / 2422 / 18**, with strict native and WASM checks passing.
Ten actual WASM guide bodies compile; these snippets are type checked, not run.

The focused five-engine, dual-DPR matrix passes **40 profiles / 560 frames /
880 operations**. An independent Python closed-form model, independently authored
source/profile targets and original D3 templates verify 1,416,600 state numbers,
990,720 path numbers and 10,320 native paths. Forty real absent-body click
negatives pass. All 560 original PNGs are hash checked and decoded for structure;
this focused driver claims no pixel gate. Its evidence is retained under
`artifacts/verification/funnel-dormant-browser-matrix-first` and
`artifacts/verification/funnel-dormant-independent-focused-final`.

The complete original motion regression also passes on the same new source-bound
package: **40 profiles / 1,930 frames / 2,340 unique PNGs**, 5,720 RGBA checks,
200 rejected candidates, 10 actual DOM negative controls, 440 native callbacks,
120 retained-node checks and 40 physical-input pairs. Its independent streamed
reader verifies all **2,432 gzip batches / 11,525 original entries**, including
11,005 effects and 32,595 peer reads, without flattening the event ledger.
The original reader and numeric replay bytes remain unchanged; the adapter only
updates scope prose and records the inherited reader hash. These ordinary and
focused results jointly establish this extension's declared acceptance scope.
Six bounded legacy Funnel/Tree lanes also pass on this package. Evidence remains
under `artifacts/verification/funnel-dormant-motion-regression-second` and
`artifacts/verification/funnel-dormant-independent-motion-final`.

The first ordinary regression attempt is retained. Its hidden/resume scenario
allowed animation to settle before hiding, so the author driver now hides before
retargeting and explicitly checks that the hidden target is unsettled. The pause,
resume and all other original gates remain intact; a fresh complete matrix passes.

The dormant CI step uses the same producer,
Chromium, four profiles and both DPRs; static workflow checks and local release
gate tests pass, with no remote CI execution asserted.

The dedicated browser owner remains SVG, and pure sampled Scenes remain usable
with Canvas/raster. Dormant state adds neither ink nor business targets. Mutable
D3 generators, item-level Widget mixing, arbitrary live UI export and GPU/worker
remain separate. No new commit, tag, release or publication is asserted. See the
[English guide](funnel-dormant-motion.md) and [中文指南](funnel-dormant-motion.zh-CN.md).

<a id="development-waffle-literal-svg-layers"></a>
## Development: Waffle literal SVG layers

The new sealed `WaffleSvgFrame` and `WaffleSvgPlan` capture actual cell ink and
full-slot group polygons once. Literal Cells, transparent native Areas, explicit
Legends and retained Glory whole-layer occurrences can be omitted, repeated and
interleaved. The dedicated projected and typed browser owners share one source
cohort with native mouse hooks and standard or retained tooltip components.
These APIs remain Unreleased.

The source reference executes original Waffle hooks, layer descriptors and mouse
functions from the vendored licensed Nivo snapshot. It covers 384 descriptor
profiles, sixteen numeric/function cases, four directions and original D3 path
generation. This cold execution is not React/browser acceptance. Native tests
compare all sixteen cases without changing original expected outputs. Independent
edge tests cover bounded oversubscription, checked request overflow, hidden
sources, corner-touching group topology, resource budgets and fixed legend boxes.

Full checks against the frozen implementation pass **2095 default tests / 2440
all-feature tests / 18 doctests**, with strict native and WASM Clippy passing.
All four Rust bodies in the two new guides compile against the actual WASM
libraries. They are type checked, not executed as applications. Records are
retained under `artifacts/verification/waffle-svg-native-seventh` and
`artifacts/verification/waffle-svg-examples-fourth`. The final prose refresh keeps
those four Rust bodies unchanged; guide compilation is part of final signoff.

The new shared raw-data path preserves the supplied Arc; old Vec constructors
preserve their original buffer and numeric admission rules. Early regression
attempts exposed and then fixed a Vec-to-Arc address regression, original
corner-touching polygon differences and strict-check issues. Failed attempts
remain recorded rather than being overwritten. Sixteen prior acceptance
receipt hashes are protected by the new tranche manifest.

The fresh complete matrix passes **10 browser/DPR contexts / 40 profiles / 2280
captures / 2160 operations / 280 candidate rejections / 20 actual DOM negative
controls**, with unchanged source and all lane processes returning zero against
`artifacts/verification/waffle-svg-producer-ninth/pkg`; matrix records are in
`artifacts/verification/waffle-svg-browser-matrix-second`. Independent readback
also passes: **7340 raw reads /
2368 effects / 712 hooks / 920 data frames / 34720 peer rows / 3100 decoded PNGs /
9,765,000,000 decoded pixels**. It covers every capture, operation, rejection and
negative control, 80 actual ResizeObserver operations, 200 native-pattern reference
cases including 120 erroneous-pattern controls, and 40 exact tiles with 6400
physical pixel cells. Forty retiring-tooltip records with a historical Some frame
have independent retirement certificates. The successful report is
`artifacts/verification/waffle-svg-independent-final-third/report.json`, SHA-256
`1892438d16a90975f4e34b1e393bec2ff20c11790c7b7ba1bb148caa9588c687`.
Its first attempt rejected the serialization of a resize CSS string and remains
preserved as a failed reader run. Final signoff checks the source, all evidence
hashes, guide compilation, quality gates and protected historical receipts; an
acceptance receipt is produced only when every check passes. The browser
driver records original native events, complete raw-data captures, every
component effect and peer observation, DOM identity/attributes, viewport PNGs,
rejected candidates and actual DOM mutation controls. A passing build or static
workflow does not establish these runtime properties. The new CI step uses the
shared source-bound producer, Chromium and both DPRs, with always-uploaded raw
artifacts; no remote CI run is claimed.

The original strict rollback PNG control exposed twelve differing Chromium edge
pixels even though accepted metadata and owned DOM were restored. A separate
pure-DOM control, without WASM or an owner, reproduced the same twelve pixels
after the original rounded HTML tooltip was shown and hidden. They persisted
across fixed RAF waits; a native path move/restore returned the cold image.
The Waffle Stage now adds only `will-change:transform` to native-content's style,
retaining its geometry, camera, overflow, isolation and pointer policy. An
independent cold control with that exact configuration matches the private
full-history before/after ROI. Sixteen cases and 112 Pillow whole-viewport
comparisons preserve both the original RED and the fixed-configuration evidence
under `artifacts/verification/waffle-svg-rollback-controls/`. This identifies a
bounded tooltip/native repaint interaction, not a particular GPU or driver cause.
The original whole-viewport rejection gate remains exact, with no AA mask,
waiting retry or pixel allowance.

The fixed patterned triangle case also retains its original two sample locations
and channel tolerance of one. Firefox's native SVG pattern rendering disproved
the former assumption that a motif-center pixel must be its pure fill color.
`scripts/waffle-svg-pattern-reference.mjs` now builds a separate native reference
only from author allocation, geometry, palette and pattern parameters, in the
same browser/DPR and carrier configuration. The complete authored 8-by-8 CSS
pixel tile is compared with exact RGBA in addition to the two sample gates.
Wrong foreground, wrong spacing and missing-circle controls must fail that same
tile comparison. Reference HTML, DOM, PNG and control records are retained in
the acceptance output tree; product pixels do not select a reference or sample.
This does not claim universal SVG primitive/pattern AA equivalence.

Actual ResizeObserver controls change root content size and restore it while
checking adopted dimensions, retained component identity and focus. The observer
uses content dimensions rather than externally transformed BCR. Stop controls
check disposed scopes, no pending RAF, rejected weak mutations, unchanged builder
and factory counts, and no hook delivery from clicks at a former Area position.
CI's existing `always()` upload of `browser-extensions-ci/**` recursively includes
the generated reference evidence, alongside `browser-extensions-*.log`.

The same package also passes the bounded legacy Waffle regression (eight
profiles / 48 frames) and the bounded Funnel DPR-1 profile-0 regression. These
separate checks retain their own scope; they do not replace any new owner case.

The dedicated owner remains SVG. Dedicated Canvas ownership, Waffle Spring and
exit tracks, per-item mixing within these built-ins, arbitrary live UI export
and full Nivo legend/React prop compatibility remain separate. GPU/worker work
stays deferred. See the [English guide](waffle-svg-layers.md) and
[中文指南](waffle-svg-layers.zh-CN.md).
