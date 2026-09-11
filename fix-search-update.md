# fix/search 分支更新记录 —— 唤出搜索「打不进字」修复

> 分支：`fix/search` · 基线：`3a682da` · 日期：2026-09-11

---

## 一、现象

唤出搜索框后窗口**确实弹出来了**，但键盘输入完全无效，同时系统播放一声「无效输入」提示音。

用户侧观察到的触发条件：

1. 首次使用搜索打开某个应用，该应用启动后再次唤出 —— **稳定复现**；
2. 某些应用位于前台时唤出 —— 偶现；
3. 与具体应用类型无稳定关联；
4. 切换输入法、点击/切换其他应用页面后 —— **自行恢复**。

---

## 二、根因

一句话：**窗口成了「前台窗口」，键盘焦点却留在别的线程 —— 输入被系统当作无效输入丢弃。**

### 2.1 Win32 的输入派发规则

键盘消息先送到**焦点窗口（focus window）**。若焦点窗口不属于**前台线程**，
系统不会把它转交给前台窗口，而是直接丢弃并播一声系统提示音。

也就是说，`GetForegroundWindow() == 本窗口` **并不等于**「打得进字」；
真正的入场券是 `GetFocus() == 本窗口`。

### 2.2 本项目的呼出链路

窗口是 `WS_EX_LAYERED | WS_EX_TOPMOST | WS_EX_TOOLWINDOW` + `WS_POPUP` 的自绘分层置顶窗，
呼出时依次发生两件事：

| 时机 | 线程 | 动作 |
| --- | --- | --- |
| 收到 `WM_HOTKEY` | 热键线程 | `SetForegroundWindow(target)`（此刻持有系统给的前台特权） |
| 收到 `WM_MEOW_HOTKEY` | 主线程 | `Launcher::show()` → `Platform::focus_window()` |

问题出在 `focus_window` 的早退条件：

```rust
// 修复前
if GetForegroundWindow() == hwnd {
    return; // ← 只看前台窗口，不看键盘焦点
}
```

于是「前台到位、焦点漂移」这一状态被误判为**已就绪**，直接 `return`，焦点再也没被收回。

### 2.3 焦点为什么会漂移

- `SetForegroundWindow` 由**另一个线程**（热键线程）发起，跨线程/跨进程的激活链路下，
  系统不保证把键盘焦点同步落到新的前台窗口；
- 通过 `ShellExecuteW` 启动外部应用时存在激活竞争，焦点归属更易被外部进程带走；
- 而原实现**全程没有一次显式 `SetFocus`**，一旦漂移就无人回收。

这同时解释了现象 4：点击窗口、切换应用、切换输入法都会触发系统重新分配焦点，
输入随即恢复 —— 因为「修复」的动作恰好就是补上那一次 `SetFocus`。

---

## 三、修复

仅改动 `src/platform/win32.rs`，共 3 处。

### 3.1 `Win32Platform::focus_window` —— 就绪判定升级为「前台 + 焦点」双确认

```rust
// 前台与键盘焦点双双到位，才算真的「就绪」喵
if GetForegroundWindow() == hwnd && GetFocus() == hwnd {
    return;
}
// …（原有 AttachThreadInput 前台锁自救逻辑保留）…
let foreground_ok = SetForegroundWindow(hwnd) != 0;
if attached { AttachThreadInput(cur_thread, fg_thread, 0); }

if foreground_ok {
    // 前台到手后立刻把键盘焦点钉回本窗：这一下才是「打得进字」的关键喵
    BringWindowToTop(hwnd);
    SetFocus(hwnd);
}
```

- 早退条件补上 `GetFocus()`；
- 抢到前台后立刻 `BringWindowToTop` + `SetFocus`，把键盘焦点钉回本窗；
- 抢前台失败时不再静默，`log::debug!` 说明已尝试附加输入线程、待点击自救。

### 3.2 `wnd_proc` 的 `WM_ACTIVATE` —— 激活时补钉焦点作为兜底

```rust
WM_ACTIVATE => {
    let active = (wparam as u32 & 0xFFFF) != 0; // WA_INACTIVE = 0 喵
    if active {
        // 分层置顶窗被激活时键盘焦点偶有漂移，这里补钉一次喵
        unsafe { SetFocus(hwnd) };
    } else {
        with_window_handler(hwnd, |h| h.on_event(WindowEvent::LostFocus));
    }
    0
}
```

不论激活由谁发起（热键、托盘、鼠标点击），只要窗口被激活就顺手确认一次焦点。

### 3.3 注释诚实化（无行为变更）

- `show_window` 的注释原写作 `SW_SHOWNA`，实现为 `SW_SHOW` —— 修正为与实现一致；
- 热键线程补注「此刻本线程握有前台特权」的缘由，避免后人再把这段挪走。

> 无 API 行为变更、无数据结构变更、无配置变更、无版本号变更。

---

## 四、覆盖路径

| 场景 | 呼出路径 | 覆盖 |
| --- | --- | --- |
| 全局热键 | 热键线程 `SetForegroundWindow` → 主线程 `show()` → `focus_window` | ✅ |
| 托盘左键 | `Command::ToggleLauncher` → `toggle()` → `show()` | ✅ |
| 点击岛体 | `on_mouse_down` → `focus_window` | ✅（原有自救保留） |

---

## 五、验证

| 项目 | 结果 |
| --- | --- |
| `cargo check --all-targets` | 通过，零警告零错误 |
| `cargo clippy`（lib 目标） | 通过，无告警 |
| `cargo test --all-targets` | **66 个测试全部通过**，0 失败 |

> Win32 的焦点/激活属系统级窗口行为，无法用单元测试覆盖；本次以
> 「API 签名逐条核对 + 全量回归」双重手段把关。实机验证仍需在有音频设备的桌面上，
> 手动复现上述 4 个场景确认提示音消失、输入即时生效。

---

## 六、遗留提醒

1. **`tests/` 存在历史 clippy 告警**（`field_reassign_with_default`、`excessive_precision`、
   `zero_divided_by_zero`），在 `cargo clippy -- -D warnings` 下会导致测试目标编译失败。
   与本次修复无关，建议后续单独清理。
2. **本机 cargo 构建脚本链路会触发 `reg.exe`**，被沙箱程序黑名单拦截，导致
   `cargo build/check` 无法直接跑通。临时以「跳过 winres 资源嵌入」的方式完成验证，
   验证后已还原 `build.rs`（工作区无残留）。正式的 `cargo build` 请在允许 `reg.exe`
   的环境执行。

---

## 七、改动清单

```
 M src/platform/win32.rs   |  28 insertions(+), 10 deletions(-)
 A fix-search-update.md
```
