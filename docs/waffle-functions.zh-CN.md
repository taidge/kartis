# Waffle 分组函数与原始应用数据

这是尚未发布的开发接口。Waffle 支持借用式分组颜色、数值格式化和边框颜色函数，
结果共同用于实际单元格或区域绘制、分组元数据、图例及交互站点。
可选的拥有式数据帧能在数值布局旁保留任意应用数据。
[English](waffle-functions.md)

## 分组函数

把借用式 `WaffleFactories` 传给 `waffle_layout_with_factories`。
未配置的函数继续使用 `WaffleLayoutOptions`；现有 `waffle_layout` 保留原有行为。
构建完成后不会保留任何函数。

| 字段 | 输入 | 输出 |
| --- | --- | --- |
| `color` | `&WaffleGroupColorContext` | `Result<Color>` |
| `formatted_value` | 原始投影数值 `f64` | `Result<String>` |
| `border_color` | `&WaffleBorderColorContext` | `Result<Color>` |

`WaffleGroupColorContext` 包含原数值数据 `RadialDatum`、`source_index`、
调色板提供的 `default_color`、绘图区和居中网格边界、总量与单位值。
单位值为总量除以网格槽数。这里的数据引用不保证与调用者输入切片具有相同地址。

`WaffleBorderColorContext` 提供相同布局边界、总量和单位值，另有可选默认颜色及两种目标：

- `WaffleBorderTarget::Group(&WaffleGroup)` 已包含解析后的填充色和格式化值、
  隐藏状态、原请求区间及容量裁剪后的占用区间。此时 `border_color` 暂时等于
  新填充色，`polygons` 为空，`area_mark_id` 为 `None`。这是早期样式输入，
  不是最终带多边形的分组。
- `WaffleBorderTarget::Empty(WaffleEmptyStyle { color, opacity })` 是共享的空单元格
  样式模板，不含源数据、分组索引或物理单元格坐标。

对每个源分组，已配置函数依次执行：颜色、数值格式化、边框颜色。
所有分组按原始顺序参与，包括隐藏组、零值、负零，以及请求区间超出网格容量的组。
全部分组完成后，空样式边框函数只执行一次；满格没有空单元格、或者源数据为空时也一样。
未配置边框函数就不会产生该角色的应用回调。

```rust
use kartis::*;

fn main() -> Result<()> {
    let mut color = |c: &WaffleGroupColorContext<'_>| {
        Ok(if c.datum.id == "late" {
            Color(210, 70, 35, 255)
        } else {
            c.default_color
        })
    };
    let mut format = |value: f64| Ok(format!("{value:.1} units"));
    let mut border = |c: &WaffleBorderColorContext<'_>| {
        Ok(c.default_color.unwrap_or(Color(30, 30, 30, 255)))
    };
    let layout = waffle_layout_with_factories(
        &ChartConfig::default(),
        &[RadialDatum::new("late", 30.), RadialDatum::new("ready", 50.)],
        &WaffleLayoutOptions {
            total: Some(100.),
            border_width: 2.,
            ..Default::default()
        },
        &mut WaffleFactories {
            color: Some(&mut color),
            formatted_value: Some(&mut format),
            border_color: Some(&mut border),
        },
    )?;
    assert_eq!(layout.groups[0].formatted_value, "30.0 units");
    assert_eq!(layout.groups[0].color, Color(210, 70, 35, 255));
    Ok(())
}
```

完整的[导出例子](../examples/waffle_factories.rs) 会写出 Cells 与 Areas 的 SVG，
另存原始数值位模式、格式化值、区间及隐藏状态表。运行时显式提供输出目录：

```text
cargo run --example waffle_factories -- artifacts/waffle-factories-example
```

## 保留原始应用数据

`WaffleDatum<D>` 包含 `data: D` 和 `projected: RadialDatum`。
应用显式给出数值投影；只有投影决定身份、分配、数值及标签。
`D` 内的其它数值会原样保留，不会被重新解释为布局值。

`WaffleDataFrame<D>::new` 接收并拥有源向量及其校验后的布局。
`with_factories` 另接收借用式 `WaffleDataFactories<D>`：

- 颜色函数接收 `WaffleDataColorContext { data: &D, computed }`。
- 边框函数接收 `WaffleDataBorderContext { data: Option<&D>, computed }`；
  空样式模板的 `data` 为 `None`。
- 格式化函数仍只接收投影数值，不接收 `D`。

`computed` 内是上文对应的基础上下文，执行顺序和回调次数保持不变。

```rust
use kartis::*;

// 原始应用数据无需实现 Clone、Debug 或 PartialEq。
struct Job {
    overdue: bool,
    note: String,
}

fn main() -> Result<()> {
    let source = vec![
        WaffleDatum::new(
            Job { overdue: true, note: "original business record".into() },
            RadialDatum::new("late", 34.),
        ),
        WaffleDatum::new(
            Job { overdue: false, note: "retained even without cells".into() },
            RadialDatum::new("zero", -0.),
        ),
    ];
    let mut color = |c: &WaffleDataColorContext<'_, Job>| {
        Ok(if c.data.overdue { Color(210, 70, 35, 255) }
           else { c.computed.default_color })
    };
    let frame = WaffleDataFrame::with_factories(
        &ChartConfig::default(),
        source,
        &WaffleLayoutOptions {
            rows: 2, columns: 3, total: Some(100.),
            ..Default::default()
        },
        &mut WaffleDataFactories { color: Some(&mut color), ..Default::default() },
    )?;
    assert_eq!(frame.data(0).unwrap().note, "original business record");
    assert_eq!(frame.source()[1].projected.value.to_bits(), (-0f64).to_bits());

    let packet = frame.cell_widgets(3, |c| {
        let b = c.computed.cell.bounds;
        Ok(Some(PlotWidgetPlacement::Rect(LayoutRect {
            x: b.x, y: b.y, width: b.width, height: b.height,
        })))
    })?;
    assert!(packet.data.values().any(|row| row.data().is_some_and(|d| d.overdue)));
    assert!(packet.data.values().all(|row| row.frame() == &frame));
    Ok(())
}
```

