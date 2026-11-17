# Kartis 图表探索站

[English](README.md)

这是 `kartis.taidge.org` 的静态 Rust／WebAssembly 展示站。可搜索目录包含 **35 种真实图表类型**，
涵盖 Cartesian、整体占比、层次、网络、地理与专用布局。缩略图与选中的交互图表使用同一份 Rust 目录；
不使用预制图表截图、React、远程数据、外部字体或统计服务。

首屏包含真实 Line 动画演示。选择图型后可切换 SVG／Canvas、换数据、播放／暂停自动演示、
调整 Spring 预设、明暗主题、下载 SVG，并查看真实作者输入 JSON 与核心 Rust 调用片段。
Line 曲线、Bar 分组／方向、Pie 内半径与 Tree 方向均改变真实布局参数。
代码面板是核心调用片段，不是完整应用；源码链接提供其余初始化和支持选项。

## 构建

需要 Python 3、Rust 1.95 或更高版本、`wasm32-unknown-unknown` target，以及与锁文件匹配的
**0.2.130** 版 `wasm-bindgen-cli`。在仓库根目录执行：

```sh
python3 site/build.py
python3 -m http.server 8080 --directory site/dist
```

访问 `http://localhost:8080`，部署目录为 **`site/dist`**。
脚本在独立 `site/wasm` workspace 执行 `cargo metadata`，使用其实际 `target_directory`，
随后执行锁定依赖的 release 构建与 wasm-bindgen。不会重定向 Cargo home、target、临时目录或全局缓存。
独立的 `site/wasm/Cargo.lock` 随源码提交；`--toolchain 1.95.0` 可选择已安装工具链。
已有 rustup 的全新构建环境可显式允许安装标准工具：

```sh
python3 site/build.py --install-tools
```

公开 `build.json` 包含资源哈希、由实际内容生成的 `asset_revision` 与可选 Git commit，不包含机器路径；没有 Git 元数据时 commit 为 null。
源码归档和静态产物不依赖某台机器上的仓库路径。

## Cloudflare Pages

构建根目录使用仓库根，命令 **`bash site/cloudflare-build.sh`**，输出 **`site/dist`**。
wrapper 先查找标准 `$HOME/.cargo/env`；没有 rustup 时通过官方 HTTPS 安装器安装最小 Rust 1.95.0，
再将 WASM target 与匹配的绑定生成器安装到正常用户位置。
仅通过 PATH 定位工具，不重定向 HOME、CARGO_HOME、target 或临时目录。
已配好工具的 CI runner 可直接使用 `python3 site/build.py`。

站点不需要 token、binding、API 服务或环境密钥。Pages Git 部署与自定义域名登记独立处理。
`.wasm` 应以 `application/wasm` 提供。`_headers` 的 `no-transform` 阻止边缘注入统计脚本，
JavaScript、CSS、WASM 与绑定 snippets 共同发布到 `assets/<asset_revision>/`，HTML 入口使用同一版本目录。
revision 核对全部实际运行文件的路径与字节，即使托管方延长缓存时间，也不会将新 JavaScript 与旧 WASM 混用。
每次构建只清理经严格路径与非链接检查的生成目录 `site/dist`。第三方地图数据归属随静态站交付。

## 真实动画与生命周期

活动画布由一个 `BrowserChart` 拥有。`animate_to` 使用 Kartis property springs 和 Rust RAF scheduler；
图形 path 没有 CSS 动画。预设实际配置 stiffness／damping，更新目标保留物理速度，稳定 Mark ID 连接不同数据集。
不兼容的 path 拓扑可以 crossfade；这不代表每种图元、时间积分或 Nivo／React 过渡完全等价。

动画尊重系统 reduced-motion，系统开启时首屏不会自动播放。
**“Pause demo”只停止未来自动换数据，当前 Spring 会自然结束。**
关闭动画会以 Reduced 模式重新挂载并即时更新数据。切换 renderer、主题、布局变体或 Spring 预设，
也会保留当前作者数据重新挂载，重置临时 hover／selection。尺寸变化通过真实 Rust 响应式布局回调处理。

页面隐藏时释放 owner，暂停数据与状态 timer。BFCache 恢复保留每种图表的作者 seed，多次 pagehide 均执行清理。
应用不深冻结输入对象，也不模拟服务器数据流。SVG 下载导出当前响应式作者目标，不包含临时选中或隐藏图例状态。
地理展示保留带归属的 176 feature 世界数据原件，明确使用 175 个非南极 feature 作为绘制子集。
当前平面示例不支持南极包极 cap 几何，因此明确排除 ATA；Data 面板公开原来源、闭环规范化与排除原因。
示例地图不作为地理边界权威。

## 验证

在仓库根使用现有 Playwright 依赖：

```sh
npm ci
npx playwright install chromium
node site/smoke.mjs
node site/explorer-acceptance.mjs
node site/cache-acceptance.mjs
```

脚本自行提供 `site/dist` 的静态 HTTP 服务，向忽略的 `site/verification` 写 JSON、截图与导出的 SVG。
`--url=https://example.com` 可检查部署站，`--out=<directory>` 指定证据目录；不会执行部署或修改远程服务。
远程模式核对实际 HTTP `no-transform` 响应头；统计脚本注入、beacon 请求、console 错误或失败请求均导致失败，
包括被中止的 beacon，不忽略网络错误。

smoke 覆盖真实预览、首屏 Spring、原六类图表 SVG／Canvas 交互、tooltip／图例／下载与重复生命周期清理。
独立 explorer 检查覆盖更多图型、选项与运动。确定性的生命周期 handler 检查与浏览器实际保留页面时的
trusted BFCache 事件证据分别记录。可复用 Showcase CI 使用同一静态构建并运行全部三个脚本。
缓存回归在真实浏览器中保留旧资源，检查新版页面完整加载同一内容版本的模块图。
这些有限场景通过不等于全部 API 的兼容性或性能承诺。
