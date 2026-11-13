# 可编辑框选

[English](editable-brush.md)

`EditableBrush` 以视口逻辑像素保存持续选区。`EditableBrushOptions` 指定 X、Y、
XY 模式、明确的规范化范围和手柄大小。默认范围为单位矩形，应用应配置实际绘图区。
X/Y 模式用范围填满非活动维。`selection()` 区分无选区和创建过程中的临时零跨度矩形。

拖动覆盖层创建选区，拖动选区移动，八个边角手柄调整相应边界；越过对边后方向翻转。
Alt 居中调整，双轴活动时 Shift 锁定主方向，Meta 强制新建，Space 临时移动当前选区。
Ctrl 过滤新手势，也可关闭修饰键策略。

Begin 后从 `session()` 取得会话。Move、End、Cancel 和修饰键更新都携带该会话，
指针操作同时携带 pointer ID。程序设置、重新配置、清空或新控制器不会接纳旧会话。
键盘控件通过受检 `Nudge`、`Clear` 发出用户操作，无需伪造指针；显式取消恢复 Begin
前的选区。

通知区分 Start、Change、End、Cancel，包含用户/程序来源、模式、选区和会话。
程序更新在接受新值前退休活动基准；空闲时重复值不通知，回调提交相同值不会形成循环。
非法坐标、手柄或配置保留原状态。

`SurfaceModel::configure_editable_brush` 独立启用编辑功能。未启用时保留旧 Shift 框选；
启用后由编辑器接管范围内的新建，范围外仍是普通平移/点击，不创建不可见的第二个框选。
范围必须位于当前绘图区。`SurfaceEvent::EditableBrush` 返回视口选区和当前数据 ID，
使用已接受的类型化锚点、裁剪、显隐和一次场景投影；自定义 Path 不改用包围盒中心。
`editable_brush_viewport_request()` 减去一次绘图区原点、选择活动轴，生成局部像素的数据域
请求，不再逆变换相机；浏览器 handle 可通过 `request_editable_brush_viewport()` 排队提交。

`BrushReconfigure::Preserve` 拒绝超出新范围的选区，`Clamp` 约束选区，`Clear` 清空。
绘图区缩小或移动时，原像素范围被约束并退休受影响手势；扩大不会缩放旧像素或自动扩展
范围，应显式重新配置。仅在绘图区和尺寸不变时，重排才可保留会话。场景替换与 reset
会取消手势。
分组显隐实际变化也会在后续 pointer-up 之前取消活动会话；重复相同显隐值不会取消。

浏览器 SVG/Canvas 通过 `BrowserOptions::editable_brush` 及 handle 的配置/选区方法使用。
位图宿主共用 Surface 控制器，并显式提供输入边界。捕获、焦点、键盘控件、发布和清理
由宿主接线负责，纯 Rust 状态机不安装监听器。

Glory 位图桥接提供有名称的拖动捕获区域，以及可用 Tab/Enter 操作的目标选择、微调、
重置和清空按钮。原生事件源若提供原始修饰键，可通过 `EditableBrushInput::Modifiers`
显式转发；浏览器适配器安装相应监听器。原生图像物理边界在拖动中改变时，取消该手势，
不继续使用旧偏移。

可恢复的浏览器 DOM 写入失败时，还原模型并重新发布完整旧 DOM，恢复最后已接受的
选区与绘制。仍按下的触点可以保留会话；物理上
已经结束的触点则退休会话并保留已接受选区。显式 Escape 或所有者取消仍恢复 Begin
前的选区。
宿主若同时阻止恢复写入，适配器报告宿主错误；持续失败的 DOM API 不保证物理画面恢复。
此恢复范围是适配器拥有的图表与框选 DOM；用户 Plot Widget 工厂及外部副作用仍遵守
各自的错误和所有权约定。

几何和修饰键规则参考原版 d3-brush 3.0.0。受限 Rust 模式明确保留几处差异：空闲重复
更新无通知、旧会话退休、单轴非活动维允许零跨度。Alt 裁剪后切换 Space 时，保持实际
当前尺寸；原版 D3 的独立边界案例会令 X 选区超出范围。普通等价轨迹和该安全差异分别
留证。原版 JavaScript oracle 和完整许可仅供开发验收，不是图表运行时依赖。
详见[验收记录](verification.md)和[第三方许可](third-party-notices.md)。
