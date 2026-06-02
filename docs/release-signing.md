# Release and Signing Strategy / 发布与签名策略

## 中文

### 目标

Clicky 的发布策略分成两层：

1. `development` 版：用于本地验证、CI 构建和日常迭代，不依赖签名与公证。
2. `release` 版：用于对外分发，需要在对应平台准备签名材料，再执行正式打包与发布。

### Windows

- 发布版建议使用代码签名证书，避免 SmartScreen 给出未信任提示。
- 本仓库的 Release 流程默认使用 Tauri 的 Windows 打包链路，签名材料可通过 GitHub Secrets 注入。
- 如果当前没有签名证书，仍可生成安装包，但这类包更适合开发验证，不适合正式分发。

### macOS

- 发布版建议使用 `Developer ID Application` 证书并完成 notarization。
- 未签名或未公证的应用在首次打开时，通常需要用户手动放行。
- 目前仓库 README 仍保留了未签名应用的首次打开说明，作为开发版和过渡版的使用指引。

### 推荐 Secret

- `APPLE_CERTIFICATE`
- `APPLE_CERTIFICATE_PASSWORD`
- `APPLE_SIGNING_IDENTITY`
- `APPLE_ID`
- `APPLE_PASSWORD`
- `APPLE_TEAM_ID`
- `WIN_CSC_LINK`
- `WIN_CSC_KEY_PASSWORD`

### 结论

1. CI 负责验证构建是否健康。
2. Release 负责产物和对外分发。
3. 签名与公证只在正式发布链路启用，避免阻塞日常迭代。

## English

### Goal

Clicky uses two release lanes:

1. `development` builds for local validation, CI, and day-to-day iteration, without signing or notarization requirements.
2. `release` builds for external distribution, which should use platform signing materials before packaging and publishing.

### Windows

- Production builds should be code-signed to avoid SmartScreen warnings.
- The repository release pipeline uses the Tauri Windows packaging flow, and signing materials can be injected via GitHub Secrets.
- Without a signing certificate, installers can still be produced, but they are better suited for internal validation than formal distribution.

### macOS

- Production builds should use a `Developer ID Application` certificate and notarization.
- Unsigned or un-notarized apps typically require a manual approval step on first launch.
- The README keeps the unsigned-app first-launch guidance as a developer-friendly transition note.

### Suggested Secrets

- `APPLE_CERTIFICATE`
- `APPLE_CERTIFICATE_PASSWORD`
- `APPLE_SIGNING_IDENTITY`
- `APPLE_ID`
- `APPLE_PASSWORD`
- `APPLE_TEAM_ID`
- `WIN_CSC_LINK`
- `WIN_CSC_KEY_PASSWORD`

### Summary

1. CI validates that the build is healthy.
2. Release produces and publishes distributable artifacts.
3. Signing and notarization remain release-only to keep day-to-day iteration unblocked.
