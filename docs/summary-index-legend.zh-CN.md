# 摘要值域、Bar 分类图例与图例动作

这些选项扩展共享 Scene、SVG、Canvas 和 CPU 渲染接口。原默认值仍为 `SampleExtent`、`BarLegendSource::Keys`、`series_order: None`，分类图例默认保持静态。

| API | 含义 |
| --- | --- |
| `BoxPlotLayoutOptions.auto_domain_source` | 选择 `SampleExtent` 或显式 `SummaryQuantiles`。 |
| `BarStyleOptions.legend_source` | 选择系列 `Keys` 或分类 `Indexes`；`legend` 仍是总开关。 |
| `BarStyleOptions.series_order` | 可选的完整来源系列 ID 排列。 |
| `Scene.legend_roles` | 以真实图例 ID 为键的稀疏 `LegendRole::Series` / `Category` 元数据。 |
| `Chart::on_legend_activate` | 观察类型化 `LegendActivation { item, role }` 意图，并启用分类动作控件。 |

`SampleExtent` 保留原 BoxPlot 样本极值域、分位数算术以及全序/ID 排序。`SummaryQuantiles` 根据可见箱体的全部五个配置摘要值推导自动端点，支持自定义和重复分位数；whisker 策略、原始极值和离群点不作为该域的输入。可空观测值继续表示缺失，不转换为零。此模式按有限数值稳定排序，相等值保留输入顺序，包括有符号零；计算结果的 `source_ids` 顺序遵循该稳定顺序，不替换任何来源身份。

摘要插值在差值和结果有限时使用原 Nivo 表达式 `x + (y - x) * t`；分位数索引恰为整数时返回原样本值。这保留普通窄域的舍入、常量及插值有符号零行为。仅当中间差值或结果非有限时，才回退到原稳定加权表达式。例如原表达式对 `[-1e308, 1e308]` 的部分分位数会溢出，而有限回退仍可生成有效摘要；这是明确的有限扩展。选择 `SummaryQuantiles` 也选择了这一统计算术，即使完整 domain 或两端固定 bounds 跳过值域推导，它仍然生效，因此分位数 bits 可以与默认模式不同。

独立 `domain_bounds` 在推导后只替换固定的一端。完整 `domain` 或两端固定 bounds 会跳过推导，同时提供 `domain` 和 `domain_bounds` 仍会报错。共享 nice、reverse、round、裁剪和尺度校验保留原含义。原有限空域回退和 raw Log/负指数 Pow 来源校验继续生效：空输入不复现 Nivo 的无穷空域，摘要值也不会掩盖原始零值或混号回退输入的错误。

对于显式配置的 UTC 入口，只有自动摘要域端点使用 Date TimeClip：校验有限 Date 范围，向零截断到毫秒，并把零规范为正零。固定端点仍必须是有界整数毫秒。原始样本、小数分位数、中位数和 `map_statistic` 查询值不会被截断。旧的非配置 UTC 入口继续不支持。摘要范围 `0.1..0.9` 因此得到 `[0, 0]` 域，同时保留小数统计量。参考 Nivo 时间工厂另有 falsy-zero 缺陷：字面零端点可能保留其默认的 2000 年日期区间。本模式保留数学上的 `[0, 0]`，不复制这一缺陷。

Bar 的 `series_order: Some(...)` 必须恰好包含每个来源系列 ID 一次，包括隐藏系列。它决定分组位置、stack/percent 累积、完整系列 palette 和 Keys 图例顺序；`None` 保留词法顺序。隐藏系列保留自己的 palette 槽位。普通及工厂上下文中的 `source_series` 仍是原始输入索引。分类 palette 保留其独立的既有顺序，显式颜色继续覆盖默认值。

Indexes 图例使用原始分类，以及有序、未隐藏的首个 key 的数值 palette 颜色，即使该 key 对应的数据为 null 或零。它在 null、墨迹、值域裁切和分类窗口过滤前建立，因此全空分类仍可有图例项，而不会伪造 bar。横向分类图例只反序一次。关闭图例、没有可见 key 或分类带宽为零时不生成分类项；实际输出项继续遵守共享的 256 项预算。自定义画笔和悬停不会替换数值图例 palette。Keys 保留原系列控制语义，包括原分类配色模式的中性符号。

角色区分看起来相同的分类 ID 和系列 ID。已有图例项缺省角色为 Series；`scene.legend_role(unknown)` 返回 `None`。角色键必须指向真实图例项，参与 Scene 校验与相等比较。分类项不跟随隐藏系列集合淡化。显式 `without_series`、hidden 状态及系列切换接口仍处理系列。自定义 Bar 布局在回调前安装角色，并将其随 accepted frame 冻结；之后修改角色会被 binding 和受控悬停校验拒绝。

