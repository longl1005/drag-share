# drag-share 0.1.0 验证报告

日期：2026-10-18（Asia/Shanghai）。范围：macOS P0/P1；无人值守，不执行任何系统 SMB 写操作。

## 自动验证证据

| 检查 | 结果 | 验证内容 |
|---|---|---|
| Rust 单元测试 | 17/17 通过 | 共享 JSON/文本解析、真实记录名、特殊字符、状态冲突、目录校验、互斥、关闭保留配置、授权取消、超时、日志轮转 |
| Vue/TypeScript 测试 | 11/11 通过 | 未知/关闭态禁用、快速重复操作、静默取消、刷新竞争、错误提示、权限与失效目录显示、拖放范围 |
| Playwright | 11/11 通过 | 明暗两套主题 × 正常/关闭/空/加载/异常状态，以及开关/复制/菜单交互；空态按钮边界断言 |
| 前端构建 | 通过 | vue-tsc 严格类型检查和 Vite 生产构建 |
| Rust 格式与静态检查 | 通过 | cargo fmt --check；cargo clippy --all-targets --locked -- -D warnings |
| 原生发布构建 | 通过 | Tauri Apple Silicon .app 4.01 MiB、DMG 1.85 MiB，包体均小于 20 MiB |
| 交付完整性 | 通过 | 完整 ad-hoc 资源签名通过 codesign --verify --deep --strict；hdiutil verify 通过；附 ZIP 与 SHA-256 |
| 原生只读自检 | 通过 | 实际查询系统状态、3 个共享记录、本机地址，warnings 为空；不修改服务与共享配置 |
| 原生驻留采样 | 通过（短时） | 进程正常运行在 AppKit 事件循环；主进程约 93 MiB RSS、28.4 MiB physical footprint，采样时 CPU 0.0% |
| 独立代码审查 | 通过 | 状态冲突、授权期间隐藏、拖放范围及 About 互斥问题已修复，复审未发现剩余重要缺陷 |

本机环境：macOS 27.0.1 / arm64，Node 22.22.3，Rust 1.92.0，Tauri 2.12.4。自动测试合计 39 项。所有界面截图使用隔离演示数据，不能证明真实设备连接已经成功。

[浅色面板](screenshots/panel-light.png) · [深色面板](screenshots/panel-dark.png) · [空状态](screenshots/panel-empty.png)

GitHub CI 在干净 macOS 环境重复前后端检查及发布编译。实现审查修复版本 d9f2042 的[完整 CI 已成功](https://github.com/longl1005/drag-share/actions/runs/39117208976)；后续提交仍可在仓库 Actions 查看。本文自动测试计数使用本机最终代码实测结果。

## 仍需人工执行的系统验收

以下检查需要系统管理员交互、实体设备、重启或长时间观测。本次按用户选择不执行，也不标记为通过。

- [ ] 点击服务开关，授权成功后，确认本机 445 端口及另一设备实际连接。关闭后确认无法建立新连接、现有共享记录不减少。
- [ ] 分别在开启、关闭、添加、移除时取消系统管理员授权；界面静默恢复且配置不变化。
- [ ] 通过系统选择器添加真实目录，再拖入真实目录；用中文、空格、单双引号目录确认可添加和访问。普通文件与重复/符号链接目录应在提权前拒绝。
- [ ] 从真实共享项取消共享，确认系统原生二次确认有效、仅移除指定记录、文件内容仍然存在。
- [ ] 验证 Finder 定位、系统设置深链、Cmd+Q、Esc/失焦隐藏、托盘左右键、原生面板的主题与毛玻璃效果。
- [ ] 验证 Control+Option+S 与常用软件不冲突，快捷键冲突时仍可通过托盘使用。
- [ ] 手动启用开机自启、注销/登录，确认只驻留托盘；随后关闭自启。
- [ ] 多显示器与不同缩放比例下，确认面板贴合对应托盘且不越界。
- [ ] 面板隐藏并驻留 1 小时后检查 RSS ≤150 MiB、CPU <1%；测冷启动 ≤1s、热呼出 ≤100ms。
- [ ] 在 macOS 13/14/15 和 Intel 机器上进一步兼容性验收；当前交付仅 arm64。

原生 UI 自动化工具在绑定此 Accessory 应用时持续超时，因此没有将托盘点击、原生授权框或系统菜单行为宣称为 UI 自动验收通过。进程采样显示主线程正常等待 AppKit 事件；后端只读自检成功。浏览器测试验证的是 Vue 布局和控制逻辑。

## RTM 需求追踪

“实现”表示交付代码包含该功能；“人工”表示其 OS 级端到端行为仍需上述验收。

| FR | 交付实现 | 本次验证 |
|---|---|---|
| 01–04 托盘/无 Dock/双态/原生菜单 | desktop.rs，Info.plist | 编译、启动采样；菜单点击人工 |
| 05–07 面板形态/定位/刷新 | desktop.rs，App.vue，useShare.ts | 浏览器布局、刷新竞争；多屏与失焦人工 |
| 08–11 服务控制/互斥/静默取消 | platform/macos.rs，service.rs，commands.rs | 平台替身单测、只读查询；真实授权人工 |
| 12–14 列表/地址/空态 | parser.rs，macos.rs，ShareList.vue | 真机只读解析、组件及浏览器测试 |
| 15–17 添加/移除/特殊字符 | security.rs，service.rs，commands.rs | 校验、重复、取消、编码与注入防护单测；真实共享人工 |
| 18 拖放目录 | App.vue，drop.ts，service.rs | 几何边界与关闭态单测；Finder 拖放人工 |
| 19–20 地址复制 | commands.rs，security.rs | 编码单测、浏览器提示；真实剪贴板/客户端人工 |
| 21 快捷键 | desktop.rs | 编译、启动；按键与冲突人工 |
| 22 开机自启 | autostart 插件，desktop.rs | 编译；登录验证人工，未更改本机登录项 |
| 23–24 系统设置/Finder | commands.rs | 编译；系统界面行为人工 |
| 25 平台契约 | platform/mod.rs，unsupported.rs | macOS 编译；非 macOS 平台未实机构建 |

## 交付边界

无云数据库、遥测、密码存储、目录内容扫描、后台定时轮询或外网服务。Windows/Linux 不提供首发功能。退出工具不会自动关闭系统 SMB。未进行 Developer ID 签名与 Apple 公证。

交付副本：`release/drag-share-0.1.0-macos-arm64.dmg`、`release/drag-share-0.1.0-macos-arm64.app.zip`、`release/SHA256SUMS.txt`。`release/` 不提交 Git，以避免仓库积累构建文件。通过 `scripts/package-release.sh` 可重新封装并校验。

最后只读复核：服务状态与全部共享记录和测试前完全一致；测试启动的原生进程、预览服务器已结束。
