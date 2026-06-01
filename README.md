# EasyInventory V1.0

EasyInventory 是一个面向 Windows 单机使用的本地库存、计价、出库打单软件，用来替代原 Excel 工作簿中的库存、客户、出入库、利润和打印流程。

当前项目位于：

```text
C:/Users/ww/Desktop/work/EasyInventory
```

## 技术栈

| 层级 | 当前实现 |
| --- | --- |
| 桌面壳 | Tauri 2 |
| 前端 | React 19、TypeScript、Vite、Ant Design、React Router、Zustand、Day.js |
| 后端 | Rust、Tauri commands、rusqlite、calamine、umya-spreadsheet |
| 数据库 | SQLite 本地文件数据库 |
| 打包 | Windows release EXE 与 NSIS 安装包 |

## 本地数据与文件

运行时数据由 Tauri 应用数据目录管理，启动时会创建 `data`、`backups`、`orders`、`exports`、`logs`、`config` 等目录。

核心路径可在系统设置页查看：

- 数据库：`data/inventory.db`
- 订单单据：`orders/客户名/单号_客户名.xlsx`
- 数据库备份：`backups/*.db`

默认 Excel 迁移源：

```text
C:/Users/ww/Desktop/work/订单库存表3.02 - 副本 (2).xlsm
```

## 当前已实现功能

### 1. Excel 一次性迁移

- 支持从原 `.xlsm` 导入商品、客户、库存流水和可解析利润行计数。
- 商品导入来源：`库存` 工作表有效区。
- 客户导入来源：`客户信息` 工作表有效区。
- 库存流水导入来源：`明细` 工作表有效区。
- 当前迁移测试验证原始表可导入：商品 `280` 个、客户 `997` 个、有效库存流水 `3` 条。
- 迁移后以 SQLite 为主数据源，原 Excel 仅作为备份保留。

### 2. SQLite 数据模型

已创建并使用以下主表：

- `products` 商品表
- `customers` 客户表
- `inventory_movements` 库存流水表
- `stock_balances` 库存余额表
- `inbound_records` 入库记录表
- `orders` 出库单表
- `order_items` 出库明细表
- `customer_product_rules` 客户商品规则表
- `monthly_credits` 月费账本表
- `documents` 单据档案表
- `settings` 系统设置表
- `backup_logs` 备份日志表

### 3. 首页

- 显示今日出库单数、商品销售额、客户实收、利润。
- 提供快速入口：快速出库、入库、商品库存、客户规则、客户管理、月费账本、每日利润。
- 显示最近出库单和最近入库记录。
- 最近出库单支持点击整行或“详情”按钮打开出库详情抽屉。
- 首页展示低库存商品数量，并可跳转到商品库存页的低库存筛选。

### 4. 快速出库

- 支持选择日期、地区、客户，选择客户后自动带出地址。
- 支持商品选择大弹窗，按类别、名称、条码筛选商品。
- 商品弹窗内可填写数量、单价、备注并直接加入出库单。
- 支持报价预览：固定价、买赠、直接折现、生成月费提示。
- 支持出库明细行内修改数量、单价、备注，并重新计算金额和规则预览。
- 支持选择可用月费进行本单抵扣。
- 保存订单时生成订单、订单明细、库存流水、月费记录，并重算库存余额。
- 支持保存并导出、保存并打印。
- 单号按 `yyyymmdd + 三位流水号` 生成。

### 5. 入库

- 支持选择日期、商品、数量、进货价和备注。
- 保存后写入入库记录和库存流水。
- 自动重算当前库存、平均进货价和库存价值。

### 6. 商品库存

- 支持按类别、商品名、条码搜索。
- 支持低库存筛选和低库存/负库存行提示。
- 支持新增、编辑、删除商品。
- 商品删除采用停用方式，避免破坏历史订单和库存流水。

### 7. 客户管理

- 支持按地区筛选、按客户名或地址搜索。
- 支持新增、编辑、删除客户。
- 客户删除采用停用方式，避免破坏历史订单和单据档案。
- 支持从客户行快速跳转到历史单据和客户商品规则。

### 8. 客户商品规则

- 支持按客户、类别筛选规则。
- 支持新增、编辑、停用、删除规则。
- 规则字段包括固定售价、每满数量、赠品商品、赠品数量、直接折现、生成月费、月费可用类别和备注。