没有动作回调时，分类图例是静态内容。`Chart::on_legend_activate` 将它们变成普通动作控件，系列继续保留开关。激活携带当前原始图例 ID、label、color 和 role。Chart 在尝试系列切换之前通知意图，并在切换前重新检查当前 accepted role。意图不代表 hidden 状态已提交；受控 hidden 状态仍遵循调用方提案/接受协议。自定义图例 renderer 会收到响应式 `role`、`hidden` 和 `label`，返回的 Widget 提供内容，不嵌套另一组交互控件。

BrowserChart 和 RasterSurface 必须在挂载选项中设置 `legend_actions: true`；挂载后仅订阅事件不会把静态分类变成按钮。浏览器动作通过 `BrowserEvent::LegendActivated` 到达，栅格动作使用 `on_legend_activate`。排队处理的宿主会依据当前 accepted Scene 重新校验身份和角色。分类动作从不隐式隐藏系列。下方按 feature 启用的 helper 展示挂载时的显式开关；应用提供 root 或 scope、字体和分类动作行为。

完整程序构造两种布局及其控件。`web` helper 用于启用 `web` 的 wasm32 构建；`raster` helper 需要 `raster`，并注册覆盖标签字符的字体。构造 Chart builder 不会模拟用户激活。

示例的 `cfg(feature = ...)` 名称属于应用包。声明应用所需的 `kartis` 和 `glory` 依赖后，转发这些 feature 名称，并提供浏览器 helper 使用的 `web_sys::Element` 类型：

```toml
[features]
web = ["kartis/web", "dep:web-sys"]
raster = ["kartis/raster"]

[target.'cfg(target_arch = "wasm32")'.dependencies]
web-sys = { version = "=0.3.107", optional = true, features = ["Element"] }
```

