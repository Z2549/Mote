# Mote · 项目记忆

最后验证：2026-11-21（7 个测试通过，Windows / Android 编译通过，clippy 无警告，`dist` 含 `Mote.exe` 与已签名的 `mote.apk`）

## 目标

电脑用 Chrome 刷抖音时，同局域网手机当遥控器。Rust 单一 workspace，两个端：

- **Windows**：常驻小窗口，接收指令并注入 Chrome。
- **Android**：全屏四键遥控，自动发现 PC，零配置。

## 不可破坏的约束

这些来自用户，优先级高于一切实现偏好：

> 像 Linear 官网或苹果备忘录一样干净：目标简单、架构简单、设计简单、实现简单。
> 砍掉标语、提示文字和次要标签。如果一个页面非得靠这些东西才看得懂，那就去改布局。
> 每个决定都要同时提升用户体验（UX）、开发者体验（DX）和智能体体验（AX），而且不能弄坏任何东西。

因此：

- 不新增设置页、连接页、引导页、标签文字或口号。
- 状态优先用图形与布局表达，不靠说明文字。唯一例外：离线时的 `IP:端口` 诊断信息、端口占用的系统错误框。
- 新功能要先证明不破坏「启动即连、按键即用」。
- 保持零构建脚本、零代码生成、无多余抽象。

## 架构

| 路径 | 职责 |
| --- | --- |
| `crates/core` | 协议、UDP 发现、TCP 服务端/客户端、重连 |
| `crates/ui` | 唯一视觉系统：配色、箭头、F/R/X 字形、状态点、发丝线 |
| `crates/desktop` | Windows only：eframe 窗口 + 托盘 + Chrome 定位 + 按键注入 |
| `crates/mobile` | Android only：cdylib 入口 + eframe 六键界面 |
| `dist/Mote.exe` | Windows 发布版（未签名，代码改动后需重编） |

依赖方向：`desktop/mobile → ui → core`。Windows API 只允许出现在 `crates/desktop`，Android 入口只允许出现在 `crates/mobile`。

## 交互契约

- 手机只用下半屏：两列三行，左列 `F / R / X`，右列 `↑ / ↓ / →`；`→` 在屏幕右下角；上半屏留白。
- 触摸按下立即发送并整格反色；唯一例外是 `→`：按下先发一次 `right`（视频后跳几秒），按住超过 300ms 后每 100ms 重复 `right` 连续快进。
- 下半屏顶部右侧 3px 状态点：实心 accent = 已连接，gray 描边 = 未连接。
- PC 窗口 340×220：居中状态点；收到指令时对应字形放大浮现并淡出；仅离线时底部显示 `IP:port`。
- 最小化即隐藏到托盘；托盘图标左键恢复、右键菜单「显示 / 退出」。
- 无鉴权、无配对、无手动 IP 输入——这是为了零配置刻意保留的取舍。

## 网络协议

- 发现：UDP `45871`。手机广播 `mote?`，PC 单播回复 `mote <control_port>\n`。
- 控制：TCP `45872`。一行一个命令，未知行静默忽略：
  `prev` / `next` / `right` / `author` / `dislike` / `x`
- 客户端启动即连接；发送失败时重新发现、重连，并把当前命令重发一次；网络线程与 UI 线程分离。
- 发现同时发往 `255.255.255.255` 和 `127.0.0.1`；后者用于本机联调与测试。
- `Server::start` 支持 `control_port = 0`（测试取空闲端口），实际端口通过 `Server::control_port()` 读取。
- 明文 ASCII 协议可手工调试：`printf 'next\n' | nc <pc> 45872`。

## Windows 实现

