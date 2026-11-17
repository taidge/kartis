# Waffle 字面 SVG 层与保留式 Glory 子树

本 API 为 Unreleased。原生默认 2095、全特性 2440、18 项 doctest 与 native／WASM 严格检查通过；
完整矩阵通过 10 个浏览器／DPR 上下文、40 场景、2280 次 capture、2160 次操作、280 次候选拒绝与
20 项真实 DOM 负控，sourceUnchanged 为真。独立读回亦通过，解码 3100 张 PNG 并核对完整原始事件与组件证据。
见[验收记录](verification.md#development-waffle-literal-svg-layers)。
[English](waffle-svg-layers.md)

## 顺序、完整来源与真实组件

`BrowserWaffle` 的 SVG program 按字面顺序绘制 Cells、Areas、Legends 和自定义整层。
应用可省略、重排或以不同 occurrence key 重复任一层；重复绘制不会复制原始业务数据或 canonical 分组。
自定义层是真正保留式 Glory 子树，不是始终位于所有 builtin 之上的页面覆盖层。

```rust
use glory::Widget;
use glory::render::EventKind;
use glory::style::{Length, PointerEvents, Style};
use kartis::browser::{BrowserWaffle, BrowserWaffleSvgProgram};
use kartis::*;

fn mount(root: web_sys::Element) -> Result<BrowserWaffle> {
    let program = BrowserWaffleSvgProgram::new(vec![
        WaffleSvgLayer::new("cells", WaffleSvgLayerKind::Cells)?,
        WaffleSvgLayer::new("areas", WaffleSvgLayerKind::Areas)?,
        WaffleSvgLayer::new("summary", WaffleSvgLayerKind::Custom("summary-ui".into()))?,
        WaffleSvgLayer::new("legends", WaffleSvgLayerKind::Legends)?,
    ])?
    .custom("summary-ui", 0, |context| {
        let frame = context.frame;
        Ok(glory::ui::widget(move |scope| {
            let count = scope.cage(0u32);
            glory::ui::button()
                .style(
                    Style::new()
                        .width(Length::Px(160.))
                        .height(Length::Px(32.))
                        .pointer_events(PointerEvents::Auto),
                )
                .label(scope.bond(move || {
                    frame.get().map_or_else(
                        || "Preparing".to_owned(),
                        |f| format!("{} groups; clicks={}", f.data.groups().len(), count.get()),
                    )
                }))
                .on(EventKind::Activate, move |_| count.revise(|n| *n += 1))
                .show_in(scope)
        }))
    })?;
    BrowserWaffle::mount_with_svg_layers(
        root,
        vec![RadialDatum::new("ready", 8.), RadialDatum::new("later", 5.)],
        ChartConfig::default(),
        WaffleLayoutOptions {
            rows: 3,
            columns: 5,
            total: Some(20.),
            allocation: WaffleAllocation::IndependentRound,
            ..Default::default()
        },
        program,
    )
}
```

请保持返回 owner 存活。`handle()` 是弱引用；drop／stop 清理原生监听、排队任务、提示与组件 runtime，
后续修改被拒绝。已拥有的历史事件帧仍保留其捕获来源。

构造 context 提供 key、revision、候选 `initial` 与 `frame: Bond<Option<BrowserWaffleSvgLayerFrame>>`。
初始 Bond 为 None；`initial` 不代表已接受。接受帧包含完整 data、实际 presentation plan、source generation、
publication、occurrence key/index、view、源 hidden ID 和 interactive 策略。
同 key／registry／revision 保留真实 Scope、根节点和本地状态；修改 revision 或删除后重新插入会创建新组件。

整层载体使用完整 Scene 坐标，可在 margin 内绘制 UI；固定 viewport 之外会裁剪，没有额外默认 plot clip。
相机只应用一次。空 host 不截获整块鼠标输入，真实按钮应如示例显式启用 PointerEvents::Auto。

默认层序为 Cells → Areas → Legends，但默认图例配置为空，与原版默认 `legends=[]` 对应。
使用 program 的 `with_legends(Vec<WaffleSvgLegendSpec>)` 配置真实原生 symbol／text／背景盒。
Spec 包含 key、`LegendOptions`、`LegendLayoutOptions` 和作者显式给定的 `item_size: TextSize`；不猜测字宽，
两份配置的 flow／align／wrap 必须一致。这是受支持的 Rust 图例协议，不等于完整 Nivo legend package。

## 三种不同的几何与遮挡

| 角色 | 绘制与业务资格 |
| --- | --- |
| Cells | 实际原 Mark，包括自定义 path／circle、圆角、填充、边框、dash、clip 与透明度；空格也有真实墨迹。它们保留 SVG 原生命中平面，但不授权分组 hook 或提示。 |
| Areas | 从未加 padding 的占用槽位 polygons 生成独立透明 path，黑色 fill 与 fill-opacity=0；只有实际当前 Area 原生目标才授权分组事件。无分组的空格没有伪造零值 datum。 |
| Legends | 独立 item 背景盒保留真实遮挡，symbol／text 不接分组事件。它不是 Area 或业务 cell。 |

透明并不意味着穿透：SVG visiblePainted 下，fill-opacity=0 的 Area、透明或零整体 opacity 的 cell 仍可能是原生目标。
关闭 owner interactive 仅禁止业务 hook／提示，不删除 Cells／Areas／legend 背景的原生命中平面。
上层 handlerless Cell 或 Area 仍可挡住下层按钮；上层 Custom 按钮也能阻止下方 Area。
省略 Areas 后不能用 `cell_at` 数学回退制造分组事件；省略 Cells 但保留 Areas 时，透明分组平面仍实际存在。

实际 cell 墨迹与完整槽位区域分别保留。自定义 cell 可偏移、无填充或为非矩形，其 group polygons 不跟着变形。
纯 `cell_at`／`area_at` 和 `waffle_area_interaction` 的半开槽位协议不等于 UA 在精确共享边界上的原生命中规则。
事件不从颜色、近似 bbox 或任意控件坐标猜测业务身份。

旧 `WaffleMode::Areas` 是有色分组 union，并非这里透明的 Areas 层。新 literal owner 只接纳 Cells mode；
纯 capture 与旧通用 Scene／Canvas／raster API 继续支持旧模式。
新 SVG IndependentRound profile 可让显式 total 小于来源值之和：请求区间保留，实际占用裁剪到容量；
legacy Vec／layout 入口的原总量准入不变。新 SVG profile 合并仅角接触的占用轮廓，旧 profile 保持边连接分组。
这一区别影响 polygon 列表与事件索引，不能将新结果冒作所有旧入口的无差别替换。

## 无需克隆原始应用类型

```rust
use std::cell::Cell;
use std::rc::Rc;

use kartis::browser::{
    BrowserWaffleData, BrowserWaffleDataMouseHandlers, BrowserWaffleDataSvgProgram,
};
use kartis::*;

struct Order {
    code: String,
    edits: Rc<Cell<u32>>,
}

fn mount_orders(root: web_sys::Element) -> Result<BrowserWaffleData<Order>> {
    let source = vec![WaffleDatum::new(
        Order {
            code: "PO-7".into(),
            edits: Rc::new(Cell::new(0)),
        },
        RadialDatum::new("ready", 8.),
    )];
    let chart = BrowserWaffleData::mount_with_factory_and_svg_layers(
        root,
        source,
        ChartConfig::default(),
        WaffleLayoutOptions::default(),
        BrowserWaffleDataSvgProgram::defaults(),
        |c| {
            let mut color = |f: &WaffleDataColorContext<'_, Order>| {
                Ok(
                    if f.data.code.starts_with("PO-") && f.data.edits.get() == 0 {
                        Color(30, 120, 180, 255)
                    } else {
                        f.computed.default_color
                    },
                )
            };
            WaffleDataFrame::with_shared_factories(
                c.config,
                c.source.clone(),
                c.options,
                &mut WaffleDataFactories {
                    color: Some(&mut color),
                    ..Default::default()
                },
            )
        },
    )?;
    let queued = chart.handle();
    chart.set_mouse_handlers(BrowserWaffleDataMouseHandlers {
        on_click: Some(Box::new(move |event| {
            let _order = event
                .data
                .data(event.source_index)
                .ok_or_else(|| ChartError("missing original order".into()))?;
            let _original_event = event.input.event.clone();
            queued.refresh()
        })),
        ..Default::default()
    })?;
    Ok(chart)
}
```

builder context 借用 config/options 与捕获的 `Arc<[WaffleDatum<D>]>`。
使用 `from_shared`、`with_shared_factories` 或组合 cell-factory 的 shared 构造器，无需克隆 D。
owner 验证返回帧仍持有同一原始池，并对应本次投影、配置和 options。
旧 Vec 构造器保留旧行为与来源存储；不能拿另一个池冒充本次 typed 请求。

浏览器只要求 D: 'static，不额外要求 Clone／Eq／Debug／Serialize／Send／Sync。
仅原始应用字段变化也会形成新的 capture／generation。typed 层、tooltip 与原生 admission 来自同一完整内部快照，
没有提交后的 raw sidecar，也不在回调交付时读取 latestVec。框架不深冻结 D 的内部可变对象，
也不推断应用选定的数值投影是否符合业务含义。

源数据、config/options、refresh、实际 resize 与 source hidden 重排执行 builder；相机、program 与提示位置更新不执行。
`set_hidden` 修改源 options.hidden 并重排剩余组，不是只把旧墨迹 opacity 设零。

ResizeObserver 读取 root 的正值 content 尺寸，不把外部 CSS 缩放后的 BCR 当布局尺寸；
只有与已接受配置不同时才排队重建。真实 resize 控制验证保留组件身份与焦点。
stop 会断开 observer、取消排队任务并移除监听；弱 handle 修改与原 Area 位置点击不能复活 owner。

## 四种原生观察与共享提示

projected／typed handler bundle 分别提供 Enter、Move、Leave、Click。
事件含完整历史 Frame、source/polygon index、generation/publication、occurrence key/index、真正原 MouseEvent、
浮点 client／viewport／Scene 坐标和 pointer 事实。Window capture 保存当时接受的 source/stage，
Window bubble 同步交付匹配注册；中途采用新来源会使旧 admission 失效，而非给旧事件换上最新 raw。

replacement 引发的同角色 Over 不重复逻辑 Enter；不会由 Move 伪造 Enter。
DOM dispatch 保留真实 isTrusted 值，本文没有新增“可信事件特权”承诺。
Enter／Move 先排队 show tip，Leave／Click 先排队 hide，再调用对应 hook；hover 不触发来源构建或 current 扩张。
标准提示读取 computed label、formatted value 与颜色 chip；可选真实 Glory tooltip 复用同一完整来源和共享预算。
source／camera 改变时，仍有效的 tooltip 按当前接受呈现重绑，历史事件不改写。
Area occurrence 删除、分组隐藏或交互停用会撤销选择，恢复 program 不凭空复活旧提示。

handler 可以排队 setter，但同步 flush 返回错误。替换／清理注册、递归 DOM dispatch、独立有界 event_errors 与 stop
遵守专属 owner 的生命周期；不以 Up 伪造 Click，不以虚构事件补偿 UA 未投递的离窗事件。

## 事务、预算和保留边界

`WaffleSvgBudget` 对每个重复 occurrence 的实际路径、资源及图例工作量计数。
整层与自定义提示共享组件上限，live／pending／retiring 一起计入，在构造器之前拒绝超预算。
无效批次丢弃候选 desired 状态；原来源／program／view 与接受的本地状态继续有效。
owned native placement 记录 parent／attribute／focus journal，全部完整 Cage 与 owner 元数据采用后才 pump runtime。
用户闭包的外部副作用不在回滚范围内。

采用后的 Widget/backend 错误是 UI failure，不谎称数据回滚：整个 carrier display:none／inert 永久隔离，
恢复需要提升组件 revision 重建。无工作或 batch 延后的 Ok pump 不能证明错误 UI 已恢复。

早期 Chromium 回滚控制曾在已接受数据与 DOM 均恢复后留下 12 个边缘差异像素。
独立纯 DOM 对照在圆角 HTML tooltip 显示再隐藏后复现相同坐标／颜色；移动并恢复 native path 后回到冷基线。
原生 SVG content 现增加 `will-change:transform`，稳定其 paint 边界；同配置独立冷基线与完整操作历史截图一致，
tooltip 与 placement 往返保持整 viewport 字节零差。这只是有限浏览器证据，不归因为特定 GPU／驱动。
实际 path 几何与原生命中资格不改，候选拒绝的整图零差门不降低。

Firefox 原生 pattern 在作者指定 motif 中心也可能混色。固定 pattern 场景使用仅由作者参数构造的独立原生 SVG
参考，保持同浏览器／DPR／carrier 配置；原两采样仍按通道差不超过 1 核对，整个作者 tile 则严格 RGBA 零差。
错误前景色、错误间距、缺失 motif 三项负控必须使原 tile 门失败，不读取产品 RGB 选择参考。
这不宣称跨引擎所有 pattern／primitive 的抗锯齿逐像素等价。

完整浏览器矩阵与受检范围内的独立读回均通过；实际方法与诊断边界见[验收记录](verification.md#development-waffle-literal-svg-layers)。
Canvas 专用 owner、Waffle Spring／exit、builtin 内逐 item Widget 混排、
组区域 Widget 工厂、HTML 层等价和任意实时 UI 导出继续单列。参见
[分组函数与数据](waffle-functions.zh-CN.md)和[已有分组区域](waffle-regions.zh-CN.md)。
