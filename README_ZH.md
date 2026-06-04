# clicky

[English](README.md) | 中文

clicky 是一个受 SwitchHosts 启发的跨平台桌面工具，专注于环境变量分组管理与一键切换。

## 当前状态

- 支持 Windows 和 macOS
- 基于 Tauri 和 React 构建桌面应用
- 前端采用分层结构：`ui / appservice / service / domain / utils`
- 支持环境变量配置管理与一键应用
- 支持切换后自动导出 IDEA 可引用的应用级 `.env` 快照
- 默认对敏感变量脱敏显示
- 支持切换后执行 `hooks.post` 命令

## 安装方式

### 方式一：从 GitHub Releases 下载

- Windows：下载 `.exe` 或 `.msi`
- macOS：下载 `.dmg`
- Release 页面：<https://github.com/askairo/clicky/releases>

### 方式二：macOS 使用 Homebrew

```bash
brew tap askairo/tap
brew install clicky
```

## 本地开发

```powershell
npm install
npm run tauri dev
```

## macOS 首次打开

如果提示“已损坏/无法打开”，通常是 macOS Gatekeeper 对未签名应用的拦截。可执行：

```bash
xattr -dr com.apple.quarantine /Applications/clicky.app
open /Applications/clicky.app
```

若仍被拦截，请在“系统设置 -> 隐私与安全性”中对 clicky 选择“仍要打开”。

## 配置与存储

- 示例配置文件：`config/environments.example.yaml`
- 当前项目以本地存储为主
- YAML 主要用于导入、导出与模板示例

## 注意事项

环境变量切换对“新进程”生效。已有终端、IDE、目标程序通常需要重启后才能读取到最新值。

如果你在 IntelliJ IDEA 中使用 `Run/Debug Configuration` 指向 `~/.clicky/env/idea/current.env`，clicky 会在每次切换后自动更新该文件，供新开的调试任务读取最新变量。

## 相关文档

- 平台差异与常见问题：`docs/platform-differences.md`
- 发布与签名策略：`docs/release-signing.md`
- 验收脚本与用例：`docs/acceptance.md`