```rust
use kartis::*;
use std::{cell::RefCell, collections::BTreeSet, rc::Rc};

fn observations(values: &[f64]) -> Vec<BoxObservation> {
    values
        .iter()
        .enumerate()
        .map(|(i, value)| BoxObservation::new(i.to_string(), "A", "", Some(*value)))
        .collect()
}

fn main() -> kartis::Result<()> {
    let config = ChartConfig {
        width: 480.,
        height: 320.,
        margin: Margin {
            left: 60.,
            right: 60.,
            top: 60.,
            bottom: 60.,
        },
        theme: Theme {
            colors: vec![Color(200, 40, 60, 255), Color(40, 80, 200, 255)],
            ..Default::default()
        },
        ..Default::default()
    };
    let source = observations(&[1., 2., 3., 4.]);
    let sample = boxplot_layout(&config, &source, &Default::default())?;
    let mut box_options = BoxPlotLayoutOptions {
        auto_domain_source: BoxPlotAutoDomainSource::SummaryQuantiles,
        orientation: StatisticsOrientation::Horizontal,
        ..Default::default()
    };
    let summary = boxplot_layout(&config, &source, &box_options)?;
    assert_eq!(sample.domain, [1., 4.]);
    assert_eq!(summary.domain, [1.3, 3.7]);
    assert_eq!(summary.boxes[0].extrema, [1., 4.]);
    assert_eq!(summary.boxes[0].quantiles, [1.3, 1.75, 2.5, 3.25, 3.7]);
    box_options.domain_bounds = Some(BoxPlotDomainBounds {
        min: Some(2.),
        max: None,
    });
    assert_eq!(
        boxplot_layout(&config, &source, &box_options)?.domain,
        [2., 3.7]
    );
    box_options.domain_bounds = Some(BoxPlotDomainBounds {
        min: None,
        max: Some(3.),
    });
    assert_eq!(
        boxplot_layout(&config, &source, &box_options)?.domain,
        [1.3, 3.]
    );

    box_options.domain_bounds = None;
    let utc = ContinuousScaleOptions {
        kind: ScaleKind::Utc,
        ..Default::default()
    };
    let base = 8_000_000_000_000_000.;
    let dates = boxplot_layout_with_scale_options(
        &config,
        &observations(&[base, base + 5.]),
        &box_options,
        &utc,
    )?;
    assert_eq!(dates.value_scale.source_domain(), [base, base + 4.]);
    assert_eq!(dates.map_statistic(base + 4.)?, 420.);
    let sub_ms =
        boxplot_layout_with_scale_options(&config, &observations(&[0., 1.]), &box_options, &utc)?;
    assert_eq!(sub_ms.value_scale.source_domain(), [0., 0.]);
    assert_eq!(sub_ms.layout.boxes[0].quantiles[2], 0.5);
    assert_eq!(sub_ms.map_statistic(0.25)?, 240.);

    let categories = ["z", "zero", "missing"].map(str::to_string);
    let bars = [
        BarSeries::new("a", vec![Some(2.), None, None]),
        BarSeries::new("z", vec![None, Some(0.), None]),
    ];
    let bar_options = BarOptions {
        value_domain: Some([-1., 3.]),
        ..Default::default()
    };
    let style = BarStyleOptions {
        legend_source: BarLegendSource::Indexes,
        series_order: Some(vec!["z".into(), "a".into()]),
        ..Default::default()
    };
    let indexed = bar_layout_with_style(&config, &categories, &bars, &bar_options, &style)?;
    assert_eq!(
        indexed
            .scene
            .legend
            .iter()
            .map(|i| i.id.as_str())
            .collect::<Vec<_>>(),
        ["z", "zero", "missing"]
    );
    assert!(
        indexed
            .scene
            .legend
            .iter()
            .all(|i| i.color == config.theme.colors[0])
    );
    assert_eq!(indexed.scene.legend_role("z"), Some(LegendRole::Category));
    assert_eq!(indexed.scene.legend_role("unknown"), None);
    let hidden_z = BTreeSet::from(["z".to_string()]);
    assert!(!indexed.scene.legend_hidden("z", &hidden_z));
    assert!(
        indexed
            .bars
            .iter()
            .any(|b| b.series == "z" && b.raw_value == 0. && b.source_series == 1)
    );
    assert!(!indexed.bars.iter().any(|b| b.category == "missing"));

    let hidden_options = BarOptions {
        hidden: hidden_z,
        ..bar_options.clone()
    };
    let hidden = bar_layout_with_style(&config, &categories, &bars, &hidden_options, &style)?;
    assert!(
        hidden
            .scene
            .legend
            .iter()
            .all(|i| i.color == config.theme.colors[1])
    );
    let keys_style = BarStyleOptions {
        legend_source: BarLegendSource::Keys,
        ..style.clone()
    };
    let keys = bar_layout_with_style(&config, &categories, &bars, &bar_options, &keys_style)?;
    assert_eq!(
        keys.scene
            .legend
            .iter()
            .map(|i| i.id.as_str())
            .collect::<Vec<_>>(),
        ["z", "a"]
    );
    assert_eq!(keys.scene.legend_role("z"), Some(LegendRole::Series));

    let category_requests = Rc::new(RefCell::new(Vec::<String>::new()));
    let requests = category_requests.clone();
    let _controls = chart(indexed.scene.clone()).on_legend_activate(move |intent| {
        if intent.role == LegendRole::Category {
            requests.borrow_mut().push(intent.item.id);
        }
    });
    let _svg = render::to_svg(&summary.scene)?;
    let _canvas_script = render::to_canvas_script(&indexed.scene, "bars")?;
    Ok(())
}

#[cfg(all(feature = "web", target_arch = "wasm32"))]
pub fn mount_browser_with_category_actions(
    root: web_sys::Element,
    scene: Scene,
    mut on_category: impl FnMut(String) + 'static,
) -> kartis::Result<kartis::browser::BrowserChart> {
    use kartis::browser::{BrowserChart, BrowserEvent, BrowserOptions};
    let host = BrowserChart::mount(
        root,
        scene,
        BrowserOptions {
            legend_actions: true,
            ..Default::default()
        },
    )?;
    host.on_event(move |event| {
        if let BrowserEvent::LegendActivated(intent) = event
            && intent.role == LegendRole::Category
        {
            on_category(intent.item.id);
        }
    });
    Ok(host)
}

#[cfg(feature = "raster")]
pub fn mount_raster_with_category_actions(
    scope: &glory::Scope,
    scene: Scene,
    fonts: render::raster::FontRegistry,
    mut on_category: impl FnMut(String) + 'static,
) -> kartis::Result<RasterSurface> {
    let host = RasterSurface::mount(
        scope,
        scene,
        fonts,
        RasterSurfaceOptions {
            legend_actions: true,
            ..Default::default()
        },
    )?;
    host.on_legend_activate(move |intent| {
        if intent.role == LegendRole::Category {
            on_category(intent.item.id);
        }
    })?;
    Ok(host)
}
```

相关协议见[独立端点](endpoint-point-bounds.zh-CN.md)、[类型化 Bar 工厂](typed-labels.zh-CN.md)及[功能范围](feature-gaps.md)。本指南说明 API 行为，不宣称浏览器、宿主运行时或全量测试已完成验收。
