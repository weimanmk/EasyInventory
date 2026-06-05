# EasyInventory

<p align="center">
  <img alt="Version" src="https://img.shields.io/badge/version-1.3.0-blue">
  <img alt="License" src="https://img.shields.io/github/license/weimanmk/EasyInventory">
  <img alt="Rust" src="https://img.shields.io/badge/rust-1.85%2B-orange">
  <img alt="Tauri" src="https://img.shields.io/badge/tauri-2.x-24C8DB">
  <img alt="React" src="https://img.shields.io/badge/react-19-61DAFB">
  <img alt="SQLite" src="https://img.shields.io/badge/sqlite-local-003B57">
</p>

<p align="center">
  Windows 本地库存、计价、出库打单与经营分析工具
</p>

EasyInventory 是一个面向 Windows 单机使用的本地库存、计价、出库打单和经营分析软件，适合小型商贸、批发、配送、门店和档口经营场景。

当前版本：`1.3.0`

## 功能概览

- 本地 SQLite 数据库，支持自动备份、手动备份、恢复和诊断。
- 初始化向导支持商户信息、行业模板、业务术语、功能开关和单据模板配置。
- 商品、客户、供应商、入库、出库、库存余额、库存流水和库存盘点管理。
- 快速出库支持商品选择、扫码查询、客户价格规则、买赠、折现和额度抵扣。
- 欠款收款、客户对账单、供应商采购台账、进销存报表和利润统计。
- 商品经营排行、客户经营分析、日/月/年利润趋势、柱状图和占比图。
- 单据档案、打印预览、订单导出、PDF 导出和历史单据重新导出。
- 通用 Excel 导入支持商品、客户、期初库存、字段映射、预览确认和导入报告。
- 历史兼容迁移保留为高级入口，用于固定格式旧数据的一次性迁移。

## 技术栈

| 层级 | 技术 |
| --- | --- |
| 桌面壳 | Tauri 2 |
| 前端 | React 19、TypeScript、Vite、Ant Design、ECharts、React Router、Zustand、Day.js |
| 后端 | Rust、Tauri commands、rusqlite、calamine、umya-spreadsheet、chrono、serde |
| 数据库 | SQLite |
| 打包 | Windows release EXE 与 NSIS 安装包 |

## 运行时数据

应用运行时会在系统应用数据目录中维护业务数据，并创建以下子目录：

- `data`：SQLite 数据库。
- `backups`：数据库备份。
- `orders`：订单单据。
- `exports`：导出文件。
- `logs`：运行日志。
- `config`：本地配置。

这些路径可以在软件的系统设置页查看和打开。

## 开发环境

建议环境：

- Node.js 20 或更新版本。
- npm。
- Rust stable toolchain。
- Windows 上的 Tauri 构建依赖。

安装依赖：

```powershell
npm install
```

启动前端开发服务：

```powershell
npm run dev
```

启动 Tauri 开发模式：

```powershell
npm run tauri:dev
```

## 常用命令

```powershell
npm run typecheck
npm run build
npm run check
npm run tauri:build
npm run package:smoke
npm run release:verify
```

命令说明：

- `npm run typecheck`：TypeScript 类型检查。
- `npm run build`：前端生产构建。
- `npm run check`：完整质量检查，包含前端构建、Rust 格式检查、clippy、Rust 测试、流程检查、浏览器 E2E 和健壮性检查。
- `npm run tauri:build`：构建 Windows release EXE 和 NSIS 安装包。
- `npm run package:smoke`：检查打包产物是否存在且非空。
- `npm run release:verify`：发布前完整验证。

## 打包产物

执行：

```powershell
npm run tauri:build
```

默认产物位于：

```text
src-tauri/target/release/easyinventory.exe
src-tauri/target/release/bundle/nsis/EasyInventory_1.3.0_x64-setup.exe
```

## 数据导入

新用户建议优先使用系统设置页中的“通用数据导入”：

1. 选择导入类型。
2. 下载示例模板或选择已有 Excel 文件。
3. 根据预览结果确认新增、覆盖、跳过和异常行。
4. 确认写入后生成导入报告。

字段名称不一致时，可以通过字段映射方案适配现有 Excel 表头。高级历史兼容迁移入口只用于固定格式旧数据迁移，日常追加导入应使用通用数据导入。

## 目录结构

```text
.
├── src/                 # React 前端
├── src-tauri/           # Tauri/Rust 后端
├── scripts/             # 构建、验收和辅助脚本
├── tests/               # 浏览器 E2E 测试
├── docs/                # 项目文档
├── package.json
└── README.md
```

## 发布前检查

发布前建议执行：

```powershell
npm run release:verify
```

该命令会完成质量检查、Tauri 打包和安装包 smoke 检查。

## 许可证

本项目采用 `GNU Affero General Public License v3.0`（AGPLv3）。

你可以在 AGPLv3 条款下使用、复制、修改和分发本项目。若修改版本通过网络服务方式提供给用户，也需要按 AGPLv3 要求向用户提供对应源码。

完整条款见 [LICENSE](LICENSE)。
