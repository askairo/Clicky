# Clicky / Clicky（中英双语）

<p align="center">
  <a href="#cn"><strong>中文</strong></a> ·
  <a href="#en"><strong>English</strong></a>
</p>

<h2 id="cn">中文</h2>

### 项目简介

Clicky 是一个受 SwitchHosts 启发的跨平台桌面工具，专注于环境变量分组管理与一键切换。

### 当前状态

- Windows + macOS 双平台支持
- 基于 Tauri + React 的桌面应用
- 前端采用分层结构：`ui / appservice / service / domain / utils`
- 支持环境变量配置管理与一键应用
- 支持切换后自动导出 IDEA 可引用的应用级 `.env` 快照
- 默认对敏感变量脱敏显示
- 支持切换后执行 `hooks.post` 命令

### 安装方式（Windows / macOS）

#### 方式一：从 GitHub Releases 下载

- Windows：下载 `.exe` 或 `.msi`
- macOS：下载 `.dmg`
- Release 页面：<https://github.com/askairo/Clicky/releases>

#### 方式二：Homebrew（macOS）

```bash
brew tap askairo/tap
brew install clicky
```

### 本地运行（开发）

```powershell
npm install
npm run tauri dev
```

### macOS 首次打开（未签名提示）

如果提示“已损坏/无法打开”，通常是 macOS Gatekeeper 对未签名应用的拦截。可执行：

```bash
xattr -dr com.apple.quarantine /Applications/Clicky.app
open /Applications/Clicky.app
```

若仍被拦截，请在「系统设置 -> 隐私与安全性」中对 Clicky 选择“仍要打开”。

### 配置与存储

- 示例配置文件：`config/environments.example.yaml`
- 当前项目以本地存储为主，YAML 主要用于导入/导出与模板示例

### 注意事项

环境变量切换对“新进程”生效。已有终端、IDE、目标程序通常需要重启后才能读取到最新值。

如果你在 IntelliJ IDEA 中使用 `Run/Debug Configuration` 指向 `~/.clicky/idea/current.env`，Clicky 会在每次切换后自动更新该文件，供新开的调试任务读取最新变量。

### 相关文档

- 平台差异与常见问题：`docs/platform-differences.md`
- 发布与签名策略：`docs/release-signing.md`
- 验收脚本与用例：`docs/acceptance.md`

<h2 id="en">English</h2>

### Overview

Clicky is a cross-platform desktop app inspired by SwitchHosts, focused on grouped environment-variable management and one-click switching.

### Current Status

- Windows + macOS supported
- Desktop app built with Tauri + React
- Frontend layered architecture: `ui / appservice / service / domain / utils`
- Supports environment-variable config management and one-click apply
- Supports automatic export of an IDEA-friendly application-level `.env` snapshot after each switch
- Sensitive values are masked by default
- Supports post-switch `hooks.post` commands

### Install (Windows / macOS)

#### Option 1: Download from GitHub Releases

- Windows: download `.exe` or `.msi`
- macOS: download `.dmg`
- Release page: <https://github.com/askairo/Clicky/releases>

#### Option 2: Homebrew (macOS)

```bash
brew tap askairo/tap
brew install clicky
```

### Local Run (Development)

```powershell
npm install
npm run tauri dev
```

### First Launch on macOS (Unsigned App Warning)

If macOS reports the app is damaged or cannot be opened, it is usually Gatekeeper blocking an unsigned app. Run:

```bash
xattr -dr com.apple.quarantine /Applications/Clicky.app
open /Applications/Clicky.app
```

If needed, go to `System Settings -> Privacy & Security` and choose "Open Anyway" for Clicky.

### Config and Storage

- Example config file: `config/environments.example.yaml`
- Local storage is primary; YAML is mainly used for import/export and templates

### Note

Environment variable updates apply to new processes. Existing terminals, IDEs, and target apps usually need to be restarted to read updated values.

If you point IntelliJ IDEA `Run/Debug Configuration` to `~/.clicky/idea/current.env`, Clicky will refresh that file after each switch so newly launched debug tasks can read the latest values.

### Related Docs

- Platform differences and FAQ: `docs/platform-differences.md`
- Release and signing strategy: `docs/release-signing.md`
- Acceptance scripts and cases: `docs/acceptance.md`
