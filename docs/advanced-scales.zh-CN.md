# 连续输入变换、多断点坐标轴与地理反算

[English](advanced-scales.md)

## 颜色输入

`ColorScaleOptions::transform` 为 Sequential 和 Diverging 提供 Linear、Log、Pow、Sqrt 和 SymLog 输入变换。默认仍为 Linear，离散色标要求 Linear。输入变换独立于颜色插值空间和命名色带。

HeatMap、日历和 Choropleth 共用该配置。将解析后的色标传给 `append_color_legend`，刻度位置、渐变取样与实际数据颜色便使用同一变换。发散中点仍是数据值，在图例中可以不处于几何中心。

Log 域要求同号非零，负指数幂也要求同号非零分支。源样本保持有限值或显式缺失。查询行为另行处理：保留 D3 有限的有符号幂外推和钳制零点极限，最终颜色参数非有限时使用 `unknown`。

空或全缺失样本沿用通用自动域 `[0,1]`，此时 Log 和负指数幂需显式提供合法域。当前受检策略不虚构非零标定边界，也不静默过滤真实零值或异号观测。

`position(value)` 返回变换后的轴比例。`value_at(offset)` 接受 `[0,1]` 内的有限比例，并返回原域内受检反算值；溢出、下溢越域或不可表示的结果明确返回错误。

## 多断点图型坐标轴

`PiecewiseScaleOptions` 保存完整 `domain`、`range` 向量和 `ContinuousScaleOptions`，原两端点选项保留 Copy 接口。数值与 UTC 配置接受 2–4096 个等长节点；域允许非严格升序或降序，包括重复节点和全常量域，真正转向仍拒绝。UTC 节点须为有界整数毫秒。重复节点可以产生跳跃，按 D3 原生变换后的 reverse-then-bisect-right 选段，包含负指数幂。[重复域节点指南](piecewise-repeated.zh-CN.md)说明常量域中点和全域 clamp 规则。

`ResolvedContinuousScale::piecewise` 和 `ChartScale::with_piecewise_options` 使用真实输出坐标。Nice 只扩展外侧节点，内部节点保持；Reverse 反转整个输出向量。刻度仍使用原值单位。Round 对实际算出的浮点值执行 ECMAScript 取整，反算保持未取整映射。

输出域可以转向或包含平台段。前向映射可用，`invert` 拒绝不唯一的全局反算。`invert_segment(index,pixel)` 显式选择分支，range 平台分支仍拒绝；重复 domain 分支可以产生常量代数反算。`is_invertible()` 表示能选定唯一 range 分支，不承诺双射或 map／invert 往返相等。

实际图型使用 `PiecewiseLineScaleOptions` 搭配 `line_layout_with_piecewise_scale_options`，或 `ScatterAxisScale::Piecewise`。它们的 range 是首节点 0、末节点 1 的绘图区比例。内部节点可转向或超出该区间，仍沿用绘图区裁剪。坐标轴、路径、点工厂、原始数据锚点和拾取共用解析后的映射。

### 数据视口

**重复节点视口集成自 Kartis 0.2.0 提供。** 原生 Line、Scatter 验收矩阵核对 4,128 条原 D3 视口记录，包含完整重排、受检拒绝和重置。浏览器绘制与平台数学边界另见[重复节点指南](piecewise-repeated.zh-CN.md)。

Line 和 Scatter 控制器保存全部原始显示断点。与原 D3 `rescaleX`/`rescaleY` 一致，导航通过累计像素变换逐个反算原输出节点。Resize 保留已接受域，仅更新后续输入使用的尺寸。

`BrowserViewportLayout::domains` 继续提供规范化外侧端点，`piecewise_domains` 额外提供保持作者顺序的全部节点：

- Line 使用基准布局的 `viewport_scale_options(&request.piecewise_domains)`。
- Scatter 使用 `viewport_scale_options_with_piecewise(request.domains,&request.piecewise_domains)`。

将返回配置传入下一次布局，普通轴使用外侧端点。不要用两个端点重建多断点映射。Scatter 的旧端点 helper 对多断点轴明确拒绝。转向或平台输出可绘制，但不能建立唯一的数据视口。静态常量域可用，导航仍要求可用的外侧跨度；真正转向、非法变换或不可表示的提案须保留已接受控制器和帧。

重复节点导航须保留原局部像素 range，并在原映射上反算 `(rangeStop - tx)/k`。另做 `[0,1]` 归一化往返可能改变不连续 Log 分支选择，或让 UTC TimeClip 相差一毫秒。range 方向已含源顺序和输出反转；Resize 须保留已接受域与原映射来源。

## 地理反算

既有十种投影均提供 `GeoProjectionContext::invert([x,y]) -> Result<Option<[lon,lat]>>`。它使用已接受的拟合 raw center、比例、平移、相机中心与 roll，包含横轴墨卡托的坐标约定。

与 `project` 相同，`invert` 检查投影固有足迹和显式 cap，不暗中裁剪绘图区；`invert_in_plot` 额外检查该矩形。非有限或超出 Scene 预算的坐标返回错误，支持足迹外返回 None。极点和接缝不要求唯一的经度表示。

浏览器 handle 提供 `scene_point_at_client`、`invert_geo_at_client` 和 `invert_geo_in_plot_at_client`；位图 handle 提供对应 `*_at_host` 方法并接受当前 `RasterSurfaceBounds`。两者复用已接受的宿主转换，Scene 相机只逆一次。显式 plot 方法检查显示及作者绘图区。

应用应将投影上下文与对应已接受帧共同保留。尺寸检查可以拒绝大小不匹配，不能识别任意同尺寸的过期上下文。

## 数值与宿主边界

独立参考执行固定版本的原 D3 函数。显式 piecewise 比例尺现统一使用固定 `libm` log／log1p／exp／expm1 原语，Nice 或视口重建去掉重复后也一样；普通两端点比例尺保留原算术。与此前严格域的 2,400 个配置、31,904 次查询比较，76 个 raw 位模式变化，最大绝对差为 `5.684341886080802e-14`；仅两处取整变化将曾记录的 SymLog `208` 修正为原 D3 的 `207`，历史值与差异名单均保留。

原生构建以及未启用 `canvas` 的独立 `wasm32` 核心，对一般有符号幂沿用平台 `powf`。`wasm32` 配合 `canvas`（含 `web`）特性时，显式 piecewise 比例尺安全导入实际宿主的 `Math.pow`，与同一运行时的原 D3 使用相同原语。该特性选择浏览器数学，不改变普通两端点比例尺或固定 `libm` 对数原语。首轮完整 WASM 矩阵在引入该导入之前记录了 72 个指数 `0.25` 配置中的 108 处失败；重建后的浏览器数学版本已在单个 Chromium 上下文完成同一 9,496 配置矩阵，没有差异或一个 ULP 取整例外。该 WASM 实测独立于原生验收，也不承诺所有引擎的超越函数逐位相同。重复节点附近一个 ULP 可选中完全不同的分支，须定位原因，不能用通用像素容差掩盖。地理精确 cap 点与奇点仍与普通反算等价分开，保持既有投影边界容差。

浏览器发布继续记录 revision。相同绘制属性不重复写入；支持 Canvas2D `reset()` 的宿主复用不变的 backing 尺寸，同时清理绘图状态，旧宿主保留尺寸重置路径。这不承诺显式替换 Scene 时保留手势所有权。受控同帧渐变异常验证的是渲染错误捕获和旧像素恢复，不涵盖任意用户回调副作用。

证据见[验收记录](verification.md)、[差距清单](feature-gaps.md)和[第三方许可](third-party-notices.md)。