### 9. 月费账本

- 支持按客户、类别、状态查询。
- 支持展示来源订单、生成金额、已使用、剩余、生成日期、可用月份和状态。
- 支持关闭、作废月费记录。
- 后端已提供可用月费查询和订单保存时的月费抵扣数据结构。

### 10. 每日利润

- 支持按日期查询每日汇总。
- 汇总字段包括出库单数、商品销售额、客户实收和利润。
- 明细表展示单号、客户、销售额、实收、折现、月费抵扣、成本和利润。

### 11. 单据档案、导出和打印

- 保存出库单后自动生成客户可读 `.xlsx` 单据。
- 单据保存到 `orders/客户名/单号_客户名.xlsx`。
- 单据档案支持按客户、单号、日期范围查询。
- 单据档案支持预览、打开、打印、重新导出已有单据。
- 支持订单作废，作废时回滚该订单库存流水、月费抵扣和单据状态。
- 后端单据模板已按原 Excel `表单!A42:J64` 打印区域复刻核心布局、表头、客户信息、明细、合计和边框样式。
- release 构建使用 `windows_subsystem = "windows"`，正常 release 启动不应拉起空白控制台窗口。

### 12. 系统设置与备份

- 系统设置页展示数据库、数据目录、订单目录、备份目录和版本号。
- 支持手动备份 SQLite 数据库。
- 启动时按 `daily_auto_backup` 设置执行每日自动备份。
- 支持打开备份目录。
- 支持在设置页触发 Excel 一次性导入并展示导入数量与异常信息。
- 支持配置每日自动备份开关、默认打印模板、默认导出格式和默认打印机。
- 支持打开导出目录。

### 13. 数据导出

- 支持导出商品资料、客户资料、入库记录、月费账本和利润报表。
- 导出文件保存到应用数据目录的 `exports` 子目录。
- 支持按日期、客户、类别、状态、关键字等条件筛选后导出。

## Tauri Commands 覆盖情况

已暴露并接入的主要命令：

- 商品：`list_products`、`create_product`、`update_product`、`disable_product`
- 客户：`list_customers`、`create_customer`、`update_customer`、`disable_customer`
- 入库：`create_inbound`、`list_inbound_records`
- 出库：`preview_quote`、`save_order`、`get_order`、`list_orders`、`export_order_document`、`print_order_document`、`print_order_document_with_options`、`void_order`
- 规则：`list_customer_product_rules`、`save_customer_product_rule`、`disable_customer_product_rule`、`delete_customer_product_rule`
- 月费：`list_monthly_credits`、`get_available_monthly_credits`、`close_monthly_credit`、`void_monthly_credit`
- 利润：`get_daily_profit_summary`、`list_profit_records`
- 单据：`list_documents`、`open_document`、`export_document`、`print_document`
- 导出：`export_data`、`open_exports_folder`
- 迁移与备份：`import_excel`、`get_import_status`、`create_backup`、`list_backups`、`open_backup_folder`
- 设置与打印机：`list_settings`、`save_settings`、`list_printers`
- 状态：`get_app_status`

## PRD 完整性检查

| 模块 | PRD 要求 | 当前状态 |
| --- | --- | --- |
| Windows 本地桌面应用 | Tauri 桌面应用，本地 SQLite | 已完成 |
| Excel 一次性迁移 | 商品、客户、库存流水、利润等迁移 | 基本完成；利润表仅统计可解析行数，历史利润未完整入库 |
| 商品管理 | 查询、新增、编辑、删除/停用、库存展示 | 已完成 |
| 客户管理 | 查询、新增、编辑、删除/停用 | 已完成 |
| 入库管理 | 入库、库存流水、库存余额、平均进货价 | 已完成 |
| 快速出库 | 客户选择、商品选择、计价、保存、导出、打印 | 基本完成 |
| 商品选择弹窗 | 类别、搜索、数量、价格、备注、加入 | 已完成 |
| 出库明细编辑 | 明细内修改数量/价格/备注并重算 | 已完成 |
| 客户商品规则 | 固定价、买赠、折现、月费 | 已完成 |
| 月费账本 | 查询、关闭、作废、可用状态 | 已完成 |
| 出库时月费抵扣 | 选择可用月费并抵扣 | 已完成 |
| 利润查询 | 每日汇总和订单明细 | 基本完成 |
| 单据导出 | 客户订单 xlsx | 已完成 |
| 单据样式 | 尽量复刻原 Excel 打印区 | 已完成核心样式并有后端测试覆盖 |
| 打印预览 | 打印前预览、选择打印机 | 已完成预览和打印机选择入口；Windows 直接定向打印失败时会打开文件供手动打印 |
| 单据档案 | 查询、打开、打印、重新导出 | 已完成 |
| 查询与导出 | 商品、客户、入库、月费、利润等导出 | 已完成 |
| 备份 | 启动自动备份、手动备份、目录打开 | 已完成 |
| 订单作废/回滚 | 作废订单并回滚库存、月费、单据状态 | 已完成 |
| 数据恢复 | 图形化恢复 | 未做；PRD 归到 V1.1 可做 |
| 登录权限、云同步、多端 | V1.0 非目标 | 未做，符合 PRD |

