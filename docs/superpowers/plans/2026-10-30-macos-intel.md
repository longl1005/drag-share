# macOS Intel 构建实施计划

目标与规格：按本次用户要求，为现有 macOS 13+ 产品补充 x86_64 DMG 和 app.zip，保留 arm64；仅提交，不 push、不发布，不交互确认。

方案：优先采用 macos-15 / macos-15-intel 原生双 runner matrix，分别指定 Rust target，完整测试后封装。备选 arm64 上交叉编译 Intel 可用于本地，但缺少 Intel 原生测试；universal 包增加体积并改变既有架构附件习惯，因此不采用。GitHub 当前 runner 文档见 releases/macos-builds.md。

文件职责：build-macos.sh 固定 target 和部署版本；package-release.sh 验证架构/签名/DMG 并封装；sha256sums.mjs 汇总同版本两架构附件；check.yml 完成双架构验证及四附件汇总；test-rust-core.sh 在临时 crate 引用真实核心模块供 Linux 检查。

## 执行清单

- [x] 阅读既有 README、PRD、验证报告、历史发布说明、Tauri / Cargo 配置及 CI。
- [x] 实现显式 target 构建、兼容无参数本机打包、保持 arm64 名称并新增 x64 名称。
- [x] 双 runner CI、只读权限、手动触发、统一附件，不创建 Release。
- [x] 校验值覆盖先后封装两架构，回归检查空输入、旧版本排除及精确摘要。
- [x] Linux 核心 Rust 测试、前端测试/类型检查、shell 与 workflow 静态检查。
- [x] 更新文档与准确标注 Intel 尚未实际构建/发布，记录未验收项。
- [x] 最终差异复核并提交。

重点复核：错误 CPU 的旧 app 应拒绝；两架构 checksum 不互相覆盖；旧版附件不混入清单；PR 构建不需要发布密钥；macOS 13 兼容性不能由较新 runner 构建成功推断。

执行说明：用户已指定不交互确认，因此本会话直接实施并验证；不新增功能逻辑、不改 Tauri 签名与 Cargo 桌面依赖图。Linux 无 GUI 测试覆盖核心业务，不替代完整 macOS CI。
