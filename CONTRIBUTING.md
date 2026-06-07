# Contributing to EasyInventory

感谢你愿意改进 EasyInventory。这个项目面向真实本地经营数据，贡献时请优先保护用户数据安全、库存准确性和金额一致性。

## 开发环境

建议环境：

- Node.js 20 或更新版本
- npm
- Rust stable toolchain
- Windows Tauri 构建依赖

安装依赖：

```powershell
npm install
```

常用命令：

```powershell
npm run dev
npm run tauri:dev
npm run check
```

## 工程约定

- 前端入口保持薄层：`src/App.tsx` 只组合 Provider，应用壳、路由、启动加载放在 `src/app`。
- 前端 API 按业务域放在 `src/api/catalog.ts`、`orders.ts`、`reports.ts`、`settings.ts`、`system.ts`，由 `src/api/inventory.ts` 聚合。
- 后端按 `commands / services / repositories / domain` 分层。Tauri command 只做入参、出参和错误转换；业务事务放 service；SQL 放 repository；金额、价格、库存等纯规则放 domain。
- 涉及订单、库存、返利、收款、备份恢复的变更必须有测试覆盖。
- 用户输入相关 SQL 必须参数化；动态排序字段必须使用白名单。
- 不要提交本地数据库、备份、日志、诊断包、构建产物或含客户信息的截图。

## 提交流程

1. 先开 Issue 或在 PR 描述中说明问题背景。
2. 修改前阅读相关业务模块和已有测试。
3. 小步提交改动，避免把重构和功能改动混在一起。
4. PR 描述中写清楚影响范围：数据迁移、备份、UI、打印、打包或兼容性。

提交 PR 前至少运行：

```powershell
npm run typecheck
npm run build
npm run test:rust
npm run audit:robustness
```

发布相关改动建议运行：

```powershell
npm run check
```

## 反馈问题

请使用 `.github/ISSUE_TEMPLATE/bug_report.md`。日志、截图和诊断包请先脱敏，不要上传客户名称、电话、地址、真实订单或库存金额等敏感信息。

## 许可证

项目采用 AGPLv3。提交贡献即表示你同意你的贡献按本仓库许可证发布。
