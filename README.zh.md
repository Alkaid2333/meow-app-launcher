# Meow App Launcher (meowal) 喵

> 类 macOS 聚焦搜索 (Spotlight) 的跨平台高性能应用启动器喵~
> Rust + GPUI 技术栈,第一阶段已支持 **Windows 基础功能闭环**喵!

## ✨ 功能亮点喵

- 🎯 **全局热键呼出**: 默认 `Ctrl+Alt+Space`,一键呼出灵动岛搜索框喵
- 🔍 **实时模糊搜索**: 支持英文、中文、拼音(全拼/首字母)模糊匹配喵
- 🎯 **自动聚焦最佳匹配**: 输入即搜,自动选中最匹配的应用喵
- ⌨️ **键盘全流程操作**: 方向键移动焦点、回车启动、Esc 隐藏喵
- 🖼️ **自动图标提取**: 从 exe/lnk 提取图标并缓存 PNG 快照喵
- 🏷️ **三种搜索模式**: 名称搜索、`t: 标签` 搜索、`i: 首字母` 搜索喵
- 🎨 **浅色/深色主题**: 内置两套基础主题,完全可自定义喵
- 🗂️ **配置持久化**: 配置/注册表存于 `./.datas` 目录(JSON)喵
- ⚡ **零功耗空闲**: 弹簧动画停稳即退出帧循环喵
- 🪟 **灵动岛双窗口**: 胶囊搜索框 + 分离的选择窗口(有匹配才出现)喵
- 📊 **彩色日志**: loguru 风格彩色控制台 + info/error 分离落盘喵

## 📦 技术栈喵

| 组件 | 版本 | 用途 |
|---|---|---|
| Rust | 1.97+ | 系统编程语言喵 |
| GPUI | 0.2.2 | 高性能 GPU 渲染 UI 框架喵 |
| gpui-component | 0.5.1 | 桌面 UI 组件库(Input 等)喵 |
| windows-sys | 0.59 | Win32 API 绑定(热键/图标/ShellExecute)喵 |
| logforth | 0.30 | 模块化日志(自定义布局/彩色/分离落盘)喵 |

## 🏗️ 架构喵

双窗口架构 + 分层框架喵:

```
src/
├─ main.rs            # 入口: 日志、全局状态、双窗口创建与联动喵
├─ app/               # 应用级状态与生命周期喵
│  ├─ mod.rs          # AppState 全局状态(查询/结果/选中/窗口位置)喵
│  ├─ config.rs       # 配置结构 + JSON 持久化喵
│  └─ actions.rs      # GPUI Action 命令定义喵
├─ views/             # 视图层喵
│  ├─ search_bar.rs   # 灵动岛胶囊搜索框窗口(核心)喵
│  └─ results.rs      # 分离的选择窗口(有匹配才出现)喵
├─ ui/                # 通用 UI 组件喵
│  ├─ theme.rs        # 浅色/深色主题色板喵
│  └─ app_item.rs     # 应用条目组件(图标+名称)喵
├─ platform/          # 平台抽象层(唯一 cfg(target_os) 的地方)喵
│  ├─ mod.rs          # PlatformCapabilities trait 喵
│  └─ win32.rs        # 热键/图标/启动/窗口控制/置顶喵
├─ animation/         # 动画内核(纯 Rust,零依赖,可单测)喵
│  ├─ springs.rs      # 弹簧物理(半隐式欧拉积分)喵
│  └─ mod.rs          # PanelSprings 通用面板动画喵
├─ search/            # 搜索逻辑喵
│  ├─ fuzzy.rs        # 子序列模糊匹配 + 打分喵
│  └─ mod.rs          # 拼音索引 + 三种搜索模式喵
├─ apps/              # 应用注册表喵
│  ├─ mod.rs          # 注册表增删查/持久化喵
│  ├─ scanner.rs      # Windows 开始菜单 .lnk 扫描喵
│  └─ icon.rs         # 图标提取 → PNG 快照缓存喵
└─ utils/             # 工具层喵
   ├─ logger.rs       # logforth 自定义布局/彩色/分离落盘喵
   └─ mod.rs          # 工具模块喵
```

**双窗口交互流程喵**:
1. 按热键 → 胶囊搜索框窗口呼出(置顶 + 激活 + 聚焦)喵
2. 输入关键字 → 搜索框更新全局结果 → 有匹配时选择窗口在搜索框下方弹出喵
3. `↑↓` 移动选中 → 选择窗口高亮同步喵
4. `Enter` / 点击结果 → 启动应用 → 两个窗口同时隐藏喵
5. `Esc` → 全部隐藏喵

