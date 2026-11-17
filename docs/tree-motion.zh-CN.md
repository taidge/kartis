# Tree 逐属性 Spring（Kartis 0.2.0）

[English](tree-motion.md)

`TreeMotion` 提供显式时钟的类型化动画属性。具有 motion 能力的 `BrowserTree`
挂载入口接入 SVG 绘制、自动 RAF、真实 Glory 组件保留及来源／图层／提示共同采用。
目标数据与实际物理绘制分别可读。参考 Nivo Canvas Tree 直接绘制当前计算数据；
Canvas Spring、Radial 动画、GPU 与 worker 不属于本 SVG 配置。

## 数据与属性

从合法 Tree 布局构造 `TreeEventFrame`，然后调用
`TreeMotion::new(frame, TreeMotionOptions::default())`。
`retarget(next)` 采用新的不可变目标，`tick(seconds)` 返回共享的 `TreeMotionFrame`。
时钟步长必须有限且位于 `0..=60` 秒。完整程序见
[tree_motion_data.rs](../examples/tree_motion_data.rs)：

```sh
cargo run --example tree_motion_data
```

| 角色 | 动画属性 | 退出目标 |
| --- | --- | --- |
| Node | X、Y、直径与 RGBA | 原目标 X/Y/color，直径归零 |
| Link | 两端点 X/Y、粗细与 RGBA | 保持原目标属性 |
| Label | X、Y 与旋转角度 | 保持原目标属性 |

新角色直接以目标属性进入。已静止的离开 Link／Label 立即退休，仍运动的离开角色
继续向原目标收敛，全部轨道静止后再移除。`finish()` 与 reduced motion 使当前属性
抵达目标，并清除离开角色。

