# BoxPlot 配色与受控悬停

[English](boxplot-color-and-hover.md)

`BoxPlotLayoutOptions::color_by` 可选择 `BoxPlotColorBy::SubGroup`（保持原默认值）或 `Group`。调色板位置按完整配置域分配，包含预留与隐藏槽位。Group 颜色也用于继承描边与默认离群点填充，不改变子组的系列 ID 或 `hidden` 的含义。

Group 图例只包含经过隐藏系列过滤后实际存在且有有效样本的组摘要，按完整组域排序；悬停透明度为零不会移除图例身份。ID 保留原始组字符串，角色为 `LegendRole::Category`；激活它不会隐式隐藏系列。既有 SubGroup 图例继续保留完整配置域。本接口尚不提供 Nivo 的任意属性访问器或 `legendLabel` 回调。

```rust
use kartis::*;

fn main() -> Result<()> {
    let config = ChartConfig::default();
    let data: Vec<_> = ["North", "South"].into_iter().flat_map(|group| {
        ["Actual", "Plan"].into_iter().flat_map(move |subgroup| {
            [2., 4., 6., 8.].into_iter().enumerate().map(move |(i, value)| {
                BoxObservation::new(format!("{group}/{subgroup}/{i}"), group, subgroup, Some(value))
            })
        })
    }).collect();
    let options = BoxPlotLayoutOptions {
        color_by: BoxPlotColorBy::Group,
        ..Default::default()
    };
    let mut controller = BoxPlotInteraction::new(
        &config, &data, &options, &BoxPlotInteractionOptions::default(),
    )?;
    let baseline = controller.layout.scene.clone();
    controller.set_active(Some(BoxPlotKey::new("North", "Actual")))?;
    assert_eq!(controller.active_keys().len(), 3);
    let _svg = render::to_svg(&controller.layout.scene)?;
    let _binding = controller.scene_interaction()?;
    controller.set_active(None)?;
    assert_eq!(controller.layout.scene, baseline);
    Ok(())
}
```

激活一个摘要时，相同组**或**相同子组的所有摘要都会高亮，与按哪一维取色无关。默认因子为 `opacity = 1`、`active_opacity = 1`、`inactive_opacity = 0.25`。因子只对整份箱体组合应用一次，包含箱边、中位线、须、端帽和实际绘出的离群点。原填充透明度与角色 RGBA 保持独立。SVG 使用父分组，Canvas 和 CPU 位图使用有界透明层，Glory 语义图表使用 CSS 父分组。

将当前实际绘制的来源 mark ID 传给 `set_hovered_mark`，或用 `key_for_mark` 将箱体、箱边、中位线、须、端帽和离群点解析回原始身份。setter 对未知或过期身份原子拒绝；`key_for_mark` 和 `outlier_for_mark` 对未知 ID 返回 `Ok(None)`。常量摘要保留逻辑中心锚点，不虚构指针命中图形。零透明度组不参与几何拾取。`replace_source` 从观测值重新计算，只有新源仍存在原身份时才保留激活状态。直接修改控制器公开布局后，后续受控操作会拒绝；应恢复已接纳帧或从源数据重建。

将 `scene_interaction()` 传入 BrowserChart 或 RasterSurface 的布局接口。语义 Chart 则接收 `controller.layout.scene`，通常通过 `Cage<Scene>` 更新。应用接收悬停事件，调用控制器发布下一帧。设置 `Chart::highlight(false)`、`BrowserHighlight::Controlled` 或 `RasterSurfaceOptions { highlight: false, ..Default::default() }`，避免叠加宿主原有的单 mark 弱化。浏览器即时更新请选择 `BrowserOptions::motion = MotionPreference::Reduced`。本 API 不自动注册设备悬停处理器。

绑定为每份摘要提供一个箱体逻辑站点，标准 Browser/Raster 几何悬停解析这些箱体站点。应用可用控制器的 `hit_test`／`key_for_mark` 和受控 setter 路由须、端帽、箱边与离群点的命中。分组结构变化可重建语义 Chart 的 mark 节点，来源 mark ID 保持稳定；保留式 SVG 则在父分组之间移动既有路径节点。

`MarkOpacityGroup` 为扁平、不重叠、按实际绘制顺序连续的 mark 集合，文字独立。Canvas 和 CPU 位图暂时拒绝组内成员自身还需要艺术混合隔离层或填充加描边透明度隔离层的分数组；BoxPlot 分别绘出的部件属于已支持路径。弹簧／属性拓扑动画拒绝含组帧；可直接替换，或对兼容成员关系使用 `Scene::interpolate`。这些边界不改变普通无组帧。

[原函数参考](../tests/fixtures/box-colorby-hover/README.zh-CN.md) 保存 Nivo 实际调色板预热、图例计算与激活 key 的执行结果。Rust 继续忽略 nullable 观测值，而记录中的上游数值强制转换可能产生零值样本。上游数字图例 ID、任意访问器和过期 active 对象不宣称与 Rust 输入等价。
