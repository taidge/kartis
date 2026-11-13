# 整体部件指针拾取

[English](whole-part-pointer.md)

通过 `BoxPlotInteraction::whole_parts_scene_interaction()` 显式选择 `PickingProfile::PaintedGeometry`。每个实际计算出的箱体仍只有一个逻辑主体站点，已输出的边框、中位线、须线、端帽与离群点都映射到这份业务身份。原 `scene_interaction()` 与 `chart(scene)` 入口保留原有主体站点行为。

`SurfacePhysicalHit` 将 `physical_mark_id` 与 `logical_site_id` 分开。可选的 `source_data` 拥有原始离群点 ID 和数值的快照。逻辑站点的数值是箱体中位数，不能替代离群点原值。不要从动画中的 `Mark.value`、ID 后缀或之后的 controller 查询重建源数据。

下面的完整程序运行与渲染器无关的 Surface 协议，并在 Glory 内存宿主挂载原有语义 Graphic 图表。启用 `raster` 后，还会在内存宿主挂载 CPU 图像和控件。这些操作不模拟物理设备，也不启动原生窗口。

```rust
use kartis::{
    surface::{SurfaceEvent, SurfaceInput, SurfaceModel, SurfaceOptions},
    *,
};
use glory::render::{GraphicStroke, StrokeCap};
use std::{cell::RefCell, rc::Rc};

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let config = ChartConfig {
        width: 480.,
        height: 300.,
        margin: Margin {
            left: 40.,
            right: 40.,
            top: 24.,
            bottom: 24.,
        },
        ..Default::default()
    };
    let mut data: Vec<_> = [0., 2., 4., 6., 8., 10.]
        .into_iter()
        .enumerate()
        .map(|(i, value)| BoxObservation::new(format!("sample-{i}"), "A", "x", Some(value)))
        .collect();
    let stroke = GraphicStroke {
        color: Color(20, 40, 60, 255),
        width: 6.,
        cap: StrokeCap::Butt,
    };
    let options = BoxPlotLayoutOptions {
        domain: Some([-2., 12.]),
        border: Some(stroke),
        median: stroke,
        whisker: stroke,
        cap_size: 0.5,
        outlier_radius: 5.,
        legend: false,
        value_axis: AxisOptions {
            visible: false,
            grid: false,
            ..Default::default()
        },
        group_axis: CategoryAxisOptions {
            visible: false,
            grid: false,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut controller = BoxPlotInteraction::new(
        &config,
        &data,
        &options,
        &BoxPlotInteractionOptions::default(),
    )?;
    let frame = controller.whole_parts_scene_interaction()?;
    let binding = frame.interaction.as_ref().expect("whole-parts binding");
    assert_eq!(binding.picking_profile, PickingProfile::PaintedGeometry);
    assert_eq!(binding.sites.len(), 1); // Parts do not become extra datums.
    let body_id = binding.sites[0].mark_id.clone();
    let physical_id = binding
        .part_data
        .iter()
        .find(|(_, datum)| datum.source_id == "sample-0")
        .expect("the original zero observation is an emitted outlier")
        .0
        .clone();
    let part = frame
        .scene
        .marks
        .iter()
        .find(|mark| mark.id == physical_id)
        .unwrap();
    let point = match part.geometry {
        Geometry::Circle { x, y, .. } => (x, y),
        _ => return Err(ChartError("expected the configured circle outlier".into()).into()),
    };
    let mut surface = SurfaceModel::new_with_interaction(
        frame.scene.clone(),
        SurfaceOptions {
            plot: frame.plot,
            zoom: false,
            pan: false,
            pinch: false,
            brush: false,
            ..Default::default()
        },
        frame.interaction.clone(),
    )?;
    let events = surface.apply(SurfaceInput::Move { id: 1, point })?;
    let old_hit = surface
        .hovered_physical_hit()
        .cloned()
        .expect("actual outlier ink");
    assert!(matches!(
        events.last(),
        Some(SurfaceEvent::PhysicalHover(Some(_)))
    ));
    assert_eq!(old_hit.physical_mark_id, physical_id);
    assert_eq!(old_hit.logical_site_id, body_id);
    assert_eq!(surface.hovered(), std::slice::from_ref(&body_id));
    assert_eq!(surface.physical_hit_at(point)?.as_ref(), Some(&old_hit));
    let original = old_hit
        .source_data
        .as_ref()
        .expect("accepted original source");
    assert_eq!(original.source_id, "sample-0");
    assert_eq!(original.value, 0.); // Never inferred from a live Mark.value.

    // Direct Surface input: contact lifecycle and explicit terminal cleanup.
    surface.apply(SurfaceInput::TouchStart { id: 2, point })?;
    surface.apply(SurfaceInput::TouchMove { id: 2, point })?;
    surface.apply(SurfaceInput::TouchEnd { id: 2, point })?;
    surface.apply(SurfaceInput::Leave)?;
    assert_eq!(surface.touch_count(), 0);
    assert!(!surface.dragging());
    assert!(surface.hovered().is_empty());
    assert!(surface.hovered_physical_hit().is_none());
    assert_eq!(surface.selected(), Some(body_id.as_str()));
    surface.apply(SurfaceInput::TouchStart { id: 3, point })?;
    surface.apply(SurfaceInput::TouchCancel { id: 3 })?;
    surface.apply(SurfaceInput::Leave)?;
    assert_eq!(surface.touch_count(), 0);

    data[0].value = Some(-1.);
    controller.replace_source(&config, &data, &options)?;
    let next = controller.whole_parts_scene_interaction()?;
    assert_eq!(
        next.interaction.as_ref().unwrap().part_data[&physical_id].value,
        -1.
    );
    surface.set_layout(next.clone())?;
    assert!(!surface.is_current_hit(&old_hit));
    assert_eq!(old_hit.source_data.as_ref().unwrap().value, 0.);
    let token = surface.frame_token();
    let mut invalid = next.clone();
    invalid
        .interaction
        .as_mut()
        .unwrap()
        .ink_owners
        .insert(physical_id, "unknown-owner".into());
    assert!(surface.set_layout(invalid).is_err());
    assert_eq!(surface.frame_token(), token);

    // This is a real semantic Graphic chart, with one atomic layout Cage.
    let app = glory::App::memory();
    let layout = app.scope().cage(next.clone());
    let observed = Rc::new(RefCell::new(Vec::<Option<SurfacePhysicalHit>>::new()));
    let pending = observed.clone();
    app.mount(
        chart_layout(layout)
            .highlight(false)
            .data_control_pointer(false)
            .on_physical_hover(move |hit| pending.borrow_mut().push(hit)),
    )?;
    app.flush()?;
    // Applications drain accepted hits outside the callback, update the
    // controller if the active owner changed, then publish a complete layout.

    #[cfg(feature = "raster")]
    {
        let raster_app = glory::App::memory();
        let raster = RasterSurface::mount_layout(
            raster_app.scope(),
            next,
            render::raster::FontRegistry::new(),
            RasterSurfaceOptions {
                highlight: false,
                ..Default::default()
            },
        )?;
        raster_app.mount(raster.widget())?;
        raster_app.flush()?;
        assert_eq!(raster.handle().frame()?.width, 480);
    }
    Ok(())
}
```

