# 开发与构建

客户端使用 Tauri 2、React、TypeScript 和 Fluent UI，认证、凭据存储、自动守护与更新逻辑由 Rust 实现。

## Windows 开发环境

- Node.js 22+ 和 npm。
- Rust stable，以及 `x86_64-pc-windows-msvc` 工具链。
- Visual Studio 的 C++ 构建工具与 Windows SDK。
- Microsoft Edge WebView2 运行时。
- 构建安装包还需要 NSIS，确保 `makensis.exe` 位于 PATH 中，或通过脚本的 `-NsisCompiler` 参数指定位置。

在仓库根目录运行：

```powershell
npm ci
npm run desktop
```

`npm run dev` 仅启动前端开发服务器；认证和桌面接口需要通过 `npm run desktop` 启动完整客户端。

## 目录结构

| 目录 | 内容 |
| --- | --- |
| `frontend/` | 界面组件、样式、桌面接口调用和前端测试 |
| `src-tauri/` | Tauri 入口、命令、窗口和托盘集成 |
| `src/` | 共用 Rust 核心，以及暂留的 egui 回归基线 |
| `scripts/` | 发布构建与安装器验证脚本 |
| `installer/` | Windows NSIS 安装和卸载流程 |
| `.github/workflows/` | 持续检查和 tag 发布流程 |

正式发行入口是 Tauri；旧 egui 入口暂留用于回归验证。

## 验证代码

```powershell
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
npm run check
npm run test:ui
cargo fmt --manifest-path src-tauri/Cargo.toml --check
cargo clippy --manifest-path src-tauri/Cargo.toml --locked --all-targets -- -D warnings
```

前端测试使用 Playwright 和 Microsoft Edge。测试数据应为虚构数据或文档保留地址，不使用真实账号、凭据或设备标识。

## 构建与发布

`npm run desktop:build` 构建客户端可执行文件。完整发行构建由 `scripts/build.ps1` 执行：

```powershell
$version = (Get-Content package.json -Raw | ConvertFrom-Json).version
.\scripts\build.ps1 -ReleaseTag "v$version"
```

脚本检查 Rust 核心、Tauri、前端及窗口配置的版本一致性，运行测试，构建 Windows 可执行文件与 NSIS 安装包，并验证安装和卸载流程。

发行文件位于 `dist/`：

- `ZHKU-NetAutoAuth.exe`：便携版。
- `ZHKU-NetAutoAuth-Setup.exe`：安装版。

发布 tag 必须与应用版本匹配。推送版本 tag 后，Release 工作流构建两个 exe，不额外打包 ZIP。构建目录、依赖缓存、生成文件和本机配置不纳入 Git。

## 配置与隐私

安装版和便携版共用 `%LOCALAPPDATA%\ZHKU-NetAutoAuth\` 中的配置：`credentials.dat` 保存当前 Windows 用户 DPAPI 加密后的账号信息，`desktop.json` 保存桌面设置。

旧配置目录中的文件会逐项迁移，已有新配置不会被覆盖。旧副本保留用于版本回退，删除凭据时会一并清理。前端不使用浏览器本地存储保存密码。

日志记录需保留原始诊断信息并脱敏密码；公开截图和日志还需遮挡账号、IP、MAC 等网络标识。文档截图使用示例数据时，应明确标注为示意图。
