<div align="center">

<img src="assets/app_icons/1024x1024.png" width="150" alt="Meow App Launcher"/>

# Meow App Launcher

**把 macOS 聚焦搜索装进「灵动岛」的 Windows 应用启动器喵。**

一个热键，万物可达：模糊搜索应用（中 / 英 / 拼音）、算式计算、Web 搜索、系统指令喵。
**Rust + Skia** 全自绘 —— 没有 WebView，没有 HTML，没有运行时依赖喵。

[![Platform](https://img.shields.io/badge/platform-Windows%2010%20%7C%2011-0078D4?logo=windows&logoColor=white)](https://github.com/Alkaid2333/meow-app-launcher)
[![Rust](https://img.shields.io/badge/rust-1.97%2B%20%C2%B7%20edition%202024-dea584?logo=rust&logoColor=white)](https://github.com/Alkaid2333/meow-app-launcher)
[![Tests](https://img.shields.io/badge/tests-111%20passing-3fb950?logo=githubactions&logoColor=white)](https://github.com/Alkaid2333/meow-app-launcher)
[![License](https://img.shields.io/badge/license-MIT-007ec6)](LICENSE)

**[English](README.md) · [简体中文](README.zh.md)**

<img src="assets/examples/basic_show.png" width="860" alt="在桌面壁纸上呼出的灵动岛"/>

<sub>`Ctrl+Alt+Space` 呼出的灵动岛 —— 搜索框与结果面板是同一块超椭圆，随结果展开喵。</sub>

</div>

---

## ✨ 功能亮点

### 搜索

- **全局热键** —— 默认 `Ctrl+Alt+Space`，可自由重绑，改完即时生效喵。
- **按住修饰键提权启动** —— 按住配置好的修饰键（默认 Shift）再确认结果，就以管理员身份运行它喵。
  提权被抽象成**平台能力**：Windows 走 UAC，Linux 后端对应 `sudo`，业务层只问「此刻按着修饰键吗」喵。
- **模糊匹配** —— 中文、英文、拼音（全拼与首字母）三种都支持，带子序列打分喵。
- **三种模式** —— 名称搜索、`t:` 标签搜索、`i:` 首字母搜索喵。
- **中文输入法** —— 组字预览、提交、取消都正确处理喵。
- **统一多源管道** —— 应用、计算器、Web 搜索、系统指令全部走同一套 `SearchItem` / `Action` 模型，
  新增一个搜索源只需要实现一个 trait 喵。

### 内置搜索源

| | |
|---|---|
| 🧮 **计算器** | 直接输入算式（支持 `+ - * / % ^` 与括号、一元负号），shunting-yard 求值，回车复制结果喵。 |
| 🌐 **Web 搜索** | 任意关键词一键跳转百度 / 必应 / 搜狗 / 谷歌 / DuckDuckGo 喵。 |
| 🖥️ **系统指令** | 锁屏、睡眠、关机、重启 —— 别名表**完全可配置**，`熄屏`、`lock`、`睡眠` 可以同时生效喵。 |

### 视觉与交互

- **一体灵动岛** —— 搜索框与结果面板是同一块超椭圆，展开时整体形变喵。
- **弹簧物理动画** —— 六条独立过渡（呼出 / 隐藏 / 展开 / 收起 + 两组扩展态），每条都能单独调时长、回弹与质量；
  动画永远可打断，每帧按当前状态重新求解喵。
- **结果级联入场** —— 呼出与展开时结果逐行错峰浮现，每行晚几十毫秒并带一点向上浮起；
  收起时改为直接定型，淡出不会拖出叠影喵。
- **滚动平滑趋近** —— 滚轮与键盘导航只挪动目标位置，真正的推进交给帧率无关的指数平滑
  （恒定步长下可精确结合），并按阈值吸附，尾巴不会无限爬行喵。
- **三档材质预设** —— 毛玻璃、云母、不透明；文字色与底材**成对解析**，任何预设下对比度都有保证喵。
- **行 / 网格两种排布**，键盘导航全几何化，鼠标悬停自动吸附喵。
- **分层窗上的锐利中文** —— 字号取整 + 关闭次像素定位 + 轻微 hinting 三件套喵。

### 桌面集成

- **系统托盘** —— 图标直接取自应用自身的 `.ico` 资源；资源管理器重启后自动重建喵。
- **子进程永远拿到最新环境** —— 岛是常驻进程，子进程本会继承它启动那一刻的环境块快照，
  于是你改完系统变量后从它打开 Terminal 仍是旧的 `PATH`。现在会重读系统键与用户键、
  按 Windows 的规矩合并（用户 `PATH` 接在系统 `PATH` 之后、会话级的 `TEMP`/`TMP` 不越界）、
  展开 `%引用%`，再在 `WM_SETTINGCHANGE` 与每次启动前差量写回自己的环境块喵。
- **配置 GUI** —— 侧边栏 + 分组卡片，主题、几何、热键、过滤、Web 搜索与指令别名都能可视化编辑喵。
- **单实例保护** —— 再次启动只会唤起已有实例，不会出现双岛互踩喵。
- **开机自启**、**开始菜单扫描**（含图标提取与 PNG 快照缓存）、**命令行 / 拖拽注册**、
  **关键词过滤规则**（隐藏不需要的启动项）喵。

### 渲染与图标

- **逐像素透明分层窗** —— Skia 负责绘制，`UpdateLayeredWindow` 负责呈现，没有合成器把戏、没有假透明喵。
- **默认 CPU 光栅，可选 GPU / OpenGL** —— GPU 初始化失败会自动回退 CPU，而不是直接挂掉喵。
- **统一图标层** —— SVG（支持 viewBox、线稿与填充混排）、PNG、ICO 光栅、字符兜底，
  全部走同一条等比缩放 + 标准内边距的路径喵。

> **现状说明喵**：目前唯一实现的后端是 Windows 10 / 11。动画内核与渲染管线是刻意做成平台无关的
> （见[架构](#-架构)），所以新增后端是「实现 `Platform` trait」而不是「重写一遍」喵。界面文案目前为中文喵。

---

## 📦 环境要求

| | |
|---|---|
| **系统** | Windows 10 / 11 (x64) 喵 |
| **Rust** | 1.97+（edition 2024）—— 仅构建时需要喵 |
| **工具链** | MSVC（`link.exe` + Windows SDK `rc.exe`）喵 |
| **网络** | 首次构建需下载 Skia 预编译二进制（约 100 MB）喵 |

没有运行时依赖，产物是自包含的便携可执行文件喵。

## 🚀 安装

预编译安装包发布在 [Releases](https://github.com/Alkaid2333/meow-app-launcher/releases) 页面
（由 Inno Setup 打包，脚本见 `template/release-build-template.iss`）喵。

### 从源码构建

```bash
git clone https://github.com/Alkaid2333/meow-app-launcher.git
cd meow-app-launcher
cargo build --release
# -> target/release/meowal.exe
```

首次构建会自动下载 Skia 预编译二进制喵。若访问 GitHub Releases 不稳定，可以改用本地缓存绕过网络
（同时也能避开需要 LLVM 的源码编译回退路径）喵：

```bash
SKIA_BINARIES_URL="file://X:/path/to/skia-binaries-<key>.tar.gz" cargo build --release
```

### 开发

```bash
cargo run                      # debug 构建，带控制台
MEOWAL_VERBOSE=1 cargo run     # 顺带打开 debug 级日志
cargo test                     # 111 个测试
```

## 🕹️ 使用

1. 启动 `meowal.exe`，首次运行会扫描系统开始菜单（通常 200+ 个条目）并建立图标缓存喵。
2. 按 `Ctrl+Alt+Space` 呼出灵动岛喵。
3. 随便输入 —— 英文、中文、拼音、算式、或者一个关键词都行喵。
4. `↑` / `↓` 移动选中，`Enter` 执行，`Esc` 隐藏喵。
5. 右键托盘图标打开配置窗口喵。

### 搜索语法

| 输入 | 结果喵 |
|---|---|
| `chrome` | 模糊匹配应用喵 |
| `计算器` | 按中文名匹配喵 |
| `jsq` | 按拼音首字母匹配喵 |
| `t: 开发` | 按标签过滤喵 |
| `i: p s` | 按首字母过滤喵 |
| `12 * (3 + 4)` | 计算器 → 回车复制 `84` 喵 |
| `rust error E0308` | 用配置的引擎做 Web 搜索喵 |
| `锁屏` / `lock` | 执行系统指令喵 |

### 命令行

```bash
meowal register "记事本" "C:\Windows\notepad.exe"
meowal register "我的工具" "D:\tools\tool.exe" -ico "D:\tools\icon.png"
```

注册是幂等的，并且写入与 GUI 相同的数据目录喵。CLI 是同一个可执行文件的另一个入口，
这也是它需要重新附加父控制台的原因（见 `src/cli.rs`）喵。

## 🏗️ 架构

```
src/
├── main.rs            # 入口: 日志、数据目录、装配、消息循环喵
├── lib.rs             # 库根 —— 可测试的核心喵
├── cli.rs             # `meowal register` 命令行喵
├── app/
│   ├── mod.rs         # AppState: 共享状态 + 命令队列喵
│   └── config.rs      # 配置结构 + JSON 持久化喵
├── window/
│   ├── launcher.rs    # 灵动岛窗口: 状态机 + 事件分发喵
│   ├── settings.rs    # 配置窗口喵
│   └── tray.rs        # 系统托盘图标 + 菜单喵
├── render/
│   ├── mod.rs         # Renderer: Skia surface 生命周期, CPU/GPU 后端喵
│   ├── layout.rs      # 纯函数布局计算(可单测)喵
│   ├── theme.rs       # 主题预设 —— 唯一定义颜色的地方喵
│   ├── shape.rs       # 连续曲率(超椭圆)几何喵
│   ├── font.rs        # 字体缓存喵
│   ├── text.rs        # 文本绘制辅助喵
│   ├── paint.rs       # 岛场景: 搜索框、面板、条目喵
│   ├── settings.rs    # 配置 GUI 绘制喵
│   ├── edit.rs        # 通用文本编辑(文本 + 光标 + 选区)喵
│   ├── icon.rs        # 统一图标层(IconSource)喵
│   └── svg.rs         # 轻量内嵌 SVG 渲染器喵
├── platform/
│   ├── mod.rs         # Platform trait —— 唯一的平台边界喵
│   ├── env.rs         # 环境变量合并 / 展开 / 差量(纯逻辑)喵
│   └── win32.rs       # Win32 实现喵
├── animation/
│   ├── mod.rs         # 启动器弹簧喵
│   ├── springs.rs     # 弹簧物理(纯数学,零依赖)喵
│   └── island.rs      # 岛体几何状态机喵
├── search/
│   ├── mod.rs         # 拼音索引 + 搜索入口喵
│   ├── fuzzy.rs       # 子序列模糊匹配 + 打分喵
│   ├── item.rs        # SearchItem / Action 模型喵
│   ├── provider.rs    # SearchProvider trait + 内置搜索源喵
│   └── calc.rs        # 计算器(shunting-yard)喵
├── apps/
│   ├── mod.rs         # 应用注册表: 增删查 / 持久化喵
│   ├── scanner.rs     # 开始菜单 .lnk 扫描喵
│   └── icon.rs        # 图标提取 -> Skia Image + PNG 缓存喵
└── utils/
    ├── mod.rs
    └── logger.rs      # logforth 布局、彩色、分离落盘喵
```

`tests/` 下有 17 个测试文件、共 111 个测试，覆盖弹簧物理、模糊匹配、搜索、布局、圆角形状、
岛体状态机、SVG 渲染、文本编辑、CLI 参数解析、环境变量两级合并与展开、资源完整性与 GPU 冒烟喵。

### 设计纪律

这些不是口号，而是由模块划分强制保证的喵：

1. **`animation/` 是纯数学。** 不碰 Skia、不碰平台调用、不做 I/O，完全可单测喵。
2. **`platform/` 是唯一允许出现 `cfg(target_os)` 的地方。** 它之上的一切只跟 trait 打交道喵。
3. **`render/` 只消费数值，绝不自己算动画。** 弹簧和布局在别处解开，渲染器只负责画出递给它的那一帧喵。
4. **动画永远可打断。** 每帧都从当前状态重新求解，而不是按时间轴回放喵。
5. **窗口是逐像素透明的分层窗。** Skia 光栅化，`UpdateLayeredWindow` 呈现喵。
6. **慢操作绝不阻塞消息循环。** 扫描与图标提取走 tokio 阻塞线程池，渲染路径只读缓存喵。
7. **组件之间通过命令队列通信**（`Rc<RefCell<AppState>>` + `VecDeque<Command>`），
   这是多窗口协调没有变成一团乱麻的关键喵。

## ⚙️ 配置

配置位于可执行文件同级的 `./.datas/config.json`（便携式 —— 目录由 `current_exe()` 解析，
所以 GUI 与 CLI 永远指向同一份数据）喵。下表所有项都能在配置 GUI 里可视化编辑喵。

| 配置项 | 默认值 | 说明喵 |
|---|---|---|
| `hotkey.enabled` | `true` | 是否启用全局热键喵 |
| `hotkey.modifiers` | `"ctrl+alt"` | `ctrl` / `alt` / `shift` / `win`，可用 `+` 组合喵 |
| `hotkey.key` | `"space"` | 触发键喵 |
| `elevate_modifier` | `"shift"` | 按住它以管理员身份启动：`disabled` / `shift` / `ctrl` / `alt` / `win` 喵 |
| `window.layout` | `"row"` | `row` 行 / `grid` 网格喵 |
| `window.icon_size` | `36.0` | 图标尺寸(px)喵 |
| `window.show_recent` | `false` | 显示最近使用喵 |
| `window.show_favorites` | `false` | 显示收藏喵 |
| `window.show_frequent` | `false` | 显示最常用喵 |
| `window.show_all` | `true` | 空查询时显示全部应用喵 |
| `theme.preset` | `"frosted_glass"` | `frosted_glass` 毛玻璃 / `mica` 云母 / `opaque` 不透明喵 |
| `search.default_mode` | `"name"` | `name` / `tag` / `initial` 喵 |
| `search.filters` | `卸载`、`uninstall` | 命中关键词的启动项会被隐藏喵 |
| `search.web_engine` | `"baidu"` | `baidu` / `bing` / `sogou` / `google` / `duckduckgo` 喵 |
| `search.web_browser` | `""` | 空 = 系统默认；支持带引号的路径与 `%1` 占位符喵 |
| `search.commands` | 4 条 | 指令别名表（`aliases` + `kind`），可自由增删改喵 |
| `island.*` | 见下表 | 岛体几何与动画喵 |
| `auto_start` | `false` | 写 Windows `Run` 键实现开机自启喵 |
| `render_backend` | `"cpu"` | `cpu` / `gpu`，GPU 失败自动回退 CPU 喵 |

<details>
<summary><code>island.*</code> 默认值喵</summary>

| 配置项 | 默认值喵 |
|---|---|
| `width` / `height` | `300` / `46` |
| `x` / `y` | `50` / `14` |
| `expandedWidth` / `expandedHeight` | `560` / `268` |
| `inputRatio` | `0.72` |
| `expandedRadius` | `34` |
| `padX` / `padY` | `18` / `15` |
| `slotHeight` | `34` |
| `summonSquash` | `0.34` |
| `margin` | `16` |
| `animFps` | `0`（跟随显示器刷新率）喵 |
| `motionMode` | `"spring"` —— `spring` / `ease` / `linear` / `instant` 喵 |
| `easing` | `ease_out_quint` |
| `reduceMotion` | `false` |
| `autoMorph` / `draggable` | `true` / `true` |
| `springs` | 每条过渡各自的 `{ duration, bounce, mass }` 喵 |

</details>

### 数据目录

```
.datas/
├── config.json     # 配置喵
├── apps.json       # 注册的应用喵
├── icons/          # 图标 PNG 快照缓存喵
└── logs/
    ├── info.log    # Info 及以下喵
    └── error.log   # Error 及以上喵
```

## 📊 日志

基于 `logforth` 自定义布局的 loguru 风格彩色日志喵：

```text
2026-09-10 19:40:12.345 | INFO     | meowal::window::launcher - 呼出搜索框喵~
2026-09-10 19:40:12.678 | DEBUG    | meowal::platform::win32 - 窗口创建成功喵
```

- 控制台：彩色输出，level、模块、消息各自着色喵。
- 落盘：512 KB × 3 份滚动，分离成 `info.log` 与 `error.log` 喵。
- 设 `MEOWAL_VERBOSE=1` 可打开 debug 级输出喵。

## 🧪 测试

```bash
cargo test
```

共 111 个测试喵。核心测试套件不需要 GPU 或显示器（GPU 那条会降级成冒烟检查）喵。

## 🗺️ 路线图

第一阶段与第二阶段（功能闭环 + 完整桌面体验）已完成，第三阶段 a（搜索中枢）接近收尾喵。
完整规划见 **[ROADMAP.md](ROADMAP.md)**，各版本交付内容见 **[CHANGELOG.md](CHANGELOG.md)** 喵。

## 🤝 参与贡献

欢迎提 Issue 和 PR 喵。动手前有两件事值得知道：

- `animation/` 与 `layout.rs` 是纯函数且带单测 —— 改这里请一并补测试喵。
- `assets/icons/*.svg` 是用户可以自行替换的素材；改 SVG 渲染器时请记得它只支持一个刻意做小的子集，
  不是完整的 SVG 实现喵。

## 💐 致谢

- **深色主题**的配色体系（窗口 / 侧边栏 / 分组卡片 / 控件色色）借鉴自 **WinIsland** —— 一个基于 Rust + skia + D3D 的灵动岛实验项目喵
  [WinIsland](https://github.com/WinIslandProject/WinIsland "WinIsland 项目仓库喵")

## 📄 License

[MIT](LICENSE) © 2026 若有人兮233
