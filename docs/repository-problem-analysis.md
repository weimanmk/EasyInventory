# EasyInventory 仓库问题分析

分析日期：2026-06-11

分析对象：`C:/Users/ww/Desktop/work/EasyInventory`

## 结论摘要

EasyInventory 当前不是“跑不起来”的项目。基础工程质量较好，TypeScript、Rust lint、Rust 单测、前端构建、核心业务流程脚本、浏览器 E2E 和项目健壮性脚本均已通过。主要问题集中在真实经营数据安全、SQLite 备份一致性、并发写入韧性、前后端契约类型安全、长期演进的模块边界和开源治理。

建议优先处理顺序：

1. 日志脱敏与诊断包隐私边界。
2. WAL 模式下数据库备份方式。
3. SQLite 连接 `busy_timeout` 与写入冲突处理。
4. 数据库迁移版本化。
5. 前端 API 类型契约和大页面拆分。
6. 真实 Tauri IPC 端到端冒烟与发布链路治理。

## 已验证通过的项目

以下命令均在本地仓库执行并通过：

| 检查项 | 结果 |
| --- | --- |
| `npm run typecheck` | 通过 |
| `npm run format:rust` | 通过 |
| `npm run lint:rust` | 通过 |
| `npm run test:rust` | 通过，73 个 Rust 测试全部成功 |
| `npm run build` | 通过 |
| `npm run e2e:flows` | 通过，9 条核心流程 |
| `npm run e2e:browser` | 通过，10 条浏览器流程 |
| `npm run audit:robustness` | 通过，32 项健壮性检查 |

这说明项目已有基本质量门禁，后续问题应按风险和维护收益分阶段处理，避免一次性大重构。

## 问题清单

### P1 日志会记录业务参数，缺少敏感信息脱敏

证据：

- `src/api/tauri.ts:6` 的 `callCommand<T>(command: string, args?: Record<string, unknown>)` 接收任意命令参数。
- `src/api/tauri.ts:8` 在调用开始时把 `args` 写入客户端日志。
- `src/api/tauri.ts:13-18` 在命令失败时写入 `error` 和 `args`。
- `src/api/tauri.ts:57-62` 将 `details` 通过 `safeStringify` 写入 `write_client_log`。
- `src-tauri/src/commands/system_commands.rs:23-25` 将前端传来的 `details` 拼入日志消息。
- `src-tauri/src/logger.rs:32` 只移除换行，没有字段级脱敏。

影响：

- 客户姓名、电话、地址、订单金额、商品明细、数据库路径等可能落到本地日志。
- README 和 CONTRIBUTING 都强调不要上传私密经营数据，但代码层面仍默认记录详细参数。
- 如果用户导出诊断包或粘贴日志给开源 Issue，容易泄露真实业务数据。

建议：

- 默认不要记录完整 `args`，只记录命令名、耗时、结果摘要和错误码。
- 建立统一脱敏函数，覆盖 `phone`、`address`、`customerName`、`customerAddress`、`merchantPhone`、`filePath`、`databasePath` 等字段。
- 对诊断包输出增加隐私提示和可选脱敏模式。
- 为日志脱敏增加单元测试，防止后续新增字段绕过。

原则说明：

- KISS：先做字段白名单/黑名单脱敏，不引入复杂日志系统。
- SRP：日志模块负责脱敏，API 调用层只传结构化摘要。

### P1 WAL 模式下直接复制数据库文件，备份可能不完整

证据：

- `src-tauri/src/app.rs:83-84` 打开 SQLite 连接时启用了 `foreign_keys` 和 `journal_mode = WAL`。
- `src-tauri/src/db.rs:539-548` 的 `create_backup_file` 直接 `fs::copy(&db_path, &backup_path)`。
- `src-tauri/src/db.rs:552-568` 的恢复逻辑会清理 `-wal/-shm`，说明代码已意识到 WAL sidecar 文件存在。

影响：

- WAL 模式下，最近提交的数据可能仍在 `inventory.db-wal`，直接复制主库文件不保证包含最新写入。
- 备份文件可能是旧快照；对库存、欠款、订单这类经营数据来说风险较高。
- 当前 `restore_database_file_creates_snapshot_and_replaces_database` 测试只覆盖主库文件替换，不能证明 WAL 场景下备份完整。