- 查找可见的 `Chrome_WidgetWin_1`：优先标题含「抖音」或「douyin」，其次当前前台 Chrome，最后任意 Chrome。
- 最小化则 `ShowWindow(SW_RESTORE)`；用 `AttachThreadInput` 辅助 `SetForegroundWindow`。
- 注入：`SendInput` 发送按下/抬起，`wVk` 映射为 `VK_UP / VK_DOWN / VK_RIGHT / VK_F / VK_R / VK_X`；必须同时填 `wScan`（`MapVirtualKeyW`），方向键还要带 `KEYEVENTF_EXTENDEDKEY`，否则 Chrome 的 `event.code` 会是空值或 `Numpad6`，抖音会忽略。
- 置前失败或 `SendInput` 返回 0 时，回退 `PostMessage(WM_KEYDOWN/WM_KEYUP)`，带 scan code。
- 托盘：`Shell_NotifyIconW` + 独立消息窗口线程；`egui::App::logic` 检测最小化后发 `ViewportCommand::Visible(false)`，恢复时先 Win32 `ShowWindow` 再同步 egui 视口状态。
- 端口绑定失败弹原生错误框后退出，避免无提示地“看起来在运行”。

## Android 实现

- eframe 0.37 + glow + `android-native-activity`；`crates/mobile` 仅 `cdylib`，不能有 bin/example，否则 cargo-apk 会误选 artifact。
- 入口：`#[unsafe(no_mangle)] pub fn android_main(AndroidApp)`，`NativeOptions.android_app = Some(app)`。
- 包名 `net.mote.remote`，minSdk 26，targetSdk 34，竖屏，保留状态栏（`MoteTheme`：纸色状态栏 + 深色图标），仅 `INTERNET` 权限。
- 打包：`cargo install cargo-apk`（已弃用但可用，后继为 xbuild），并设置 `ANDROID_HOME` / `ANDROID_NDK_HOME`。
- `dist/mote.apk`：arm64 release 成品，约 2.3 MB。签名走 `CARGO_APK_RELEASE_KEYSTORE` 与 `CARGO_APK_RELEASE_KEYSTORE_PASSWORD`，keystore 在 `%USERPROFILE%\.android\mote-release.jks`，不入库。

## 验证命令

```bash
cargo test -p mote-core

# 本机预览/类型检查需要 X11 后端；Android 正式构建不需要这一项
cargo check -p mote-mobile --features eframe/x11

cargo check -p mote-mobile --target aarch64-linux-android

# Android 成品（需 SDK/NDK 与 cargo-apk；输出 target/release/apk/mote.apk）
cargo apk build --manifest-path crates/mobile/Cargo.toml \
  --release --target aarch64-linux-android

cargo clippy -p mote-core -p mote-ui -p mote-mobile \
  --features eframe/x11 --all-targets -- -D warnings
cargo clippy -p mote-desktop --target x86_64-pc-windows-gnu -- -D warnings

# Windows 发布版（Linux 交叉编译；在 Windows 本机可去掉 --target）
cargo build --release -p mote-desktop --target x86_64-pc-windows-gnu
```

工具链：edition 2024；eframe/egui 0.37 要求 Rust ≥ 1.95。

## 已知边界

- 局域网内无鉴权，同网段任何设备都能控制。
- 访客 Wi-Fi / AP 隔离会阻断广播发现。
- PC 消失后，手机可能短暂显示「已连接」，下一次按键时纠正。
- 首次运行需在 Windows 防火墙中允许「专用网络」。
- 不保持手机屏幕常亮、无震动、无可配置键位。
- 端口固定；被占用时 PC 端弹错并退出。
- APK 的签名、图标打包依赖本机 SDK/NDK，不在 Rust 代码内。

## 改动守则

1. 改变命令名或语义，必须同步更新 `Command::wire/parse`、VK 映射、`Command::ALL` 顺序和测试。
2. 新增状态优先改布局或图形；不要用文案兜底。
3. 视觉常量只在 `crates/ui` 定义，不在各端复制颜色。
4. 平台 API 不进入 `core` / `ui`；保持 `core` 可在任意桌面平台测试。
5. 任何改动后重跑「验证命令」；涉及 Windows 端时重编 `dist/Mote.exe`。
6. 修改架构、端口、交互或边界时，同步更新本文件。
