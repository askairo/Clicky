# Clicky / Clicky（中英双语）

## 中文

### 项目简介

Clicky 是一个受 SwitchHosts 启发的桌面工具，用于切换环境变量（不仅限于 hosts）。

### 当前状态

- Windows 优先
- 基于 Tauri + React 的桌面应用
- 前端采用分层结构：`ui / appservice / service / domain / utils`
- 支持环境变量配置管理与一键应用
- 默认对敏感变量脱敏显示
- 支持切换后执行 `hooks.post` 命令

### 运行方式（Windows）

```powershell
npm install
npm run tauri dev
```

### 配置与存储

- 示例配置文件：`config/environments.example.yaml`
- 当前项目以本地存储为主，YAML 主要用于导入/导出与模板示例

### 注意事项

环境变量切换对“新进程”生效。已有终端、IDE、目标程序通常需要重启后才能读取到最新值。

---

## English

### Overview

Clicky is a desktop tool inspired by SwitchHosts for switching environment variables (not limited to hosts).

### Current Status

- Windows-first
- Desktop app built with Tauri + React
- Frontend layered architecture: `ui / appservice / service / domain / utils`
- Supports environment-variable config management and one-click apply
- Sensitive values are masked by default
- Supports post-switch `hooks.post` commands

### Run (Windows)

```powershell
npm install
npm run tauri dev
```

### Config and Storage

- Example config file: `config/environments.example.yaml`
- Local storage is primary; YAML is mainly used for import/export and templates

### Note

Environment variable updates apply to new processes. Existing terminals, IDEs, and target apps usually need to be restarted to read updated values.
