# Acceptance Cases / 验收用例

## 中文

### 执行入口

```powershell
.\scripts\run-acceptance.ps1
```

### 覆盖范围

1. `acceptance_export_import_roundtrip`
   - 验证导出文件可生成、可预览导入、可重新导入并保持变量内容。
2. `acceptance_hooks_run_and_report`
   - 验证 `hooks.post` 会被执行，并且结果会回传到前端可见的回执结构中。
3. `acceptance_windows_apply_and_detect`
   - 仅 Windows 生效。
   - 验证环境应用后，`detect_active_environments` 可以识别当前激活环境。
   - 同时验证 `runtime_capabilities` 的平台提示。

### 验收口径

- `npm run build` 通过。
- `cargo check` 通过。
- `cargo test acceptance_ -- --test-threads=1` 通过。
- CI 上能看到 Windows 与 macOS 的 bundle 产物上传。

## English

### Entry Point

```powershell
.\scripts\run-acceptance.ps1
```

### Coverage

1. `acceptance_export_import_roundtrip`
   - Confirms that export output is generated, previewable, and importable without losing variable data.
2. `acceptance_hooks_run_and_report`
   - Confirms that `hooks.post` is executed and that the result is returned in the response payload.
3. `acceptance_windows_apply_and_detect`
   - Windows only.
   - Confirms that after applying an environment, `detect_active_environments` can identify the active environment.
   - Also validates the platform hint from `runtime_capabilities`.

### Acceptance Criteria

- `npm run build` passes.
- `cargo check` passes.
- `cargo test acceptance_ -- --test-threads=1` passes.
- CI uploads Windows and macOS bundle artifacts.