数据帧通过共享存储拥有一个源向量。克隆数据帧或 typed Widget 负载不会克隆 `D`。
基础数据帧不要求 `D` 实现 `Clone`、`PartialEq`、`Debug`、`Send` 或 `Sync`；
能否跨线程传递仍由实际成员类型决定。保留式 Widget 负载要求
`cell_widgets` 的 `D: 'static`，其它数据帧方法没有这项约束。

帧的相等性按捕获身份判断。克隆帧与原帧相等；独立构建的帧不同，即使其数值布局
和绘制完全相等。这样只有原始字段变化时也能通知 Widget 数据比较，不会因应用自定义
相等性忽略字段或把正负零视为相等而漏掉更新。接口保留传入值，但不会冻结应用拥有的内部可变性。

## Typed Widget 与交互

`frame.cell_widgets(revision, placement)` 返回
`PlotWidgetsData<WaffleDataCellWidgetData<D>>`。
布局回调能读取真实单元格及 Mark 上下文与可选原始数据。
每项负载提供 `computed()`、`data()`、`frame()`；空单元格没有分组或原始数据。
负载会保留整个源集合，包括没有占用单元格的组。同组多个单元格继续共享数值分组快照。

适配器只选择实际存在的单元格 Mark。Areas 模式的已占用单元格属于整体区域 Mark，
所以只有空单元格 Mark 可参与 cell Widget，不会伪造 area Widget。
原有测量槽校验和 128 槽预算继续适用。
如何把数据包接到真实 Glory 组件，参见 [typed plot Widget](typed-plot-widgets.zh-CN.md)。

`frame.interaction()` 克隆数值 `SceneInteraction`；
`frame.area_interaction()` 建立经过校验的完整槽分组事件区，详见
[Waffle 分组事件区](waffle-regions.zh-CN.md)。两个返回值都不包含 `D`：
如果仍需访问原始数据，应保留帧或 typed Widget 负载。调用这些方法不会重新执行分组函数。

## 组合自定义单元格几何

使用 `waffle_layout_with_factories_and_cell_factory`；需要原始数据时使用
`WaffleDataFrame::with_factories_and_cell_factory`。
全部分组函数和空样式模板完成后，才调用第一个单元格函数。
原始数据版上下文提供 `data: Option<&D>` 与最终
`computed: WaffleCellContext`；空单元格的 `data` 为 `None`。

现有单元格工厂按遍历顺序仅作用于实际绘制的单元格 Mark，能读到解析后的填充色、
描边和分组格式化值。分组函数不会按每个单元格重新执行。
返回 `computed.inherit(...)` 可继承这些样式。主动改自定义墨迹时，实际颜色可以与
分组或图例不同，沿用[自定义 Waffle 单元格](waffle-custom.zh-CN.md)的既有合同；
实际 Mark 仍是物理绘制及自定义墨迹拾取的依据。

## 校验与失败边界

在用户函数前校验静态源数据、选项、调色板及默认来源、普通几何。
颜色函数不能挽救无效源数据或调色板配置；格式化函数不能绕过无效的已配置格式。
格式化结果上限是 `MAX_LABEL_BYTES`，即 1024 个 UTF-8 字节，不是 Unicode 字符数。
原始帧的源条目数量受 `MAX_MARKS` 限制。

显式边框函数可处理无法解析的继承式对比色：当当前角色与背景无法求出默认颜色时，
`default_color` 为 `None`，函数需自行返回有效颜色。未配置边框函数时仍保留原解析错误，
空单元格模板也一样。

失败不会返回候选布局或帧，借用式基础输入不会被修改。
原始帧构建会消耗源向量；失败时丢弃候选数据，不会把向量返还给调用者。
先前保留的帧及其 Widget 数据不变。日志、计数器、共享应用对象等由回调自行执行的副作用无法回滚。

这些构造函数本身不会发布浏览器事务，也不会建立自动 Waffle owner。
接入 Chart 宿主时，应先准备候选布局和匹配的 Widget 数据包，再请求宿主采用，
遵循该宿主的校验与生命周期规则。本接口不表示已经提供 React 组件、
React Spring 时间积分、Waffle 专用原生鼠标处理器或 GPU 等价行为。

## 验证范围

原生测试覆盖旧结构及 SVG 保持、回调顺序、隐藏/零值/溢出分组、样式消费者、
原始对象身份与存活期、Widget 共享和失败情形。
独立提取并执行的上游参考验证计算和函数输入，不是浏览器绘制或 React 生命周期测试。
浏览器后端、组件状态、像素检查及受检边界单独记于
[验收记录](verification.md#development-waffle-functions-and-owned-data)。
