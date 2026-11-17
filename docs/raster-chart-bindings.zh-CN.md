# CPU 图像 Chart 的类型化输入

此 API 属于 0.1.0 之后的开发版。Chart 绘制 CPU PNG 时，可以保留完整的
`SceneInteraction`：

```rust,ignore
chart_layout(frame)
    .raster(fonts, options)
    .on_physical_hover(|hit| { /* 实际墨迹身份与来源数据 */ });
```

输入帧必须有显式 `PickingProfile::PaintedGeometry` 绑定。规范业务值，包括
`None` 与 `Some(0.0)`，继续与画笔标量、物理别名分离。已有类型化覆盖层组件及
`plot_widgets_composed` 共同受图像发布事务约束。发布器返回 `Err` 时，旧 PNG、
绑定、相机、控制与组件已接受数据均保留。之后发生的异步传输或图片解码失败属于
宿主观察，不能撤销已接受的模型。

Glory 在包住 Image 的物理 View 上接收有限图表指针输入。坐标先归一到图像逻辑尺寸，
拾取时只逆变换一次已接受的 `RasterOptions.view`。DPR 影响 PNG 分辨率，不改变输入
坐标。拾取同时检查图像视口与原始源 plot。点击移动量在相机逆变换前计算。相机变化
取消进行中的手势、更新物理命中帧身份，并重新拾取静止指针。

经过测量的提示框使用类型化锚点，并应用已接受的相机。组件仍是真实的 Glory 子组件，
自行接收输入；不会将图像像素转换成模拟组件点击。这继续使用已有覆盖层或严格替换
组合，不支持在 PNG 内嵌入任意实时子组件。

仅传 Scene 的 `raster_chart` 不推断类型化绑定。`RasterSurface` 保留独立的模型与输入
API。真实输入需要交互宿主支持：Glory Web 支持正向轴对齐缩放和平移，原生图表输入
宿主拒绝视口祖先上的变换。原生应用必须显式提供图片加载与来源发布。Memory 人工
派发测试只证明端口协议，不等于操作系统输入验收。自动弹簧、任意实时组件导出及
GPU/worker 渲染仍是独立任务。

[English](raster-chart-bindings.md)
