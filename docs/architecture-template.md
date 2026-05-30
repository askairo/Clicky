# Frontend Architecture Template / 前端架构模板

## 中文

### 1. 分层模型

建议采用以下前端分层：

- `ui`：页面渲染与交互入口
- `appservice`：业务编排与用例流程
- `service`：外部访问（API/IPC/网关）
- `domain`：领域模型与业务转换规则
- `utils`：通用技术工具（不含业务语义）

### 2. 依赖方向（强约束）

允许：

- `ui -> appservice -> service`
- `ui -> domain`（仅在必要时引用类型）
- `appservice -> domain`
- `service -> domain`（可选，用于 DTO 映射）

不允许：

- `domain` 依赖 `ui/appservice/service`
- `service` 依赖 `ui/appservice`
- 跳层调用导致职责混乱

### 3. 推荐目录结构

```text
src/
  ui/
    pages/
    hooks/
    components/
    styles/
  appservice/
  service/
  domain/
    entity/
    vo/
    dto/
    assembler/
    index.ts
  utils/
  main.tsx
```

### 4. 命名规范

- 页面：`XxxPage.tsx`
- 页面模型 Hook：`useXxxPageModel.ts`
- 业务编排：`xxxAppService.ts`
- 外部访问：`xxxService.ts` 或 `xxxApi.ts`
- 领域实体：`xxxEntity.ts`
- 值对象：`xxxVo.ts`
- 数据传输对象：`xxxDto.ts`
- 转换器：`xxxAssembler.ts`

### 5. 各层职责

`ui`
- 负责渲染和用户交互
- 维护页面状态
- 调用 appservice，不直接写业务流程

`appservice`
- 编排业务流程
- 组合多个 service 调用
- 返回页面可消费的结果（如 `ok/message/data`）

`service`
- 封装外部调用细节
- 隔离请求/响应协议
- 不处理页面状态

`domain`
- 定义业务语义（entity/vo/dto）
- 提供映射与纯业务规则
- 优先纯函数

`utils`
- 仅放通用技术工具
- 避免业务词汇

### 6. 编码原则

- 小函数、单一职责
- 业务流程与 UI 事件分离
- 副作用控制在边界层（`ui/service`）
- 避免大而全文件

### 7. 错误处理建议

- `service`：将底层错误转换为可识别错误
- `appservice`：转换为用例级结果
- `ui`：只负责展示用户可理解信息

### 8. 存量项目迁移清单

1. 识别职责混合文件
2. 将外部调用下沉到 `service`
3. 将业务流程提炼到 `appservice`
4. 将模型与转换提炼到 `domain`
5. 让页面文件回归渲染职责
6. 在评审中检查分层依赖

### 9. 最小评审清单

- 文件是否在正确层？
- 依赖方向是否符合约束？
- UI 文件是否混入业务流程？
- 映射逻辑是否集中在 `domain/assembler`？
- 函数粒度是否足够小？

### 10. 可选增强

- 为每层增加 README
- 使用 lint 做边界依赖检查
- 增加 ADR（`docs/adr/`）记录架构决策

---

## English

### 1. Layer Model

Use the following frontend layers:

- `ui`: page rendering and interaction entry
- `appservice`: business orchestration and use-case flow
- `service`: external access (API/IPC/gateway)
- `domain`: business model and transformation rules
- `utils`: generic technical helpers (non-business)

### 2. Dependency Rules (Strict)

Allowed:

- `ui -> appservice -> service`
- `ui -> domain` (types only when needed)
- `appservice -> domain`
- `service -> domain` (optional, for DTO mapping)

Not allowed:

- `domain` depending on `ui/appservice/service`
- `service` depending on `ui/appservice`
- layer-skipping shortcuts

### 3. Recommended `src` Layout

```text
src/
  ui/
    pages/
    hooks/
    components/
    styles/
  appservice/
  service/
  domain/
    entity/
    vo/
    dto/
    assembler/
    index.ts
  utils/
  main.tsx
```

### 4. Naming Conventions

- Page: `XxxPage.tsx`
- Page-model hook: `useXxxPageModel.ts`
- App service: `xxxAppService.ts`
- External access: `xxxService.ts` or `xxxApi.ts`
- Domain entity: `xxxEntity.ts`
- Value object: `xxxVo.ts`
- DTO: `xxxDto.ts`
- Assembler: `xxxAssembler.ts`

### 5. Layer Responsibilities

`ui`
- Render views and collect user input
- Keep page state
- Call appservice instead of embedding business flow

`appservice`
- Orchestrate business flow
- Combine multiple service calls
- Return UI-friendly result objects (`ok/message/data`)

`service`
- Encapsulate external call details
- Isolate request/response protocols
- No page-state logic

`domain`
- Define business vocabulary (entity/vo/dto)
- Hold mapping and business rules
- Prefer pure functions

`utils`
- Generic technical helpers only
- Avoid business terms

### 6. Coding Principles

- Small functions with single responsibility
- Separate business flow from UI event handlers
- Keep side effects at edge layers (`ui/service`)
- Avoid large mixed-responsibility files

### 7. Error Handling Pattern

- `service`: map low-level errors to typed errors
- `appservice`: convert to use-case-level results
- `ui`: display user-facing messages only

### 8. Migration Checklist for Existing Projects

1. Identify mixed-responsibility files
2. Move external calls into `service`
3. Move business flow into `appservice`
4. Move model/mapping into `domain`
5. Keep page files focused on rendering
6. Add dependency-boundary checks in review

### 9. Minimal Review Checklist

- Is this file in the correct layer?
- Does dependency direction follow rules?
- Is business flow leaking into UI files?
- Are mappings centralized in `domain/assembler`?
- Are functions small and explicit enough?

### 10. Optional Enhancements

- Add per-layer README files
- Enforce boundary checks with lint rules
- Add ADRs under `docs/adr/`
