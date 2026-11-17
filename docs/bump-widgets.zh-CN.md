# Bump 实时点组件

此 API 自 Kartis 0.2.0 提供。`bump_point_widgets` 将
`BumpLayout` 中实际存在的点 Mark 接入测量式、类型化 Glory 实时组件，可组合普通点、
自定义 Path、首尾文字工厂，以及既有 SVG、Canvas 和 CPU 图像宿主。

定位回调借用当前 layout、computed series、原 series/datum、计算 point 与实际 Mark。
返回数据每个选中系列仅共享一份 `Arc<BumpSeries>`，保留原索引、类型化 X、可空 rank、
映射坐标、点代表色和 plot。数字 `1` 与字符串 `"1"` 不混淆，原数值符号位保留。
当前几何和画笔从接受帧 Mark 读取，与原始数据分别提供。

首次回调前会预检整帧：源/计算数据、PointScale 域/range、映射位置、折线分段、点身份
和实际 sites/anchors 必须一致。隐藏系列和缺失点不造角色；真实零尺寸、透明 Mark
仍有组件角色。返回 `None` 可跳过组件，回调错误不返回候选包；回调自身外部副作用
不具备事务性。公开 `BumpLayout` 结构保持不变，校验当前帧自洽性，不声称具有私有历史
来源证明。完全一致的公开帧编辑可以接受；不能从布局反推出原 style/accessor 或点工厂。

`AtDatum` 使用原始数据锚点，自定义 Path 的墨迹偏移不会改变它。`bump_widget_frame`
保留准确 plot（含空布局），并显式选择实际绘制拾取，不改变旧 `into_scene_interaction`
行为。拥有所有权的系列快照不随原输入之后的修改变化。点代表色、系列线色及实际画笔
含义不同，不能互相反推。

```rust
//! Live Glory components for Bump ranking points.
use glory::{App, Widget};
use kartis::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let series = [BumpSeries::new(
        "north",
        vec![
            BumpDatum::new(1_f64, 2.),
            BumpDatum::gap("middle"),
            BumpDatum::new("tail", 0.),
        ],
    )];
    let layout = bump_layout(
        &ChartConfig::default(),
        &series,
        &BumpLayoutOptions::default(),
    )?;
    let widgets = bump_point_widgets(&layout, 0, |_| {
        Ok(Some(PlotWidgetPlacement::AtDatum {
            offset: [-20., -10.],
            measured: TextSize {
                width: 40.,
                height: 20.,
            },
        }))
    })?;
    let replace_ink = widgets
        .layout
        .slots
        .iter()
        .map(|slot| slot.mark_id.clone())
        .collect();
    let frame = bump_widget_frame(&layout)?;
    let app = App::memory();
    app.mount(chart_layout(frame).plot_widgets_composed(
        PlotWidgetsComposition {
            widgets,
            replace_ink,
        },
        |context: PlotWidgetCompositionContext<BumpPointWidgetData>| {
            let frame = context.frame;
            Ok(glory::ui::widget(move |scope| {
                let count = scope.cage(0u32);
                glory::ui::button()
                    .label(scope.bond(move || {
                        frame.with(|f| {
                            format!(
                                "{}: {:?}, rank {:?} ({} clicks)",
                                f.data.series.label,
                                f.data.point.raw_x,
                                f.data.point.rank,
                                count.get()
                            )
                        })
                    }))
                    .on(glory::render::EventKind::Activate, move |_| {
                        count.revise(|v| *v += 1)
                    })
                    .show_in(scope)
            }))
        },
    ))?;
    app.flush()?;
    app.clear()?;
    Ok(())
}
```

Nivo 的正常 compute 流程在创建 `pointComponent` 前过滤 null-rank 点，保留原始源索引，
折线单独保留缺口；零值、size0 点仍创建组件。默认 Point 只读取位置、size/color 和
border，以 React Spring 动画过渡，将负动画半径夹到零，并使用 pointer-events:none；
业务输入由 line/mesh 图层负责。Kartis 使用稳定的类型化类别业务 ID，而非 Nivo 的
系列加源索引 ID。Glory 子组件可以拥有自身输入，Rust 的受控 Mark opacity 与上游默认
Point 行为分别描述。本适配不声称移植 React reconciliation、SpringValues 或自动弹簧。

参见[完整应用](../examples/bump_widgets.rs)与共享宿主模型。替换、revision、拒绝与作用域
清理由[事务组合](widget-composition.zh-CN.md)负责；浏览器[按 Scene 顺序组合](ordered-widgets.zh-CN.md)提供绘制交错；也可使用 `plot_widgets_with_data` 纯覆盖层。
Scene 导出包含图表几何和标签，不包含任意实时 Widget 快照。原生 OS 输入、自动 spring
及 GPU/worker 继续保留。浏览器纯色夹具使用保留 JSON 数据的真实 font0 Button，仅验证
角色、布局、输入，不声称任意字形严格像素等价。

专项 SVG/Canvas 矩阵覆盖四浏览器 DPR1/2：192 帧、11,856 项硬颜色采样、真实子组件/
来源输入、revision 和拒绝回滚；四项独立变异负控均命中预期门禁。安装项目锁定的 Node
依赖后可运行便携检查：

```sh
node scripts/bump-point-widget-reference.mjs --frozen
node scripts/bump-widgets-acceptance.mjs --chromium-only
```

具体范围和保留的失败见[验收记录](verification.md#development-bump-live-point-components)，
不能据此声明完整图族目录全部通过。
