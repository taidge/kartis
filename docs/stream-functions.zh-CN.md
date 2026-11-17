# Stream 函数配置

这是开发中的 API，尚未包含在已发布的 0.1.0 包中。
`StreamFactories` 提供八个同步、借用的函数入口，直接参与布局构建，结果用于
实际绘制、计算元数据、图例、提示、切片以及[类型化点 Widget](stream-widgets.zh-CN.md)。
构建器不会保留这些函数。[English](stream-functions.md)

## 函数与上下文

普通点几何使用 `stream_layout_with_factories`；需要组合现有点几何工厂时，使用
`stream_layout_with_factories_and_dot_factory`。未设置的函数继承现有选项，
两个入口均不向 `StreamLayoutOptions` 增加字段。

| bundle 字段 | 输入 | 输出 |
| --- | --- | --- |
| `label` | `StreamLayerPathContext` | 层标签 `String` |
| `color` | `StreamLayerPathContext` | 层 `Color` |
| `formatted_value` | 非空的原始 `f64` | 格式化 `String` |
| `border_color` | `StreamLayerStyleContext` | 面积边框 `Color` |
| `dot_size` | `StreamDotStyleContext` | Scene 像素单位的圆直径 |
| `dot_color` | `StreamDotStyleContext` | 点 `Color` |
| `dot_border_width` | `StreamDotStyleContext` | Scene 像素单位的边框宽度 |
| `dot_border_color` | `StreamDotStyleContext` | 点边框 `Color` |

所有函数均返回 `Result`。三类上下文都借用原始 `BarSeries`，并提供原始系列索引
和 plot 边界。`StreamLayerPathContext` 还提供原始层 ID，以及使用完整 Scene
坐标的可选面积 `Geometry`；这里传递的是 Rust 几何，不是 SVG 路径字符串。
`StreamLayerStyleContext` 提供完整计算层和可选路径。
`StreamDotStyleContext` 提供同一个完整层、当前 sample、配置的位置，以及
`style` 中声明的 `StreamDotOptions`。层包含全部 sample，包括空值，绝不是
尚未构建完成的前缀。

## 完整应用

此[示例](../examples/stream_functions.rs) 挂载真实 Glory Memory Chart，重建源数据，
并检查已接受的格式化值。Memory 验证宿主协议，不代表操作系统输入验收。

