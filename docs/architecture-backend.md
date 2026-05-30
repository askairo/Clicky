# Clicky Backend Architecture / Clicky 后端架构说明

## 中文

### 1. 目标与边界

`src-tauri` 是 Clicky 桌面端的 Rust 后端，核心职责：

1. 对前端暴露 Tauri Commands（本地 API）
2. 执行业务编排（环境、分组、导入导出、应用流程）
3. 统一封装存储与系统能力（SQLite、Windows 环境变量、hooks）

它不是独立部署的远程服务，而是随桌面应用一起运行的本地进程。

### 2. 分层结构（当前落地）

```text
src-tauri/src
├─ lib.rs                  # 应用启动与依赖装配（薄入口）
├─ controller/
│  └─ commands.rs          # Tauri command 入口（控制层）
├─ appservice/
│  └─ clicky_appservice.rs # 业务编排层（流程）
├─ service/
│  ├─ storage_service.rs   # 数据存储服务（SQLite/YAML迁移）
│  └─ system_service.rs    # 系统能力服务（环境变量落盘、hooks）
└─ domain/
   ├─ mod.rs
   └─ model.rs             # 领域模型（entity/dto/vo集合）
```

### 3. 分层职责

1. `controller`
   - 接收前端参数，调用 `appservice`
   - 不承载复杂业务逻辑
2. `appservice`
   - 编排业务流程与规则校验
   - 组合调用 `service`
3. `service`
   - `storage_service`：CRUD、schema 初始化、YAML->DB 初始化迁移
   - `system_service`：操作系统相关能力与命令执行
4. `domain`
   - 统一定义 `ConfigFile / GroupDef / EnvDef / 请求响应 DTO`

### 4. 执行流程（主链路）

1. 前端 `invoke(...)` 调用 Tauri command
2. `controller::commands` 接收并转发到 `appservice`
3. `appservice` 执行业务编排与规则校验
4. 需要存储时调用 `storage_service`
5. 需要系统操作时调用 `system_service`
6. 返回结构化结果给前端

### 5. 启动流程

1. 入口 `lib.rs::run()`
2. `init_storage()`：创建并校验 SQLite schema
3. 若是首次运行：尝试从 YAML 读取并写入 DB
4. 注册所有 command 并启动 Tauri

### 6. 设计原则（与当前代码一致）

1. 入口文件薄化：`lib.rs` 只做启动和装配
2. 控制与业务分层：`controller` 不写业务细节
3. 业务与基础能力分层：`appservice` 不直接耦合 OS 细节
4. 小方法优先：按场景拆分函数，避免超长函数
5. 领域模型集中：跨层结构定义统一在 `domain`

### 7. 后续可继续优化

1. 拆分 `clicky_appservice.rs` 为多个文件（group/environment/import_export/apply）
2. 在 `domain` 进一步细分 `entity/vo/dto/assembler`
3. 引入统一错误类型（替代大量 `Result<T, String>`）
4. 增加分层单元测试（service/appservice 分别测试）

---

## English

### 1. Goal and Scope

`src-tauri` is the Rust backend for the Clicky desktop app. Its responsibilities:

1. Expose Tauri Commands as local APIs for the frontend
2. Orchestrate business flows (groups, environments, import/export, apply)
3. Encapsulate persistence and system capabilities (SQLite, Windows env vars, hooks)

It is not a standalone remote server; it runs locally with the desktop app.

### 2. Layered Structure (Current)

```text
src-tauri/src
├─ lib.rs                  # Bootstrap and wiring (thin entry)
├─ controller/
│  └─ commands.rs          # Tauri command entrypoints (control layer)
├─ appservice/
│  └─ clicky_appservice.rs # Business orchestration layer
├─ service/
│  ├─ storage_service.rs   # Persistence service (SQLite + YAML migration)
│  └─ system_service.rs    # System capability service (env apply, hooks)
└─ domain/
   ├─ mod.rs
   └─ model.rs             # Domain models (entity/dto/vo set)
```

### 3. Layer Responsibilities

1. `controller`
   - Receives frontend input and forwards to `appservice`
   - Keeps business logic minimal
2. `appservice`
   - Owns business orchestration and rule validation
   - Composes `service` calls
3. `service`
   - `storage_service`: CRUD, schema init, YAML-to-DB bootstrap migration
   - `system_service`: OS-level operations and hook execution
4. `domain`
   - Shared model definitions (`ConfigFile`, `GroupDef`, `EnvDef`, request/response DTOs)

### 4. Runtime Flow (Main Path)

1. Frontend calls `invoke(...)`
2. `controller::commands` receives and delegates to `appservice`
3. `appservice` runs orchestration and validations
4. Call `storage_service` for persistence
5. Call `system_service` for OS operations
6. Return structured result to frontend

### 5. Startup Flow

1. Entry: `lib.rs::run()`
2. `init_storage()`: create/check SQLite schema
3. First run: attempt YAML load and persist to DB
4. Register commands and start Tauri

### 6. Design Principles (Aligned with Current Code)

1. Thin entrypoint: `lib.rs` only bootstraps and wires modules
2. Control/business separation: `controller` avoids business details
3. Business/base capability separation: `appservice` avoids OS coupling
4. Small-method preference: split by scenario, avoid long functions
5. Centralized domain models: shared structures live in `domain`

### 7. Next Improvements

1. Split `clicky_appservice.rs` into focused modules (group/environment/import_export/apply)
2. Further split `domain` into `entity/vo/dto/assembler`
3. Introduce unified error types (reduce `Result<T, String>`)
4. Add layered unit tests (`service` and `appservice`)

