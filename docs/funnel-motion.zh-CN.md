# Funnel 逐属性 Spring（Kartis 0.2.0）

[English](funnel-motion.md)

Funnel 动画区分已接受的业务来源与正在显示的 Scene。`FunnelMotion` 为 SVG、Canvas
和 raster 提供显式时钟；专用 SVG 浏览器 owner 另提供自动 RAF 与保留式 Glory 组件
订阅。既有入口保持即时显示，启用动画后才使用 Spring。

## 纯模型与绘制

```rust
use kartis::*;

pub fn sample() -> Result<String> {
    let config = ChartConfig::default();
    let options = FunnelLayoutOptions::default();
    let first = FunnelSvgFrame::new(&config,
        &[FunnelDatum::new("a", 8.), FunnelDatum::new("b", 4.)], &options)?;
    let next = FunnelSvgFrame::new(&config,
        &[FunnelDatum::new("a", 5.), FunnelDatum::new("b", 7.)], &options)?;
    let mut motion = FunnelMotion::new(first, FunnelMotionOptions::default())?;
    motion.retarget(next)?;
    motion.tick(0.08)?;
    render::to_svg(motion.frame().scene())
}
```

使用 `frame.scene()` 绘制实际采样；`frame.target().scene()` 始终是不可变目标。
既有 `render::draw`、`draw_canvas` 和 raster 接口均可接收采样 Scene，角色与 item
映射保留目标的槽位。Frame 拥有其数据，相等表示同一构造 capture，历史采样可以继续
读取。模型不保留应用工厂，也不单独授予输入资格。`tick` 接受 0..=60 秒内的有限值；
配置、预算、重定向和采样均通过校验后才替换模型。

## 浏览器时钟与组件

```rust
use kartis::*;
use kartis::browser::*;

pub fn mount(root: web_sys::Element) -> Result<BrowserFunnel> {
    let owner = BrowserFunnel::mount_with_svg_layers(root,
        vec![FunnelDatum::new("a", 8.), FunnelDatum::new("b", 4.)],
        ChartConfig::default(), FunnelLayoutOptions::default(),
        BrowserFunnelSvgProgram::defaults())?;
    owner.enable_motion(FunnelMotionOptions::default())?;
    owner.flush()?;
    Ok(owner)
}

pub fn deterministic_step(owner: &BrowserFunnel) -> Result<()> {
    owner.set_motion_clock(BrowserFunnelMotionClock::Manual)?;
    owner.flush()?;
    owner.step_motion(0.016)
}
```

启用后默认使用自动 RAF。Manual 精确采用调用者提供的秒数，并拒绝在 owner 回调
内部同步 step。业务构建增加来源 generation；物理采样增加 publication，且不重放
业务工厂。相机、字面层顺序及隐藏 series 命令保留同一接受来源。隐藏或零尺寸表面
暂停自动时间，恢复时重置墙钟基线，不补跑隐藏时间。
`set_motion_options(None)` 排队提交，在下一次成功 flush／采用时直接显示目标；动画
偏好可跟随系统、减少动画或显式启用。

```rust
use glory::Widget;
use kartis::*;
use kartis::browser::*;

pub fn add_status(program: BrowserFunnelSvgProgram) -> Result<BrowserFunnelSvgProgram> {
    program.custom_animated("status", 0, |context, motion| {
        Ok(glory::ui::widget(move |scope| {
            glory::ui::text().content(scope.bond(move || {
                let source = context.frame.get();
                let pose = motion.get();
                format!("source {}, sample {}",
                    source.as_ref().map_or(0, |f| f.source_generation),
                    pose.as_ref().map_or(0, |f| f.data.sample_serial()))
            })).show_in(scope)?;
            Ok(())
        }))
    })
}
```

使用该 registry 前须加入对应 custom 层 occurrence。既有来源 Bond 与新增动画 Bond
均派生自同一个完整 Cage。保留式提示可使用 `set_tooltip_component_with_motion`；
`set_mouse_handlers_with_motion` 提供原生输入接纳时捕获的物理显示证书。
`BrowserFunnelData<D>` 具有相同能力，D 保留在接受的目标 Frame 中，不增加 trait 要求。

原生输入落在实际显示的 body 路径上。capture 与 bubble 之间 publication 改变时，
旧观察失效；之前的事件数据与动画证书仍保留历史来源。提示选择跟随仍有资格的接受
来源。precommit 候选失败保留旧模型、绘制、组件状态和焦点；应用闭包外部副作用不
回滚，adopt 后后端故障继续使用既有 UI 错误隔离规则。

## Nivo 行为与边界

body 与 border 使用独立的 0→1 Spring 和 D3 字符串插值。中途变更路径时，旧端点为
上一次提交的目标，而非当前显示采样；新路径决定命令拓扑，数值 token 按顺序配对，
没有旧配对的新数值保持目标常量。样式轨道独立保留速度。填充／边框 opacity、格式化
文字和目标边框可见性即时更新。

part 与标签保留类型化业务 key；分隔线区分 before、after 与 terminal。绑定注解
按 binding index 保留：轮廓几何、连接线 progress 和内置 note 位置分别动画。
自定义 note Scene 与无角色的额外绘制采用目标值。新增 source part 直接显示目标，
删除 source part 立即退场；删除后重新加入同一个 key 时建立全新轨道，没有退出过渡。

只要 source key 仍在，body 与 border 轨道就会保留，包括零宽／零 opacity 而省略的
边框，以及真正完全收拢的 body。休眠轨道仍接收目标、推进时钟、判断静止并响应减弱
动画。墨迹恢复时保留样式速度；路径变化从上次提交目标重新开始，包括捕获的收拢
目标。休眠轨道不会新增 Scene Mark 或输入目标。有非零邻居的单个零尺寸阶段不一定
完全收拢。见[休眠角色与自定义捕获](funnel-dormant-motion.zh-CN.md)。

Rust 使用具有显式静止阈值的解析阻尼 Spring，不运行 React Spring。几何和颜色遵守
Scene 限制，自定义构造捕获的实际墨迹直接参与动画，不用默认 profile 重建。
任意可变 D3 generator、逐 item Widget 混排、任意实时 UI 导出与 GPU／worker 继续
单列。受检范围见[验收记录](verification.md)。
