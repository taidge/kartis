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