**设计纪律喵**:
1. `animation/` 是纯 Rust 数学库,不依赖 GPUI/平台,可单测喵
2. `platform/` 是唯一出现 `cfg(target_os)` 的目录,向上只暴露 trait 喵
3. `views/` 只消费数值层结果(弹簧值、alpha),不自己算动画喵
4. 动画永远可打断: 每帧根据当前状态重新计算喵

## 🚀 构建与运行喵

```bash
# 开发模式(建议开 verbose 看 debug 日志)喵
MEOWAL_VERBOSE=1 cargo run

# 生产模式喵
cargo build --release
./target/release/meowal.exe
```

### 使用喵

1. 首次启动自动扫描系统开始菜单应用(约 200+ 个)喵
2. 按 `Ctrl+Alt+Space` 呼出灵动岛搜索框喵
3. 输入关键字(英文/中文/拼音)实时搜索,有匹配才弹出选择窗口喵
4. `↑`/`↓` 移动选中,`Enter` 启动应用,`Esc` 隐藏喵
5. 支持 `t: 标签名` 按标签搜索、`i: 首字母` 按首字母搜索喵

### 测试喵

```bash
cargo test        # 15 个单元测试(弹簧物理/模糊匹配/拼音搜索)喵
```

### 自动冒烟测试喵

```bash
MEOWAL_TEST_TOGGLE=1 cargo run   # 自动: 呼出 → 搜索 chrome → 清空 → 隐藏喵
```

## 📊 日志喵

loguru 风格彩色日志(基于 logforth 自定义布局)喵:

```text
2026-08-22 22:30:15.123 | INFO     | meowal::views::search_bar - 呼出搜索框喵~
2026-08-22 22:30:15.456 | DEBUG    | meowal::platform::win32 - 窗口可见性: true 喵
```

- 控制台: 彩色输出,时间灰白、level 彩色(INFO绿/WARN黄/ERROR红/DEBUG蓝)、模块青色喵
- 落盘: `./.datas/logs/` 下分离 `info.log`(Info 及以下)和 `error.log`(Error 及以上)喵
- 滚动: 512KB × 3 份喵
- 调试: `MEOWAL_VERBOSE=1` 开启 debug 级别喵

## ⚙️ 配置喵

首次运行生成 `./.datas/config.json`,可手动编辑喵:

| 配置项 | 默认值 | 说明喵 |
|---|---|---|
| `hotkey.modifiers` | `ctrl+alt` | 全局热键修饰键喵 |
| `hotkey.key` | `space` | 全局热键主键喵 |
| `window.width/height` | `640/480` | 选择窗口尺寸(px)喵 |
| `window.layout` | `row` | 应用排列: `row` 行 / `grid` 网格喵 |
| `window.icon_size` | `36` | 图标显示大小(px)喵 |
| `window.show_all` | `true` | 空查询时显示全部应用喵 |
| `theme.mode` | `light` | 主题: `light` / `dark` 喵 |

数据目录 `./.datas`:
- `config.json` - 配置喵
- `apps.json` - 注册的应用喵
- `icons/` - 图标 PNG 快照(文件名 = 应用名)喵
- `logs/` - 分离的滚动日志(info.log / error.log)喵

## 🗺️ 路线图喵

### ✅ 第一阶段(当前): Windows 基础闭环
- [x] 灵动岛双窗口: 胶囊搜索框 + 分离的选择窗口(有匹配才出现)喵
- [x] 透明无边框置顶窗口 + 弹簧呼出动画喵
- [x] 全局热键呼出(RegisterHotKey)+ 呼出自动置顶(TOPMOST)喵
- [x] 首次呼出完整显示(修复 SW_SHOWNOACTIVATE 只显示边框问题)喵
- [x] 应用自动扫描(开始菜单 .lnk) + 图标提取缓存喵
- [x] 英文/中文/拼音模糊搜索 + 三种搜索模式喵
- [x] 键盘导航 + 回车启动 + 点击结果启动 + Esc 隐藏喵
- [x] 配置/注册表 JSON 持久化 + 浅色/深色主题喵
- [x] logforth 彩色日志(loguru 风格)+ info/error 分离落盘喵

### 🔜 第二阶段: 完整桌面体验
- [ ] 系统托盘(打开配置/重启/退出)喵
- [ ] 配置 GUI(窗口位置/大小/主题/热键可视化配置)喵
- [ ] 手动注册应用(命令行 `meowal register` + GUI 拖拽)喵
- [ ] 毛玻璃/云母背景效果喵
- [ ] 最近使用/收藏/最常用统计展示喵
- [ ] 首字母分组 + Tag 管理喵

### 🔭 未来拓展
- [ ] Everything 式文件查找喵
- [ ] QuickLook 式悬浮文件预览喵
- [ ] 搜索引擎接入喵
- [ ] 插件体系 + 快捷脚本指令喵

## 📄 License

MIT
