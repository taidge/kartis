# Funnel 原始应用数据（Unreleased）

[English](funnel-data.md)

类型化 API 将不透明应用数据贯穿完整 Funnel 来源；受检范围见
[验收记录](verification.md#development-funnel-owned-application-data)。
既有 `FunnelDatum`、`FunnelSvgFrame`、布局和未泛型化浏览器入口不变，GPU/worker 继续后置。

## 不透明数据与显式投影

`FunnelDataDatum<D>` 同时拥有 `data: D` 和 `projected: FunnelDatum`。应用负责数值与
布局投影；任意附加字段不会被强制转换为 Scene value，也不由库自动序列化。
D 不需要 Clone、PartialEq、Debug、Serialize、Send 或 Sync。纯模型可持有非 static
借用；保留式浏览器 owner 要求 `D: 'static`。D 内部的可变性仍由应用负责：修改 Cell
本身不会排队提交新的来源。

```rust
use std::{cell::Cell, rc::Rc};
use kartis::*;

pub struct BusinessData {
    pub status: String,
    pub raw_metric: f64,
    pub local: Rc<Cell<u32>>,
}

pub fn build_data() -> Result<FunnelDataFrame<BusinessData>> {
    let source = vec![FunnelDataDatum::new(
        BusinessData {
            status: "priority".into(),
            raw_metric: -0.,
            local: Rc::new(Cell::new(0)),
        },
        FunnelDatum::new("leads", 8.),
    )];
    let mut color = |context: &FunnelDataColorContext<'_, BusinessData>| {
        Ok(if context.data.status == "priority" {
            Color(210, 70, 40, 255)
        } else {
            context.computed.default_color
        })
    };
    FunnelDataFrame::with_factories(
        &ChartConfig::default(),
        source,
        &FunnelLayoutOptions::default(),
        &mut FunnelDataFactories {
            color: Some(&mut color),
            ..Default::default()
        },
    )
}
```

`FunnelDataFrame<D>` 拥有一个原始来源 Arc 和受检查的 `FunnelSvgFrame`。
`source()` 返回数据／投影行，`data(source_index)` 返回可选原数据引用；
`computed()`、`config()`、`options()`、`layout()`、`scene()` 访问已验证投影。
Frame 的 Clone 与 Debug 不克隆或格式化 D。既有纯 `FunnelSvgFrame` 不增加浏览器
Any 载荷，保留其原有 Send/Sync 属性。

完整 Frame 以捕获身份判等。即使所有投影字段和绘制字节相同，新构建仍是新 Frame。
`shares_source()` 仅表示共享同一个原始来源 Arc；配置／注解捕获可以共享原数据，
但完整 Frame 不因此相等。`from_shared` 与 `with_shared_factories` 接受
`Arc<[FunnelDataDatum<D>]>`，可在不要求 D:Clone 的前提下重建。浏览器 builder 必须
保留上下文提供的原 Arc，不能重新制造一个值相等的来源来冒充同一捕获。

`with_computed` 接受真实构建的投影，用于显式、合法的墨迹或 Scene extras 编辑。
它逐字段检查原投影来源，包括浮点位和嵌套 paint 坐标；不会从 Mark 反推 D，也不证明
用户任意绘制函数只依赖 D。

## 函数相位与绑定注解

`FunnelDataFactories` 可配置 `color`、`border_color`、`label_color`、`size`、`stage`。
各角色获得原始 datum、完整 raw source 切片和原有 `computed` 角色上下文。
`formatted_value` 保留只接受 number 的旧格式化合同。函数只为本次构建借用，不存入结果。

尺寸／来源检查先于样式。颜色、格式化和阶段样式保持既有源序与相位；阶段几何回调
只对原本可绘制的 body 调用。早期颜色或尺寸阶段不会虚构尚未产生的最终样式集合。
无墨迹角色的显式样式结果仍保存在 computed cache。未配置角色不会让旧默认流程多跑
一轮用户函数。

注解阶段的 matcher／note 上下文携带完整 typed frame 和原有 computed context。
matcher 可检查整个 computed parts 集合，并按真实 source index 查询 D。
自定义 note 返回真实、透明、全尺寸 Scene，而不只是格式化字符串：

```rust
use kartis::*;

pub struct NoteData {
    pub flagged: bool,
    pub caption: String,
}

pub fn bind_notes(frame: &FunnelDataFrame<NoteData>) -> Result<FunnelDataFrame<NoteData>> {
    let mut spec = FunnelAnnotationSpec::new(
        FunnelAnnotationMatch::Predicate("flagged".into()),
        FunnelAnnotationKind::Dot,
        FunnelAnnotationNote::Custom("caption".into()),
        FunnelAnnotationPosition::Relative(30.),
        FunnelAnnotationPosition::Relative(-20.),
    );
    spec.size = Some(8.);
    let mut matcher = |context: FunnelDataAnnotationMatchContext<'_, NoteData>| {
        Ok(context.data.flagged)
    };
    let mut note = |context: FunnelDataAnnotationNoteContext<'_, NoteData>| {
        let mut scene = context.scene()?;
        scene.labels.push(Label {
            rotation: 0.,
            x: context.computed.x,
            y: context.computed.y,
            text: context.data.caption.clone(),
            color: Color(35, 45, 65, 255),
            size: 13.,
            anchor: TextAnchor::Start,
        });
        Ok(scene)
    };
    frame.with_annotations(
        &[spec],
        &mut FunnelDataAnnotationFactories {
            matcher: Some(&mut matcher),
            note: Some(&mut note),
        },
        &FunnelAnnotationStyle::default(),
    )
}
```

`with_annotations_with_budget` 提供同一流程的显式预算。重新绑定总是使用未附注解的
computed base，不累积旧注解墨迹。原始来源和投影来源保持对应，不重放布局／样式函数。
`with_annotation_bundle` 接受相同原 computed source capture 的已计算 bundle，不再次
执行回调。`annotation_data(binding_index)` 查询绑定的原始 datum。
几何、绘制顺序和静态范围见[注解指南](funnel-annotations.zh-CN.md)。

`widgets(revision, placement)` 为既有 legacy Mark 角色增加 typed payload，使用原始
未附注解的角色投影，同时携带完整 typed frame。closing separator 保持 `data: None`；
注解装饰不成为业务角色。这不新增字面 SVG owner builtin 内逐 item 的 Source Widgets。

## 类型化浏览器 owner

`BrowserFunnelData<D>` 是新增的 SVG owner；使用弱 typed handle 时必须保留 owner。
mount 前应给根元素明确的正 CSS 尺寸。以下例子组合真实保留式整层组件、类型化 Click
hook 和真实 tooltip 组件：

```rust
#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub fn mount_data(
    root: web_sys::Element,
) -> kartis::Result<kartis::browser::BrowserFunnelData<AppData>> {
    use glory::Widget;
    use kartis::browser::*;
    use kartis::*;

    let program = BrowserFunnelDataSvgProgram::<AppData>::new(vec![
        FunnelSvgLayer::new("parts", FunnelSvgLayerKind::Parts)?,
        FunnelSvgLayer::new("labels", FunnelSvgLayerKind::Labels)?,
        FunnelSvgLayer::new("annotations", FunnelSvgLayerKind::Annotations)?,
        FunnelSvgLayer::new("extras", FunnelSvgLayerKind::SceneExtras)?,
        FunnelSvgLayer::new("panel", FunnelSvgLayerKind::Custom("summary".into()))?,
    ])?
    .custom("summary", 0, |context| {
        let frame = context.frame;
        Ok(glory::ui::widget(move |scope| {
            glory::ui::text()
                .content(scope.bond(move || {
                    frame.get().and_then(|f| f.data.data(0).map(|d| d.name.clone()))
                        .unwrap_or_default()
                }))
                .show_in(scope)
        }))
    })?;
    let owner = BrowserFunnelData::mount_with_factory_and_svg_layers(
        root,
        vec![FunnelDataDatum::new(
            AppData { name: "Qualified leads".into() },
            FunnelDatum::new("leads", 8.),
        )],
        ChartConfig::default(),
        FunnelLayoutOptions::default(),
        program,
        |context| FunnelDataFrame::with_shared_factories(
            context.config,
            context.source.clone(),
            context.options,
            &mut FunnelDataFactories::default(),
        ),
    )?;
    let handle = owner.handle();
    handle.set_mouse_handlers(BrowserFunnelDataMouseHandlers {
        on_click: Some(Box::new(|event| {
            let data = event.data.data(event.source_index)
                .ok_or_else(|| ChartError("clicked raw source is missing".into()))?;
            let _name = &data.name;
            Ok(())
        })),
        ..Default::default()
    })?;
    handle.set_tooltip_component(0, |context| {
        let frame = context.frame;
        Ok(glory::ui::widget(move |scope| {
            glory::ui::text()
                .content(scope.bond(move || {
                    frame.get().and_then(|f| f.datum().ok().map(|d| d.name.clone()))
                        .unwrap_or_default()
                }))
                .show_in(scope)
        }))
    })?;
    handle.flush()?;
    Ok(owner)
}

pub struct AppData {
    pub name: String,
}
```

builder 的 `source` 是 Arc 引用；返回 Frame 必须保留这个 raw pool 及其真实已验证
投影。snapshot、layer、原生 hook、tooltip 的类型都包含 `FunnelDataFrame<D>`、
已接受 source generation 和 publication；原生输入另外保留真实 event／occurrence
信息。layer 构造时有显式 tentative `initial`，其已接受 Bond 从 None 开始；tooltip
构造也从 None 开始。layer／tooltip Bonds 由完整私有 source snapshot 派生，不分别
提交 raw 与 projected 数据。跨 Runtime 诊断读取 Bond 应使用 untracked；Glory effect
不跨独立 Runtime 订阅。

`set_source(Vec<FunnelDataDatum<D>>)` 排队来源更新，包括投影完全不变的 raw-only 改动。
新构建仍产生新的完整 cohort，并更新保留 typed views；稳定 revision 保留组件本地状态。
仅 program/view/hidden 的操作不重放来源工厂。setter 排队，`flush()` 处理共同事务；
原生回调应排队操作，不能重入 flush。

捕获的原生事件拥有该真实事件发生时的历史已接受 Frame，不能到新来源按业务 ID 查找
再冒充原数据；dispatch admission 会拒绝过期 source／presentation lease。tooltip
选择则将原业务／occurrence 选择重新绑定到新候选，同时更新 data/generation/publication；
若目标被移除或失去资格则清除。这是两种不同生命周期。标准 tooltip 仍使用 projected
label 与 formatted value；自定义 typed 组件可以显示任意 raw 字段。

## 拒绝边界与未完成项

builder、typed source 证书、predicate、note 或 precommit owned placement 被拒绝时，
不会替换已接受 raw/projection/layer/tooltip cohort。用户函数自身的外部副作用不能回滚。
adopt 后的 managed backend 错误仍属于 postcommit UI error，沿用失败视图隔离策略；
本 API 不创造上游并未提供的 backend-only 事务保证。新 revision／remount 与清理规则
仍按专用 Funnel owner 处理。

逐属性 Spring 由独立[动画 API](funnel-motion.zh-CN.md)提供，上游 Funnel 无退出过渡。
任意可变 D3 generator、逐 item Source Widget 混排、任意实时 UI 导出和更广 OS 输入
等价仍单列。纯 computed Scene 可用于既有 Canvas API；新增
typed 浏览器 owner 只支持 SVG，GPU/worker 后置。

NativeComponent 活动策略保留原有原生事件边界。省略主体不会合成 Leave 或清空受控
current；tooltip 则仍可因失去绘制资格而清除。验收宿主会先显式聚焦真实保留按钮，
再发送可信键盘激活，因为不同浏览器的点击聚焦行为不同；这些设置不合成业务鼠标回调。
见[原生事件指南](funnel-events.zh-CN.md)。