```rust
//! A complete Glory application using Stream's eight borrowed functions.
use std::cell::Cell;

use glory::App;
use kartis::stack::{StackOffset, StackOptions};
use kartis::*;
fn source(value: f64) -> Result<SceneInteraction> {
    let categories = [CategoryKey::from("first"), CategoryKey::from("last")];
    let series = [
        BarSeries::new("sales", vec![Some(10.), Some(value)]),
        BarSeries::new("returns", vec![Some(2.), None]),
    ];
    let options = StreamLayoutOptions {
        stack: StackOptions {
            offset: StackOffset::Zero,
            ..Default::default()
        },
        dots: Some(StreamDotOptions::default()),
        legend: true,
        ..Default::default()
    };
    let formats = Cell::new(0);
    let mut label =
        |ctx: &StreamLayerPathContext<'_>| Ok(format!("{} ({})", ctx.id, ctx.series.values.len()));
    let mut color = |ctx: &StreamLayerPathContext<'_>| {
        Ok(if ctx.id == "sales" {
            Color(30, 140, 90, 255)
        } else {
            Color(190, 80, 40, 255)
        })
    };
    let mut format = |v: f64| {
        formats.set(formats.get() + 1);
        Ok(format!("{v:.0} tickets"))
    };
    let mut border = |ctx: &StreamLayerStyleContext<'_>| Ok(ctx.layer.color);
    let mut size = |ctx: &StreamDotStyleContext<'_>| Ok(8. + ctx.sample.source_index as f64 * 2.);
    let mut dot_color = |ctx: &StreamDotStyleContext<'_>| Ok(ctx.layer.color);
    let mut width = |_: &StreamDotStyleContext<'_>| Ok(2.);
    let mut dot_border = |_: &StreamDotStyleContext<'_>| Ok(Color(255, 255, 255, 255));
    let layout = stream_layout_with_factories(
        &ChartConfig::default(),
        &categories,
        &series,
        &options,
        &mut StreamFactories {
            label: Some(&mut label),
            color: Some(&mut color),
            formatted_value: Some(&mut format),
            border_color: Some(&mut border),
            dot_size: Some(&mut size),
            dot_color: Some(&mut dot_color),
            dot_border_width: Some(&mut width),
            dot_border_color: Some(&mut dot_border),
        },
    )?;
    assert_eq!(formats.get(), 3); // Three Some values; area summaries reuse these strings.
    assert_eq!(layout.layers[1].samples[1].formatted_value, None);
    assert_eq!(
        layout.layers[0].samples[1].formatted_value,
        Some(format!("{value:.0} tickets"))
    );
    let mut frame = layout.into_scene_interaction()?;
    // Bound Glory charts explicitly pick the actual painted geometry.
    frame.interaction.as_mut().unwrap().picking_profile = PickingProfile::PaintedGeometry;
    kartis::surface::SurfaceModel::new_with_interaction(
        frame.scene.clone(),
        kartis::surface::SurfaceOptions {
            plot: frame.plot,
            ..Default::default()
        },
        frame.interaction.clone(),
    )?;
    Ok(frame)
}
fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let app = App::memory();
    let input = app.scope().cage(source(20.)?);
    let frame = app.scope().bond(move || input.get());
    app.mount(chart_layout(frame).on_reject(|e| eprintln!("Stream example rejected: {e:?}")))?;
    app.flush()?;
    input.revise(|old| *old = source(30.).unwrap());
    app.flush()?;
    assert!(input.with(|s| {
        s.interaction
            .as_ref()
            .unwrap()
            .sites
            .iter()
            .any(|s| s.formatted_value.as_deref() == Some("30 tickets"))
    }));
    app.clear()?;
    Ok(())
}
```

## 求值与继承

首次调用函数前，构建器检查完整源数据、选项、堆叠、比例尺、身份、sample 投影坐标、
相关原始路径及原始资源预算。随后按现有布局顺序处理各层：先对该层每个 `Some`
格式化一次，再依次计算 `label`、`color` 和可见面积的边框。接着按原始数据顺序
处理有资格的 sample，依次调用点尺寸、颜色、边框宽度和边框颜色函数。若提供自定义
点几何工厂，它在对应点的函数样式解析后调用。Scene 仍保持先面积、后点的绘制顺序。

面积汇总使用最后一个原始 sample，格式化值复用该 sample 已计算的字符串，不再次
调用格式化函数。如果最后一个 sample 为 `None`，汇总值和格式化值均保持 `None`，
不会向前寻找非空值。Rust 保留 `None`、`Some(0.0)` 以及原始带符号零，
这是明确的可空合同，不模拟 JavaScript 对 null 的数值转换。

显式 `label` 返回的 `""` 会保持为空，不会由 ID 或静态标签映射替换。真实面积仍可
绘制，但空标签不会生成虚假的规范面积交互站点或锚点。sample 标签保留类别后缀，
例如 `" · first"`；计算元数据和图例标签中的层标签仍为空。没有标签函数时，
现有静态空标签回退规则保持不变。

显式层 `color` 完整覆盖静态的逐系列颜色查找。因此，当合法的固定序数域不包含该
系列，且未知值策略为 `Missing` 时，函数仍能提供颜色。声明的序数配置本身仍须通过
校验。`layout.colors` 保留静态域和映射，并非函数结果缓存；应从计算层和当前 Mark
读取实际结果。已有显式面积 paint 配置仍然生效，因此代表色不等于每个渐变像素。

显式颜色与边框颜色函数不会先解析被它们覆盖的静态继承色。例如，不再使用的静态
`Contrast` 在透明绘制上可能失败，但显式替代色仍可成功。未提供替代函数时，实际
需要的继承颜色仍遵循原校验。可见面积的边框宽度为零时，显式面积边框颜色函数仍会
调用；有资格的点最终边框宽度为零时，显式点边框颜色函数也仍会调用。缺省函数不会
引入对未使用默认边框颜色的额外解析。

