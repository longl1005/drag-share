# macOS 双架构构建与发布准备

现有 v0.1.0–v0.1.2 发布说明记录历史 arm64 交付，不回写为 Intel 已发布。本次保持版本 0.1.2 和已发布 arm64 附件，新增 Intel 构建能力；尚未运行远端 CI、生成真实 Intel 包或完成 Intel 真机验收。

## CI

`.github/workflows/check.yml` 在 push、pull_request 和 workflow_dispatch 时运行：

| runner | Rust target | 附件架构 |
| --- | --- | --- |
| `macos-15` | `aarch64-apple-darwin` | `arm64` |
| `macos-15-intel` | `x86_64-apple-darwin` | `x64` |

采用原生双 runner，避免交叉编译后的执行测试依赖 Rosetta。标签来源：[GitHub runner 文档](https://docs.github.com/en/actions/how-tos/write-workflows/choose-where-workflows-run/choose-the-runner-for-a-job)。不使用已退役 macos-13 或即将退役的 macos-14。未来 Intel runner 退役时需重新评估迁移，不能将 arm64 runner 当作 Intel 原生测试环境。

每个架构执行前端测试与类型检查、Rust 单测/格式/Clippy、Playwright，再显式 target 构建 app 与 DMG；检查 Mach-O 架构、ad-hoc 签名和 DMG 完整性，使用 ditto 保留应用资源封装 ZIP。最低系统版本维持 macOS 13.0；runner 构建通过不能代替 macOS 13 真机兼容性验证。

在 Actions 选择 **Verify drag-share → Run workflow**，选择包含本次修改的分支。也可在该分支已推送后执行：

```sh
gh workflow run check.yml --ref <branch>
```

下载成功运行的 `macos-release-assets` artifact，其中包含：

- `drag-share-0.1.2-macos-arm64.dmg`
- `drag-share-0.1.2-macos-arm64.app.zip`
- `drag-share-0.1.2-macos-x64.dmg`
- `drag-share-0.1.2-macos-x64.app.zip`
- `SHA256SUMS.txt`（四个文件的统一清单）

流水线只有 contents: read 权限，不创建 Release、不上传 Release 附件。按现有发布规程人工验收后，再单独决定正式发布和更新下载链接。本次不执行这些操作。

## macOS 本地复现

安装 Node.js 22、Rust 1.92+（rustup）、Xcode Command Line Tools，然后：

```sh
npm ci
scripts/build-macos.sh x86_64-apple-darwin
# 在 Apple Silicon 上构建 arm64：
scripts/build-macos.sh aarch64-apple-darwin
```

脚本自动添加 target、设置 `MACOSX_DEPLOYMENT_TARGET=13.0` 并用 Cargo.lock 构建。显式 target 的 bundle 位于 `src-tauri/target/<target>/release/bundle/`，封装结果在忽略提交的 `release/`。支持 CARGO_TARGET_DIR；建议使用绝对路径。

在 Apple Silicon 上也可使用 Intel target 交叉构建，需 Xcode SDK 包含 x86_64 支持；运行 Intel 应用需 Rosetta 或 Intel Mac，交叉构建不算 Intel 原生测试通过。CI 默认使用 Intel runner，无须这一回退方案。

直接 `npm run tauri build` 后，仍可无参数运行 `scripts/package-release.sh` 封装当前主机架构。显式 target 构建后则传相同 target。打包前检查应用 CPU，拒绝错误架构的旧 app。

## 校验与 Linux 检查

`node scripts/sha256sums.mjs release` 为当前 package.json 版本的已有 arm64/x64 DMG 和 ZIP 生成清单（忽略历史版本文件），在先后打包两种架构后保留两者条目。CI 汇总作业要求两个架构均成功并收齐四个文件，再重建统一清单。

```sh
cd release
shasum -a 256 -c SHA256SUMS.txt
```

Linux 不生成 DMG。可执行：

```sh
npm ci
npm test
npm run build
scripts/test-rust-core.sh
```

核心测试脚本在临时 crate 引用实际后端模块，不拷贝实现、不链接 Tauri/GTK，使用应用 Cargo.lock 作为依赖解析起点；临时 crate 的精简依赖图会调整锁文件，不修改仓库锁文件。需要 Rust 1.92+，首次运行需要访问 crates.io。完整桌面 Rust 检查仍由两个 macOS CI runner 执行。

待验收：CI 实际构建与四附件校验、Intel 真机启动/菜单栏/快捷键/SMB 管理员授权与设备连接、macOS 13 最低版本兼容性。签名仍为 ad-hoc，未新增 Developer ID 或公证。
