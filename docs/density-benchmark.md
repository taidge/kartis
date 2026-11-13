# 高密度原生性能基线

`examples/density_bench.rs` 是确定性、无新增依赖的原生吞吐基准。它真实执行 scatter 布局、SVG 序列化、20,000 mark 的 Scene 校验和输出、PointMesh 构建和最近查询；启用 `raster` 后，还用显式注册的 ABeeZee 字体真实生成 RGBA。它不模拟浏览器帧率，也不把不同机器上的 Nivo/JavaScript 数值拿来比较。

## 重现

在 kartis 自己的 workspace 中查询实际产物目录，再通过 CLI 显式传入报告路径。没有 Cargo home、target 或 TEMP/TMP 重定向，也不会安装字体或读取系统字体目录。

```powershell
Set-Location D:/Works/glory-rs/kartis
$densityMetadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
$densityReport = Join-Path $densityMetadata.target_directory 'density-benchmark-raster.md'
$densityCpu = (Get-CimInstance Win32_Processor).Name
cargo run --release --features raster --example density_bench -- `
  --report $densityReport --machine $densityCpu --iterations 20
```

省略 `--features raster` 可以单独运行默认依赖图；指定另一个 `--report` 路径以保留两次报告。`--iterations` 接受 1..=200，默认 20。`--report` 必须为绝对路径。非 Windows 环境可直接为 `--machine` 提供硬件说明；示例没有依赖 PowerShell。

## 采样与正确性

- 每个 operation/fixture 先单列首次调用，再进行两次不计时预热和 20 次独立样本，报告毫秒、median、nearest-rank p95、min/max。首次调用不是操作系统缓存清空后的 cold start；同一进程中先前的操作也会预热共享代码和分配器。
- 图幅固定为 1200×720，确定性整数取模生成数据。scatter 为一个 series，保持默认坐标轴、默认自动 domain，点半径改为 2。布局样本包含校验、轴/点/临时线布局、分配和销毁；SVG 样本对已经布局的 Scene 调用 `to_svg`，包含 Scene 校验、字符串构造和销毁。
- 20,000 mark 边界是独立构建的圆形 Scene，半径 2，另有一个真实文字标签；不把它误报为 scatter 的 20,000 数据点能力。Scene 的 mark 上限为 20,000，但 scatter 的输入上限为 10,000，并会额外生成轴 marks。
- Mesh 使用同一个确定性 2D 分布，分别测 1k/5k/10k/20k sites。构建包含调用者点列表的 clone、树构建和销毁；查询在已经构建的树上进行，一个样本含 2,000 次 nearest 调用，返回借用、不分配 ID 字符串。每种规模的前 32 个查询与全量遍历的欧氏距离、相同距离按 ID 字典序规则核对，共 128 次；核对不计入查询耗时。
- 栅格样本使用仓库已有 `tests/fonts/ABeeZee-Regular.ttf`（OFL 许可同目录保留），字体注册在计时外。DPR=1，默认 `RasterOptions`，真实包含校验、缓冲区分配、文字 shaping、绘制、RGBA 转换和结果销毁。PNG 压缩、文件写入不计时。每张输出为 1200×720×4=3,456,000 字节；这是输出缓冲区长度，不是进程峰值内存。
- 计时使用 `Instant`；通过 `black_box` 消费结果。完整报告还记录工具链、机器、运行时间、feature 和结果长度。此例不采样 RSS、分配次数或峰值内存。

## 本机实测（2026-11-10）

Intel Core i7-14700KF，20 核/28 逻辑处理器，Windows 11 Pro build 26300，`x86_64-pc-windows-msvc`；`rustc 1.99.1 (58cf8f9c3 2026-10-13)`，LLVM 23.1.1，Cargo 默认 release 优化配置。采样前暂停团队的全量编译与浏览器验收；未做进程绑核、CPU 锁频或操作系统后台服务隔离。以下保留实际观察到的波动，没有筛掉慢样本或取多轮最好成绩。

默认 feature 的一次完整运行，单位为 **ms/sample**；query 行一个 sample 是 **2,000 次查询**：

| 操作 | 点/mark/site 数 | 首次 | warm median | warm p95 | warm min | warm max |
|---|---:|---:|---:|---:|---:|---:|
| scatter 布局 | 1,000 | 3.684 | 3.697 | 4.050 | 3.526 | 4.137 |
| scatter SVG | 1,000 | 2.977 | 2.871 | 3.070 | 2.808 | 3.558 |
| scatter 布局 | 5,000 | 18.088 | 17.821 | 18.555 | 17.265 | 18.819 |
| scatter SVG | 5,000 | 14.648 | 14.235 | 14.768 | 13.817 | 14.779 |
| scatter 布局 | 10,000 | 36.420 | 35.384 | 36.360 | 34.765 | 37.245 |
| scatter SVG | 10,000 | 28.147 | 28.611 | 29.116 | 27.902 | 29.235 |
| Scene 构建并校验 | 20,000 | 8.697 | 7.572 | 7.931 | 7.308 | 8.845 |
| 已有 Scene 校验 | 20,000 | 3.068 | 3.002 | 3.175 | 2.908 | 3.223 |
| 边界 Scene SVG | 20,000 | 46.297 | 78.927 | 95.593 | 45.400 | 97.022 |
| PointMesh 构建 | 1,000 | 2.833 | 0.357 | 0.584 | 0.351 | 0.929 |
| PointMesh 查询 ×2,000 | 1,000 | 0.571 | 0.558 | 0.865 | 0.544 | 1.557 |
| PointMesh 构建 | 5,000 | 4.282 | 2.468 | 2.765 | 1.565 | 4.578 |
| PointMesh 查询 ×2,000 | 5,000 | 0.527 | 0.824 | 1.868 | 0.757 | 6.519 |
| PointMesh 构建 | 10,000 | 6.121 | 6.092 | 8.881 | 5.602 | 8.883 |
| PointMesh 查询 ×2,000 | 10,000 | 1.322 | 0.789 | 2.202 | 0.765 | 3.828 |
| PointMesh 构建 | 20,000 | 13.919 | 8.988 | 16.165 | 7.484 | 18.216 |
| PointMesh 查询 ×2,000 | 20,000 | 0.991 | 0.944 | 3.229 | 0.819 | 3.519 |

另一次启用 `raster` 的完整 release 运行，实际 RGBA 绘制结果如下；其余同批 operation 也保留在原始报告中，不用于与 default 做 feature 开销差值推断：

| 实际 RGBA 绘制 | 数据点/mark 数 | 首次 | warm median | warm p95 | warm min | warm max |
|---|---:|---:|---:|---:|---:|---:|
| scatter + 坐标轴/文字 | 1,000 | 7.371 | 7.124 | 7.233 | 6.801 | 7.368 |
| scatter + 坐标轴/文字 | 5,000 | 15.366 | 15.240 | 15.531 | 15.009 | 15.642 |
| scatter + 坐标轴/文字 | 10,000 | 24.907 | 44.414 | 53.371 | 41.095 | 53.413 |
| 边界 Scene + 一个标签 | 20,000 | 76.203 | 75.110 | 91.004 | 48.736 | 91.874 |

1k/5k/10k scatter 实际分别生成 1,015/5,017/10,017 marks，SVG 长度分别为 728,316/3,634,488/7,268,517 UTF-8 字节；20k 边界 SVG 为 8,348,308 字节。两轮各自的 128 次暴力 oracle 核对全部通过。首次调用有时快于后续样本，特别是大 SVG 和 raster；单次进程采样不能解释调度、频率或分配器导致的波动，所以同时报告 p95 与 min/max。

本轮原始报告位于实际 metadata 返回的 target 下：

- `D:/Works/glory-rs/kartis/target/density-benchmark-default.md`，记录 Unix seconds `1794283104`。
- `D:/Works/glory-rs/kartis/target/density-benchmark-raster.md`，记录 Unix seconds `1794283074`。

这些 target 产物可随构建清理；上面的固定结果表保存在仓库文档中。报告路径不参与 Cargo target 配置。

## 可接受规模与使用方式

这个默认 profile 已实际接受并完成 10k scatter、20k primitive Scene/SVG、20k PointMesh，以及默认 raster 预算下的全部绘制。在本机本例中，1k scatter 单次布局和序列化各在几毫秒范围，适合作为频繁重布局的起始规模；5k/10k 已明显增加同步工作量。10k 的布局与 SVG median 合计约 64ms，20k SVG median 约 79ms，不能据此宣称逐帧全量重建可达到 60FPS。

高密度交互应复用已接受的 Scene 和 PointMesh，在数据/尺寸改变时重建；查询本身不应触发全量布局或 SVG 序列化。20k 查询批次 median 0.944ms，即该固定查询集平均约 0.472µs/查询，但不是单次查询的 p95，也不是任意数据分布的保证。频繁更新的应用仍需用自己的图幅、标签、路径和宿主端到端测量；预算上限只表示受支持的有界输入规模。

## 默认资源边界

基准必须实际确认以下拒绝，否则非零退出：scatter 10,001 点、Scene/SVG 20,001 marks、PointMesh 20,001 sites，以及启用 raster 后 `max_operations=1`、`max_pixels=1`。预算错误不通过降低输出质量或截断输入来掩盖。

输入点数、Scene mark 数和交互树 sites 数是不同预算；没有统一的“20k 数据任意图表”承诺。默认 raster 还受 250,000,000 operation 单位和 67,108,864 physical pixels 限制；这些是安全边界，不能代替时延目标。详细计时只适用于本例固定图幅、样本分布、半径和标签数量，不覆盖海量文字、复杂路径、DOM 布局或端到端交互。
