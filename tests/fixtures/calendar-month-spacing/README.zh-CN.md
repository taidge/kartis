# 原版Calendar月份间距参考

`node scripts/calendar-month-spacing-reference.mjs <released-node_modules>` 生成参考，加 `--check` 验证。不安装依赖。producer在子进程显式设置 `TZ=UTC`，执行未修改的Nivo0.99完整 `computeLayout`、`bindDaysData`、年/月图例位置函数和原版 `alignBox`。使用真实Lodash4.17.21与完整原版D3 UMD。原源码和许可证在 [vendor](vendor/source-pins.json)，运行依赖哈希在 [reference.json](reference.json)。

既有版本固定为d3-time3.1.1、d3-time-format4.1.1、d3-array3.2.5。使用dist-only发布文件，不以重写的日期算法替代原函数。

十组横纵、闰年/多年和0/6/8/12/22间距场景共5120日期、168月份。原始day位置、月path/bounds与年/月标签位置完整保留。同周Jan31/Feb1在间距8时恰差8px。另保留三个原版negative/NaN/Infinity结果，非有限值以 `n:NaN`/`n:Infinity` 字符串记录；不与Rust有限非负profile冒充等价。

## 实际占用周资格

原Calendar预留 `max(timeWeeks(yearStart,nextYear).length)+1`；本库既有布局用 `max(ceil((civilDayCount+January1Weekday)/7))`。二者按原D3-time的civil-year事实独立计算。2023多年两组原版预留54列、实际占用53列，高度令两者格宽均10px。按作者宽度、十二份月间距及各自列数独立算两个origin，记录明确5px差异；其他八组不修正。没有读取候选输出或颜色拟合offset。

Rust参考测试以1e-9核全部5120日期矩形、168月bounds及两向实际path边界顶点距离。两组仅使用明确的既有occupied-week对齐资格，不宣称全局native props相等；原记录不被改写。任意首周日、长方格、padding月union、header/基线和显式时区聚合按Rust既有行为另测。

源码重放不等于渲染器、浏览器或真实设备验收。
