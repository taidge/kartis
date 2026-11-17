# 严格实时 Widget 组合

[English](widget-composition.md)

这是开发中的 API，未包含在已发布的 0.1.0 包中。

`Chart::plot_widgets_composed` 挂载真实 Glory Widget，并可替换指定 Scene 标记的墨迹。
它是**终结构建方法**：先配置回调、控件与栅格发布器，最后再调用它。
已有 `plot_widgets` 和 `plot_widgets_with_data` 仍是叠加 API，保留原来的子控件修订局部失败策略。

## 源数据、Widget 与绘制共用一个候选

`PlotWidgetsComposition<T>` 包含 `widgets: PlotWidgetsData<T>` 与 `replace_ink: BTreeSet<String>`。
每个替换 ID 都必须有测量插槽，元数据键必须恰好对应唯一的插槽 ID。
128 个插槽上限、真实源标记要求，以及原有位置和裁剪验证仍然适用。
空数据包也保留 `T`；浏览器挂载会拒绝不同元数据类型，以及叠加和严格组合模式之间的切换。

以下完整示例用真实 Button 替换 Bullet measure 的墨迹，同时保留 range、marker、行标签与原业务数据。
Scene 和组合包从同一拥有数据的受控源派生。

```rust
use std::collections::BTreeSet;

use glory::render::EventKind;
use glory::{App, Widget};
use kartis::*;

fn proposal(value: f64)
    -> Result<(Scene, PlotWidgetsComposition<BulletPartWidgetData>)>
{
    let rows = [BulletDatum {
        id: "Revenue".into(), ranges: vec![30., 70., 100.],
        measures: vec![value], markers: vec![80.],
    }];
    let layout = bullet_layout(
        &ChartConfig { width: 600., height: 200., ..Default::default() },
        &rows,
        &BulletLayoutOptions {
            min: Some(0.), max: Some(100.), legend: false,
            ..Default::default()
        },
    )?;
    let widgets = bullet_part_widgets(&layout, 0, |context| {
        if context.part.kind != BulletPartKind::Measure { return Ok(None); }
        let Geometry::Rect { x, y, width, height } = &context.part.geometry
            else { return Err(ChartError("expected a measure rectangle".into())); };
        Ok(Some(PlotWidgetPlacement::Rect(LayoutRect {
            x: *x, y: *y, width: *width, height: *height,
        })))
    })?;
    let replace_ink: BTreeSet<_> = widgets.layout.slots.iter()
        .map(|slot| slot.mark_id.clone()).collect();
    Ok((layout.scene, PlotWidgetsComposition { widgets, replace_ink }))
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let app = App::memory();
    let input = app.scope().cage(proposal(60.)?);
    let scene = app.scope().bond(move || input.with(|p| p.0.clone()));
    let composition = app.scope().bond(move || input.with(|p| p.1.clone()));
    app.mount(chart(scene).legend(false).plot_widgets_composed(
        composition,
        |context| {
            let frame = context.frame;
            Ok(glory::ui::widget(move |scope| {
                let clicks = scope.cage(0usize);
                glory::ui::button()
                    .label(scope.bond(move || frame.with(|current| format!(
                        "Measure {} / {} clicks",
                        current.data.part.raw_value, clicks.get()
                    ))))
                    .on(EventKind::Activate, move |_| clicks.revise(|n| *n += 1))
                    .show_in(scope)
            }))
        },
    ))?;
    let next = proposal(75.)?;
    input.revise(|p| *p = next);
    app.flush()?;
    app.clear()?;
    Ok(())
}
```

实时读取使用 `context.frame`。其 `TypedPlotWidgetFrame<T>` 包含原始 `Mark`、当前插槽几何和可见性，以及类型化源数据。
新 `PlotWidgetCompositionContext<T>` 在工厂和子控件构建期间是暂定候选：
新控件从创建起隐藏且 inert，随后暂停，直到提交。
提交后其 Bond 跟随已接受的更新；拒绝会销毁该候选作用域和 Bond。
已有控件在暂存期间始终读取旧已接受帧，不会读到被拒绝的候选数据。

