# Funnel 绑定注解（Kartis 0.2.0）

[English](funnel-annotations.md)

此增量 API 将注解规格绑定到完整的已计算 Funnel parts。验收范围及边界见
[验收记录](verification.md#development-funnel-bound-annotations)。
既有[字面 SVG 层](funnel-svg-layers.zh-CN.md)、
[鼠标 hook](funnel-events.zh-CN.md)和 [tooltip](funnel-tooltip.zh-CN.md)保留各自
已说明的验收范围。

## 通过权威 Frame 绑定

`FunnelSvgFrame::with_annotations` 返回新的不可变 Frame，保留原始源、part 样式及
构造来源，并增加独立注解 bundle。`annotations()` 返回此 bundle；
`annotation_source()` 返回基础 Frame。这不是手工添加 SceneExtras 的别名。

```rust
use kartis::{
    CategoryKey, FunnelAnnotationFactories, FunnelAnnotationKind, FunnelAnnotationMatch,
    FunnelAnnotationNote, FunnelAnnotationPosition, FunnelAnnotationSpec, FunnelAnnotationStyle,
    FunnelSvgFrame,
};

pub fn annotate_part(frame: &FunnelSvgFrame, id: CategoryKey) -> kartis::Result<FunnelSvgFrame> {
    let mut spec = FunnelAnnotationSpec::new(
        FunnelAnnotationMatch::Id(id),
        FunnelAnnotationKind::Circle,
        FunnelAnnotationNote::Text("Review this stage".into()),
        FunnelAnnotationPosition::Relative(60.),
        FunnelAnnotationPosition::Relative(-30.),
    );
    spec.offset = 6.;
    frame.with_annotations(
        &[spec],
        &mut FunnelAnnotationFactories::default(),
        &FunnelAnnotationStyle::from_chart(&frame.config().theme),
    )
}
```

匹配方式为 `All`、类型化 `Id(CategoryKey)` 或 `Predicate(String)`。predicate key
分派给传入的借用 matcher；字符串与数值 ID 保持不同身份。绑定按规格顺序，再按计算
源顺序排列。没有匹配就不产生 binding。只读 binding 保留 spec/part/source 索引、
类型化 ID、形状、note、中心、有效尺寸及 connector/text 计算结果。bundle 还提供原始
source、规格、theme 和透明的独立注解 Scene。

绑定使用 `part.center` 与原始 `part.bounds`，无 body 墨迹的 part 也可被绑定。不会从
编辑后的路径或 current 尺寸扩展反推尺寸。派生 size 为 `max(width, height)`。
circle/rect 的 offset 给全部派生尺寸增加两倍 offset；dot 忽略 offset。非零显式
size 覆盖派生值；显式零按上游 JavaScript `size || derived` 行为回退。circle/dot 的
size 是直径。派生位置及 rect 尺寸覆盖可选的规格 x/y、width/height，保留原 binder
的属性覆盖顺序。非法最终尺寸会拒绝，不会静默钳制。

note X/Y 各自使用 `Relative` 或 `Absolute`。相对值从 part 中心偏移；绝对值使用
plot 局部坐标，Frame 仅加一次图表 margin。binding 输出为完整 Scene 坐标。
note width 默认 120，text offset 为 8，rect radius 为 6。

connector 严格包含三点：形状边界、note 折点、水平 note 线终点。circle 从指向 note
方向的半径边界起始；dot 从中心起始。rect 使用上游八方向的边/角规则，不是向矩形求
最近点。文字位于 note 线上方，并按上游角度规则向左或向右展开。

## 借用 predicate 与自定义 note 绘制

所有 predicate 绑定完成后才执行 custom-note 回调。回调只在本次构建中借用，不保留
或因渲染重放。predicate 收到完整 Frame、全部 parts 及当前 spec/part/source 上下文。
custom note 收到绑定 part、原始 datum 和已计算的完整 Scene 文字位置。

```rust
use kartis::{
    Color, FunnelAnnotationFactories, FunnelAnnotationKind, FunnelAnnotationMatch,
    FunnelAnnotationMatchContext, FunnelAnnotationNote, FunnelAnnotationNoteContext,
    FunnelAnnotationPosition, FunnelAnnotationSpec, FunnelAnnotationStyle, FunnelSvgFrame,
    Geometry, Mark,
};

pub fn custom_notes(frame: &FunnelSvgFrame) -> kartis::Result<FunnelSvgFrame> {
    let spec = FunnelAnnotationSpec::new(
        FunnelAnnotationMatch::Predicate("first-two".into()),
        FunnelAnnotationKind::Dot,
        FunnelAnnotationNote::Custom("marker".into()),
        FunnelAnnotationPosition::Relative(50.),
        FunnelAnnotationPosition::Relative(-20.),
    );
    let mut matcher = |context: FunnelAnnotationMatchContext<'_>| {
        Ok(context.key == "first-two" && context.source_index < 2)
    };
    let mut note = |context: FunnelAnnotationNoteContext<'_>| {
        let mut scene = context.scene()?;
        let mut mark = Mark::new(
            "note-marker",
            Geometry::Rect {
                x: context.x,
                y: context.y - 10.,
                width: 14.,
                height: 12.,
            },
            Color(20, 180, 210, 255),
        );
        mark.label.clear();
        scene.marks.push(mark);
        Ok(scene)
    };
    frame.with_annotations(
        &[spec],
        &mut FunnelAnnotationFactories {
            matcher: Some(&mut matcher),
            note: Some(&mut note),
        },
        &FunnelAnnotationStyle::from_chart(&frame.config().theme),
    )
}
```

custom Scene 必须透明、与源 viewport 一致，且不能引入业务 series、label series 或
legend target。实际混合绘序、clip、font、文字描边及 opacity group 在私有 ID 下保留。
回调返回后不会再隐式平移。custom note 是 Scene 绘制，不是保留的 React/Glory 逐注解
组件。字面 Custom 层仍支持真实保留式 Glory 整层组件，可读取完整 annotated Frame。

`FunnelAnnotationStyle`（也称 `FunnelAnnotationTheme`）分别控制 outline、symbol、
link 和 text。绘制遵循 SVG 注解顺序：可选背景 connector、形状 halo、形状、前景
connector、note。文字 outline 是前景文字前实际绘制的描边文字。半透明文字保留真实
两次绘制，不简化为不透明近似。这是 SVG 组合经共享 Scene renderer 投影，不承诺原
Nivo 独立 Canvas helper 具有同一绘制顺序。
circle 与圆角 rect 可以降为有限 Scene 路径，不承诺与 Nivo 原生 SVG primitive 具有
完全相同的边缘栅格化结果。

浏览器 fixture 记录 matcher 的完整基础 Frame DTO 及 spec/source 执行顺序。
custom-note 日志读取实际 key、spec/source 索引、原始值位模式及计算 x/y，不声称在
浏览器逐字段读回全部 NoteContext。共享原生模型测试另行覆盖完整借用上下文。

细 dot 像素检查在全部作者 pose 与浏览器中使用同一固定参考方法。便携
[参考 helper](../scripts/acceptance-funnel-dot-reference.mjs) 通过冷态数值
Spring/JSX adapter 执行锁定的原始 Nivo `DotAnnotationOutline.tsx`，并在独立浏览器
进程绘制其真实原生 SVG circle。原源码、许可证、commit 和 hash 保留在
[source index](../tests/fixtures/funnel-annotations/original/source-index.json)。
这里执行原始绘制函数，不测量 React 或 Spring 的时间积分。

参考输入只来自作者模型：dot 中心、直径 12 与直径 6 的绘序、底层 body 颜色或
隐藏/省略后的白色、字面 occurrence 数量、相机、外部 CSS 缩放和 DPR。每个 pose 按
这些参数及源码 hash 缓存，并保存 HTML、DOM、完整 PNG 和进程收据。产品路径与
像素不参与选择参考或采样。原有几何环带规则选出的两个完整物理像素 cell 保持不变，
每通道最大差值仍为 1；tooltip 的不透明遮挡由独立 overlay 模型另行合成。

纯原生控制在 Firefox DPR1 的同一 cell 中复现了 native circle 与独立数学 cubic
路径的抗锯齿混色，说明原来的纯粉色期望不成立。固定 circle 参考保留这种实际原生
混合，不更换采样点、形状或容差。这一有界比较不承诺全部 SVG primitive、路径降级、
字体或任意 pose 的抗锯齿完全相同。各引擎使用同一参考算法，没有按通过颜色选择的
fallback。

Chromium 的纯 SVG/foreignObject 控制发现：局部修改 glyph 描边再恢复，即使 DOM 和
几何完全相同，仍可有一个通道的 1 级差异。属性与 style 两种修改在固定两轮中均复现。
因此 glyph 绘制贡献实验在恢复原属性后，将同一个完整 SVG stage 隐藏两个 RAF 回调，
恢复其原 style，再等待两个 RAF 回调，最后仍要求整 viewport 精确一致。控制在任何
peer 获得焦点前执行，并验证 node 身份、owner 和 peer Cages 均不变。这是实验显式
清理步骤，不是产品回滚，也不放宽拒绝测试：source/函数/预算拒绝仍要求 viewport
完全不变。原始控制保存在
`artifacts/verification/funnel-annotations-glyph-native-control-first` 与
`artifacts/verification/funnel-annotations-glyph-native-control-second`。

## 字面层、更新与输入

`FunnelSvgLayerKind::Annotations` 独立于 Parts、Labels 和 SceneExtras。默认层将其
放在 Labels 之后、SceneExtras 之前。显式层列表必须主动包含它；省略不擦除 binding
元数据，重复则在每个 occurrence 绘制同一不可变注解源。各 occurrence 使用独立资源
命名空间并计入工作预算。空注解不增加源 body 或伪造业务 site。

注解绑定全部计算源 parts。owner 的 hidden series 不过滤这种独立装饰。
source/config/options/current 重建可产生新的绑定 cohort；仅 program/camera/hidden
呈现更新不重放 matcher 或 note 函数。完整来源、layer Cages 与 tooltip 呈现在 pump
组件 runtime 前共同采用。

原生注解墨迹遵循 SVG `visiblePainted`，可以覆盖 body。其 path 或 text 不会获得
Funnel part 身份或 part 鼠标 hook。绘在其上方的 Parts 仍可成为实际 body 目标。这也
适用于 custom note，以及保持 SVG 原生指针目标资格的透明 paint；有注解不等于能够
通过它拾取 body。

## 预算与失败边界

`funnel_annotations` 和 `funnel_annotations_with_budget` 构建独立 bundle。Frame 的
`with_annotations_with_budget` 与 `with_annotation_bundle` 支持单次组合。预计算
bundle 必须属于相同构造 capture；另一个 Frame 即使数据外观相同，也不是合法来源。
重新应用注解会替换旧注解层，不会串接旧 paint 或重放工厂。

`FunnelAnnotationBudget` 限制 specs、绑定工作、binding 数量、Marks、Labels 及路径
命令。可静态验证的内容先于回调检查；真实 predicate 命中和自定义绘制仍须后续检查。
不能因后面的回调失败，就声称前面用户回调未运行。错误不会产生已接纳候选 Frame。
在浏览器 owner 中，被拒的构建或自有 placement 保留旧已接纳数据、呈现及 peer Cages。
外部回调副作用不能回滚。提交后的 Glory UI 错误仍遵循独立宿主失败边界。

此功能不增加注解 Spring/退出过渡、任意 React note 元素、逐 note 保留式 Widget 或自动
viewport 避碰。不透明应用数据可使用新增的[类型化 Funnel 数据 API](funnel-data.zh-CN.md)。
WebKit 未投递窗口退出事件的既有限制仍见
[原生事件指南](funnel-events.zh-CN.md)。