`Move` 使用视口逻辑坐标。`physical_hit_at` 是纯查询，不会制造悬停事件。严格拾取检查实际拥有的实色 RGBA 填充或描边、线宽、端点、虚线段、裁剪、隐藏系列和组透明度。主体透明时，只要中位线或其他归属部件仍有实际墨迹，逻辑站点就可以保留。整组完全透明时没有物理命中，但仍可保留唯一的逻辑键盘控件。程序不会新增透明命中 mark 或伪造矩形区域。

此 profile 只接纳 `Picking::Geometry`，拒绝最近点、切片与极坐标交互。首版实色拾取不支持参与物理拾取的 mark 使用渐变或图案填充。曲线描边使用有界展平，裁剪路径也遵循自身受检近似范围；这不是对任意矢量路径的解析精确拾取承诺。

成功替换源、隐藏状态或视图后，旧命中的 token 失效。被拒绝的候选保留已接受帧和 token。即使新源复用同一物理 ID，旧命中中的 `source_data` 仍是原先不可变的快照。`SurfaceModel::is_current_hit` 检查对应 model 的接受 token。应用替换自己的源时，也应清理排队中的旧观察，避免把旧的同 ID 命中套到新 controller 源上。

语义图表使用一个 `Cage<SceneInteraction>` 传给 `chart_layout`，让 Scene 与 binding 一起发布。此入口绘制原有 Graphic 树，并要求显式的严格绑定。逻辑数据按钮禁用指针事件，但保留键盘和焦点能力；只供控件使用的 Scene 投影不会用于实际绘制。`Chart::data_control_pointer` 也接受动态 `Vary<bool>`，供会切换 profile 的适配器使用；旧入口默认仍为 true。`on_physical_hover` 会观察实际部件，涵盖同一箱体内部两个部件之间的切换。回调应只排队保存不可变观察，然后在回调之外、激活 owner 真正变化时调用 `set_hovered_mark` 并发布完整布局。物理事件位于同次操作的逻辑事件之后。

BrowserChart 的 layout API 接受相同布局，在 `BrowserEvent::Interaction` 中报告 `SurfaceEvent::PhysicalHover`。RasterSurface 使用 `mount_layout`／`set_layout` 与现有 `on_event` 回调。Raster 坐标来自实际图像宿主 View，不包括单独的控件区域。图像、控件、binding 和回调源在校验及图像发布成功后一起接受。严格 Raster 与 `editable_brush` 的组合明确不支持，在发布前拒绝；原有 legacy brush 路径保持原行为。

Glory 坐标通道保留 View 内的分数坐标以及同一次观察的视口尺寸，适配器只映射一次到 Scene 尺寸，不进行 DPR 取整。鼠标与笔使用 move/down/up，触摸使用接触 start/move/end/cancel，并在终止时清理，避免留下鼠标式的持久 hover。直接使用 Surface 的示例显式调用终止 `Leave`。键盘激活仍使用逻辑站点。BoxPlot controller 负责关联透明度时，应用应选择 `highlight(false)`、`BrowserHighlight::Controlled` 或 raster 的 `highlight: false`；即时更新可选择浏览器 reduced motion。

交互宿主必须提供有限 `PlotPointer` 能力。静态输出只记录事件兴趣，不会制造输入。Web 通道支持正向、与坐标轴平行的二维缩放和平移；旋转、倾斜、透视或不支持的变换链会拒绝对应观察。Apace 通道目前拒绝具有任何 computed transform 祖先的观察。适配器不会猜测逆变换。owner 销毁后，通道和回调一并退役；更新源不会让已分离的旧节点变成新数据。

本指南说明有限指针输入与受控关联，不宣称通过物理设备认证、支持任意平台变换、合成器截图确定性，或提供 GPU／worker 执行。GPU 与 worker 工作继续后置。
