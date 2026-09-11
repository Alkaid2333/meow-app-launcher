# fix/release 分支更新记录 —— Release 拖拽注册失效 + 应用栏点击偏移修复

> 分支：`fix/release` · 基线：`3a682da` · 日期：2026-09-11

---

## 一、现象

两个问题均出现在**打包安装、以 release 模式运行**的配置 GUI 中：

1. **拖拽注册失效**：把 `.lnk` / `.exe` 等文件拖入配置窗口，应用注册无反应；
   debug 直接运行时正常，安装后（尤其以提升权限运行时）稳定复现。
2. **应用栏点击偏移**：在「应用」页点击某个应用行，被点击的应用不会被选中，
   反而选中其**上方间隔若干个**的应用；列表越靠后偏移越明显。

---

## 二、根因

### 2.1 拖拽注册失效 —— UIPI 拦截 `WM_DROPFILES`

配置窗口通过 `DragAcceptFiles` 接收拖入文件。但 Windows 的 **UIPI（用户界面特权隔离）**
规定：**低完整性级别进程无法向高完整性级别窗口投递消息**。

打包安装后，若应用以管理员/更高权限运行（例如安装到 `Program Files` 后由提权进程拉起），
而资源管理器仍是普通权限，拖拽产生的 `WM_DROPFILES` 会在系统消息过滤器层被直接拦截，
窗口根本收不到消息 —— 于是「拖入注册失效」。

原实现只调用了 `DragAcceptFiles`，**没有对消息过滤器放行**，这是 release 环境才暴露的原因。

### 2.2 应用栏点击偏移 —— 命中索引与查表索引计数口径不一致

配置 GUI 的命中测试分两层：

- **绘制层**（`src/render/settings.rs`）：遍历页面行，只对 `AppPick` 行递增 `pick_i`，
  把 `(矩形, RowHit::AppPick(pick_i))` 写入命中区 —— 即 `pick_i` 是「**第几个应用行**」。
- **交互层**（`src/window/settings.rs`）：`app_name_at(index)` 收到命中索引后，
  却对**页面内所有行**（按钮、标签、过滤行……）统一计数 `idx`，再试图匹配第 `index` 个。

「应用」页在 AppPick 行之前还有「关键词过滤」组 + 「注册」组的 3 个按钮/标签行，
两套计数口径错位，导致命中索引 `i` 被当成「全页行号」去查，取回的是上方偏移处的应用。

`chip_label_at`（标签芯片）存在完全相同的口径错误，一并修复。

---

## 三、修复

共改动 2 个文件。

### 3.1 `src/platform/win32.rs` —— `enable_file_drop` 增加 UIPI 放行

```rust
pub fn enable_file_drop(&self, window: &PlatformWindow) {
    use windows_sys::Win32::UI::Shell::DragAcceptFiles;
    use windows_sys::Win32::UI::WindowsAndMessaging::{ChangeWindowMessageFilterEx, MSGFLT_ALLOW};
    unsafe {
        let hwnd = window.hwnd() as HWND;
        DragAcceptFiles(hwnd, 1);
        // UIPI 放行喵: 打包安装后若以更高完整性级别运行,低权限资源管理器
        // 拖入的 WM_DROPFILES 会被系统消息过滤器拦截,导致拖拽注册失效喵。
        ChangeWindowMessageFilterEx(hwnd, WM_DROPFILES, MSGFLT_ALLOW, null_mut());
    }
}
```

`ChangeWindowMessageFilterEx(hwnd, WM_DROPFILES, MSGFLT_ALLOW, ...)` 把 `WM_DROPFILES`
加入本窗口的消息白名单，使低权限进程（资源管理器）的拖拽消息可以穿透 UIPI 送达。

### 3.2 `src/window/settings.rs` —— 索引计数对齐

`app_name_at` / `chip_label_at` 改为**只数同类行**（`AppPick` / `Chip`），
与绘制层 `pick_i` / `chip_i` 的计数口径完全一致：

```rust
/// 按命中索引取应用名喵(只数 AppPick 行,与绘制层 pick_i 计数一致)喵
fn app_name_at(&self, index: usize) -> Option<String> {
    let page = self.pages.get(self.current_page)?;
    let mut idx = 0;
    for group in &page.groups {
        for row in &group.rows {
            if let SettingsRow::AppPick { name, .. } = row {
                if idx == index {
                    return Some(name.clone());
                }
                idx += 1;
            }
        }
    }
    None
}
```

`chip_label_at` 同理（只数 `Chip` 行）。

> 无 API 行为变更、无数据结构变更、无配置变更、无版本号变更。

---

## 四、覆盖路径

| 场景 | 路径 | 覆盖 |
| --- | --- | --- |
| 拖拽注册（普通权限） | 资源管理器拖入 → `WM_DROPFILES` → `drop_files` | ✅（原有逻辑保留） |
| 拖拽注册（提升权限） | 同上，经 `ChangeWindowMessageFilterEx` 放行 | ✅（本次新增） |
| 应用行点选 | `RowHit::AppPick(i)` → `pick_app(i)` → `app_name_at(i)` | ✅（口径对齐） |
| 标签芯片删除 | `RowHit::Chip(i)` → `remove_chip(i)` → `chip_label_at(i)` | ✅（口径对齐） |

---

## 五、验证

| 项目 | 结果 |
| --- | --- |
| `cargo check`（dev） | 通过，零错误 |
| `cargo check --release` | 通过，零错误（复用本地 skia 预编译缓存） |
| `cargo test` | **全部通过**，0 失败 |

> UIPI 属系统级窗口行为，无法用单元测试覆盖；点击偏移的索引口径以
> 「绘制层 `pick_i` 与交互层 `app_name_at` 逐行对照」确认一致。实机验证需在
> 打包安装后（含提升权限场景）手动拖入文件与点选应用行确认。

---

## 六、遗留提醒

1. **release 构建依赖 skia 预编译缓存**：本机无 LLVM，`skia-bindings` 无法源码编译。
   本次验证通过重建 `.tmp/skia-binaries` 本地缓存完成；正式打包请确保缓存可用或
   网络可达预编译源。
2. **`fix/release` 分支原为孤儿分支**（无任何提交），已将其基点指回 `main`（`3a682da`）
   后提交，历史链恢复正常。

---

## 七、改动清单

```
 M src/platform/win32.rs   |  7 insertions(+), 1 deletion(-)
 M src/window/settings.rs  | 22 insertions(+), 11 deletions(-)
 A fix-release-update.md
```