ID 与修订号相同则保留子控件及本地状态，数据和几何在提交时更新。
元数据按 `T::PartialEq` 比较，不按分配地址比较。
修改修订号可能替换整个包装节点。
新键工厂失败、已有键新修订的工厂失败、子控件构建失败，都会拒绝**整个严格候选**。
它不会像原叠加修订保留协议那样，让旧子控件接受新源数据。
初始子控件构建也先于首次 PNG 发布器调用完成。

模型、插槽、元数据、构建或同步绘制和发布失败，都会保留旧已接受的 Scene、类型化数据、子控件与底图。
宿主不会回写应用的候选输入源，应修正后重新提出候选。
即使 authored Scene 未变，仅替换集合变化也会重新绘制并更新拾取。
被拒绝的严格修订可以在后续候选中重试。
应用回调在候选作用域之外产生的外部副作用无法回滚。

## 绘制与输入

原始 authored Scene 保持不变。指定物理标记的墨迹从宿主绘制投影中移除，
不会把 `Mark.opacity` 设为零，也不会修改交给 Widget 的源标记。
全部 Scene 标签、坐标轴、标题、图例及规范逻辑控件保留，可见性仍遵循宿主通常的资格规则。
库不会推断每标记的标签归属：替换某一个 measure 不会顺便删除 Bullet 行标题。

指定的原物理几何会从指针拾取中排除，包括最近站点指针查询。
其他物理别名 ID 不会自动被替换。
逻辑数据、程序选择、明确选择的交互辅助及十字线注释继续遵循原有策略。
子 Widget 自行处理指针和键盘动作，不会自动选择底层数据。

源标记和插槽仍存在时，隐藏或完全裁剪会保留子控件状态；部分裁剪插槽可见但 inert。
有效标记或分组透明度为零时，插槽隐藏且 inert，源 `Mark` 不变。
正的分组透明度不会在任意 Widget 内自动复刻分组 alpha、混合或原 Scene 层次；其外观由 Widget 负责。
移除插槽时，应一并删除同键元数据；若也要移除 Scene 标记，应在同一候选提交。

Widget 处于顶层，不复刻 Nivo 同层 React 组件顺序、`animatedProps`、弹簧轨迹或 Scene 分组混合。
Scene 导出不包含实时 Widget。
导出 authored Scene 仍会画出其原墨迹；宿主过滤后的底图 PNG 会移除替换墨迹，但也不包含上方实时 Widget。

## 其他宿主与暂存预算

CPU 图片先配置
`raster_chart(scene, fonts, raster_options).raster_image_source(publisher)`，
最后调用终结方法 `plot_widgets_composed(composition, factory)`，沿用已有图片发布准入。
原生应用提供自己的图片提供器与发布器。
这没有给 `RasterSurface` guard 新增组合 API，实际绘制源仍遵循语义 Graphic 与裁剪限制。

浏览器 SVG 和 Canvas 使用 `BrowserChart::mount_with_plot_widgets_composed`。
`set_plot_widgets_composed` 仅更新组合包；
`set_layout_with_plot_widgets_composed` 一起提交 Scene、交互元数据和组合包。
两个 setter 也存在于 `BrowserHandle`。
它们在接受绘制前暂存子控件，并可拒绝过时、停止或重入的候选。
排队工作在宿主绘制或显式 flush 时应用。

拒绝并保留旧帧的保证适用于提交前。全部类型化行采用后，普通动态子绑定属于提交后的 UI
观察，其错误在已接受帧上报告，不撤销已经观察的数据或外部 effect。浏览器叠加宿主与按
来源顺序的宿主遵循相同边界。

每个宿主在已有已接受控件之外，最多允许跨暂存阶段累计 256 个新建但未提交的子控件。
检查先于额外工厂调用；同修订复用不占新暂存额度，retarget 阶段共用一份预留。
提交或丢弃被拒绝和退役的阶段只释放一次额度。
大量排队新修订之前，应先 flush 或退役已有暂存工作；预算拒绝保留已接受帧。

原叠加协议见[类型化叠加 Widget](typed-plot-widgets.zh-CN.md)，
分角色元数据见 [Bullet Widget](bullet-widgets.zh-CN.md)。

浏览器需要原始 Scene 顺序交错和跨真实组件的一次性组透明度时，使用独立的[按 Scene 顺序组合接口](ordered-widgets.zh-CN.md)。本文顶部层入口保持原有行为。
