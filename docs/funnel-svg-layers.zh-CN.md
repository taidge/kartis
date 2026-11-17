# Funnel SVG 函数层（Kartis 0.2.0）

[English](funnel-svg-layers.md)

此 API 自 Kartis 0.2.0 提供。浏览器输入、绘制与生命周期检查，与原生领域测试及 WebAssembly
编译分别验收，具体范围见[验收记录](verification.md#development-funnel-literal-svg-layers)。
本文说明当前 API 与边界，不承诺完整兼容 Nivo Funnel。

`FunnelSvgFrame` 和 `FunnelSvgPlan` 可用于原生 Rust；`BrowserFunnel` 需要 WebAssembly
和 `web` feature，是独立的 SVG owner。现有 Funnel 布局函数、通用 `BrowserChart` 和
Canvas 渲染入口保持原有 API。

## 权威来源 Frame

通过真正的 Funnel 构建入口创建 Frame：

```rust
use kartis::*;

fn source_frame() -> Result<FunnelSvgFrame> {
    let config = ChartConfig {
        width: 480.,
        height: 320.,
        title: "转化漏斗".into(),
        ..Default::default()
    };
    let source = [
        FunnelDatum::new("visits", 100.),
        FunnelDatum::new("trials", 40.),
        FunnelDatum::new("sales", 10.),
    ];
    let options = FunnelLayoutOptions {
        current: Some("trials".into()),
        current_size_extension: 8.,
        ..Default::default()
    };
    let mut border = |context: &FunnelPartStyleContext<'_>| {
        Ok(if context.part.is_current {
            Color(220, 80, 20, 255)
        } else {
            context.default_color
        })
    };
    FunnelSvgFrame::with_factories(
        &config,
        &source,
        &options,
        &mut FunnelFactories {
            border_color: Some(&mut border),
            ..Default::default()
        },
    )
}
```

Frame 持有原始配置、带类型的来源 ID、原始数值、options 与完整布局。
`source()`、`parts()`、`separators()`、`styles()`、`scene()` 和 `layout()` 读取同一批数据。
字符串类别 `"1"` 与数值类别 `1` 不混同，原始数值位模式保留正负零。克隆共享存储；
Frame 相等性采用捕获身份，避免普通浮点相等性掩盖新的原始数据批次。

`styles()` 缓存构造期颜色、填充透明度、有效边框宽度、边框颜色/透明度和可选标签颜色。
即使对应 Mark 或 Label 没有墨迹，显式样式函数的真实结果仍保留。读取 Frame、重复层或
改变浏览器视图不会重放函数。旧禁用角色规则保持：禁用标签且没有标签颜色函数时，
`label_color` 是 `None`；未使用边框且没有显式样式函数时，可保留原有 part 颜色回退，
不会强行解析一个原本无需求值、可能报错的继承对比色。这不是始终计算所有 Nivo 默认
样式的新增 profile。

还可使用 `with_size`、`with_size_and_factories`、`with_stage_factory`、
`with_factories_and_stage_factory` 和 `with_size_factories_and_stage_factory`。
这些借用函数只在本次构建中执行，不保存在 Frame 中；应用回调的外部副作用无法回滚。

`from_layout(layout)` 读取真实构建入口保存的私有来源凭证，不根据几何猜测 options。
来源和 part 元数据必须仍匹配构造结果或真实 stage factory 的结果。合法 Scene 装饰及
墨迹编辑与原始来源分开；没有 `label_series` owner 的 Label 是装饰，不伪造 Funnel part。

`funnel_svg_validate_inputs` 只检查静态输入，不执行尺寸/样式函数、继承对比色或最终几何
验证。应用颜色函数可能让默认情况下无法解析的颜色变得合法；预先运行默认布局不能
替代该静态检查。

## 字面绘制顺序

每个 `FunnelSvgLayer` 包含唯一 occurrence key 和一种角色：`Separators`、`Parts`、
`Labels`、`SceneExtras` 或 `Custom(registry_key)`。向量顺序就是真实绘制顺序；默认是
分隔线、parts、标签、Scene 装饰。Parts 包含实际来源顺序中的原始边框和主体，标签及
分隔线保持独立角色。

```rust
use kartis::*;

fn repeated_plan(frame: &FunnelSvgFrame) -> Result<FunnelSvgPlan> {
    FunnelSvgPlan::new(
        frame,
        vec![
            FunnelSvgLayer::new("separators", FunnelSvgLayerKind::Separators)?,
            FunnelSvgLayer::new("parts-under", FunnelSvgLayerKind::Parts)?,
            FunnelSvgLayer::new("application", FunnelSvgLayerKind::Custom("panel".into()))?,
            FunnelSvgLayer::new("parts-over", FunnelSvgLayerKind::Parts)?,
            FunnelSvgLayer::new("labels", FunnelSvgLayerKind::Labels)?,
            FunnelSvgLayer::new("extras", FunnelSvgLayerKind::SceneExtras)?,
        ],
        FunnelSvgBudget::default(),
        FunnelSvgExtrasPolicy::RequireLayer,
    )
}
```

纯 Plan 只描述 custom registry，不创建 Widget；浏览器 Program 必须在挂载前注册它。
省略 builtin 会省略实际墨迹，重复则绘制同一来源的另一个视图，不复制业务数据、逻辑
控件、来源 Mark ID，也不重复执行构建函数。每个 occurrence 有独立 DOM/资源命名空间。

`mark_indices()` 返回原 Scene 索引，允许重复；`body_mark_indices()` 是字面计划中实际
主体 Mark 索引的并集，只是来源投影描述，不是 alpha、clip、遮挡或像素覆盖率判据。
不存在的零宽主体不会被伪造，边框和分隔线不会变成主体业务目标。

`SceneExtras` 是真实无来源 owner 的 Scene 项目，不等同于 Nivo 的绑定 `annotations`。
默认 `RequireLayer` 要求非空 extras 有显式 occurrence；通过
`FunnelSvgExtrasPolicy::Omit` 可以明确省略。复合透明度组必须在每个 occurrence 内保持
完整、连续和成员顺序，跨角色拆分会拒绝。层数、工作量、来源命令和组件预算都有边界；
重复绘制消耗重复工作与资源。

## 真正保留的 Glory 子树

应用需要启用 `web` 的 `kartis`、Glory，以及提供 `Element` 的 `web-sys`。挂载前给根
元素明确的非零 CSS 宽高。owner 读取根尺寸并观察后续变化。必须保留返回的 owner，
handle 和组件 current 能力都是弱引用。

```rust
#[cfg(all(feature = "web", target_arch = "wasm32"))]
fn mount(root: web_sys::Element) -> kartis::Result<kartis::browser::BrowserFunnel> {
    use glory::Widget;
    use glory::render::EventKind;
    use glory::style::{Length, PointerEvents, Style};
    use kartis::browser::{
        BrowserFunnel, BrowserFunnelSvgLayerContext, BrowserFunnelSvgProgram,
    };
    use kartis::*;

    let program = BrowserFunnelSvgProgram::new(vec![
        FunnelSvgLayer::new("separators", FunnelSvgLayerKind::Separators)?,
        FunnelSvgLayer::new("parts", FunnelSvgLayerKind::Parts)?,
        FunnelSvgLayer::new("summary", FunnelSvgLayerKind::Custom("summary-ui".into()))?,
        FunnelSvgLayer::new("labels", FunnelSvgLayerKind::Labels)?,
        FunnelSvgLayer::new("extras", FunnelSvgLayerKind::SceneExtras)?,
    ])?
    .custom("summary-ui", 0, |context: BrowserFunnelSvgLayerContext| {
        let frame = context.frame;
        let current = context.current;
        Ok(glory::ui::widget(move |scope| {
            let current = current.clone();
            let count = scope.cage(0u32);
            let error = scope.cage(String::new());
            let sales = CategoryKey::from("sales");
            glory::ui::button()
                .style(
                    Style::new()
                        .width(Length::Px(180.))
                        .height(Length::Px(32.))
                        .pointer_events(PointerEvents::Auto),
                )
                .label(scope.bond(move || {
                    frame.get().map_or_else(
                        || "准备中".to_owned(),
                        |accepted| format!(
                            "{} parts; clicks={}; {}",
                            accepted.data.parts().len(), count.get(), error.get(),
                        ),
                    )
                }))
                .on(EventKind::Activate, move |_| {
                    match current.set_current_part(Some(&sales)) {
                        Ok(()) => count.revise(|value| *value += 1),
                        Err(reason) => error.revise(|value| *value = reason.to_string()),
                    }
                })
                .show_in(scope)
        }))
    })?;

    BrowserFunnel::mount_with_factory_and_svg_layers(
        root,
        vec![FunnelDatum::new("visits", 100.), FunnelDatum::new("sales", 20.)],
        ChartConfig::default(),
        FunnelLayoutOptions::default(),
        program,
        |context| FunnelSvgFrame::new(context.config, context.source, context.options),
    )
}
```

整个层是 SVG `foreignObject` 内 XHTML 宿主中的真实 Glory 子树，可包含控件和 Glory
`Graphic`，不是 React 组件、原始 SVG 节点工厂、字符串回调，也不是字面顺序之外的独立
顶层 overlay。宿主坐标从 plot 局部原点开始；固定 plot clip 和 camera 只应用一次，
不要再次叠加 margin。原生 Funnel profile 仍使用完整 Scene 坐标。轴向 CSS 缩放与逻辑
camera、DPR 分开。

carrier 默认不接收指针，交互子元素需显式开启 pointer events，如上面的按钮。位于主体
上方的子元素可以阻止主体命中，位于下方则可能被后续原生墨迹覆盖。
`.interactive(false)` 关闭 owner 自动原生主体 hover 路由，不禁止应用自定义控件或显式
current 命令。

## 数据及组件生命周期

`BrowserFunnelSvgLayerContext` 提供 key、revision、候选 `initial`、
`Bond<Option<BrowserFunnelSvgLayerFrame>>` 和 current。构造期 Bond 是 `None`，不能把
`initial` 当作已接受快照。接受后的 Frame 包括完整数据、source generation、publication、
occurrence key/index、view、hidden series 和 interactive 状态。
`svg_layer_frame(key)` 可读取 builtin 和 custom 的已接受 Frame，不存在的 key 返回 None。

每个 occurrence 有自己的 Glory runtime，用一个完整 Cage 承载 Frame。所有候选层数据
与 owner 状态先共同接纳，再 pump 各 runtime。这不等于让不同 WebApp 的 tracked Bond
共享 runtime；普通组件跟踪自己的 Frame，跨 runtime 的刻意检查应使用显式 untracked
读取。

相同 occurrence key、registry 和 revision 会保留真实子 Scope 与本地状态，支持来源更新、
重排、camera 和 hidden 变化。新 revision 重建该子树；同一保留身份下换一个函数不会
自动要求重建。移除、替换、stop 或 owner drop 会退休 Scope，旧 current 能力随后报错。

`current.set_current_part(Some(&id))` 与 None 提交真实 owner 队列，不同步改 Bond，也不在
组件回调中 flush。下一 RAF 处理队列，或由回调之外的应用代码调用 `owner.flush()`。
未知 ID、构造/pending 阶段、已停止或已退休 Scope、重入写入均返回错误。只读快照能读取
已接受批次，构建/提交 busy 期间不允许 setter 重入。

来源、配置、options、current、resize 变化和 `refresh()` 会调用来源 builder。仅 Program、
camera（`set_view`）和 `set_hidden` 更新复用已接受 Frame，不重放布局、尺寸、样式或已
保留的 custom factory。source generation 随成功来源重建推进，publication 随成功显示
更新推进。`BrowserFunnelSvgHit` 保存当时的 Frame、GEN/pub 和真实 occurrence，后续接纳
不会给历史命中重新贴新批次。

hidden 集合使用真实 `FunnelPart.series`，不是显示标签或丢失类型的 datum ID；应从已接受
Frame 读取。camera 通过 `ZoomTransform` 设置，本 API 不添加通用 Chart 的拖动/滚轮手势。

## 输入与失败边界

自动 current 变化来自实际已接受、已连接且属于 owner 的原生 SVG 主体 path。命中使用
SVG `visiblePainted`，不是只支持纯色的 picking profile，也不是非零 alpha 近似；透明
SVG 墨迹不能一概当作 `pointer-events:none`。gradient/pattern 保留。省略 Parts、主体
不存在、series 隐藏、边框、分隔线、标签和任意 custom HTML 不伪造主体命中；组件显式
current 命令是独立的应用行为。

可能失败的来源函数、factory 和自有 carrier 放置先于接纳。拒绝保留已接受来源/显示，并
消耗该失败批次；后续有效 current 命令从已接受输入出发，不自动重试失败的 desired 配置。
`snapshot().error` 报告被拒工作；用户函数的外部副作用不能撤销。

UI pump 在接纳之后。postcommit Glory UI 错误进入 `ui_errors`，保留已接纳元数据并隐藏
失败的 custom 视图，不伪称 precommit 回滚。失败实例持续保持 `display:none` 与 inert，
后续来源更新成功或 Glory 批处理结束也不会自动恢复。提高该 custom 的 revision，
以新实例恢复显示；重建会重置组件本地状态。
自有 carrier 的 journal 不代表 Glory 已提供“在接纳前事务式 flush 全部托管 backend
操作”的上游 API。

WebKit 中，鼠标停留时替换已悬停的 SVG 路径，随后移出窗口可能完全不投递离窗事件。
独立普通 SVG 替换控制复现了此情况。owner 按实际收到的输入清除 current，不能推断没有
任何事件通知的离开。离窗验收先向已接纳的新主体投递真实 Move，再检查真实 trusted Out。
停留替换后无后续投递输入的情况仍是输入限制，不承诺操作系统事件等价。

## 尚未完成的范围

四个公开原生 part 鼠标 hook、标准与保留式 Glory tooltip 已实现并通过独立浏览器验收。
受检配置与边界见[事件指南](funnel-events.zh-CN.md)、
[tooltip 指南](funnel-tooltip.zh-CN.md)和
[验收记录](verification.md#development-funnel-native-events-and-tooltips)。

本 API 尚未实现 Funnel builtin 内与 Source Widgets 的交错绘制、part Element 绑定、
绑定 annotations、自动 Funnel Spring/path 插值与退出过渡、泛型应用 raw datum，或
任意可变 D3 area/line generator 对象。现有 stage 几何函数与 Funnel Widget adapter
仍是独立入口。

`original_body_geometry` 和 `original_border_geometry` 使用捕获的插值方式适配原始四点
profile；实际自定义或编辑墨迹应读取 `body_mark`、`border_mark`。不能把自绘主体冒充
原 profile。来源 `scene()` 不是重复 occurrence 和任意 Glory 子树的扁平导出；此功能
不承诺自定义 UI 的可移植导出。
