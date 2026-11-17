# 准确的视口截图采样

`scripts/acceptance-viewport.mjs` 为浏览器验收提供只读截图与采样：捕获完整配置视口，
记录 surface 真实 DOM 矩形和 DPR，将逻辑 surface 坐标映射到物理 PNG 索引，
最后只进行一次 floor。

```javascript
import {captureViewport,rgbaAtLogical} from '../scripts/acceptance-viewport.mjs';
await surface.scrollIntoViewIfNeeded();
const capture = await captureViewport(page, surface);
const sample = rgbaAtLogical(capture, {
    point: [120.5, 80.25], // 已应用应用当前接受的画笔相机
    size: [600, 260],     // 逻辑 surface 视口尺寸
});
console.log(sample.client, sample.index, sample.rgba);
```

工具不会滚动、改 CSS、隐藏光标、选择邻近像素或修改应用状态。截图前后要求 DOM 矩形、
视口、滚动及 visual viewport 元数据完全一致；PNG 尺寸必须精确等于配置视口乘 DPR。
越过逻辑 surface 或实际截图视口的查询直接拒绝。部分离屏 surface 只能查询实际可见点。
visual viewport 缩放或偏移不受支持。调用者提供正向、轴对齐映射；单一包络不能描述
旋转、斜切或透视。几何稳定也不证明墨迹或应用帧稳定，调用者还需控制接受帧与动画状态。

元素截图可能在分数 CSS 边界外补齐整数像素。260px 高的 surface 在 DPR1 下可能产生
261px 高的 PNG，把 261/260 当成内容缩放会选错相邻行。已有
`acceptance-png.mjs::pixel` 继续保留原拟合图像用途，历史报告保留；新工具不倒推所有旧
采样一律通过或失败。

独立脚本 `scripts/viewport-capture-acceptance.mjs` 使用原生 CSS 颜色、逐物理像素编码的
Canvas，以及单独由浏览器布局的点标记，验证分数原点、正向非均匀缩放和滚动，不执行
Kartis 绘制。未滚动坐标还有固定独立预期索引；滚动使用浏览器实际结果，不假定请求值
就是实际值。真实截图后移动 DOM 的对照必须拒绝。CI 使用 `--chromium-only`，完整矩阵
覆盖 Chrome、Firefox、WebKit、Edge 的 DPR1/2。

滚动后的 fixed-Canvas 像素编码颜色保留为诊断：此原生夹具中 Firefox 分数滚动会让其
栅格相位偏移一到两像素。DOM 点标记的坐标/索引断言仍是严格门禁；未滚动的分数和缩放
场景执行硬颜色比较，滚动时另有宽青色 CSS 面板的可见颜色检查。没有从邻近颜色挑选
预期，也没有放宽像素门槛。

Bar font8 问题与此分开：准确整视口采样仍复现原 G98，所以坐标修正不能解决那份字形
比较。激活透明 Canvas 的原生 DOM 对照可复现，独立非 LCD 文字配置也与局部字形区域
完全一致。这些诊断解释了该载体的浏览器合成/文字路径，不代表任意文字统一像素等价，
也没有改变生产启动参数。参见[验收记录](verification.md)。

`node scripts/native-text-compositor-diagnostic.mjs` 可运行便携原生诊断，显式固定英文文档
语言、合成文字及几何，分别记录默认和诊断用非 LCD 配置，包含 font0 和黑字对照。
它不加载图表宿主，也不声明统一的通过颜色；文档语言属于载体条件，computed CSS 中
font-family 相同本身不能证明文字布局相同。

Skia 的[栅格化设计说明](https://docs.skia.org/docs/dev/design/raster_tragedy/)讨论了整像素/
子像素文字及 gamma、对比度调整。这是背景资料；具体复现由原生实验提供，尚未定位
唯一的 Skia 实现分支。