几何工厂对点的最终轮廓拥有决定权。它的 `StreamDotContext.fill` 和 `stroke`
包含已解析的绘制样式，`inherit` 保留这些样式。几何和样式上下文中的 `style`
均保持声明配置，`style.size` 不会被函数返回值改写。自行创建几何的工厂需要自行
决定尺寸；尺寸函数控制普通圆形的直径。

## 资格与边界

隐藏层也会计算标签、层颜色及非空值的格式化结果。面积边框函数只在存在可见面积时
调用。点样式和自定义点几何工厂要求启用 dots、层未隐藏、sample 非空；
`Some(0.0)` 和零尺寸点仍有资格。`dots: None` 与 `None` sample 保留现有切片
占位数据，不成为点函数角色。宿主层面的隐藏是独立的呈现操作。

隐藏层不会生成 sample Mark，因此其 1,024 字节层标签可以合法保留，不会为不存在
的 Mark 拼接类别后缀。没有标签或颜色函数请求路径时，隐藏面积路径也不会构建。
提供任一函数即请求原始几何，必须通过相同的有限坐标检查，包括控制坐标绝对值不超过
1,000,000 像素。Natural 曲线控制点可能超出 sample 坐标，这一边界尤其需要注意。
仅提供格式化或点函数，不会请求该未使用的隐藏面积路径。

函数字符串限制为 1,024 个 UTF-8 字节；可见 sample 的最终拼接标签也必须满足该
预算。点直径必须有限且位于 0 至 4,096 像素；边框宽度必须恰为零，或位于
0.000001 至 256 像素。原有源数据、Scene 命令、paint stop 和身份预算仍生效。
函数输出及自定义几何在产生时接受检查；后续错误不会返回部分布局，但无法撤销
先前回调对外部状态造成的副作用。

`dot_size` 与任何已配置 glyph 组合都会在回调前被拒绝，因为圆直径与 glyph 面积
或标称尺寸使用不同单位。其他函数可以与 glyph、自定义几何组合。仅描边 glyph
可以通过 `dot_border_width` 获得所需边框；返回零宽度会在几何回调前被拒绝。
没有合资格点时，不会为了计算函数而虚构描边角色。没有宽度函数时，旧静态校验不变。

## 兼容与证据范围

Nivo Stream 提供函数形式的标签、颜色、值格式化、面积边框色及四种点样式属性。
它的层标签和颜色 accessor 接收 ID 与已生成路径，点样式接收计算数据；Kartis
采用上述借用的 Rust 上下文。扁平系列输入、类型化类别键、可空值与 `Geometry`
路径是明确的 Rust 接口，不是通用 JavaScript 属性 getter，也不把数值 ID 与
字符串 ID 视为同一身份。

Rust 构建器格式化隐藏 sample，并计算隐藏层标签和颜色；边框及点调用遵循 Rust
真实面积、点的资格。Nivo 的渲染调用还取决于实际挂载的 layers 与 `enableDots`，
其公开 Stream datum 值类型为数值。这里不将资格差异描述成完全相同的回调轨迹。
交互开关不会取消原本有资格的零尺寸点角色。

记录中的 96 组配置对照使用改动前实际编译的构建器，比较默认路径重构前后的
Debug 布局与 SVG 字节，范围限于这些记录配置，而非所有可能输入。额外检查覆盖
完整层上下文、空值格式化、显式颜色覆盖、隐藏数据准入，以及类型化 Memory/CPU
宿主事务。浏览器证据应区分声明元数据、真实源绘制、图例 DOM 标签与输入，
本函数示例不隐含源字形标签或 Widget 字体样本的验收。

这些函数不实现 React 组件、自动弹簧动画、任意 JavaScript 图层执行，也不承诺
跨渲染器的普遍文字或像素一致性。实时 Widget 使用独立的[点角色适配器](stream-widgets.zh-CN.md)；
Scene 导出包含结果绘制，不包含实时 Glory 子组件。GPU 与 worker 渲染属于独立能力。
