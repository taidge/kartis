# 原生工作集与 WASM 内存采样

2026-11-10，Windows 11 build 26300、Intel Core i7-14700KF、Rust 1.99.1。这里采样真实 OS 进程计数器及实际导出的 `WebAssembly.Memory`，不从图片长度或 JS heap 推断整个进程内存。这是独立内存运行，不把同步采样后的时间当作绘制性能或 FPS。

## 重现

所有路径均来自实际 workspace 的 Cargo metadata；没有 Cargo home、target、TEMP/TMP 重定向。Windows 原生采样器使用 .NET Process API，子进程以隐藏、无 shell 模式启动，不引入 unsafe Rust 或额外依赖。

```powershell
Set-Location D:/Works/glory-rs/kartis
./scripts/native-memory.ps1 -Cycles 2
./scripts/native-memory.ps1 -Cycles 2 -Raster

$memoryMetadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
$browserReport = Join-Path $memoryMetadata.target_directory 'gallery/browser-memory-report.json'
$gcReport = Join-Path $memoryMetadata.target_directory 'gallery/browser-memory-gc-report.json'
# 先按 browser-host/README 启动当前fixture的8814服务。
node scripts/browser-memory.mjs --channel chrome --cycles 2 --output $browserReport
node scripts/browser-memory.mjs --channel chrome --cycles 2 --collect-garbage --output $gcReport
```

`scripts/check.ps1 -RunMemory` 可选运行两个原生 profile；浏览器脚本独立运行。原生 `--cycles`/`-Cycles` 和浏览器 `--cycles` 均限制1..3。浏览器复用已安装的 Playwright/browser，不下载或重定位浏览器。原生驱动仅支持 Windows，浏览器 OS 计数器支持 Windows/Linux；本轮 Linux 分支没有实机验证。

## 原生协议与实际结果

`examples/memory_probe.rs` 是专用协议程序，需由采样脚本驱动。每个1k/5k/10k/20k fixture真实构建圆形 Scene、导出SVG、建立PointMesh；可选显式ABeeZee字体的1200×720 RGBA及PNG。到每个checkpoint时输出PID/阶段/输出长度，父进程读取该子进程OS计数器并确认后才继续。`black_box` 在确认后消费对象，Scene、SVG、mesh及对应RGBA/PNG在采样时真实存活。

两轮default共34个checkpoint，raster共50个，全部完成。单位 **MiB=1,048,576 bytes**。第二轮20k的各阶段为累计持有状态，不是该operation独占内存：

| 阶段 | default工作集 | default private commit | raster工作集 | raster private commit |
|---|---:|---:|---:|---:|
| Scene存活 | 12.39 | 13.15 | 13.08 | 13.23 |
| 再持有SVG | 20.31 | 21.90 | 20.90 | 21.88 |
| 再持有mesh | 23.41 | 26.06 | 24.14 | 26.19 |
| 再持有RGBA | — | — | 27.50 | 29.56 |
| 再持有PNG | — | — | 28.84 | 31.40 |
| 输出全部drop | 5.07 | 1.41 | 5.69 | 1.44 |

default/raster启动工作集分别4.36/4.55 MiB；OS报告进程生命周期峰值分别23.89/29.40 MiB。工作集包含驻留的代码、共享页和分配器保留区；private commit是Windows私有已提交虚拟内存，不等于驻留物理内存。峰值来自OS生命周期高水位，不只取checkpoint最大值。drop不要求工作集回到启动数值，也不据两轮数据宣称无泄漏。

原始结果：`target/native-memory-default.json`、`target/native-memory-raster.json`，含可执行文件/探针源码/采样器SHA256，且运行前后核对可执行文件未变。这两个feature profile是不同进程/二进制，不能把它们的简单差值当作某个feature的精确内存开销。

## 浏览器与 WASM

Chrome **157.0.8103.39**，1280×1000 viewport、DPR=2、SVG/Canvas各1k/5k/10k/20k、两轮，每个fixture真实发布8个变化帧。测量mounted、changed、stopped；所有SVG实际path计数与源点数一致，Canvas实际1280×720 backing为3,686,400字节，stop后DOM严格回到84节点基线。容量从fixture实际初始化返回的 `memory.buffer.byteLength` 读取，未调用grow或估算。报告记录实际加载的WASM response字节数及SHA256，而非从可能已重建的磁盘文件回推。

普通运行49个snapshot：WASM线性内存启动 **2.13 MiB**，最大及最后 **52.31 MiB**。浏览器各拥有进程的报告工作集合计最高1103.98 MiB、private commit合计最高838.45 MiB。CDP `SystemInfo.getProcessInfo` 给出该独立浏览器实例的PID和类型，OS采样逐PID保留。**工作集合计包含重复共享页，不是去重后的物理RAM**，也不等于页面独占内存。

另一次明确开启 `--collect-garbage` 的65个snapshot，在每个自然stop采样之后，再用CDP `HeapProfiler.collectGarbage` 采样诊断值。第二轮20k SVG停止后的合计工作集965.79 MiB，显式GC后700.98 MiB；private commit693.47→424.13 MiB。WASM容量仍52.31 MiB。两次运行分别保留，不把显式GC当作应用的自然回收策略，也不把不同进程的最大值相减推断净开销。

结果分别在`target/gallery/browser-memory-report.json`、`browser-memory-gc-report.json`。JS heap API原值、各进程working set/peak/private commit、PID、类型、DOM、backing和WASM容量均逐阶段记录。WASM容量含运行时、活对象及可复用分配器空间；WebAssembly内存不会因Rust对象drop自动收缩，容量不能证明活分配量。GPU进程的OS工作集也不代表显存分配。完整live allocator统计、显存及compositor端到端帧率仍是剩余差距。
