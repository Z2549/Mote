# Mote

Chrome 刷抖音，手机当遥控器。同一局域网，打开即连，按下即用。

Rust 单 workspace，两个端：Windows 常驻小窗负责把按键注入 Chrome，Android 全屏六键负责发指令。

## 下载

到 [Releases](https://github.com/Z2549/Mote/releases/latest) 取最新版本：

| 文件 | 平台 |
| --- | --- |
| `Mote.exe` | Windows 10/11 x64，免安装 |
| `mote.apk` | Android 8.0+，arm64 |

也可以直接使用仓库里的 `dist/Mote.exe` 与 `dist/mote.apk`，两者与 release 资产一致。

## 使用

1. 电脑运行 `Mote.exe`。首次运行若弹出防火墙提示，允许「专用网络」。
2. 手机安装 `mote.apk`，连到同一个 Wi-Fi。
3. 打开手机端，右下状态点亮起即已连上，直接按键。

电脑端窗口 340×220，最小化即隐藏到托盘，托盘左键恢复、右键「显示 / 退出」。

## 按键

手机只用下半屏，两列三行：

| 左列 | 作用 | 右列 | 作用 |
| --- | --- | --- | --- |
| `F` | 进入 / 退出作者主页 | `↑` | 上一个视频 |
| `R` | 不感兴趣 | `↓` | 下一个视频 |
| `X` | 原样透传 `X` | `→` | 快进几秒；按住 300ms 后每 100ms 连发 |

## 协议

明文 ASCII，一行一个命令，方便手工调试。

- 发现：UDP `45871`。手机广播 `mote?`，电脑单播回复 `mote <control_port>\n`。
- 控制：TCP `45872`。命令为 `prev` / `next` / `right` / `author` / `dislike` / `x`，未知行静默忽略。

```bash
printf 'next\n' | nc <电脑IP> 45872
```

## 构建

要求 Rust ≥ 1.95（edition 2024，eframe/egui 0.37）。

```bash
cargo test -p mote-core

# Windows 发布版
cargo build --release -p mote-desktop

# Android（需 ANDROID_HOME / ANDROID_NDK_HOME 与 cargo-apk）
cargo install cargo-apk
cargo apk build --manifest-path crates/mobile/Cargo.toml \
  --release --target aarch64-linux-android
```

Android 产物签名可用 `CARGO_APK_RELEASE_KEYSTORE` 与 `CARGO_APK_RELEASE_KEYSTORE_PASSWORD` 指定 keystore。

## 结构与边界

```
crates/core    协议、UDP 发现、TCP 服务端/客户端、重连
crates/ui      唯一视觉系统：配色、字形、状态点
crates/desktop Windows only：eframe 窗口 + 托盘 + Chrome 定位 + 按键注入
crates/mobile  Android only：cdylib 入口 + eframe 六键界面
```

- 局域网内无鉴权，同网段任何设备都能控制；这是为「零配置」刻意保留的取舍。
- 访客 Wi-Fi / AP 隔离会阻断广播发现，此时手机保持未连接。
- 端口固定，被占用时电脑端弹原生错误框后退出。
- `Mote.exe` 未签名，Windows SmartScreen 可能提示；`dist/mote.apk` 已签名。

## License

[MIT](LICENSE)