默认 mass/stiffness/damping 为 `1/120/14`，对应命名的
[React Spring gentle 预设](https://www.react-spring.dev/docs/advanced/config)。
Kartis 使用解析物理求解器及自身的显式静止阈值，不据此声称 React Spring 的积分、
颜色插值、过期或调度逐帧相同。旋转采用直接数值路径。

## 身份、来源与边界

Node／Label 沿用全局唯一的扁平业务 ID；Link 使用结构化 `(source_id, target_id)`，
不会因分隔符产生键碰撞。重新挂接父节点保留 Node／Label 实例，同时更换入边键。
这是现有 Rust 身份契约的延续；Nivo 使用祖先路径 UID。

每行提供 `phase()`、`instance()`、类型化 `props()` 和独立的 `states()`。
7/9/3 组状态包含 value、target、velocity。真正退休前重入保留实例、当前位置与速度；
退休后再次进入则获得新实例并直接使用目标属性。

Live 行共享新的完整 `TreeEventFrame`，Leaving 行保留原不可变 frame，包括原始正负零、
源索引与 Mark／Label 元数据。仅原始值变化也会更新目标数据。标签按实际
`Scene::label_series` 查找来源；未归属注释另列，重复且不明确的 owner 会被拒绝。

每次成功 retarget 增加 `target_serial`；非零且未静止的 tick 以及显式收敛增加
`sample_serial`。零时步不推进物理状态。每角色预算涵盖 Live 与 Leaving；非法时间、
配置或 union 超限不改变已接受状态。部分预算检查发生在候选复制与分配之后。

`set_options(next)` 原子更新弹簧参数与 union 上限，保留当前位置、目标、速度与
实例；新的静止阈值在下一次 tick 生效。单独配置变化不发布新的采样 frame。
Frame 相等性表示共享同一个不可变快照，也能区分仅原始正负零不同的数据捕获。

## 显式原生绘制

`tree_motion_paint(motion.frame())` 为四个直角方向构建经过验证的
`TreeMotionPaintFrame`。其 Scene 包含采样后的节点几何与纯色、链接端点、粗细与
颜色，以及标签坐标和旋转；`target()` 保留原目标。节点边框、自定义 paint、裁剪、
透明度与文字沿用目标属性，当前混合绘制顺序及未归属注释保持不变。

退场图形使用私有 paint ID，`target_item` 对退场返回 `None`；当前来源索引与实例
映射各自独立。历史标签字体、裁剪和来源数据保留到退休。这些映射描述来源关系，
单独的绘制 Scene 不会安装输入处理器。

默认投影拒绝编辑过的原生几何、编辑过的链接描边、Radial 方向和多 Mark 透明度组。
通过 `tree_motion_paint_with_renderers` 可提供显式的逐帧几何或描边配方。
Mark renderer 必须保留业务元数据，完整候选验证后才返回；后续配方失败不能撤销
renderer 自身的外部副作用。不支持的方向与透明度组在调用任何 renderer 前拒绝，
也包括仍然引用旧来源的退场角色。

展示值分别限制各数值通道，包括非负直径／粗细及 RGBA；原始物理值和速度保留。
这些限制不替代组合自定义几何的验证。原目标 Scene 及任意 Mark Path 保持不变，
可通过 `frame().target().scene()` 读取；它不是动画绘制 Scene。
无 Mark、零值或禁用角色不会因具有动画属性而自动获得绘图或输入资格。

## 浏览器调度与输入

显式选择 `BrowserRenderer::Svg`，通过 `mount_with_motion`、
`mount_with_factory_and_motion` 或 `mount_with_svg_layers_and_motion` 挂载。
`BrowserTreeOptions::default()` 沿用通用图表的默认 renderer，因此 SVG 需明确选择。
旧静态挂载保持原有契约，不支持事后启用自动 motion。

默认 `BrowserTreeMotionClock::Automatic` 使用 RAF；切换为 `Manual` 后，
`advance_motion(seconds)` 通过相同的绘制与组件采用路径精确推进。
`motion_frame()` 返回已采用的数值、来源 generation、publication 与配置序号。
候选失败保留已接受的物理状态和时钟。自动动画从空闲状态启动新目标时重置时基；
原生按住期间暂停自动替换。隐藏、脱离文档或零尺寸 Surface 停止循环动画请求；
重新显示通过 ResizeObserver 恢复，也可显式 `refresh()` 恢复重新挂接的 owner。

`set_motion_options(Some(next))` 保留速度并更新配置；`None` 收敛到当前目标、清除退场，
同时保留存活 Glory Scope。同批重新启用 motion 与更新目标时，从旧姿态开始动画。
系统 reduced-motion 偏好仍生效。

原生几何与输入资格使用采样后的存活图形，事件与 tooltip 仍携带当前目标数据，
包括采样描边尚可见但目标宽度为零的链接。节点固定 anchor 使用目标位置；链接 cursor
anchor 使用真实指针。plot 与隐藏 series 仍约束自动选择；最上层 Mesh 捕获最近节点
输入，若需直接操作其下原生链接，应选择 `BrowserTreePicking::Geometry` 或省略 Mesh。
退场图形不获得当前来源的自动输入资格。

## Glory 组件与字面图层

`mount_with_motion_and_animated_components` 或
`mount_with_factory_svg_layers_motion_and_animated_components` 创建真实 Glory 组件。
`BrowserTreeAnimatedWidgetContext` 提供候选 `initial` 及来自同一已采用快照的
`frame`、`animated_props` 两个 Bond；构造期间两者为 `None`。应用使用类型化 Node／
Link／Label 属性定位内容。宿主提供完整 Scene carrier、只应用一次 camera，并按固定
plot 裁剪；外部 CSS 缩放保持在浏览器外层。

同 occurrence／instance／revision 保留真实 Scope 与本地状态。退场 frame 的
`data_generation` 标识历史来源，`source_generation` 标识当前目标。退场组件 inert；
退休前重入复用原 Scope，退休后进入则新建。删除 occurrence 立即清理其 Scope；
新增 occurrence 不为已有退场行构造组件。禁用 Link 不创建 Link 组件；启用但零尺寸／
透明的逻辑角色可由应用显式提供 UI，不因此伪造原生图形或自动输入资格。

字面 occurrence 保持原生／UI 混排顺序。整层 `.custom_animated` 通过同一快照派生的
Bond 同时接收目标层数据与共享 motion；普通 `.custom` 保持目标数据契约。
来源／样式工厂仅在目标构建时运行；组件工厂仅为新实例或 revision 运行。采样、camera
与隐藏 series 更新保留现有组件。存活与候选 Scope 联合限额；宿主提交前失败恢复旧
publication，Glory 延后 backend 写入失败沿用已提交错误报告边界。

默认原生投影严格检查编辑过的几何。`mount_with_motion_renderer` 系列接收显式逐帧
presenter，返回 paint 必须引用该候选的确切 `TreeMotionFrame`。普通入口在采用前拒绝
非标准来源绘制顺序；字面 SVG program 可显式安排顺序。同一 owner 的数值时钟由各
occurrence 共享，不为每层建立独立 React Spring 引擎。

独立原生检查、原版 Nivo 配方与实际浏览器记录见[验收说明](verification.md)。这些证据
对应上述 Rust 配置，不代表 React Spring 时间积分、任意 Widget 导出、所有输入设备
或 GPU/worker 等价。
