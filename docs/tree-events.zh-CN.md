# Tree 浏览器原生鼠标回调（Kartis 0.2.0）

[English](tree-events.md)

浏览器适配提供节点、连线各七个独立回调。受检原生输入合同已通过五种浏览器配置、两种 DPR 的完整矩阵，具体场景与边界见[验收记录](verification.md#development-tree-native-mouse-hooks)。已有构造函数、`BrowserTreeOptions`、`on_event` 与 `on_update` 保留原接口。

## 注册回调

```rust
use kartis::Result;
use kartis::browser::{
    BrowserTreeHandle, BrowserTreeMouseHandlers, BrowserTreeNodeMouseActivity,
};

fn display_node_clicks(
    tree: &BrowserTreeHandle,
    mut display: impl FnMut(String) + 'static,
) -> Result<()> {
    let mut handlers = BrowserTreeMouseHandlers {
        node_activity: BrowserTreeNodeMouseActivity::NativeComponent,
        ..Default::default()
    };
    handlers.nodes.on_click = Some(Box::new(move |event| {
        display(format!(
            "{}: raw={}, aggregate={}, position={:?}",
            event.data.datum().label,
            event.data.datum().value,
            event.data.node().value,
            event.input.scene,
        ));
        Ok(())
    }));
    tree.set_mouse_handlers(handlers)
}
```

`handlers.nodes` 与 `handlers.links` 都有 `on_mouse_enter`、`on_mouse_move`、`on_mouse_leave`、`on_mouse_down`、`on_mouse_up`、`on_click` 和 `on_double_click`。每个槽位都是独立可选的 boxed `FnMut`，返回 `kartis::Result<()>`。更换此 bundle 不会替换 `on_event`／`on_update`，也不构建来源或重放组件工厂。`clear_mouse_handlers()` 清除此通道；没有任何回调的 bundle 不会启用新输入策略。

## 数据与调用相位

`BrowserTreeNodeMouseEvent`／`BrowserTreeLinkMouseEvent` 包含不可变 typed `data`、已接受的 `source_generation` 和 `input`。`TreeEventFrame` 共享完整原始来源、计算节点／连线、解析后的连线样式、活动、主题、配置和来源 Scene。原始值保留正负零，aggregate 单列；节点索引属于原始扁平输入顺序，计算连线顺序和端点来源索引各自独立。选择或克隆目标共享该数据池，不重放布局或样式函数。

`data.mark()` 可为空；存在时只是原来源 Mark，不证明当前物理命中。没有墨迹、零尺寸、空业务标签的节点和禁用连线仍可有有效数据，不伪造 canonical 交互站点。当前来源画笔／几何编辑不会回写构造时的节点中心或连线样式。

`tree.source_generation()` 不构建数据，直接读取当前已采用的来源 generation，包含已接受的 activity 更新，与 Core revision／字面层 publication 不同。应用异步结果前可将其与保存事件的 generation 比较。stop／drop 后该 getter 返回错误；组件 effect 可以读取已经采用的帧。

`input` 保留实际 `web_sys::MouseEvent`、语义 `kind`、精确 client／逻辑 viewport／逆相机 Scene 坐标、可选 pointer 身份、明确 `origin` 和可选 SVG occurrence key/index/publication。PointerEvent 是 MouseEvent 子类，实际原生类型仍可读；Mesh 派生 Leave 因而可携带真正的 `pointermove`。重复 occurrence 不复制业务 ID。

坐标与 pointer-capture 标志属于 envelope 的原生捕获阶段快照。图表传输可在晚回调前建立捕获，因此 Down 快照可为 `captured == false`，即使回调运行时 Surface 已取得捕获。原始 event 仍是活的 DOM 对象，分发结束后的 `current_target()` 不保留当时快照。

回调在同一个原生事件的 Window 晚冒泡阶段同步执行，必要图表传输先预留，所有者和回调借用已经释放。它不从 RAF 更新、`SurfaceEvent::Activated` 或 Up 反推。payload 在应用回调替换来源前捕获本事件的已接受数据；自动 current／提示意图可以已排队，而 payload 仍是上次渲染的计算数据。回调内 flush 不改写已捕获 payload。

交付以浏览器实际分发事件为前提。默认右键菜单可能中断后续指针输入；需要副按钮释放回调的宿主可以显式处理 `contextmenu` 并阻止其默认动作。注册 Tree 回调本身保留浏览器的默认菜单行为。

真实 DOM Click 仍属于 Click，即使 PointerEvent 的 primary 为 false；真实键盘 Click 的 detail 可为零。程序 `SurfaceInput` 激活不伪造原生 Click。DOM 合成事件明确 `is_trusted() == false`；旧 Owner／来源模型仍可从合成 DOM Move 更新 activity，托管提示保留可信输入门槛。NativeComponent 的合成原生节点／连线／空白 Move 保留显式 current、不建立可信提示，合成 Enter/Leave 不预留新的 current 或提示。上述属于浏览器协议，不代表 OS 笔／触摸设备认证。

## 路由配置

| 来源 | 合同 |
| --- | --- |
| 普通 SVG | 真实、合资格的节点／连线路径优先；Nearest 下其它 Surface 位置使用明确的 source-model nearest。这是 Rust 混合配置，不是不可见的 Nivo Mesh 层。 |
| 字面 SVG 层 | 按真实顺序、省略、重复、遮挡、隐藏／替换墨迹和已接受 occurrence 判断；只有实际 Mesh 或匹配的捕获 contact 启用其 nearest，自定义 UI 不穿透。 |
| Mesh／source-model nearest | 原生进入区域时查询最近节点，可产生 Enter；首次命中的 Move 也产生 Enter；同节点产生 Move；A→B 当次只 Leave(A)，不追加 Enter/Move。Down/Up/Click/DoubleClick 按本次实际坐标查询，只更新内部比较状态，不设置外部 current／提示。 |
| Canvas Geometry | 原始已绘制来源几何定义逻辑角色。空白→节点的 Move 只 Enter；A→B 为 Leave(A)+Enter(B)；后续同角色 Move 才 Move；节点→空白 Leave。这是明确的来源几何派生相位，不是逐节点原生 DOM 事件。 |
| Canvas／原始 Canvas | 只有节点回调；注册连线回调或连线绑定在替换已接受注册前返回错误。原始回调像素不会自动成为命中几何。 |
| 捕获来源 | 按当前坐标重新查询；Geometry Up/Click 落空不伪发原 Down 节点。匹配的 contact 保留 SVG occurrence；`CapturedSource` 区分实际 DOM target 与来源查询。 |
| Glory UI | 未绑定组件、标签、自定义层、overlay 与嵌套图表拥有自己的输入，转发业务角色需要明确绑定。 |

稳定业务 ID、occurrence key 与 binding 身份共同归一化 hover；接受新帧或重绘 DOM 不会凭空制造 Enter。重排来源使用本事件当前已接受的数据重新解析。这不同于 Nivo 按 Mesh 数组位置保存 cursor 及 React effect 时序；删除／退休目标不能被改标成其它来源。

相机更新把节点移出静止指针位置后，真实 pointer Out 可以结束先前已观察的 hover。历史 DOM 路径此时可能已变为被动；Leave 描述旧 hover，不表示重新命中该路径，且角色仍须在当前已接受来源中解析。这不允许从退休或 inert 画笔产生 Enter、Move 或离散回调。

## 当前节点与双击

`BrowserTreeNodeMouseActivity::Owner` 保留已有自动原生 Move 活动配置。启用回调后，真实 SVG／明确绑定节点的 Enter/Leave 也会在回调前预留 current 和提示。

`NativeComponent` 面向实际 SVG 节点与明确绑定节点，包括 Canvas 上的真实 Glory UI。Enter/Leave 修改 current，节点 Move 只更新节点提示；SVG Geometry 下的 Link／空白 Move 不替换已显式设置的 current。匹配的原生节点捕获手势保留此策略；Canvas 来源几何、Nearest 和原始 Canvas 保留所有者／模型活动行为。该策略要求存在活动回调注册，不是独立的全局图表模式。

默认 `BrowserTreeDoubleClickBehavior::KeepChartReset`，即使存在 DoubleClick 回调也保留旧 Reset。`CallbacksOnlyOnHandledTarget` 只对合资格且确实有 DoubleClick 回调的目标抑制 Reset；空白、已清除的注册和普通通用图表仍保留 Reset。两次真实 Click 和随后 DoubleClick 分开计数。

## 组件绑定与生命周期

实际元素属于已接受的自有子树后，调用 `bind_node_mouse_element(source_id, element)` 或 `bind_link_mouse_element(target_source_id, element)`；Tree 连线由子节点来源 ID 标识。保留返回的 `BrowserTreeMouseBinding`，Drop 或幂等 `unbind()` 移除绑定；最内层绑定优先。未连接、inactive、跨所有者和嵌套图表元素返回错误，候选构造期不能绑定未接受元素。原生 Scene 标签被动；真实 Glory Label Widget 保留原有 UI 行为。

绑定不伪造物理 Mark，也不改变 canonical selection，origin 为 `BoundComponent`，数据来自当前已接受来源。元素／角色被移除、occurrence 退休或 owner 停止后不能交付旧事件。回调可替换注册或来源、flush、解绑、stop；返回的旧回调不能覆盖新注册。成功的显式来源更新使旧 Up/Click lineage 退休，采用前拒绝保留已接受手势。

回调错误通过 `BrowserEvent::Error` 报告，不回滚应用外部副作用或已预留的原生传输。在晚冒泡观察前停止传播，不会稍后重放未交付的 envelope；清理与活动 RAF 独立。

该接口不承诺 React SyntheticEvent 身份、任意泛型 Datum 字段、D3 平局语义、OS 设备等价、自动 Spring／退出或实时 Widget 导出。Glory 0.8 已单列的受管后端错误／重新挂载边界继续保留；GPU/worker 属于后续工作。
