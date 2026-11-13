# BrowserChart 高密度变化帧验收与计时

本记录测量真实 Chrome 中的现有 SVG 和 Canvas BrowserChart，不包含新的 GPU 后端。
所有计时使用 release WASM；测试范围为 640×360 逻辑像素中 1,000、5,000、
10,000 和 20,000 个不透明矩形。每个 mark 有稳定 ID 和可命中的数据标签，
每个变化帧都会改变全部矩形的位置。Scene 继续受原有 marks/path budgets 检查。

外围 legend、data controls 和 tooltip 均关闭，避免把 20,000 个按钮的成本计入
绘图后端。鼠标命中仍通过真实 BrowserChart native listeners 和 SurfaceModel 验证。

## 本次实测（2026-11-10）

2026-11-10 12:24（UTC+08:00），Windows 10.0.26300，Intel Core i7-14700KF
（20 cores / 28 logical processors），HeadlessChrome 157.0.0.0，1280×1000 viewport，
DPR 2。一次完整运行的八组 case 全部通过，
零 page errors。下面单位均为毫秒；DOM 列为 density root 内的实际后代元素数，
不含 root 本身。

| 后端 | marks | cold mount+flush | warm median | warm p95 | 实际 DOM 后代 |
| --- | ---: | ---: | ---: | ---: | ---: |
| SVG | 1,000 | 22.30 | 8.75 | 9.00 | 2,012 |
| SVG | 5,000 | 76.80 | 44.50 | 53.90 | 10,012 |
| SVG | 10,000 | 150.50 | 88.80 | 91.40 | 20,012 |
| SVG | 20,000 | 371.30 | 181.80 | 203.00 | 40,012 |
| Canvas | 1,000 | 4.80 | 2.25 | 2.40 | 7 |
| Canvas | 5,000 | 19.10 | 10.50 | 10.80 | 7 |
| Canvas | 10,000 | 36.90 | 21.15 | 21.40 | 7 |
| Canvas | 20,000 | 71.10 | 43.45 | 43.90 | 7 |

SVG 的每个 mark 对应一个实际 path 和 title。Canvas 每组均只有一个 backing，
实际大小为 1280×720，即 3,686,400 RGBA bytes；逻辑尺寸仍为 640×360。
20k 同步变化帧时间在本 workload 中已明显超过 16.7ms，不能据此声称高密度场景
达到 60 FPS。Canvas 的 DOM 数量固定也不等于计算量或总内存固定。

八组均验证：首尾 shape 的实际 box/像素、变化前后 geometry 或 Canvas checksum 不同、
八个不同 accepted revision、真实 hover/click 命中当前 mark，以及 stop 后 document
DOM 元素计数精确回到 **80** 的基线。每个 case 都重新 mount 一个独立 guard。

该次 JS heap 采样起点约 3.19 MB、终点约 19.66 MB，中间最高采样约 57.97 MB，
后续曾下降到约 13.37 MB。它不是连续采集的峰值，也不是进程 RSS；这些未强制 GC
的采样只保留作上下文，不据此证明或否定完整内存泄漏。

### 先失败再修复的小数坐标命中

首次 20k SVG 验收失败时，目标矩形只有约 1.915×1.882 CSS px。
JS `elementFromPoint` 命中了真实路径，但 Rust hit 返回空。原因是旧 web-sys
`client_x/client_y` 的整数 getter 丢失小数：实际 client 坐标约
`(29.71982684, 269.99053816)`，surface 原点 `(20, 260.40625)`，截断后局部 Y
变成 `8.59375`，落在矩形顶部 `8.64315609` 之外。

BrowserChart 现使用显式 f64 getter 读取 JS Number，pointer 和 wheel listeners
都保留小数。最终八组测试没有移动目标或放松命中断言；报告保留 requested/delivered
坐标、surface 原点和实际 mark bounds，20k SVG/Canvas 均验证自然小数位置 hover 和 click。

## 方法

- cold：从创建场景开始，到 `BrowserChart::mount` 和显式 `flush` 返回的单次墙钟时间。
- warm：先执行一个真实变化帧预热，再执行八个不同几何 phase；每次从 Rust 场景生成、
  `set_scene` 到显式 `flush` 返回，用 `performance.now()` 计时。
- 另记录同一变化更新开始到下一次 `requestAnimationFrame` 的间隔。该数值受调度和
  浏览器显示管线影响，与同步 flush 时间分开报告，不据此声称 60 FPS。
- median 为八次样本中间两项平均值；p95 使用 nearest-rank，因此八次样本的 p95
  就是样本最大值。样本少，不能视为稳定尾延迟估计。
- DOM 计数来自真实 document；Canvas backing 记录实际物理像素及其 RGBA 字节数。
  RGBA 字节数只描述该 backing buffer，不是 Canvas 引擎或进程总内存。
- 像素采样、SVG 路径验证、真实鼠标命中和 cleanup 都在计时之外执行。SVG 验证实际
  首尾路径的 box，并将实际第一个 path 单独解码采样；Canvas 验证实际 backing 中的
  首尾色块。所有 case 都验证几何变化、独立 revision 和当前点命中。
- `performance.memory` 存在时记录 used/total JS heap；它不是进程 RSS，也不覆盖完整
  WASM linear memory、DOM/native/backend 分配。没有强制 GC；cleanup 后 heap 未回到
  初值不单独构成内存泄漏结论。强制验证的是 DOM 回到同一基线和 guard teardown。

计时顺序固定为 SVG 1k→5k→10k→20k，再 Canvas 同序；不是随机化、跨机器或长期重复
实验。cold 表示当前已加载 WASM 页面的首次 fixture mount，**不包含浏览器启动、网络、
WASM 下载/编译**。后台其他应用、JIT、GC、CPU 调频和 Chrome 内部加速均可能影响结果。

## 复现

先运行 `examples/browser-host/build.ps1`，按该独立 workspace 的正常 Cargo 配置构建；
脚本通过 `cargo metadata --no-deps --format-version 1` 获取产物路径，不重定向缓存。
然后用本地静态服务提供 `examples/browser-host/`（默认 `http://127.0.0.1:8814`）。

`scripts/density-browser-acceptance.js` 导出可复用 `(page) => report` 函数；调用者先加载
host 页面并等待 `window.hostReady`。脚本建立自己的 density root/guard，完成后移除，
恢复原有 panel 与 reduced-motion 设置。原有 host fixture 的 Entries 不被替换。

完整机器可读结果写入 `target/gallery/density-browser-report.json`，包含全部原始样本、
user agent、DPR、DOM、backing、可选 JS heap 以及命中/像素/cleanup 结果。

## 适用边界

这是简单矩形、高密度浏览器绘图与生命周期验收；不是复杂路径、字体、mesh、动画、
GPU、OS 原生窗口或移动设备性能承诺。任何吞吐量结论都应与完整 workload、浏览器、
机器和样本数一起解释。没有测量进程 RSS、功耗、辅助技术或跨操作系统平台行为。
