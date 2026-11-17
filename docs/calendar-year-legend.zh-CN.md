# Calendar 年份标签

[English](calendar-year-legend.md)

这些 API 自 Kartis 0.2.0 提供。

`calendar_layout_with_year_legend` 为 FullYears 增加显式年份标签定位和可借用、可失败的纯文本格式化函数。这里的年份标签是年面板旁的文字，不是 `Scene.legend` 图例行。原有 `calendar_layout` 和 `calendar_layout_with_color_profile` 保持完整的既有输出；`CalendarLayoutOptions` 没有新增字段。

```rust
use kartis::*;

let config = ChartConfig::default();
let options = CalendarLayoutOptions {
    mode: CalendarLayoutMode::FullYears,
    start: Some((2024, 1, 1)),
    end: Some((2024, 12, 31)),
    ..Default::default()
};
let placement = CalendarYearLegendOptions {
    position: CalendarLegendPosition::After,
    offset: 18.,
};
let prefix = String::from("年度");
let mut text = |year: &CalendarYearLegendContext| {
    Ok(format!("{prefix} {}", year.year))
};
let layout = calendar_layout_with_year_legend(
    &config, &[], &options, Some(&placement), Some(&mut text),
)?;
# Ok::<(), ChartError>(())
```

最后两个参数相互独立。定位为 `None` 时保留原来的页眉位置、锚点和旋转；格式化函数为 `None` 时显示整数年份。两者均为 `None` 时，FullYears 的 Scene、面板、日期、月份和颜色比例尺都与旧入口相同。新入口始终要求 FullYears；Compact 继续使用已有入口。Nivo TimeRange 本身也排除了这些年份标签配置。

需要颜色空间或调色方案时，使用 `calendar_layout_with_color_profile_and_year_legend(config, data, options, color_profile, placement, formatter)`；它接受原有颜色入口的相同 profile。带时区的观测可先通过已有日期聚合函数转换为民用日期，再调用本入口；本次未增加带时区的便捷入口。

## 位置与样式

`CalendarYearLegendOptions::default()` 为 `Before`、偏移 10 像素。负偏移沿原版 Nivo 公式将原点移向面板内部；绝对值足够大时也可能越过另一侧边界。设**已经应用方向转换**的真实 `CalendarPanel.grid` 为 `(x, y, width, height)`，未作基线补偿的原点与基础旋转如下：

| 方向 | 位置 | 原点 | 基础旋转 |
| --- | --- | --- | --- |
| Horizontal | Before | `(x - offset, y + height / 2)` | −90° |
| Horizontal | After | `(x + width + offset, y + height / 2)` | −90° |
| Vertical | Before | `(x + width / 2, y - offset)` | 0° |
| Vertical | After | `(x + width / 2, y + height + offset)` | 0° |

显式定位使用 `TextAnchor::Middle`，继承 `year_style.size` 与 `year_style.color`，未设置的值继续回退到主题。最终旋转为基础旋转加 `year_style.rotation`，规范到 `[−180°, 180°)`。附加旋转是 Kartis 扩展；参考的 Nivo 年份标签配置没有对应的旋转属性。

Kartis 保留现有 alphabetic 基线换算：字体大小为 `s`、最终旋转为 `r` 时，存入 `Label` 的坐标为 `(origin.x - sin(r) * s / 3, origin.y + cos(r) * s / 3)`。SVG、Canvas 和 CPU 渲染器使用同一标签。这是近似居中处理，不保证与 Nivo 在具体字体上的视觉基线完全相同。

定位 profile 只移动文字，保留既有年份页眉空间、日期几何、月份边界、拾取标识、标签顺序与 `Scene.legend`。计算使用包含 `month_spacing` 的实际面板边界。接口不会自动扩展边距；超出视口的文字可能被裁剪，应配合设置图表边距和偏移。

## 格式化与验证

`CalendarYearLegendTextFactory<'a>` 是借用的 `FnMut(&CalendarYearLegendContext) -> Result<String>`。上下文提供 `year: i32`、按时间排列的面板 `index` 和实际已定向的 `grid`。它返回纯文字，不是任意 React 组件或 Glory Widget 工厂。

定位输入通过验证后，每个实际输出的年份标签调用一次格式化函数。`year_labels: Some(false)` 禁用年份标签，也不调用格式化函数。沿用现有行为，`hidden_years` 隐藏该年的日期和归属标签，但保留共享年份装饰，因此对应年份的格式化函数仍会执行。空字符串保留空标签槽位，不抑制或重排其他标签。显示文字不会更改 `Scene.legend` 元数据。

偏移必须有限且在 `-1024..=1024` 像素内。每个返回字符串不得超过 `MAX_LABEL_BYTES`（目前为 1024 个 UTF-8 字节）；恰好达到限制的 Unicode 字符串可以使用。已有的最多 32 年、12,000 天、尺寸、旋转和几何限制继续生效。无效输入、超预算文字或回调错误均返回 `Err`，不返回部分布局。回调自身对外部状态的副作用不会回滚。

## 参考与验收

参考 Nivo 提交为 `0dae2c32052a573f9f1d66ec1b453b572119b265`：

- `packages/calendar/src/types.ts` 定义 `yearLegend`、`yearLegendOffset` 和 `yearLegendPosition`；`TimeRangeSvgProps` 排除这些属性。
- `packages/calendar/src/props.ts` 默认显示整数年份，位置为 `before`，偏移为 10。
- `packages/calendar/src/hooks.ts` 的 `useYearLegends` 调用 `compute/calendar.ts` 中的 `computeYearLegendPositions`。
- `packages/calendar/src/CalendarYearLegends.tsx` 使用 middle 锚点及 translate/rotate，没有 `dy` 或显式文本基线属性；`packages/text/src/Text.tsx` 透传属性和主题样式。
- `packages/calendar/src/CalendarCanvas.tsx` 在逐标签平移和旋转前设置 center 对齐与 `middle` 基线。

上述路径用于标识上游源码，使用本接口不依赖该源码仓库。年份标签位置遵循原版四种公式，以 Kartis 面板边界为输入。实际占用周列、页眉预留和基线仍遵循 Kartis 既有策略；这不表示与 Nivo 所有参数的布局坐标或字体渲染全面等价。

`tests/calendar_year_legend.rs` 验证双方向与前后位置、负偏移及零值和正负边界、月份间距、样式继承及旋转组合、默认完整布局与颜色 profile 不变、隐藏及禁用标签、空文字与转义 Unicode、字节预算、回调错误、SVG/Canvas 序列化和真实 Lato 字体 CPU 绘制：

```sh
cargo test --test calendar_year_legend --features raster
```
