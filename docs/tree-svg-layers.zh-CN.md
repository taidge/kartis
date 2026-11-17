# Tree SVG 函数层（Kartis 0.2.0）

[English](tree-svg-layers.md)

浏览器接线需要 WebAssembly 与 `web`；`TreeSvgFrame`、`TreeSvgPlan` 及布局构造函数也可用于普通 Rust。受检浏览器矩阵覆盖五种浏览器配置、两种 DPR 和 3690 帧，证据与边界见[验收记录](verification.md#development-tree-svg-function-layers)。已有 Tree 构造函数和浏览器挂载入口保留原有默认行为。

`BrowserTreeSvgProgram` 将按字面顺序绘制的 Tree 层与任意 Glory 子树组合。自定义子树挂载在真实 SVG `foreignObject` 内的 XHTML 容器中，可包含 Glory 控件和 `Graphic` 绘图。这是对 Nivo SVG 函数层的 Glory 适配，不是 React 组件，也不是任意原始 SVG 节点插入接口。

## 按字面顺序绘制

每个 `TreeSvgLayer` 有唯一的 occurrence key，以及 `Links`、`Nodes`、`Labels`、`Mesh`、`SceneExtras` 或 `Custom(registry_key)` 类型。传入向量就是绘制顺序。省略内建层便省略其墨迹；重复内建层会再次绘制同一来源的视图。重复视图具有独立 DOM/资源命名空间，不复制业务 Mark、规范控件或来源 ID。背景只绘制一次。

`BrowserTreeSvgProgram::defaults()` 按 extras、links、nodes、labels、mesh 排序；`new(vec![])` 创建空程序。默认 `RequireLayer` 策略要求未归属的 Scene 注释具有 `SceneExtras` 层；若明确省略它们，可使用 `with_extras(TreeSvgExtrasPolicy::Omit)`。Scene 透明度组必须在单个 occurrence 中完整、连续，拆分该组的程序会被拒绝。

整个层的工厂注册名与各 occurrence key 分开指定：

```rust
use glory::Widget;
use kartis::{Result, TreeSvgLayer, TreeSvgLayerKind};
use kartis::browser::{BrowserTreeSvgLayerContext, BrowserTreeSvgProgram};

fn program<W: Widget + 'static>(
    factory: impl FnMut(BrowserTreeSvgLayerContext) -> Result<W> + 'static,
) -> Result<BrowserTreeSvgProgram> {
    BrowserTreeSvgProgram::new(vec![
        TreeSvgLayer::new("links", TreeSvgLayerKind::Links)?,
        TreeSvgLayer::new("under", TreeSvgLayerKind::Custom("panel".into()))?,
        TreeSvgLayer::new("nodes", TreeSvgLayerKind::Nodes)?,
        TreeSvgLayer::new("over", TreeSvgLayerKind::Custom("panel".into()))?,
        TreeSvgLayer::new("labels", TreeSvgLayerKind::Labels)?,
    ])?
    .with_extras(kartis::TreeSvgExtrasPolicy::Omit)
    .custom("panel", 0, factory)
}
```

两个 `panel` occurrence 拥有独立的子组件状态。occurrence key、注册名和 revision 不变时，重排程序会保留子组件；改变 revision 会重建。在相同保留身份下替换工厂不会执行新工厂；需要重建时应修改 revision。

## 已接受的数据与坐标

工厂接收 `BrowserTreeSvgLayerContext`。其中 `initial` 是暂定构造输入；`frame` 是初始为 `None` 的保留式 `Bond<Option<BrowserTreeSvgLayerFrame>>`，组件应从该 Bond 绘制已接受数据。帧包含 occurrence key/index、来源 generation、publication、逻辑相机、隐藏 ID、交互开关和 `TreeSvgFrame`。

`TreeSvgFrame` 提供完整原始扁平来源、计算后节点和边、解析后的边样式、活动状态、主题、plot、来源 Mark 查询、extras、节点 ID 索引及 SVG link generator。来源数值保留原始正负零位。节点坐标属于外层 Scene；在自定义容器内使用 `inner_position(node.position)` 减去 plot 原点。容器尺寸为 `inner_size()`，宿主只应用一次 plot margin 和已接受相机，不要再次叠加这些变换。

存在 Labels occurrence 时会请求正常 Tree 标签计算；全部省略时跳过归属节点标签的构造及 label 函数。已有 Tree 标签可见性选项仍生效。自定义层可以独立使用完整节点与来源数据，不会因此获得伪造的 Scene 标签。权威构造函数接收含 `layers`、`labels_required` 的 `BrowserTreeSvgBuildContext`；使用 `tree_svg_layout_with_activity` 保持这一调度合同。

## 浏览器 owner 与来源组件

使用 `BrowserTree::mount_with_svg_layers` 或 `mount_with_factory_and_svg_layers` 挂载；通过 `BrowserTreeHandle::set_svg_program` 更换程序，通过 `svg_layer_frame(key)` 读取任一已声明内建／自定义 occurrence 的已接受帧，读取内建层不创建额外子组件。要求 SVG renderer、受控高亮、Transform 视图下的来源 Geometry picking，不支持 crosshair、editable brush 或强制动画。

`mount_with_factory_and_svg_layers_and_components` 还能组合现有 Node/Link/Label Widgets，要求 `source_ordered: true`。每个 occurrence 都使用覆盖完整 Scene 的 foreignObject 承载容器；普通内建层包含原生 SVG 视图，含来源组件的内建层将原生 SVG 绘制段与 Widget 按来源顺序交错，重复内建层拥有独立组件实例。自定义内容在固定的 viewport 绘图区裁剪内使用 HTML 相机，应用盒子与坐标继续以绘图区为基准。标签不拦截输入；真实绘制的装饰边即使没有规范节点控件，也能遮挡下方自定义子树的输入。

## 输入与当前节点

Mesh occurrence 是处于其字面顺序位置的真实输入层。最近节点活动需要该层、`interactive(true)` 和 `BrowserTreePicking::Nearest`。更晚的自定义子组件可在其上接管输入，更晚的 mesh 也能拦截子组件上的输入。节点和边通过投影后的来源墨迹参与输入；省略视觉角色不会生成伪命中目标，隐藏及已替换的来源墨迹也不进入该投影。Mesh 诊断具有单独的 512 个 Voronoi 站点限制，并非 Tree 总节点限制。

`context.current.set_current_node(Some(id))` 将更新排入之后的 owner 处理，`None` 清除当前节点。这个弱能力会检查已接受的来源 generation 与 occurrence 存活状态，构造期间、退役后或 stop 后调用会被拒绝；它不会在组件回调中 flush。首次已接受帧的 effect 可以排入操作；设置已经请求的当前节点是空操作。`interactive(false)` 关闭自动来源/mesh 输入，但保留自定义 Glory 控件及其显式 current-node 能力。

## 事务、预算与限制

已接受的来源状态、来源组件帧、自定义层帧和受管 tooltip 数据共享接纳边界，然后才执行这些更新的 pump 与旧组件清理。构造函数及其初始 effect 仍会在候选准备期间执行：自定义层的 Bond 为 `None`，`initial` 输入明确是暂定值；来源组件继续遵循既有的候选帧构造合同。同步来源准备、自有遮罩／承载容器的 DOM 写入及语义放置检查可在接纳前拒绝，保留已接受数据、自有显示和手势。

Glory 0.8 将托管渲染器的写入排队，交给后续 runtime pump 执行；本适配尚未提供这些托管后端写入的回滚事务。渲染后端错误可能留下已应用的前缀，并使 runtime 进入故障状态；遇到此类后端故障需要重新挂载所有者。响应式属性错误在已接纳的帧上报告，并隐藏受影响的显示，另有其恢复路径。effect 前的数据共同接纳不代表托管原生 DOM 写入已在接纳前完成。

失败的 desired 程序只尝试一次，普通原生输入继续使用 accepted 程序；显式 refresh 或作者更新可以重试。只改变顺序或重复次数不会重放布局／样式函数，来源角色选择不变时复用 placement。相机／隐藏状态重投影复用自定义子组件，空闲 flush 不重建它们。

`TreeSvgBudget` 限制层数、重复来源工作量、路径命令数以及共享的存活/待定组件数。已有组件与候选新增组件必须同时符合预算，不能先退役已接受组件来为候选腾位置。默认上限为 128 层、128 个组件、200,000 工作量和 2,000,000 来源命令。这是准入限制，不代表任意子组件的成本保证。

此 API 不提供 React/Spring 过渡、原始 React SVG/defs 插入，或 Nivo 节点、连线各七种鼠标事件的独立 handler 属性（SVG 共 14 个）。SVG link generator 不绑定 Canvas context。导出来源 Scene 不会重现 foreignObject Glory 子树或字面重复层视图。GPU/worker 渲染及这些剩余事件、导出合同属于后续工作。
