# Funnel 原生鼠标事件（未发布）

[English](funnel-events.md)

该 Unreleased API 扩展专属 SVG `BrowserFunnel` owner。原生事件与 tooltip 浏览器验收
已通过：10 个配置、40 个 profile、2,000 帧、160 次拒绝及 20 个真实 DOM 负控。
产物与 Chromium 固定 `--disable-gpu` 测试配置见
[验收记录](verification.md#development-funnel-native-events-and-tooltips)。
[字面 SVG 层](funnel-svg-layers.zh-CN.md)介绍挂载和源事务，
[Tooltip](funnel-tooltip.zh-CN.md)介绍标准文字和保留的 Glory 组件。这些 hook 不新增
Canvas 拾取、Mesh 路由或任意元素绑定。

## 安装四个独立 hook

`BrowserFunnel` 及其弱引用 `BrowserFunnelHandle` 提供
`set_mouse_handlers(BrowserFunnelMouseHandlers)` 和 `clear_mouse_handlers()`。
安装一个 bundle 会立即替换全部四个槽，不重建源。必须保持拥有它的 `BrowserFunnel`
存活。以下示例使用 `wasm32-unknown-unknown` 目标的浏览器 API。

```rust
use kartis::ChartError;
use kartis::browser::{
    BrowserFunnelHandle, BrowserFunnelMouseActivity, BrowserFunnelMouseEvent,
    BrowserFunnelMouseHandlers,
};

fn observe(event: BrowserFunnelMouseEvent) -> kartis::Result<()> {
    let datum = event
        .data
        .source()
        .get(event.source_index)
        .ok_or_else(|| ChartError("missing event source row".into()))?;
    let part = event
        .data
        .parts()
        .get(event.part_index)
        .ok_or_else(|| ChartError("missing event part".into()))?;
    let original_event = event.input.event.clone();
    let _observation = (
        datum,
        part,
        original_event,
        event.source_generation,
        event.publication,
        event.input.occurrence,
    );
    Ok(())
}

pub fn install_handlers(owner: &BrowserFunnelHandle) -> kartis::Result<()> {
    owner.set_mouse_handlers(BrowserFunnelMouseHandlers {
        on_mouse_enter: Some(Box::new(observe)),
        on_mouse_move: Some(Box::new(observe)),
        on_mouse_leave: Some(Box::new(observe)),
        on_click: Some(Box::new(observe)),
        activity: BrowserFunnelMouseActivity::NativeComponent,
    })
}
```

| 槽 | 语义 kind | 原始 DOM 事件 |
| --- | --- | --- |
| `on_mouse_enter` | `Enter` | `pointerover` |
| `on_mouse_move` | `Move` | `pointermove` |
| `on_mouse_leave` | `Leave` | `pointerout` |
| `on_click` | `Click` | `click` |

每次回调接收实际的 `web_sys::MouseEvent` 对象；如果它也是 `PointerEvent`，原对象
身份仍被保留。Click 不从 Up、Core activation 或其他 hook 重建。合格事件在 Window
捕获阶段记录，在 Window 冒泡阶段同步交付：回调执行时 `event.event_phase()` 为 3，
`current_target()` 为 Window。若交付前注册或已接纳呈现发生变化，捕获资格失效；
owner 不会把旧事件附到新 Frame。正常 DOM 传播规则仍然适用：被停止传播的事件不一定
到达 Window 冒泡监听器。

## 原始数据与实际目标

`BrowserFunnelMouseEvent` 含完整不可变的 `FunnelSvgFrame`、`source_generation`、
`publication`、计算结果的 `part_index`、原源数据的 `source_index` 和 `input`。
Frame 提供捕获的 config、原始 source、options、layout、计算样式及实际 Scene 墨迹。
索引属于这个 Frame，而非稍后的 owner 快照。克隆 Frame 不会重放工厂。

`input` 含 `kind`、原始 `event`、`client`、逻辑 `viewport`、相机逆变换后的
`scene`、可选的指针事实（`id`、`kind`、`primary`、`captured`），以及
`occurrence { key, index, publication }`。坐标依据事件捕获时已接纳的几何。字面重复层
共享业务源，但具有不同的 occurrence key 和 index。source generation 标识一次已接纳
的源重建，publication 标识一次已接纳的 SVG 源/层呈现；两者都不是事件序号。

只有已接纳 occurrence 中实际连接的 body path 能提供 part。边框、分隔线和标签保持
被动。自定义层的真实按钮可以阻挡下面的 body。省略 Parts、缺少 body、隐藏 series
或关闭交互不会制造目标。遵循 SVG `visiblePainted` 规则：opacity 为零的 paint 仍可能
接收原生指针输入。这不是按非零 alpha 像素拾取，也不限定为纯色近似。

存在 PointerEvent 元数据时，hover 阶段要求 primary 且非 touch。Click 排除 touch，
但不因 `isPrimary == false` 而拒绝：浏览器生成的鼠标 click 和键盘 click 都可能具有
该值。键盘 click 仍须实际指向已接纳的 body。这里没有 `isTrusted` 安全门；连接的
owned DOM 事件保留普通 DOM 语义，`is_trusted()` 报告原始事实。物理验收使用真实
trusted 输入，不把合成输入升级为 trusted。

## Current、tooltip 与稳定 hover 身份

既有挂载在既没有 handler、也未配置 tooltip 时，保留旧 `Owner` 活动策略。
`BrowserFunnelMouseHandlers::default()` 选择 `NativeComponent`。配置 tooltip 也选择
该策略，除非 handler bundle 显式指定 `Owner`。清除 handler 后，若已配置 tooltip 则
恢复 tooltip 策略，否则恢复旧 Owner 行为。

在 `NativeComponent` 下，Enter 在应用 hook 前排队 current 和 tooltip 选择；匹配
的 Leave 排队清除两者。Move 只更新 tooltip 选择，保留手动 current。Click 只调用
hook，不选择 current、不显示或移动 tooltip。这些预留操作只是排队；回调中的 getter
仍返回已接纳快照。Owner 保留原有自动 current 和恢复行为，包括原生 current 批次
被拒后，由真实 Move 重试。这种恢复不会制造 typed Enter。

物理 hover 身份由 occurrence key 和原 body Mark ID 组成。源驱动的 DOM 替换可能
为同一业务角色再次投递原始 `pointerover`。只要该 hover lease 仍有效，这个 Over
不会再交付 typed Enter，也不会重复 Enter 的自动 current/tooltip 副作用。后续真实
Move 仍独立交付。不同 occurrence，或真实 Leave 清除后再进入，仍可 Enter。Move
永远不会改写为 Enter；原始已投递事件可独立于这些 typed 观察被记录。

## 在回调中排队更新

hook 中可以调用 setter 排队。hook 中的 `flush()` 返回错误，不能在原生分发中途
采用新数据或 pump 组件 runtime。RAF 在分发结束后处理排队工作，应用也可在回调外
flush。下例用 Click 更新器替换整个 handler bundle，是第一个示例的另一种安装方式。

```rust
use kartis::ChartError;
use kartis::browser::{BrowserFunnelHandle, BrowserFunnelMouseHandlers};

pub fn install_click_update(owner: &BrowserFunnelHandle) -> kartis::Result<()> {
    let queued_owner = owner.clone();
    owner.set_mouse_handlers(BrowserFunnelMouseHandlers {
        on_click: Some(Box::new(move |event| {
            let mut next = event.data.source().to_vec();
            let datum = next
                .get_mut(event.source_index)
                .ok_or_else(|| ChartError("missing clicked source row".into()))?;
            datum.value *= 0.9;
            queued_owner.set_source(next)
        })),
        ..BrowserFunnelMouseHandlers::default()
    })
}
```

这里有意从历史 Click Frame 构造排队源；需要合并并发编辑的应用应自行决定冲突策略。
排队更新稍后被接纳，不会改变回调里的 Frame。同批 source 和 tooltip 命令共同接纳
或拒绝；不能绕过失败的 source 批次单独采用其中的 tooltip 变化。外部回调副作用不能
回滚。

回调可以替换或清除 handler；返回的旧回调不会覆盖新注册。递归分发另一个 owned
原生事件会在其自动 intent 或回调交付前被拒绝，但 owner 不停止无关 DOM 监听器。
回调返回的 `Err` 记入有界 `snapshot().event_errors` 日志，后续成功 flush 不会清除
它。源拒绝的 `error` 和提交后 UI 错误的 `ui_errors` 是独立通道；不承诺 Rust/
JavaScript trap 具有返回 `Result::Err` 的行为。

回调中允许 `stop()`，且它是幂等的。Stop/drop 移除原生监听器、待清理 envelope、
RAF/resize 工作和 tooltip 呈现，不再交付回调；修改操作拒绝已停止的 owner。inactive
快照仍可读取，停止后的 `tooltip()` 返回 `Ok(None)`。弱 handle 不会让已丢弃的
owner 继续存活。

## Tooltip 的时间语义与浏览器限制

hook 是历史观察：其 Frame、generation、publication 和 occurrence 属于原生捕获
时刻。tooltip 是当前呈现：最后选择的 typed part ID 和 occurrence 会按新接纳的
source/plan 重新绑定。因此源或呈现更新后，tooltip Frame 及其证书也会刷新；移除或
省略 body 会清除选择。这个显式 rebase 与保留 Nivo 捕获的 part 对象有所不同。仅
tooltip 指针更新保留源 generation 和 SVG publication；光标位置不是新的源/层 epoch。
参见 [tooltip 时间与定位](funnel-tooltip.zh-CN.md)。

WebKit 在悬停 path 被替换且指针保持静止后，可能完全不投递窗口退出事件。独立纯
SVG 替换控制可以复现，此时 owner 没有原始 Out 可观察。若 Out 来自当前已接纳 body，
即使坐标在 viewport 外也能清除匹配 lease。测试会先向替换后的 body 投递真实 Move，
再检查其真实 Out。不会用 timeout 或伪造的全局回退声称恢复未投递事件。
[SVG 输入边界](funnel-svg-layers.zh-CN.md)说明了这一剩余限制；这里不声称操作系统
事件投递等价。