建议：

- 优先使用 SQLite backup API 生成一致性备份。
- 如果继续文件复制，复制前应执行 checkpoint，并确保没有活跃写事务。
- 备份完成后对备份文件执行 `PRAGMA integrity_check` 或打开校验。
- 增加 WAL 写入后立即备份的回归测试。

原则说明：

- KISS：使用 SQLite 官方 backup 能力比手动处理 WAL 文件更简单可靠。
- YAGNI：不需要设计复杂备份格式，先保证单机备份一致性。

### P1 运行时连接未设置 `busy_timeout`，并发写入可能出现瞬时锁错误

证据：

- `src-tauri/src/app.rs:81-85` 的 `AppState::connection()` 只设置 `foreign_keys` 和 `journal_mode`。
- 并发相关测试中多处手动设置 `busy_timeout(Duration::from_secs(10))`，例如 `src-tauri/src/commands.rs:1569`、`src-tauri/src/orders.rs:372`。

影响：

- 前端可能同时触发保存、刷新、备份、日志写入等多个 command。
- SQLite 单写者模型下，未设置等待时间容易直接返回 `database is locked`。
- 测试使用了更宽松的连接配置，不能完全代表真实运行时。

建议：

- 在 `AppState::connection()` 中统一设置 `busy_timeout`。
- 对关键写操作保留事务边界，并在服务层对可重试锁错误做有限重试。
- 增加使用 `AppState::connection()` 的并发回归测试，而不是测试里单独配置连接。

原则说明：

- DRY：连接级配置应集中在 `AppState::connection()`。
- LSP：测试连接和生产连接行为应尽量一致，避免测试替代不了真实路径。

### P2 数据库迁移缺少版本化机制

证据：

- `src-tauri/src/db.rs:12` 的 `init_schema` 负责创建所有表。
- `src-tauri/src/db.rs:314-346` 的 `ensure_compatible_schema` 仅做少量列补齐。
- 未看到 `PRAGMA user_version`、迁移版本表或按版本排序的迁移脚本。

影响：

- 目前版本升级靠 `CREATE TABLE IF NOT EXISTS` 和零散 `ALTER TABLE`，短期可用，长期会难以追踪历史。
- 后续涉及字段重命名、数据回填、索引调整、约束变化时，容易出现不同老用户数据库状态不一致。
- 开源贡献者难以判断应该把 schema 变更放在哪里、如何验证升级。

建议：

- 引入最小迁移表或 `PRAGMA user_version`。
- 每个版本迁移函数只做一件事，并有幂等测试。
- 把“新库建表”和“旧库迁移”分开表达。

原则说明：

- OCP：新增迁移应追加新步骤，而不是不断修改旧初始化逻辑。
- SRP：初始化和升级迁移职责分离。

### P2 前端 API 契约类型较弱，容易把错误推迟到运行期

证据：

- `src/api/tauri.ts:6` 以字符串命令名调用 Tauri command。
- `src/api/catalog.ts:12-30`、`src/api/orders.ts:19-57`、`src/api/settings.ts:21-31` 多处使用 `Record<string, unknown>` 作为 payload/filter。
- `src/shared/types.ts` 已维护大量 DTO，但请求类型未充分贯穿 API 层。

影响：

- 命令名拼写、参数字段、返回结构变化不能被 TypeScript 完整捕获。
- 后端 Rust DTO 和前端调用参数容易漂移。
- 业务页面中表单值经常以弱类型对象传入 API，后续重构风险偏高。

建议：

- 建立 typed command map：命令名、请求类型、响应类型一一对应。
- 优先替换高风险写操作：出库、入库、收款、备份恢复、通用导入。
- 保留 `Record<string, unknown>` 只用于真正动态的字段映射场景。

原则说明：

- DRY：避免在页面、API wrapper、Rust DTO 中重复维护隐式契约。
- ISP：按业务命令提供精确接口，减少“万能 payload”。

### P2 设置页仍承担过多状态和流程编排

证据：