## TODO

### P0/P1：已补齐

1. 快速出库主表行内编辑。
2. 出库时月费抵扣 UI。
3. 打印预览。
4. 单据档案重新导出入口。
5. 通用业务数据导出。
6. 首页低库存统计与跳转。
7. 客户管理历史单据/规则快捷入口。
8. 订单作废/回滚。
9. 系统设置项编辑。
10. 打印机选择入口。


### P2：后续版本可做

1. 数据恢复图形化入口。
2. 批量导入客户商品规则。
3. 盘点功能。
4. 利润趋势图和更丰富的经营分析。
5. 单据模板可视化编辑。
6. 更系统的前端自动化测试或端到端测试。

## 常用命令

在项目根目录执行：

```powershell
cd "C:/Users/ww/Desktop/work/EasyInventory"
npm install
npm run build
npm run tauri:dev
npm run tauri:build
```

当前机器 `cargo` 不一定在 PATH 中，可使用固定路径执行后端测试：

```powershell
cd "C:/Users/ww/Desktop/work/EasyInventory/src-tauri"
& "C:/Users/ww/.cargo/bin/cargo.exe" test
```

如需指定打包目录：

```powershell
cd "C:/Users/ww/Desktop/work/EasyInventory"
$env:PATH = "C:/Users/ww/.cargo/bin;$env:PATH"
$env:CARGO_TARGET_DIR = "C:/Users/ww/Desktop/work/EasyInventory/src-tauri/target-pack"
npm run tauri:build
```

## 已知构建产物

最近一次检查到的打包产物：

```text
C:/Users/ww/Desktop/work/EasyInventory/src-tauri/target-pack/release/easyinventory.exe
C:/Users/ww/Desktop/work/EasyInventory/src-tauri/target-pack/release/bundle/nsis/EasyInventory_1.0.0_x64-setup.exe
```

## 验证记录

当前 README 依据以下代码与配置检查结果整理：

- 页面路由：`src/App.tsx`
- 前端 API 封装：`src/api/inventory.ts`
- 核心页面：`src/pages/*`
- 商品选择弹窗：`src/components/ProductPickerModal.tsx`
- 数据库初始化：`src-tauri/src/db.rs`
- Tauri commands：`src-tauri/src/commands.rs`
- 订单规则：`src-tauri/src/orders.rs`
- Excel 迁移：`src-tauri/src/excel.rs`
- 单据生成：`src-tauri/src/reports.rs`
- 打包配置：`src-tauri/tauri.conf.json`

已存在的关键后端测试覆盖：

- Excel 原始表导入数量：商品 `280`、客户 `997`、有效流水 `3`
- 报价预览中的买赠、折现、月费金额计算
- 订单作废时库存和月费使用回滚
- 单据 `.xlsx` 样式与原 Excel 打印区域核心结构比对

本次 README 更新后已执行：

```powershell
npm run build
& "C:/Users/ww/.cargo/bin/cargo.exe" test
```

结果：

- `npm run build`：通过，TypeScript 与 Vite 生产构建成功。
- `cargo test`：通过，`5 passed; 0 failed`。
- `cargo test` 当前仍有 2 个既有 `dead_code` warning，不影响测试结果。
