# drag-share · SMB 共享助手

macOS 菜单栏中的 SMB 共享工具。点击图标或按 **Control + Option + S**，管理本机共享目录、复制局域网地址，用完关闭服务，目录配置继续保留。

基于 Tauri 2、Vue 3、TypeScript 和 Rust。首发支持 **macOS 13+**；当前交付包为 **Apple Silicon (arm64)**。Windows / Linux 仅预留适配接口，尚不提供 SMB 功能。

## 使用

1. 打开 `SMB 共享助手.app`，应用仅驻留菜单栏，不显示 Dock 图标。
2. 点击菜单栏图标，再点击「开启共享」，在 macOS 原生授权框中完成授权。
3. 点击「＋ 添加」选择文件夹，或将 Finder 中的文件夹拖入面板底部。系统可能再次要求授权。
4. 双击目录或打开「··· → 复制目录链接」，在同一局域网设备的 SMB 客户端中连接；顶部「复制」获取本机地址。
5. 「关闭共享」停止服务并保留目录配置。「取消共享」仅注销选定共享，绝不删除本地文件。

首次使用时，请确认「系统设置 → 通用 → 共享 → 文件共享 → 选项」已允许 SMB，并允许所需 macOS 账户通过 SMB 访问。应用不替你启用账户或保存密码；系统现有目录权限决定访问权限。访客是否可用以目录徽标及系统配置为准。手机/PC 必须在可互通的局域网内，隔离 Wi-Fi、防火墙和客户端配置会影响连接。

- 左键托盘：显示/隐藏面板；右键：服务开关、添加目录、系统设置、自启、关于和退出。
- Esc 或点击面板外：收起面板；原生选择器/授权期间保留面板。
- Cmd+Q 或右键「退出应用」：退出工具。**退出工具不会主动停止系统 SMB 服务。**
- 开机自启默认关闭，由右键菜单控制；自启后不自动弹出面板。
- 服务状态与系统开关不一致时显示「状态待确认」，不把未确认的状态显示为已关闭。

交付构建未使用 Apple Developer 证书签名或公证。系统可能提示无法验证开发者；请按 macOS 提供的「隐私与安全性 → 仍要打开」流程自行确认。不需要关闭 Gatekeeper。当前包不适用于 Intel Mac；Intel 构建需在相应工具链/构建环境另行验证。

## 本地开发

需要 Node.js 22、npm、Rust 1.92+、Xcode Command Line Tools 和 macOS 13+。

```sh
npm ci
npm run tauri dev
```

桌面开发模式是真实系统适配器，开关/增删操作会触发原生管理员授权。

独立浏览器预览不调用系统命令：

```sh
npm run dev
# http://127.0.0.1:1420/?preview=normal&theme=light
# preview=normal|off|empty|loading|error, theme=light|dark
```

浏览器中的目录为隔离演示数据，窗口底部会标明「浏览器预览 · 不操作系统」。预览中的选择目录、移除和系统设置不会执行系统操作。

## 验证与打包

```sh
npm run check
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml --lib --locked
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets --locked -- -D warnings
npx playwright install chromium
npx playwright test
npm run tauri build
```

- `.app`：`src-tauri/target/release/bundle/macos/`
- `.dmg`：`src-tauri/target/release/bundle/dmg/`
- 原生只读自检：`src-tauri/target/release/drag-share --read-only-check`。输出真实服务状态、共享记录和地址；不创建 GUI、不注册快捷键、不修改共享和登录项。
- 单元测试使用临时目录和替代平台对象，不进行系统 SMB 写操作；测试关闭向用户日志目录写审计日志。
- `.github/workflows/check.yml` 在 macOS 执行单元测试、类型检查、Rust 检查、浏览器测试与发布编译，不执行需要管理员授权的测试。

## 架构与安全

- `src/components/`：与原型一致的状态区、共享列表和拖放区。
- `src/useShare.ts`：互斥交互、静默取消、刷新及提示；过期刷新不能覆盖新结果。
- `src-tauri/src/platform/`：`SmbPlatform` 契约、macOS 实现和其他平台 Stub。
- `service.rs`：目录规范化、重复校验、操作互斥、执行后复核。
- `commands.rs` / `desktop.rs`：有限的 IPC、托盘、面板、系统对话框和系统集成。
- `process.rs` / `security.rs`：固定命令、参数转义、AppleScript 字符串转义和超时控制。

应用使用普通用户权限运行，只在写系统共享配置时经 `osascript` 调起原生管理员授权。不存储管理员或 SMB 密码，不扫描目录内容，不上传数据，不轮询后台状态。每次打开面板刷新真实数据。共享记录优先使用系统 JSON 输出，保留中文、引号、换行与空格；删除时按系统要求使用记录名而非物理路径。

操作日志仅保存到 `~/Library/Logs/SMBShare/ops.jsonl`，5 MiB 轮转并最多保留 3 份历史日志；文件权限为当前用户可读写。日志包含操作路径，未包含文件内容或密码。删除该日志不会影响共享配置。

已知构建兼容：macOS 27 的 dyld 会拒绝部分经 LLVM strip 处理的过程宏动态库，Cargo 仅对构建期 host 库关闭 strip，交付二进制仍裁剪。参考 [Rust #157750](https://github.com/rust-lang/rust/issues/157750)。

## 交付与验收记录

[需求文档](docs/PRD.md) · [交互原型](docs/prototype.html) · [实施计划](docs/superpowers/plans/2026-10-18-drag-share.md) · [验证报告与人工验收](docs/VERIFICATION.md)