- `src/pages/SettingsPage.tsx` 约 746 行。
- `src/pages/SettingsPage.tsx:81-100` 同时维护多个 Form 和 10 多个状态。
- `src/pages/SettingsPage.tsx:115-162` 的 `refresh` 一次拉取状态、备份、设置、打印机、审计、诊断、商户、术语、功能、模板、映射、初始化状态。
- `src/pages/SettingsPage.tsx:177-445` 包含商户保存、术语保存、行业模板、通用导入、备份恢复、自检、诊断导出等多个业务流程。

影响：

- 页面已经拆出多个 Card，但核心副作用仍集中在单个页面组件。
- 任一设置功能变化都可能影响整个页面刷新和状态同步。
- 单元测试难度高，后续新设置项会继续扩大组件。

建议：

- 提取 `useSettingsBootstrap` 管理初始加载和刷新。
- 提取 `useGenericImportWorkflow` 管理通用导入状态。
- 提取 `useBackupRestoreActions` 管理备份恢复确认与反馈。
- 每次只迁移一个 workflow，避免大范围重构。

原则说明：

- SRP：页面负责布局，hook 负责流程状态。
- KISS：按现有功能边界逐步拆分，不引入全新状态框架。

### P2 后端若干文件职责偏重，长期维护成本较高

证据：

- `src-tauri/src/reports.rs` 约 2212 行，包含数据导出、系统打印机、Excel/PDF 单据生成、报表查询相关测试。
- `src-tauri/src/generalization.rs` 约 1853 行，包含初始化、行业模板、通用导入、字段映射、模板导出、导入报告和测试。
- `src-tauri/src/commands.rs` 约 1690 行，主要聚合 command，同时承载大量后端测试。
- `src-tauri/src/reports.rs:94-205` 是公开导出/打印入口，`src-tauri/src/reports.rs:744-920` 开始进入单据 Excel 模板细节，文件内上下文跨度较大。

影响：

- 文件级认知负担高，新贡献者很难快速定位职责。
- 报表查询、文档渲染、打印、导出格式属于不同变化原因，放在同一文件会增加冲突。
- 测试紧贴巨型生产文件会让模块边界更难演进。

建议：

- 将 `reports.rs` 拆为 `reports/query.rs`、`documents/excel.rs`、`documents/pdf.rs`、`documents/printing.rs`。
- 将 `generalization.rs` 拆为 `setup.rs`、`generic_import/parser.rs`、`generic_import/report.rs`、`generic_import/template.rs`。
- 将大型 `#[cfg(test)] mod tests` 挪到按模块组织的测试文件，保留私有函数测试入口时再使用小的内部测试。

原则说明：

- SRP：按变化原因拆分。
- YAGNI：先拆最常改的导入和单据模块，不做全项目重排。

### P2 测试覆盖强，但缺少真实 Tauri IPC 到 SQLite 的端到端冒烟

证据：

- `tests/e2e/core-flows.spec.ts:221` 通过 `installTauriMock` 安装 mock。
- `tests/e2e/core-flows.spec.ts:453-454` 注入 `__EASY_E2E_CALLS__` 和 `__TAURI_INTERNALS__`。
- `scripts/e2e-flow-check.mjs` 是源码字符串检查，能防止入口丢失，但不是运行时测试。
- `.github/workflows/ci.yml` 运行 typecheck、build、Rust checks、source flow、browser E2E、robustness audit，但没有运行 `tauri:build`、`package:smoke`、`release:manifest`。
- `package.json:25` 的 `release:verify` 才包含 `tauri:build`、`package:smoke`、`release:manifest`。

影响：

- UI mock E2E 可以证明交互流程和命令调用，但不能证明真实 WebView IPC、Rust command 绑定、SQLite 文件路径和权限完整工作。
- 发布前打包链路主要依赖人工运行 `release:verify`，CI 不能及时发现安装包或清单问题。

建议：

- 在 CI 增加轻量级 release smoke job，可先只运行 `npm run package:smoke` 和 `npm run release:manifest` 的可验证部分。
- 对关键 command 增加一个真实临时数据库集成测试入口。
- 保留 mock E2E，用它验证 UI；另加少量真实 Tauri 冒烟，用它验证集成边界。

原则说明：

- KISS：不要把所有 E2E 都变成真实桌面自动化，只补关键边界。
- DRY：CI 与 `release:verify` 的发布前门禁尽量复用同一脚本。

### P2 构建产物体积已有风险信号，缺少明确性能预算

证据：

本地 `npm run build` 输出：

