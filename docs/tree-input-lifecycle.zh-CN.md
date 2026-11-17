# Tree 指针输入生命周期

[English](tree-input-lifecycle.md)

此 API 自 Kartis 0.2.0 提供。完整应用和四个活动修改器见
[Tree 活动指南](tree-activity.zh-CN.md)。

`BrowserTree` 在处理图表输入前登记主鼠标/笔按压。当移动和按下发生在同一动画帧之前时，
最新的自动活动请求会等待原生 click 完成；旧来源、指针目标和已接受的类型化子控件保持一致。
Chart Down/Move/Up 和普通 Glory UI 事件继续处理，不因按钮仍按住而反复调用布局工厂。

共享图表也推迟会替换点击目标的自动系列高亮和 crosshair 绘制。真实来源拖动或框选超过
移动阈值后可以继续重绘；这不暂停整个 UI 或手势模型。Widget 按压期间保持原生父节点，
直到 click 结束。没有生成 click 的 Up 由输入任务之后的清理任务释放，代际标记防止旧任务
结束后继按压。

因此，快速点击可以先在原活动帧上完成，再接受最后一个延后的 current-node 更新。
最近点活动仍独立于严格墨迹命中；程序化 Chart dispatch 不伪装成原生按压或最近点移动。

## 显式请求与错误

来源、配置、Tree 选项和 Widget revision 更新仍是显式候选请求。只有请求真正接受后，
旧按压和延后的悬停才取消；旧 Widget click 不能激活新数据。后继主按压开启新序列，
键盘激活保持独立。

静态验证、builder、子控件构造及提交前绘制失败保留已接受帧。仍持有原生 capture 的来源
contact 可以在候选拒绝后完成原点击；已经释放或取消的 contact 不会被回滚复活。
期望请求仍被保留，显式 refresh 或后续活动请求可以重试，但空闲时不循环重试。

这一边界保留已接受的数据与点击所有权，普通 UI 展示仍可更新。例如，回滚可能重新显示
旧来源的 hover tooltip，或按当前共享 Series 高亮重绘旧来源。后者通过独立挂载的旧 Scene、
相同公开高亮策略及逻辑悬停核对完整图像。因此，完整 DOM 或 backing 图像字节可能与先前
按住时的展示不同。

类型化数据提交后，普通动态 UI 绑定的错误在已接受帧上报告，不撤销用户 effect 已经观察
到的数据。叠加宿主和按来源顺序的宿主采用相同边界，见[组合指南](widget-composition.zh-CN.md)
和[按 Scene 顺序组合](ordered-widgets.zh-CN.md)。

## 生命周期与策略

匹配的 cancel、capture 丢失和 Window blur 退休活动 contact。为阻止被取消 contact 的迟到
click，可以保留一个不活跃的指针 ID/代际标记；它不持有 DOM 引用，也不轮询动画帧。
匹配 Up、后继真实主按压或 stop 会移除标记。stop/drop 清理监听、释放任务、待处理意图及图表资源。

`BrowserTreeOptions::default()` 使用 `BrowserHighlight::Controlled`，调用者显式指定的
`Series` 会被遵守。`BrowserTree::highlight()` 及 handle 的对应方法读取实际共享图表策略。

输入范围包含该 owner 的实际 Surface 和旁边的叠加 Widget 层，两者之间移动不算离开。
嵌套图表和另一个挂载图表各自拥有输入，不能仅凭相同 class 名认领其它图表的控件。

Engine 帧内同步投递的焦点保留最新原生指针状态，包含移动保留子控件时恢复的焦点。
该投递边界在事件入队时记录。帧外 Widget 焦点保留共享图表原有的悬停处理，程序焦点仍
独立于 Tree current-node 输入。

替换 revision 时，旧聚焦子控件可能在同步移除过程中触发 blur。即使该回调短暂观察到它仍
连接 DOM，该退休 scope 已不属于接受行；其旧 revision 和已知身份可与新行区分。
新接受行仍必须与 owner 一致，不保证每个同步 DOM 观察点都只有最终数量的按钮。

相同 revision 的数据采用过程中，同步 focus 事件也可能先看到尚未刷新的旧 DOM 文本，
而真实 typed Frame Bond 已含新数据。稳定后的 DOM 与实时 Bond 分别核验，不将每个原生
事件当作 DOM 文本事务边界。

Touch 不驱动 Tree current-node。DOM 注入的 cancel/blur 只是协议证据，不冒充 OS 触控或
窗口焦点投递验收。有限输入保证不等同于任意浏览器/OS 调度、严格整图等价、自动 Spring/退出
或实时 Widget 导出。GPU/worker 继续后置。
