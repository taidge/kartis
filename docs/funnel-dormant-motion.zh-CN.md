# Funnel 休眠动画状态（Kartis 0.2.0）

[English](funnel-dormant-motion.md) · [动画指南](funnel-motion.zh-CN.md)

每个 source part 拥有自己的 body 与 border Spring，即使角色暂时没有墨迹也保留。
零边框宽度、零边框 opacity 和完全收拢的 body 不会丢弃状态。休眠路径使用构造时
捕获的几何，继续接收目标，并与可见路径使用同一时钟推进；它们同样参与
`is_settled`、`finish`、减弱动画及角色／几何预算。删除 source key 才会丢弃状态，
重新加入该 key 时从新目标开始。

## 查询没有 Mark 的角色

```rust
use kartis::*;

pub fn dormant_border() -> Result<f64> {
    let config = ChartConfig::default();
    let source = [FunnelDatum::new("a", 8.)];
    let mut options = FunnelLayoutOptions { border_width: 0., ..Default::default() };
    let first = FunnelSvgFrame::new(&config, &source, &options)?;
    let mut motion = FunnelMotion::new(first, FunnelMotionOptions::default())?;
    options.border_width = 6.;
    motion.retarget(FunnelSvgFrame::new(&config, &source, &options)?)?;
    motion.tick(0.05)?;
    Ok(motion.frame().part_property(0, FunnelMotionPartRole::Border,
        FunnelMotionMarkChannel::StrokeWidth)?.unwrap().value)
}
```

默认 Spring 在这里采样得到约 `0.8365095561160942`。`part_property`、
`part_path_progress` 和 `part_geometry_states` 使用当前 source 序号，不要求实际
Mark 存在。原有 `mark_property` 与 `path_progress` 查询仍描述实际 Mark 槽位。
这些查询都不会新增墨迹、逻辑站点、提示选择或输入目标。

## 为每个来源捕获自定义几何

原有 stage-factory 构造器只对原本可绘制的 body 调用应用函数。一个从未执行过的
函数无法提供自定义的收拢目标；对这种未知自定义 body 启用动画会在采用前返回错误。
需要支持这些来源的动画时，使用显式的全来源捕获：

```rust
use kartis::*;

pub fn custom_target(source: &[FunnelDatum]) -> Result<FunnelSvgFrame> {
    let config = ChartConfig::default();
    let frame = FunnelSvgFrame::new(&config, source, &FunnelLayoutOptions::default())?;
    frame.with_all_stage_factory(&mut |context| {
        Ok(context.inherit(Geometry::Circle {
            x: context.part.center[0], y: context.part.center[1], radius: 12.,
        }))
    })
}
```

此操作对每个 source part 调用工厂一次，包括没有 body Mark 的来源。无墨迹结果
仅作为动画描述符保留，不产生绘制或交互；不会重放尺寸／样式工厂。应在绑定注解前
应用，因为该操作针对注解来源 cohort。对于拥有应用数据的 Frame，使用
`data.with_computed(data.computed().with_all_stage_factory(&mut factory)?)`
可保留同一个原始来源池；动画模型不保留回调。

`part_geometry_origin` 区分内置、已执行 stage-factory 与旧构造器未调用的捕获。
存在实际编辑后的 Mark 时，使用该 Mark 的真实几何；自定义角色缺席时使用已捕获的
几何，不推测默认轮廓。非法几何／绘制样式或超出潜在工作预算会拒绝候选。应用回调
的外部副作用不回滚。

纯采样 Scene 可交给 SVG、Canvas 和 raster；专用浏览器动画 owner 为 SVG。
GPU／worker 继续后置。运行验收单独记录在[验收说明](verification.md)中。
