# 更新日志

本文件记录 meow-app-launcher 所有值得留痕的变更喵。

格式参考 [Keep a Changelog](https://keepachangelog.com/zh-CN/1.1.0/)，
版本号遵循[语义化版本](https://semver.org/lang/zh-CN/)喵。

## [Unreleased]

暂无待发布变更喵。

## [1.7.0] - 2026-09-13

### Added
- **应用管理中心** —— 全新独立窗口,陈列**全部**注册应用(配置页此前只显示前 40 个): 左侧列表支持
  网格 / 行双排版、关键词过滤(名称与标签)与滚动条,右侧详情卡可查看并直接编辑
  名称 / 路径 / 自定义图标 / 标签 / 收藏,并提供「打开所在位置」「移除应用」动作;
  改名冲突会被拒绝,修改即时重同步搜索索引并持久化喵。入口双通道: 配置 GUI「应用」页按钮 +
  托盘「应用管理」菜单项喵。
- **应用卡片右键菜单** —— 岛体结果列表中的应用条目支持右键弹出上下文菜单:
  启动 / 收藏(取消收藏) / 打开所在位置 / 复制路径 / 移除注册,文案随状态变化喵。
  菜单模态期间岛不会被失焦误隐藏喵。
- **named-pipe IPC(CLI 联动)** —— `meowal show / hide / toggle / query <文本>` 经命名管道
  与运行中的实例完整联动,查询文本可直接注入搜索框喵。服务端随 GUI 启动常驻
  (`\\.\pipe\meow-app-launcher`),命令在主线程执行;二次启动唤起也优先走 IPC
  (保留 PostMessage 回退)喵。
- **配置面板动效** —— 配置 GUI 滚动平滑趋近 + 页面切换淡入,动效进行中自动切到动画帧率、
  静止后回低频心跳;「常规 · 显示」新增「配置面板动效」开关可整体关闭喵。
- **配置 GUI 内容区滚动条** —— 内容溢出时在右缘绘制轨道与滑块,支持按住拖动;
  滚出视口的行不再登记命中区,看不见的控件点不到喵。
- **深色主题** —— 新增「深色」主题预设（暗夜玻璃,浅色文字),岛体与配置 GUI / 应用管理中心整套
  配色(窗口/侧边栏/分组卡片/控件)同步适配,「常规 · 外观」主题循环第 4 档可切换喵。
  深色配色体系借鉴自作者另一 Skia 灵动岛实验项目 winisland(已在 README 致谢)喵。
- **应用管理中心: 一键扫描 + 屏蔽词** —— 工具栏新增「重新扫描」按钮(后台扫描,完成后合并注册表);
  列表底部新增「屏蔽词」横条,回车添加关键词、点击芯片删除,与配置 GUI 共用同一份过滤规则喵。
- **搜索指令独立分组** —— 系统指令(锁屏/睡眠/关机/重启)不再与应用混排,单独成「指令」分组
  固定垫底,Row/Grid 两种排版下都不会被误触喵(分组头由既有分组机制渲染,纯函数 + 单测守门)。

### Changed
- **设置 GUI 的 `RowDesc` 数据驱动化(ROADMAP D5)** —— 新增 `window/settings_data` 配置项注册表:
  开关 / 数值步进 / 循环选项三类可调项的元数据(身份、标签、归属分组、读写函数)收进一张表,
  页面构建、命中分发、脏判定、恢复默认全部查同一张表,新增配置项从「改 4 处」降到「改 1 处」喵。
  主题 / 帧率 / 排版 / 渲染后端 / 搜索引擎 / 运动引擎 / 缓动曲线等循环选项统一为 `Choice` 行,
  渲染层新增 `SettingsRow::Choice` 与 `RowHit::Choice` 喵。
- **配置 GUI「应用」页瘦身** —— 应用陈列与打标签整体迁入应用管理中心,页面只留「管理中心入口 +
  关键词过滤」,节省内容空间喵;拖拽注册配置窗口后会自动打开管理中心展示结果喵。
- **编辑交互统一** —— 配置 GUI 与应用管理中心统一为「点击输入框全选进入编辑,点击其它区域自动
  提交草稿并释放焦点」,窗口隐藏前也会先提交,不再出现挂着焦点丢改动的情况喵。

### Fixed
- **配置 GUI 滚动内容覆盖页头** —— 内容区裁剪矩形此前从窗口顶部 (y=0) 开始,向上滚动时
  分组卡片会盖在「页头标题栏」之上;现裁剪上沿卡在页头分隔线下方,滚动内容不再侵入页头喵。
- **应用管理中心滚动后点击偏移** —— 列表条目的命中区此前直接用内容坐标登记,滚动后与绘制位置
  错开一个滚动量;现统一换算回屏幕坐标,网格/行两种排版都恢复准确喵。
- **应用管理中心关闭按钮无效** —— 顶栏拖动命中区先于关闭钮登记,重叠区域点击被「拖窗口」抢走;
  现关闭钮先登记命中,点击即关喵。
- **应用管理中心屏蔽词条点击穿透** —— 列表内容总高度误把可视高度当基底,滚动上限虚高,
  条目命中区可以滚进面板底部压住屏蔽词横条;现修正高度公式(内边距×2+行数×行高)、
  条目命中区做视口剔除、屏蔽词横条先登记命中,三层保险后点击不再穿透喵。

## [1.6.1] - 2026-09-11

### Changed

- **砍掉过度工程** —— 净删 667 行代码与 3 个依赖喵。渲染层与配置层的若干抽象在没有第二个实现的情况下
  只增加理解成本，本轮按「先有重复、再抽抽象」的原则回收喵。

### Fixed

- **唤出搜索偶发「打不进字」** —— 分层置顶窗呼出后可能已抢到前台、键盘焦点却留在别的线程，
  此时输入会被系统判为无效输入直接丢弃，并附赠一声提示音喵。`focus_window` 的就绪判定由
  「只看前台窗口」升级为「前台 + 键盘焦点」双确认，抢到前台后立刻 `BringWindowToTop` + `SetFocus`
  把焦点钉回本窗；`wnd_proc` 的 `WM_ACTIVATE` 在窗口被激活时补钉一次 `SetFocus` 兜底喵。
- **Release 打包后拖拽注册失效** —— 提权运行后，低完整性级别的资源管理器投递的 `WM_DROPFILES`
  会被 UIPI 在消息过滤器层拦截；`enable_file_drop` 补 `ChangeWindowMessageFilterEx` 放行喵。
- **配置 GUI 应用栏点击偏移** —— `app_name_at` / `chip_label_at` 的命中索引按「全页行号」计数，
  与绘制层只数同类行的 `pick_i` / `chip_i` 口径不一致，导致点谁不选谁；现改为只数
  `AppPick` / `Chip` 行，与绘制层完全对齐喵。

## [1.6.0] - 2026-09-08

### Added

- **配置 GUI 搜索页新增「Web 搜索」分组** —— 自定义浏览器输入行（带 placeholder）、搜索引擎循环切换按钮喵。
- **配置 GUI 搜索页新增「指令模块」分组** —— 每条指令一行：别名输入框（逗号/空格分隔，回车提交，保序去重）、
  命令类型循环按钮、删除按钮；底部「添加指令」按钮，新增后自动聚焦别名框喵。
- `SettingsRow::Input` 支持 `placeholder` 字段，空值时可显示提示文案喵。

### Changed

- 编辑槽体系扩展 `EditFocus`：新增 `command`（下标驱动）与 `browser`（单例）两类编辑槽，
  与既有的标签/过滤词槽共用同一套互斥逻辑与快捷键链路喵。

## [1.5.0] - 2026-09-07

### Added

- **统一图标渲染层 `render/icon.rs`** —— `IconSource{Builtin / Raster / Fallback}` 单一出口，
  统一 8% 标准内边距；启动器与配置 GUI 全部图标绘制收口到此，删除三处重复实现喵。
- **自定义浏览器** —— 新增 `search.web_browser` 配置：留空走系统默认；支持带引号的路径与 `%1` 占位符，
  `Platform::open_url` 失败时自动回退系统默认浏览器喵。
- **指令别名表可配置** —— 新增 `search.commands`（`aliases` + `kind`），中英文及自定义别名随意增删改，
  同一条命令的多个别名自动取最高分去重喵。
- 新增 `calculator` / `web` / `power` 三枚内置 SVG 图标喵。

### Fixed

- **SVG 渲染器三处根因修复**：
  - 支持 `viewBox` 映射（`SvgIcon.view`），非 24×24 画布的图标不再被缩成一个点，绘制时等比缩放居中喵。
  - 改为**逐元素**判定 stroke/fill 风格，不再整文件查 `fill="none"` 字符串，线稿与填充可混排喵。
  - **修复属性匹配的子串误命中** —— `attr()` 匹配裸 key 时会撞上元素名（如在 `circle` 里找 `r` 命中了
    `ci**r**cle`），前缀剥离失败直接返回 `None`，导致**所有 circle 元素被静默丢弃**喵。
    修法：匹配 `"{key}="` 整体喵。

## [1.4.0] - 2026-09-07

### Added

- **搜索中枢地基** —— `SearchItem`（标题/副标题/图标/动作）与 `Action`
  （`Launch` / `OpenUrl` / `CopyText` / `SystemCommand`）统一模型喵。
  结果展示与激活动作彻底分离，一切搜索源共用同一条管道喵。
- **`SearchProvider` trait** —— `SearchProvider { name, query(ctx) -> Vec<Scored> }` + `builtin_providers()`喵。
- **计算器 `search/calc.rs`** —— 词法白名单 → shunting-yard（一元负号独立栈项、幂右结合）→ RPN 求值；
  非有限结果返回空。回车复制结果，`0.1 + 0.2` 正确显示 `0.3`喵。
- **Web 搜索跳转** —— 百度 / 必应 / 搜狗 / 谷歌 / DuckDuckGo，查询词自动百分号编码喵。
- **系统命令** —— 锁屏 / 睡眠 / 关机 / 重启，中英双语关键词匹配喵。
  实现走 `LockWorkStation` / `SetSuspendState` / `AdjustTokenPrivileges` + `ExitWindowsEx`，
  不借道 `shutdown.exe`，避免控制台闪窗喵。
- **单实例保护** —— `CreateMutexW("Local\\MeowAppLauncher.SingleInstance")`；
  二次启动改为 `FindWindowW` + `PostMessageW` 转交唤起意图喵。

### Changed

- `ListItem::App(AppInfo)` 重构为 `ListItem::Item(SearchItem)`喵。
- 多源聚合分数域约定：计算器 `3.0` 置顶 → 系统命令 `模糊分 + 2.0` → 应用原始模糊分 → Web `0.0` 垫底；
  `t:` / `i:` 定向模式不注入内置源；统一 30 条上限喵。

## [1.3.3] - 2026-09-07

### Fixed

- **托盘图标缺失** —— 旧实现只带 `NIF_MESSAGE` 占位，再用像素画的灰圆点在深色任务栏上几乎隐形喵。
  改为 `LoadImageW` 从 exe 资源（id=1）按 `SM_CXSMICON` 加载应用同款 `.ico`，
  `create_tray` 一次带齐 `NIF_MESSAGE | NIF_ICON | NIF_TIP`；新增 `TaskbarCreated` 广播监听，
  资源管理器重启后自动重建图标喵。
- **再次唤出无法输入 / 点击无反应** —— 两个根因：多次启动导致双实例互踩（已由单实例互斥 + 消息转交解决），
  以及 Windows 前台锁拒绝 `SetForegroundWindow` 喵。
  新增 `Platform::focus_window`：已是前台直接返回，否则 `AttachThreadInput` 附加前台线程后再设前台；
  点击岛体时先抢回键盘焦点自救喵。
- **开机自启「启动不起来」** —— 取证结论：`Run` 键一直有效、安装副本日志显示每日均有启动会话，
  真凶是托盘图标不可见造成的感知问题，随上一条一并解决喵。

### Changed

- `build.rs` 新增 `locate_rc_toolkit()` —— 按 `MEOWAL_RC_TOOLKIT` > 扫描 Windows Kits 取最高版本的顺序
  显式定位 `rc.exe`，绕开被安全策略拦截的 `reg.exe` 探测喵。

## [1.3.2] - 2026-09-04

### Added

- **拖放注册** —— 拖入 `.lnk` / `.exe` 到配置窗口即可注册，成功后自动跳转到「应用」页并记录新增数喵。
- **`Platform::install_cli_command()`** —— 把 exe 目录写入用户 `PATH`（`HKCU\Environment\Path`，
  保留 `REG_EXPAND_SZ` 与既有条目，幂等），并广播 `WM_SETTINGCHANGE`（独立线程 + 200 ms 超时，绝不阻塞启动）喵。
- `app::config::data_dir()` —— 数据目录改为**便携式**，固定为可执行文件同级的 `.datas`喵。

### Fixed

- **热键实时修改不生效** —— `RegisterClassW` 注册的窗口类是进程级的，第二次注册必然失败
  （`ERROR_CLASS_ALREADY_EXISTS`）导致新热键线程直接返回，必须重启才恢复喵。
  修法：类已存在时忽略继续；`RegisterHotKey` 加约 200 ms 轻量重试以避开旧线程销毁竞态喵。
- **Release 启动弹控制台** —— `build.rs` 在 release 档为 `meowal` 加
  `/SUBSYSTEM:WINDOWS` + `/ENTRY:mainCRTStartup`（后者必需，否则 CRT 去找 `WinMain` 会链接失败）；
  debug 档保持 Console 子系统方便看日志喵。
- **Release 下 CLI 输出不可见** —— `cli.rs` 新增 `attach_parent_console()`，
  通过 `AttachConsole(ATTACH_PARENT_PROCESS)` + `CreateFileW("CONOUT$")` + `SetStdHandle` 重绑 stdio；
  双击启动无父控制台时静默跳过喵。
- **拖入文件没反应** —— `DragQueryFileW` 的计数哨兵是 `0xFFFFFFFF`，旧代码传了 `0xFFFF`，
  被当成文件索引从而永远返回 0喵。
- **命令行注册「无效」** —— 数据目录曾基于 `current_dir()`，终端与 GUI 的 CWD 不同会注册到不同的 `.datas`喵。
  统一改用 `data_dir()`（基于 `current_exe()`）后两端口径一致喵。

## [1.3.1] - 2026-09-04

### Added

- **应用图标资产** —— `assets/app_icons/` 提供 16 ~ 1024 多分辨率 PNG 与真正的 6 分辨率 `app-icon.ico`喵。
- **配置 GUI 品牌徽标** —— `include_bytes!` 内嵌 128 px PNG，`Image::from_encoded` 惰性解码 +
  `OnceLock` 缓存 + 圆角裁剪，解码失败兜底齿轮图标喵。
- exe 图标由 `winres` 构建依赖在 `build.rs` 中嵌入（MSVC 走 SDK 的 `rc.exe`）喵。

### Fixed

- **GPU 模式配置 GUI 渲染错乱** —— 双窗口各持一个 WGL 上下文却共享主线程，
  绘制前未确保本上下文 current 会把命令发到另一个窗口喵。
  修法：`Renderer::canvas()` 开头 `gpu.make_current()`喵。
- **GPU → CPU 切换卡死** —— `Drop` 时 Skia Surface / DirectContext 会下发 GL 删除命令，
  上下文非 current 时落到错误上下文导致卡死喵。
  修法：`Drop` 在字段析构前先 `make_current()`，`resize()` 重建前同理喵。
- 预构建 Skia 二进制从 C 盘临时目录归位到项目 `.tmp/skia-binaries/`喵。

## [1.3.0] - 2026-09-03

### Added

- **开机自启** —— `AppConfig.auto_start` + `Platform::set_auto_start`，
  写/删 `HKCU\...\CurrentVersion\Run\MeowAppLauncher`，幂等喵。
- **CPU / GPU 双渲染后端** —— `RenderBackend` 配置可切换，GPU 走 OpenGL/WGL，
  初始化失败自动回退 CPU；`Platform::create_gpu_context` + `GpuContext` trait 保持平台无关喵。
- **测试目录独立** —— 测试从 `src/` 内联搬到独立的 `tests/` 喵。

## [1.0.1] - 2026-09-02

### Changed

- **胶囊形状改为真圆弧** —— `shape::rounded_rect_path` 从超椭圆折线采样换成
  四次 `conic_to` 圆锥曲线（权重 `cos45° ≈ 0.7071068`）喵。
- **锐利文字三件套** —— 字号 `.round()` + `set_subpixel(false)` + `FontHinting::Slight`，
  文本 x / 基线与图标圆心一并像素取整喵。
- **阴影去重影** —— 从「整条模糊路径下移」改为裁剪到岛体下方，并按岛高自适应减弱，
  消除扩展态四周的光晕喵。
- **动画卡顿三重修复** —— 消息泵 `timeBeginPeriod(1)`（位于 `Win32::Media`）、
  定时器仅在档位变换时重设、窗口位置未变时跳过 `SetWindowPos`喵。

## [0.1.0] - 2026-08-22

第一个可运行版本：**Windows 基础功能闭环**喵。

### Added

- 分层架构骨架（`app` / `views` / `ui` / `platform` / `animation` / `search` / `apps` / `utils`），
  `platform/` 为唯一出现 `cfg(target_os)` 的目录喵。
- 弹簧动画内核（半隐式欧拉积分，纯 Rust 可单测）喵。
- Win32 平台层：全局热键、图标提取、`ShellExecuteW` 启动、透明无边框置顶窗口喵。
- 子序列模糊匹配 + 打分 + 拼音索引，支持名称 / `t:` 标签 / `i:` 首字母三种模式喵。
- 开始菜单 `.lnk` 扫描与图标 PNG 快照缓存喵。
- 配置 JSON 持久化与主题喵。

> **架构转折** —— 初版基于 GPUI 构建，但受限于其迭代期的不稳定性
> （例如 `gpui-component` 的 `Root` 会无条件把透明窗口整窗刷成不透明背景），
> 于 2026-08-24 转向 Google Skia 官方绑定 **skia-safe**，改为「单窗口 + 内部自绘」喵。
> 该决策确立了沿用至今的渲染架构与 `platform/` 边界纪律喵。

[Unreleased]: https://github.com/Alkaid2333/meow-app-launcher/compare/v1.7.0...HEAD
[1.7.0]: https://github.com/Alkaid2333/meow-app-launcher/compare/v1.6.1...v1.7.0
[1.6.1]: https://github.com/Alkaid2333/meow-app-launcher/compare/v1.6.0...v1.6.1
[1.6.0]: https://github.com/Alkaid2333/meow-app-launcher/compare/v1.5.0...v1.6.0
[1.5.0]: https://github.com/Alkaid2333/meow-app-launcher/compare/v1.4.0...v1.5.0
[1.4.0]: https://github.com/Alkaid2333/meow-app-launcher/compare/v1.3.3...v1.4.0
[1.3.3]: https://github.com/Alkaid2333/meow-app-launcher/compare/v1.3.2...v1.3.3
[1.3.2]: https://github.com/Alkaid2333/meow-app-launcher/compare/v1.3.1...v1.3.2
[1.3.1]: https://github.com/Alkaid2333/meow-app-launcher/compare/v1.3.0...v1.3.1
[1.3.0]: https://github.com/Alkaid2333/meow-app-launcher/compare/v1.0.1...v1.3.0
[1.0.1]: https://github.com/Alkaid2333/meow-app-launcher/compare/v0.1.0...v1.0.1
[0.1.0]: https://github.com/Alkaid2333/meow-app-launcher/releases/tag/v0.1.0
