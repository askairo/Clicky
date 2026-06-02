# Platform Differences and FAQ / 平台差异与常见问题

## 中文

### 平台差异

- Windows：Clicky 将变量写入当前用户环境，并广播环境变更通知；新开进程可以读取到最新值，旧进程通常需要重启。
- macOS：Clicky 通过 `launchctl setenv` 更新当前会话，同时生成 `~/.clicky/env.sh` 作为 shell 持久化快照，方便终端类启动链路读取。
- 其他平台：当前只保留占位实现，不作为正式支持目标。

### 为什么需要重启进程

环境变量通常在进程启动时读取。Clicky 可以把新值写入系统层或会话层，但无法强制已经运行中的程序自动重新加载自己的环境快照。

因此：

1. 新开终端、新启动的桌面程序会读取最新值。
2. 已打开的 IDE、终端、后端服务通常需要重启。
3. 如果应用内部有自己的配置缓存，也可能需要手动刷新。

### 团队变量命名规范

- 使用稳定、具业务语义的前缀，例如 `ZNDER_`。
- 同一分组内保持变量命名一致，避免同义变量重复出现。
- 敏感值优先使用 `*_PASS`、`*_TOKEN`、`*_SECRET` 等易识别后缀。
- 建议按系统/服务域拆分前缀，例如数据库、缓存、消息队列分开命名。

### 常见问题

- 问：为什么切换后状态显示已生效，但某个程序还是老值？
  - 答：通常是因为那个程序已经在运行，需要重启后才会重新读取环境变量。
- 问：为什么 macOS 还会提示未验证？
  - 答：如果还没有完成正式签名和公证，Gatekeeper 仍可能要求用户手动放行。
- 问：为什么有些变量在敏感模式下会被隐藏？
  - 答：这是为了避免在界面上直接暴露密码、令牌类值。

## English

### Platform Differences

- Windows: Clicky writes to the current user's environment and broadcasts the environment-change notification; new processes can read the updated values, while existing ones usually need a restart.
- macOS: Clicky updates the current session with `launchctl setenv` and also generates `~/.clicky/env.sh` as a shell persistence snapshot for terminal-style startup flows.
- Other platforms: only placeholder support is kept for now and they are not the formal target.

### Why a Restart Is Needed

Environment variables are usually read at process start. Clicky can write the new value into the system or session layer, but it cannot force a running app to reload its own environment snapshot.

So:

1. New terminals and newly started desktop apps will read the latest value.
2. Existing IDEs, terminals, and background services usually need a restart.
3. Apps with their own config cache may also need a manual refresh.

### Team Naming Guidance

- Use stable, domain-specific prefixes such as `ZNDER_`.
- Keep names consistent within the same group to avoid duplicate semantics.
- Prefer obvious suffixes for sensitive values, such as `*_PASS`, `*_TOKEN`, or `*_SECRET`.
- Split prefixes by domain when possible, for example database, cache, and messaging.

### FAQ

- Q: Why does Clicky show the environment as active, but one program still sees the old value?
  - A: That program is usually already running and must be restarted to reload the new environment.
- Q: Why does macOS still warn about verification?
  - A: If signing and notarization are not complete yet, Gatekeeper may still require manual approval.
- Q: Why are some values hidden in the UI?
  - A: To avoid exposing passwords or token-like values directly in the interface.