- 主入口 chunk：约 719.23 KB，gzip 后约 233.72 KB。
- `vendor-echarts` chunk：约 555.04 KB，gzip 后约 190.19 KB。
- CSS：约 242.72 KB，gzip 后约 42.05 KB。
- `vite.config.ts` 将 `chunkSizeWarningLimit` 设置为 800，当前主入口不会触发告警。

影响：

- 对桌面应用不是致命问题，但首屏启动、低配机器 WebView 加载和后续功能增长会受影响。
- 当前阈值偏宽，容易让主入口继续膨胀。

建议：

- 增加 bundle analyze 或简单的 chunk size 检查脚本。
- 优先分析 Ant Design、表格、图表相关入口是否被主 chunk 提前引入。
- 将性能预算写入 CI，避免无意引入整包依赖。

原则说明：

- KISS：先建立预算和观测，不急于复杂优化。
- YAGNI：只优化真实进入首屏的依赖。

### P3 开源治理材料还不完整

证据：

- 仓库已有 `README.md`、`CONTRIBUTING.md`、`LICENSE`、Issue/PR 模板和 Release Checklist。
- 未看到 `SECURITY.md`、`CHANGELOG.md`、`CODE_OF_CONDUCT.md`、`SUPPORT.md`。
- `CONTRIBUTING.md` 提醒不要上传敏感数据，但没有安全漏洞披露流程。

影响：

- 开源用户不知道安全问题应私下反馈还是公开 Issue。
- 版本变化、破坏性变更、数据迁移说明缺少稳定入口。
- 对真实经营数据软件来说，安全披露和隐私边界比普通工具更重要。

建议：

- 新增 `SECURITY.md`：说明漏洞报告方式、敏感日志处理、支持版本。
- 新增 `CHANGELOG.md`：按版本记录功能、修复、迁移、已知问题。
- 可选新增 `SUPPORT.md`：说明社区支持范围和商业/财务/税务责任边界。

原则说明：

- KISS：先补最小治理文档，不引入复杂流程。
- OCP：版本记录和安全披露后续可持续追加。

### P3 本地工作区存在大量忽略产物，影响分析和协作体验

证据：

- `git status --short --ignored` 显示 `node_modules/`、`dist/`、`release/`、`test-results/`、`src-tauri/target/`、多组 `src-tauri/target-pack-*`。
- `.gitignore:1-4`、`.gitignore:11-12` 已忽略这些目录，说明它们不应进入版本库。

影响：

- 本地搜索、IDE 索引、备份、压缩和杀毒扫描都会变慢。
- 新贡献者如果直接复制工作区，容易误以为这些是源码的一部分。

建议：

- 保留 `.gitignore`，清理本地构建产物。
- 发布产物统一放到可复现的 release 输出目录，不长期堆在源码树下。
- 文档中说明哪些目录是生成产物。

原则说明：

- YAGNI：生成物不应长期占据源码工作区。
- DRY：打包产物由脚本生成，不靠人工保存多份历史目录。

## 建议路线图

### 第一阶段：数据安全与可靠性

1. 为日志增加脱敏与默认摘要模式。
2. 改造备份为 SQLite 一致性备份。
3. 在运行时连接统一设置 `busy_timeout`。
4. 为以上三项补回归测试。

### 第二阶段：可维护性

1. 为数据库迁移引入版本号。
2. 替换高风险写操作的 `Record<string, unknown>`。
3. 拆分 `SettingsPage.tsx` 的通用导入和备份恢复 workflow。
4. 拆分 `reports.rs` 中文档渲染与报表查询。

### 第三阶段：开源发布治理

1. 增加 `SECURITY.md` 和 `CHANGELOG.md`。
2. 将发布 smoke 纳入 CI 或独立手动触发 workflow。
3. 增加 bundle size 预算。
4. 清理本地忽略产物，保持源码树轻量。

## 总体评价

EasyInventory 的业务覆盖和自动化检查基础不错，尤其 Rust 侧对库存、订单、并发和报表已有较多测试。当前最值得投入的不是大范围重写，而是把真实经营数据软件必需的可靠性边界补齐：日志不泄露、备份可信、并发写入稳定、迁移可追踪。按上述顺序处理，可以在不破坏现有功能的前提下明显提升开源项目质量。
