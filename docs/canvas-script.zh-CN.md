# 显式 CanvasScript 绘制表面

`render::to_canvas_script` 和 `render::to_canvas_script_with_blends` 保持现有 Direct 重放。`CanvasScriptOptions` 通过 `to_canvas_script_with_options` 与 `to_canvas_script_with_blends_and_options` 提供显式主线程 Offscreen2D 暂存绘制。它不创建 worker、不转移目标所有权、不安装事件，也不引入 GPU backend。

```rust
use kartis::{render::*, *};

let mut scene = line(
    &ChartConfig::default(),
    &[Series::new("values", vec![Point::new(0., 1.), Point::new(1., 2.)])],
    &LineOptions::default(),
)?;
scene.typography.default.family = Some(vec![FontFamily::System]);
let options = CanvasScriptOptions {
    surface: CanvasScriptSurface::Offscreen2d {
        fallback: CanvasScriptFallback::Error,
    },
    ..Default::default()
};
let script = to_canvas_script_with_options(&scene, "chart", &options)?;
# Ok::<(), kartis::ChartError>(())
```

应用先创建指定 ID 的 HTML canvas，加载字体并建立预期文档布局，再执行返回的同步脚本。示例显式选择作者的 System 字体，该选择独立于绘制表面。Sans 仍为 `sans-serif`，System 仍为 `system-ui`，命名字体仍使用带引号的 CSS 名称；没有替换字体族。Offscreen context 没有 DOM 语言祖先，generic family、locale 和回退字体的解析可能与 HTML Canvas 或 SVG 不同。该选项不能解决全部既有 default-Sans 对照问题，也不承诺所有平台字体相同。

`Offscreen2d` 先在暂存 canvas 中画完整 Scene，再将物理像素复制到同一个 HTML canvas。路径、绘制顺序、渐变、pattern、裁剪、旋转标签和独立标记混色复用现有重放，DPR 计算保持一致。如果已存在的目标 2D context 报告非 sRGB 或关闭 alpha，暂存模式会拒绝。`CanvasScriptFallback::Error` 在缺少 OffscreenCanvas、构造失败或无法获得 2D context 时抛出 JavaScript 错误；`Direct` 允许这些可用性故障转用现有 Direct 重放。预算、绘制或发布错误不会静默切换表面。

`max_extra_buffer_pixels` 用于 Offscreen 模式的资源计划，包括可用性回退：构造器存在时的一个完整暂存帧、全部不同的 pattern tile，以及共享 blend scratch 的最大尺寸。它不计算目标 canvas。受检范围为 1..=134217728 像素，默认 67108864；每个 pattern、工作量和 blend 的既有限制也继续生效。像素预算统计 backing pixels，不等于经证明的进程内存占用，浏览器开销与像素格式另计。Direct 模式保持既有预算与输出。

累计准入先于额外 buffer 分配和目标 resize。创建出的暂存对象会在成功、绘制失败、发布失败和 2D context 获取失败时显式清零尺寸；既有 pattern 与 blend scratch 生命周期保持不变。这是暂存绘制，不是 BrowserChart 的事务协议：发布前拒绝可保留旧 bitmap，但发布会先 resize 可见目标，再调用 `drawImage`；若该调用失败，旧 bitmap 可能已经清空。不承诺发布失败时回滚或恢复旧帧。Direct 回退沿用既有 Direct 行为。

能力检查必须显式记录。不支持 Offscreen2D 的引擎可验收 Error 或 Direct 回退，该结果不能算作实际 Offscreen2D 绘制。帧对照和字体 profile 应记录实际采用的路径。
