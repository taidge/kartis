# Kartis 图表探索站

[English](README.md)

这是 `kartis.taidge.org` 的静态 Rust／WebAssembly 展示站。可搜索目录包含 **35 种真实图表类型**，
涵盖 Cartesian、整体占比、层次、网络、地理与专用布局。缩略图与选中的交互图表使用同一份 Rust 目录；
不使用预制图表截图、React、远程数据、外部字体或统计服务。

首页提供交互式折线图、可搜索的图表索引和按类别筛选的预览目录。小屏幕下可通过菜单打开索引。
选择图型后，图表占满内容区宽度。使用 **Chart settings** 跳到下方配置区，或使用 **Back to chart** 返回图表。可切换 SVG／Canvas、换数据、播放／暂停自动演示、
调整 Spring 预设、明暗主题、下载 SVG，并查看真实作者输入 JSON 与核心 Rust 调用片段。
选择布局预设后，可以在同一组数据上独立调整该图表的属性与绘图区边距（**Plot margins**，50% 至 150%）。例如，华夫图可保留填充方向，分别调整网格行列数和格间距。桑基图可固定节点宽度，比较节点间距与连线透明度。地图可保留投影方式，调整经纬网间隔和标记半径。Code 面板显示核心 Rust 调用和选中属性对应的覆盖设置。

属性面板按用途分组。有坐标轴的图表可以分别调整支持的轴与网格。例如，折线图可隐藏 X 轴，同时保留垂直网格。所有图表都能选择色板，并为最多三个数据标记添加标签。有独立数据标签的图表还可以隐藏这些标签并保留刻度文字。分类配色提供 Tableau 10 和 Dark 2，连续配色提供 Viridis 和 Magma。

将指针移到数据上，可以比较完整提示和仅数值提示，切换数字格式，或关闭提示。SVG 和 Canvas 都用独立轮廓标出当前数据项，关闭提示后仍可看到轮廓。图表静止时，移动指针会保留已经画好的底图。有原生图例的图表可以横排、纵排或隐藏图例，已隐藏的系列会保留其可见性设置。

切换配置和更新数据时，图表会复用原来的绘图区。SVG 在已有节点上更新变化的属性；删除一组刻度时，其他位置不变的刻度会保留原节点。Canvas 重绘图元变化前后占用的区域，并按原顺序恢复重叠的图形和文字。画布尺寸或背景改变，以及密集重叠需要大范围重绘时，会使用完整重绘。调整弹簧参数会保留正在运动的图元的位置和速度。

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

调整单项属性、布局预设或边距时，已挂载的 `BrowserChart` 使用当前数据更新。点击 **Shuffle** 换数据会保留所选配置。可以在数据过渡期间修改属性，再在弹簧停止前换一次数据。Rust 在动画帧中驱动 `animate_to`，根据稳定的图元 ID 将当前画面过渡到最新配置和数据。路径拓扑不兼容时可交叉淡化。

可选择 Default、Gentle、Wobbly、Stiff、Slow、Molasses 六种弹簧预设，也可分别调整刚度和阻尼。**Replay fade-in** 使用当前弹簧参数，让图元从透明状态重新显现。**Pause demo** 停止自动换数据，当前过渡会自然结束。系统开启“减少动态效果”时，演示默认暂停，数据和布局更新即时生效，淡入重播按钮停用。

在 SVG 和 Canvas 之间切换时，新渲染器使用当前数据挂载。主题和动画设置直接更新已有图表；在过渡中关闭动画会立即显示目标画面。尺寸变化由 Rust 布局回调处理。

页面隐藏时释放 owner，暂停数据与状态 timer。BFCache 恢复保留每种图表的作者 seed，多次 pagehide 均执行清理。
应用不深冻结输入对象，也不模拟服务器数据流。SVG 下载导出当前响应式作者目标，不包含临时选中或隐藏图例状态。
地理展示保留带归属的 176 feature 世界数据原件，明确使用 175 个非南极 feature 作为绘制子集。
当前平面示例不支持南极包极 cap 几何，因此明确排除 ATA；Data 面板公开原来源、闭环规范化与排除原因。
示例地图不作为地理边界权威。

## 验证

在仓库根使用现有 Playwright 依赖：

```sh
npm ci
npx playwright install chromium firefox
node site/smoke.mjs
node site/explorer-acceptance.mjs
node site/partial-frame-acceptance.mjs
node site/cache-acceptance.mjs
```

脚本自行提供 `site/dist` 的静态 HTTP 服务，向忽略的 `site/verification` 写 JSON、截图与导出的 SVG。
`--url=https://example.com` 可检查部署站，`--out=<directory>` 指定证据目录；不会执行部署或修改远程服务。
远程模式核对实际 HTTP `no-transform` 响应头；统计脚本注入、beacon 请求、console 错误或失败请求均导致失败，
包括被中止的 beacon，不忽略网络错误。

smoke 覆盖真实预览、首屏 Spring、原六类图表 SVG／Canvas 交互、tooltip／图例／下载与重复生命周期清理。
explorer 检查覆盖更多图型、选项与动画。局部绘制专项将非整数 DPR 下的动画和文字与独立完整重绘的 Canvas 逐像素对照。确定性的生命周期 handler 检查与浏览器实际保留页面时的
trusted BFCache 事件证据分别记录。Showcase CI 对同一静态构建运行上述脚本。
缓存回归在真实浏览器中保留旧资源，检查新版页面完整加载同一内容版本的模块图。
这些有限场景通过不等于全部 API 的兼容性或性能承诺。
