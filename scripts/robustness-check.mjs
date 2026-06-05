import { existsSync, readFileSync } from 'node:fs';
import path from 'node:path';

const root = process.cwd();

function read(relativePath) {
  const fullPath = path.join(root, relativePath);
  if (!existsSync(fullPath)) {
    throw new Error(`缺少文件：${relativePath}`);
  }
  return readFileSync(fullPath, 'utf8');
}

function requireText(file, text, reason) {
  const content = read(file);
  if (!content.includes(text)) {
    throw new Error(`${file} 缺少：${reason}`);
  }
}

function requireAll(file, checks) {
  for (const [text, reason] of checks) {
    requireText(file, text, reason);
  }
}

const checks = [
  {
    name: '项目自动化测试入口完整',
    run() {
      requireAll('package.json', [
        ['"version": "1.3.0"', '前端包版本为 V1.3'],
        ['"e2e:flows": "node scripts/e2e-flow-check.mjs"', '前端核心流程验收脚本'],
        ['"e2e:browser": "npm run build && node scripts/browser-e2e-check.mjs"', '浏览器级前端 E2E 脚本'],
        ['"package:smoke": "node scripts/package-smoke-check.mjs"', '安装包产物 smoke 检查脚本'],
        ['"release:verify": "npm run check && npm run tauri:build && npm run package:smoke"', '发布验收脚本'],
        ['"test": "npm run typecheck && npm run format:rust && npm run lint:rust && npm run test:rust && npm run e2e:flows && npm run e2e:browser && npm run audit:robustness"', 'npm test 聚合测试、流程验收、浏览器 E2E 和健壮性检查'],
        ['"check": "npm run build && npm run format:rust && npm run lint:rust && npm run test:rust && npm run e2e:flows && npm run e2e:browser && npm run audit:robustness"', 'npm run check 聚合构建、流程验收、浏览器 E2E 和质量门禁'],
        ['"lint:rust": "cd src-tauri && cargo clippy --all-targets -- -D warnings"', 'Rust clippy 严格检查']
      ]);
      requireAll('playwright.config.ts', [
        ['channel: \'chrome\'', '浏览器 E2E 使用系统 Chrome'],
        ['node ./node_modules/vite/bin/vite.js preview', '浏览器 E2E 使用构建产物预览']
      ]);
      requireAll('scripts/browser-e2e-check.mjs', [
        ['expectedCount = 10', '浏览器 E2E 覆盖 10 条核心流程'],
        ['浏览器 E2E 通过', '浏览器 E2E 通过提示']
      ]);
      requireAll('scripts/package-smoke-check.mjs', [
        ['easyinventory.exe', 'release EXE 产物检查'],
        ['EasyInventory_${pkg.version}_x64-setup.exe', 'NSIS 安装包产物检查']
      ]);
      requireAll('src-tauri/Cargo.toml', [['version = "1.3.0"', 'Rust 包版本为 V1.3']]);
      requireAll('src-tauri/tauri.conf.json', [['"version": "1.3.0"', 'Tauri 打包版本为 V1.3']]);
    }
  },
  {
    name: 'Ant Design App Provider 可用',
    run() {
      requireAll('src/App.tsx', [
        ['App as AntApp', '导入 AntApp Provider'],
        ['<AntApp>', '包裹应用内容'],
        ['</AntApp>', '闭合 AntApp Provider']
      ]);
    }
  },
  {
    name: '客户端详细日志链路可用',
    run() {
      requireAll('src-tauri/src/logger.rs', [
        ['writes_sanitized_log_line_to_daily_file', '日志落盘和换行清洗测试']
      ]);
      requireAll('src-tauri/src/lib.rs', [['write_client_log', '注册 write_client_log 命令']]);
      requireAll('src-tauri/src/commands.rs', [
        ['pub fn write_client_log', '后端客户端日志命令'],
        ['logger::error("command"', '统一记录后端命令错误链']
      ]);
      requireAll('src/api/tauri.ts', [
        ['writeClientLog', '前端写日志工具'],
        ['命令返回失败', '记录 API 失败'],
        ['durationMs', '记录 API 耗时']
      ]);
    }
  },
  {
    name: '商品/客户/规则保存链路有可诊断日志',
    run() {
      for (const file of ['src/pages/ProductsPage.tsx', 'src/pages/CustomersPage.tsx', 'src/pages/RulesPage.tsx']) {
        requireAll(file, [
          ['表单校验未通过', '记录表单校验失败'],
          ['保存后刷新完成', '记录保存后的刷新结果']
        ]);
      }
    }
  },
  {
    name: '利润类别筛选有回归测试',
    run() {
      requireAll('src-tauri/src/reports.rs', [
        ['list_profit_records_filters_by_order_item_category', '利润类别筛选测试'],
        ['COUNT(*) > 0 FROM order_items', '通过订单明细类别过滤利润记录']
      ]);
    }
  },
  {
    name: '散客固定客户不会丢失',
    run() {
      requireAll('src-tauri/src/db.rs', [
        ['GUEST_CUSTOMER_NAME', '固定客户常量'],
        ['guest_customer_name', '默认客户显示名设置'],
        ['guest_customer_id', '默认客户固定 ID 设置'],
        ['ensure_guest_customer', '确保散客存在'],
        ['ensure_guest_customer_creates_active_fixed_customer', '散客创建测试'],
        ['ensure_guest_customer_reactivates_existing_guest_customer', '散客恢复测试'],
        ['ensure_guest_customer_renames_existing_fixed_customer_by_id', '默认客户改名后仍复用固定 ID 测试']
      ]);
      requireAll('src-tauri/src/app.rs', [['db::ensure_guest_customer', '启动时补齐散客']]);
      requireAll('src-tauri/src/excel.rs', [['db::ensure_guest_customer', 'Excel 导入后补齐散客']]);
      requireAll('src-tauri/src/commands.rs', [
        ['db::guest_customer_name', '按配置读取默认客户名'],
        ['名称不能修改', '禁止改名'],
        ['不能删除', '禁止删除'],
        ['create_customer_record_with_legacy_guest_name_reuses_configured_guest_customer', '旧默认客户名不会创建重复客户测试']
      ]);
    }
  },
  {
    name: '导出/打印不会拉起空白 PowerShell 窗口',
    run() {
      requireAll('src-tauri/src/reports.rs', [
        ['creation_flags(0x08000000)', 'Windows 子进程隐藏控制台窗口']
      ]);
    }
  },
  {
    name: 'V1.2 功能入口完整',
    run() {
      requireAll('src-tauri/src/lib.rs', [
        ['find_product_by_barcode', '扫码命令注册'],
        ['list_suppliers', '供应商列表命令注册'],
        ['create_supplier', '供应商新增命令注册'],
        ['list_customer_balances', '欠款余额命令注册'],
        ['create_payment', '收款命令注册'],
        ['list_inventory_report', '进销存报表命令注册']
      ]);
      requireAll('src/App.tsx', [
        ['SuppliersPage', '供应商管理页面路由'],
        ['ReceivablesPage', '欠款收款页面路由'],
        ['InventoryReportPage', '进销存报表页面路由'],
        ['/suppliers', '供应商管理菜单入口'],
        ['/receivables', '欠款收款菜单入口'],
        ['/inventory-report', '进销存报表菜单入口']
      ]);
      requireAll('src/components/ProductPickerModal.tsx', [
        ['findProductByBarcode', '扫码枪按条码加入商品'],
        ['扫码后回车自动加入', '扫码输入入口']
      ]);
      requireAll('src/pages/InboundPage.tsx', [
        ['supplierId', '入库关联供应商'],
        ['api.suppliers', '入库加载供应商选项']
      ]);
      requireAll('src/pages/ExportPage.tsx', [
        ['inventory_report', '导出进销存报表']
      ]);
    }
  },
  {
    name: 'V1.2 测试覆盖关键风险',
    run() {
      requireAll('src-tauri/src/orders.rs', [
        ['concurrent_orders_keep_unique_order_numbers', '高并发订单号测试'],
        ['large_order_save_finishes_under_three_seconds', '100 行订单性能测试']
      ]);
      requireAll('src-tauri/src/db.rs', [
        ['high_volume_product_and_customer_queries_stay_under_two_seconds', '万级商品客户查询测试']
      ]);
      requireAll('src-tauri/src/reports.rs', [
        ['inventory_report_summarizes_purchase_sales_and_gifts', '进销存汇总功能测试'],
        ['high_volume_inventory_report_stays_under_two_seconds', '高数据量进销存报表测试']
      ]);
      requireAll('src-tauri/src/commands.rs', [
        ['barcode_scan_returns_only_active_exact_match', '扫码精确匹配测试'],
        ['customer_balance_subtracts_normal_payments_and_ignores_voided_payments', '欠款收款余额测试'],
        ['supplier_crud_helpers_disable_and_reload_supplier', '供应商停用测试'],
        ['create_payment_rejects_invalid_order_customer_pair', '收款关联订单校验测试'],
        ['create_inbound_rejects_disabled_supplier', '入库供应商状态校验测试']
      ]);
    }
  },
  {
    name: 'V1.3 数据安全和库存准确入口完整',
    run() {
      requireAll('src-tauri/src/db.rs', [
        ['audit_logs', '审计日志表'],
        ['inventory_adjustments', '库存调整表'],
        ['stocktake_records', '盘点记录表'],
        ['restore_database_file', '数据库恢复底层函数']
      ]);
      requireAll('src-tauri/src/lib.rs', [
        ['restore_backup', '恢复备份命令注册'],
        ['create_inventory_adjustment', '库存调整新增命令注册'],
        ['create_stocktake', '盘点新增命令注册'],
        ['list_audit_logs', '审计日志命令注册']
      ]);
      requireAll('src/App.tsx', [
        ['InventoryControlPage', '库存盘点页面路由'],
        ['/inventory-control', '库存盘点菜单入口']
      ]);
      requireAll('src/pages/SettingsPage.tsx', [
        ['restoreBackup', '设置页恢复备份入口'],
        ['审计日志', '设置页审计日志入口']
      ]);
      requireAll('src/pages/InventoryControlPage.tsx', [
        ['扫码或输入条码后回车定位', '盘点页扫码定位入口'],
        ['locateScannedProduct', '盘点页扫码定位逻辑']
      ]);
    }
  },
  {
    name: 'V1.3 库存调整和恢复测试覆盖关键风险',
    run() {
      requireAll('src-tauri/src/commands.rs', [
        ['inventory_adjustment_updates_stock_and_void_reverses_it', '库存调整和作废反冲测试'],
        ['stocktake_records_difference_and_void_reverses_it', '盘点差异和作废反冲测试'],
        ['module: "order"', '订单保存和作废写入审计日志'],
        ['module: "rule"', '规则变更写入审计日志']
      ]);
      requireAll('src-tauri/src/db.rs', [
        ['restore_database_file_creates_snapshot_and_replaces_database', '恢复前快照和数据库替换测试']
      ]);
    }
  },
  {
    name: 'V1.4 客户对账单入口完整',
    run() {
      requireAll('src-tauri/src/models.rs', [
        ['CustomerStatementRequest', '客户对账单请求模型'],
        ['period_discount_amount', '客户对账单本期优惠字段']
      ]);
      requireAll('src-tauri/src/reports.rs', [
        ['pub fn customer_statement', '客户对账单计算函数'],
        ['export_customer_statement', '客户对账单导出函数'],
        ['"customer_statement" => export_customer_statement', '客户对账单导出类型分发'],
        ['customer_statement_computes_opening_and_ignores_voided_records', '客户对账单金额滚动测试'],
        ['export_customer_statement_outputs_opening_rows_and_summary', '客户对账单导出结构测试']
      ]);
      requireAll('src-tauri/src/commands.rs', [
        ['pub fn get_customer_statement', '客户对账单命令']
      ]);
      requireAll('src-tauri/src/lib.rs', [
        ['get_customer_statement', '客户对账单命令注册']
      ]);
      requireAll('src/api/inventory.ts', [
        ['customerStatement', '前端客户对账单 API'],
        ['get_customer_statement', '前端调用客户对账单命令']
      ]);
      requireAll('src/shared/types.ts', [
        ['CustomerStatementDto', '前端客户对账单类型'],
        ['periodDiscountAmount', '前端客户对账单本期优惠字段']
      ]);
      requireAll('src/App.tsx', [
        ['CustomerStatementPage', '客户对账单页面路由'],
        ['/customer-statement', '客户对账单菜单入口']
      ]);
      requireAll('src/pages/ExportPage.tsx', [
        ['customer_statement', '导出页客户对账单类型']
      ]);
    }
  },
  {
    name: 'V1.4 供应商采购台账入口完整',
    run() {
      requireAll('src-tauri/src/models.rs', [
        ['SupplierPurchaseLedgerRequest', '供应商采购台账请求模型'],
        ['SupplierPurchaseLedgerDto', '供应商采购台账返回模型']
      ]);
      requireAll('src-tauri/src/reports.rs', [
        ['pub fn supplier_purchase_ledger', '供应商采购台账查询函数'],
        ['supplier_purchase_ledger_summarizes_details_and_monthly_trend', '供应商采购台账汇总、明细和趋势测试']
      ]);
      requireAll('src-tauri/src/commands.rs', [
        ['pub fn get_supplier_purchase_ledger', '供应商采购台账命令']
      ]);
      requireAll('src-tauri/src/lib.rs', [
        ['get_supplier_purchase_ledger', '供应商采购台账命令注册']
      ]);
      requireAll('src/api/inventory.ts', [
        ['supplierPurchaseLedger', '前端供应商采购台账 API'],
        ['get_supplier_purchase_ledger', '前端调用供应商采购台账命令']
      ]);
      requireAll('src/shared/types.ts', [
        ['SupplierPurchaseLedgerDto', '前端供应商采购台账类型'],
        ['monthlyTrend', '前端供应商采购月度趋势字段']
      ]);
      requireAll('src/App.tsx', [
        ['SupplierLedgerPage', '供应商采购台账页面路由'],
        ['/supplier-ledger', '供应商采购台账菜单入口']
      ]);
      requireAll('src/pages/SupplierLedgerPage.tsx', [
        ['EChart', '供应商采购月度趋势图'],
        ['查看明细', '供应商相关入库明细入口']
      ]);
    }
  },
  {
    name: 'V1.4 基础资料批量编辑入口完整',
    run() {
      requireAll('src-tauri/src/models.rs', [
        ['BatchUpdateProductsRequest', '商品批量编辑请求模型'],
        ['BatchUpdateCustomersRequest', '客户批量编辑请求模型'],
        ['BatchUpdateSuppliersRequest', '供应商批量编辑请求模型'],
        ['BatchUpdateResultDto', '批量编辑返回模型']
      ]);
      requireAll('src-tauri/src/commands.rs', [
        ['pub fn batch_update_products', '商品批量编辑命令'],
        ['pub fn batch_update_customers', '客户批量编辑命令'],
        ['pub fn batch_update_suppliers', '供应商批量编辑命令'],
        ['batch_update_products_updates_requested_fields_only', '商品批量编辑测试'],
        ['batch_update_customers_and_suppliers_update_requested_fields', '客户和供应商批量编辑测试']
      ]);
      requireAll('src-tauri/src/lib.rs', [
        ['batch_update_products', '商品批量编辑命令注册'],
        ['batch_update_customers', '客户批量编辑命令注册'],
        ['batch_update_suppliers', '供应商批量编辑命令注册']
      ]);
      requireAll('src/api/inventory.ts', [
        ['batchUpdateProducts', '前端商品批量编辑 API'],
        ['batchUpdateCustomers', '前端客户批量编辑 API'],
        ['batchUpdateSuppliers', '前端供应商批量编辑 API']
      ]);
      requireAll('src/pages/ProductsPage.tsx', [
        ['rowSelection', '商品表格多选'],
        ['batchUpdateProducts', '商品页批量编辑调用'],
        ['批量编辑${terms.product}', '商品页批量编辑抽屉']
      ]);
      requireAll('src/pages/CustomersPage.tsx', [
        ['rowSelection', '客户表格多选'],
        ['batchUpdateCustomers', '客户页批量编辑调用'],
        ['批量编辑${terms.customer}', '客户页批量编辑抽屉']
      ]);
      requireAll('src/pages/SuppliersPage.tsx', [
        ['rowSelection', '供应商表格多选'],
        ['batchUpdateSuppliers', '供应商页批量编辑调用'],
        ['批量编辑供应商', '供应商页批量编辑抽屉']
      ]);
    }
  },
  {
    name: 'V1.4 客户商品规则批量导入入口完整',
    run() {
      requireAll('src-tauri/src/models.rs', [
        ['CustomerProductRuleImportPreviewDto', '客户商品规则导入预览模型'],
        ['CustomerProductRuleImportResultDto', '客户商品规则导入结果模型']
      ]);
      requireAll('src-tauri/src/commands.rs', [
        ['pub fn preview_customer_product_rule_import', '客户商品规则导入预览命令'],
        ['pub fn import_customer_product_rules', '客户商品规则确认导入命令'],
        ['parse_customer_product_rule_import_rows', '客户商品规则 Excel 解析'],
        ['customer_product_rule_import_previews_then_imports_valid_rows', '客户商品规则导入测试']
      ]);
      requireAll('src-tauri/src/lib.rs', [
        ['preview_customer_product_rule_import', '客户商品规则导入预览命令注册'],
        ['import_customer_product_rules', '客户商品规则确认导入命令注册']
      ]);
      requireAll('src/api/inventory.ts', [
        ['previewRuleImport', '前端规则导入预览 API'],
        ['importRules', '前端规则确认导入 API']
      ]);
      requireAll('src/pages/RulesPage.tsx', [
        ['批量导入${terms.rule}', '规则页批量导入抽屉使用术语配置'],
        ['previewRuleImport', '规则页导入预览调用'],
        ['importRules', '规则页确认导入调用'],
        ['异常行不会写入', '规则页导入确认说明']
      ]);
    }
  },
  {
    name: 'V1.5 商品经营排行入口完整',
    run() {
      requireAll('src-tauri/src/models.rs', [
        ['ProductRankingRequest', '商品经营排行请求模型'],
        ['ProductRankingRowDto', '商品经营排行返回模型'],
        ['rank_by', '商品经营排行指标字段']
      ]);
      requireAll('src-tauri/src/reports.rs', [
        ['pub fn product_ranking', '商品经营排行查询函数'],
        ['export_product_ranking', '商品经营排行导出函数'],
        ['"product_ranking" => export_product_ranking', '商品经营排行导出类型分发'],
        ['product_ranking_summarizes_sales_profit_and_gift_cost', '商品经营排行销量利润赠品成本测试']
      ]);
      requireAll('src-tauri/src/commands.rs', [
        ['pub fn get_product_ranking', '商品经营排行命令']
      ]);
      requireAll('src-tauri/src/lib.rs', [
        ['get_product_ranking', '商品经营排行命令注册']
      ]);
      requireAll('src/api/inventory.ts', [
        ['productRanking', '前端商品经营排行 API'],
        ['get_product_ranking', '前端调用商品经营排行命令']
      ]);
      requireAll('src/shared/types.ts', [
        ['ProductRankingRankBy', '前端商品经营排行指标类型'],
        ['ProductRankingRowDto', '前端商品经营排行结果类型']
      ]);
      requireAll('src/App.tsx', [
        ['ProductRankingPage', '商品经营排行页面路由'],
        ['/product-ranking', '商品经营排行菜单入口']
      ]);
      requireAll('src/pages/ProductRankingPage.tsx', [
        ['EChart', '商品经营排行图表'],
        ['product_ranking', '商品经营排行导出类型'],
        ['giftCostAmount', '商品经营排行赠品成本字段']
      ]);
      requireAll('src/pages/ExportPage.tsx', [
        ['product_ranking', '导出页商品经营排行类型'],
        ['rankBy', '导出页商品排行指标字段']
      ]);
    }
  },
  {
    name: 'V1.5 客户经营分析入口完整',
    run() {
      requireAll('src-tauri/src/models.rs', [
        ['CustomerAnalysisRequest', '客户经营分析请求模型'],
        ['CustomerAnalysisRowDto', '客户经营分析返回模型'],
        ['favorite_products', '客户偏好商品字段'],
        ['average_repurchase_days', '客户复购间隔字段']
      ]);
      requireAll('src-tauri/src/reports.rs', [
        ['pub fn customer_analysis', '客户经营分析查询函数'],
        ['export_customer_analysis', '客户经营分析导出函数'],
        ['"customer_analysis" => export_customer_analysis', '客户经营分析导出类型分发'],
        ['customer_analysis_ranks_sales_profit_balance_and_preferences', '客户经营分析销售利润欠款偏好测试']
      ]);
      requireAll('src-tauri/src/commands.rs', [
        ['pub fn get_customer_analysis', '客户经营分析命令']
      ]);
      requireAll('src-tauri/src/lib.rs', [
        ['get_customer_analysis', '客户经营分析命令注册']
      ]);
      requireAll('src/api/inventory.ts', [
        ['customerAnalysis', '前端客户经营分析 API'],
        ['get_customer_analysis', '前端调用客户经营分析命令']
      ]);
      requireAll('src/shared/types.ts', [
        ['CustomerAnalysisRankBy', '前端客户分析排行指标类型'],
        ['CustomerAnalysisRowDto', '前端客户分析结果类型']
      ]);
      requireAll('src/App.tsx', [
        ['CustomerAnalysisPage', '客户经营分析页面路由'],
        ['/customer-analysis', '客户经营分析菜单入口']
      ]);
      requireAll('src/pages/CustomerAnalysisPage.tsx', [
        ['EChart', '客户经营分析图表'],
        ['customer_analysis', '客户经营分析导出类型'],
        ['favoriteProducts', '客户偏好商品展示字段'],
        ['averageRepurchaseDays', '客户复购间隔展示字段']
      ]);
      requireAll('src/pages/ExportPage.tsx', [
        ['customer_analysis', '导出页客户经营分析类型'],
        ['balance_amount', '导出页客户欠款排行指标']
      ]);
    }
  },
  {
    name: 'V1.5 利润同比环比入口完整',
    run() {
      requireAll('src-tauri/src/models.rs', [
        ['comparison_period', '利润趋势对比周期字段'],
        ['sales_change_rate', '销售额增长率字段'],
        ['profit_change_rate', '利润增长率字段']
      ]);
      requireAll('src-tauri/src/reports.rs', [
        ['profit_comparison_range', '利润同比环比周期计算'],
        ['percent_change', '利润同比环比增长率计算'],
        ['comparison_period', '利润趋势对比字段赋值'],
        ['profit_analytics_groups_daily_monthly_and_yearly', '利润日月年趋势测试'],
        ['profit_change_rate', '利润同比环比测试断言']
      ]);
      requireAll('src/shared/types.ts', [
        ['comparisonPeriod', '前端利润对比周期类型'],
        ['profitChangeRate', '前端利润增长率类型']
      ]);
      requireAll('src/pages/ProfitPage.tsx', [
        ['同比/环比分析', '利润页同比环比表格'],
        ['comparisonTitle', '利润页日月年对比标题'],
        ['profitChangeRate', '利润页利润增长率展示'],
        ['salesChangeRate', '利润页销售额增长率展示']
      ]);
    }
  },
  {
    name: 'V1.5 单据模板配置和 PDF 导出入口完整',
    run() {
      requireAll('src-tauri/src/db.rs', [
        ['template_store_name', '单据模板店名默认设置'],
        ['template_show_barcode', '单据模板条码显示设置'],
        ['template_orientation', '单据模板纸张方向设置']
      ]);
      requireAll('src-tauri/src/models.rs', [
        ['template_store_name', '保存模板店名字段'],
        ['template_show_barcode', '保存模板条码显示字段'],
        ['template_margin', '保存模板页边距字段']
      ]);
      requireAll('src-tauri/src/reports.rs', [
        ['OrderTemplateSettings', '单据模板配置结构'],
        ['order_template_settings', '从设置读取单据模板配置'],
        ['write_order_workbook_with_template', '按模板生成 xlsx 单据'],
        ['write_order_pdf', '生成 PDF 单据'],
        ['export_order_pdf_document', '导出订单 PDF 单据'],
        ['order_template_settings_apply_to_workbook', '模板配置影响 xlsx 测试'],
        ['order_pdf_document_writes_valid_pdf', 'PDF 单据生成测试']
      ]);
      requireAll('src-tauri/src/commands.rs', [
        ['pub fn export_order_pdf_document', '订单 PDF 导出命令'],
        ['pub fn export_document_pdf', '单据档案 PDF 导出命令'],
        ['template_store_name', '模板店名保存逻辑']
      ]);
      requireAll('src-tauri/src/lib.rs', [
        ['export_order_pdf_document', '订单 PDF 导出命令注册'],
        ['export_document_pdf', '单据档案 PDF 导出命令注册']
      ]);
      requireAll('src/api/inventory.ts', [
        ['exportOrderPdf', '前端订单 PDF 导出 API'],
        ['exportDocumentPdf', '前端单据档案 PDF 导出 API']
      ]);
      requireAll('src/pages/SettingsPage.tsx', [
        ['单据模板设置', '设置页模板配置入口'],
        ['template-preview', '设置页模板预览区域'],
        ['恢复默认模板', '设置页恢复默认模板入口']
      ]);
      requireAll('src/pages/DocumentsPage.tsx', [
        ['导出 PDF', '单据档案 PDF 导出按钮'],
        ['exportDocumentPdf', '单据档案调用 PDF 导出 API']
      ]);
      requireAll('src/pages/CustomerStatementPage.tsx', [
        ['导出 PDF', '客户对账单 PDF 导出按钮'],
        ['exportCustomerStatementPdf', '客户对账单调用 PDF 导出 API']
      ]);
      requireAll('src-tauri/src/reports.rs', [
        ['customer_statement_pdf_document_writes_valid_pdf', '客户对账单 PDF 生成测试'],
        ['export_customer_statement_pdf_document', '客户对账单 PDF 导出函数']
      ]);
    }
  },
  {
    name: 'V1.6 自检诊断和流程验收入口完整',
    run() {
      requireAll('src-tauri/src/models.rs', [
        ['DataSelfCheckDto', '数据自检返回模型'],
        ['DataSelfCheckIssueDto', '数据自检异常模型'],
        ['DiagnosticSummaryDto', '诊断中心摘要模型'],
        ['DiagnosticPackageDto', '诊断包返回模型']
      ]);
      requireAll('src-tauri/src/commands.rs', [
        ['pub fn run_data_self_check', '运行数据自检命令'],
        ['pub fn export_data_self_check', '导出数据自检命令'],
        ['pub fn get_diagnostic_summary', '诊断中心摘要命令'],
        ['pub fn export_diagnostic_package', '导出诊断包命令'],
        ['run_data_self_check_record', '数据自检核心函数'],
        ['data_self_check_detects_core_data_inconsistencies', '数据自检异常测试']
      ]);
      requireAll('src-tauri/src/lib.rs', [
        ['run_data_self_check', '运行数据自检命令注册'],
        ['export_diagnostic_package', '导出诊断包命令注册']
      ]);
      requireAll('src/api/inventory.ts', [
        ['runDataSelfCheck', '前端运行数据自检 API'],
        ['exportDataSelfCheck', '前端导出数据自检 API'],
        ['diagnosticSummary', '前端诊断摘要 API'],
        ['exportDiagnosticPackage', '前端导出诊断包 API']
      ]);
      requireAll('src/pages/SettingsPage.tsx', [
        ['诊断中心', '设置页诊断中心入口'],
        ['运行数据自检', '设置页运行自检按钮'],
        ['导出诊断包', '设置页导出诊断包按钮'],
        ['最近日志', '设置页最近日志展示']
      ]);
      requireAll('scripts/e2e-flow-check.mjs', [
        ['新增商品', '前端新增商品流程验收'],
        ['快速出库', '前端快速出库流程验收'],
        ['返利额度生成和抵扣', '前端返利额度流程验收'],
        ['单据导出', '前端单据导出流程验收']
      ]);
      requireAll('tests/e2e/core-flows.spec.ts', [
        ['初始化向导和设置页会完成通用化配置', '浏览器初始化和通用化设置流程'],
        ['新增商品流程会在浏览器中保存并刷新', '浏览器新增商品流程'],
        ['快速出库流程会选择商品、使用返利额度并保存订单', '浏览器快速出库和返利额度抵扣流程'],
        ['单据档案流程会预览、导出 PDF、重新导出并作废订单', '浏览器单据档案流程'],
        ['客户对账单和数据导出流程会导出 Excel 与 PDF', '浏览器客户对账和 PDF 导出流程']
      ]);
    }
  },
  {
    name: 'V1.7-V1.9 通用化配置入口完整',
    run() {
      requireAll('EasyInventory_通用化改造_PRD.md', [
        ['当前项目检查快照', '通用化 PRD 基于当前项目检查'],
        ['V1.7 通用化基线需求', 'V1.7 通用化需求'],
        ['V1.8 行业模板需求', 'V1.8 行业模板需求'],
        ['V1.9 导入与迁移增强', 'V1.9 导入迁移需求']
      ]);
      requireAll('src-tauri/src/generalization.rs', [
        ['pub fn setup_status', '初始化状态后端函数'],
        ['pub fn save_merchant_profile', '商户信息保存后端函数'],
        ['merchant_remark', '商户备注配置读写'],
        ['pub fn save_term_settings', '术语保存后端函数'],
        ['pub fn save_feature_flags', '功能开关保存后端函数'],
        ['pub fn industry_templates', '行业模板后端函数'],
        ['"delivery"', '配送出库单模板 ID'],
        ['"配送出库单"', '配送出库单模板名称'],
        ['pub fn preview_generic_import', '通用导入预览后端函数'],
        ['pub fn preview_generic_import_headers', '通用导入表头预览后端函数'],
        ['pub fn confirm_generic_import', '通用导入确认后端函数'],
        ['pub fn export_generic_import_report', '通用导入报告导出后端函数'],
        ['pub fn export_generic_import_template', '通用导入模板导出后端函数'],
        ['pub fn save_import_mapping', '字段映射保存后端函数'],
        ['generic_import_header_preview_suggests_visual_mapping', '表头预览推荐字段映射单测'],
        ['AppendSuffix', '通用导入重复数据追加后缀策略'],
        ['generic_template_spec', '通用商品客户期初库存模板定义'],
        ['unique_text_value', '追加后缀生成唯一名称'],
        ['generic_import_append_suffix_creates_unique_rows_and_report', '追加后缀和导入报告单测'],
        ['generic_import_template_exports_all_supported_types', '通用导入模板导出单测']
      ]);
      requireAll('src-tauri/src/lib.rs', [
        ['get_setup_status', '初始化状态命令注册'],
        ['complete_setup', '完成初始化命令注册'],
        ['save_merchant_profile', '商户信息保存命令注册'],
        ['save_term_settings', '术语保存命令注册'],
        ['save_feature_flags', '功能开关保存命令注册'],
        ['apply_industry_template', '行业模板应用命令注册'],
        ['preview_generic_import', '通用导入预览命令注册'],
        ['preview_generic_import_headers', '通用导入表头预览命令注册'],
        ['confirm_generic_import', '通用导入确认命令注册'],
        ['export_generic_import_report', '通用导入报告导出命令注册'],
        ['download_import_template', '通用导入模板下载命令注册'],
        ['save_import_mapping', '字段映射保存命令注册']
      ]);
      requireAll('src-tauri/src/commands.rs', [
        ['pub fn preview_generic_import_headers', '通用导入表头预览命令实现'],
        ['pub fn export_generic_import_report', '通用导入报告导出命令实现'],
        ['pub fn download_import_template', '通用导入模板下载命令实现'],
        ['safe_file_name', '导入报告文件名安全处理']
      ]);
      requireAll('src-tauri/src/db.rs', [
        ['setup_completed', '初始化完成设置默认值'],
        ['merchant_name', '商户名设置默认值'],
        ['merchant_remark', '商户备注设置默认值'],
        ['industry_template', '行业模板设置默认值'],
        ['term_customer', '客户术语设置默认值'],
        ['feature_monthly_credit', '功能开关设置默认值'],
        ['template_store_name", "我的商行"', '单据默认店名通用化']
      ]);
      requireAll('src/App.tsx', [
        ['api.setupStatus()', '前端启动读取初始化状态'],
        ['api.merchantProfile()', '前端启动读取商户信息'],
        ['api.termSettings()', '前端启动读取术语配置'],
        ['api.featureFlags()', '前端启动读取功能开关'],
        ['SetupPage', '初始化向导路由'],
        ['features.monthlyCredit', '功能开关控制月费菜单'],
        ['terms.customer', '菜单使用术语配置'],
        ['merchant.name', '界面显示商户名称']
      ]);
      requireAll('src/pages/SetupPage.tsx', [
        ['首次使用初始化', '初始化向导页面'],
        ['api.completeSetup', '初始化完成调用'],
        ['api.industryTemplates', '初始化向导加载行业模板'],
        ['api.merchantProfile', '初始化向导重新打开时读取当前商户信息'],
        ['api.termSettings', '初始化向导重新打开时读取当前术语配置'],
        ['api.featureFlags', '初始化向导重新打开时读取当前功能开关'],
        ['api.settings', '初始化向导重新打开时读取当前默认单据设置'],
        ['api.documentTemplates', '初始化向导读取单据模板列表'],
        ['defaultPrintTemplate', '初始化向导配置默认单据模板'],
        ['配送出库单', '初始化向导包含配送出库单兜底选项'],
        ['数据导入', '初始化向导包含数据导入步骤'],
        ['importPlan', '初始化向导记录导入计划'],
        ['downloadTemplate', '初始化向导支持下载通用导入模板'],
        ['历史兼容迁移', '初始化向导隔离历史兼容迁移提示'],
        ['logoPath', '初始化向导支持商户 Logo 路径'],
        ['remark', '初始化向导支持商户备注']
      ]);
      requireAll('src/pages/SettingsPage.tsx', [
        ['商户信息', '设置页商户信息区'],
        ['Logo 路径', '设置页商户 Logo 路径配置'],
        ['name="remark"', '设置页商户备注配置'],
        ['行业模板与功能开关', '设置页行业模板和功能开关区'],
        ['业务术语', '设置页术语配置区'],
        ['重新打开初始化向导', '设置页可重新打开初始化向导'],
        ['通用数据导入', '设置页通用导入区'],
        ['高级：历史兼容迁移', '设置页历史迁移隔离区'],
        ['清空并重建', '历史兼容迁移明确提示会重建业务表'],
        ['自动备份当前数据库', '历史兼容迁移明确提示会自动备份'],
        ['previewGenericImportHeaders', '设置页读取 Excel 表头调用'],
        ['visualFieldMapping', '设置页可视化字段映射状态'],
        ['读取表头', '设置页读取表头按钮'],
        ['套用已保存映射', '设置页套用已保存映射按钮'],
        ['previewGenericImport', '通用导入预览调用'],
        ['confirmGenericImport', '通用导入确认调用'],
        ['exportGenericImportReport', '通用导入报告导出调用'],
        ['downloadImportTemplate', '通用导入模板下载调用'],
        ['append_suffix', '设置页支持追加后缀策略'],
        ['下载模板', '设置页通用导入模板下载按钮'],
        ['导出报告', '设置页导入报告按钮'],
        ['saveImportMapping', '字段映射保存调用'],
        ['兼容迁移', '历史兼容迁移按钮']
      ]);
      requireAll('src/api/inventory.ts', [
        ['previewGenericImportHeaders', '前端导入表头预览 API'],
        ['preview_generic_import_headers', '前端调用导入表头预览命令'],
        ['exportGenericImportReport', '前端导入报告 API'],
        ['export_generic_import_report', '前端调用导入报告命令'],
        ['downloadImportTemplate', '前端导入模板下载 API'],
        ['download_import_template', '前端调用导入模板下载命令']
      ]);
      requireAll('src/shared/types.ts', [
        ['append_suffix', '前端重复策略类型包含追加后缀'],
        ['GenericImportHeadersDto', '前端导入表头预览类型'],
        ['GenericImportReportRequest', '前端导入报告请求类型'],
        ['logoPath?: string', '前端商户 Logo 路径类型'],
        ['remark?: string', '前端商户备注类型']
      ]);
      requireAll('src/pages/HomePage.tsx', [
        ['features.customerRules', '首页按功能开关显示规则入口'],
        ['features.supplierLedger', '首页按功能开关显示供应商台账入口'],
        ['features.receivables', '首页按功能开关显示欠款收款入口'],
        ['terms.product', '首页商品文案读取术语配置'],
        ['terms.customer', '首页客户文案读取术语配置'],
        ['terms.category', '首页详情类别列读取术语配置'],
        ['terms.credit', '首页返利额度文案读取术语配置']
      ]);
      requireAll('src/pages/RulesPage.tsx', [
        ['features.customerRules', '规则页直达时按功能开关显示关闭态'],
        ['features.monthlyCredit', '规则页按返利开关裁剪返利字段'],
        ['功能已关闭', '规则页关闭态提示'],
        ['terms.rule', '规则页标题读取术语配置'],
        ['terms.customer', '规则页客户文案读取术语配置'],
        ['terms.product', '规则页商品文案读取术语配置'],
        ['terms.credit', '规则页返利文案读取术语配置']
      ]);
      requireAll('src/pages/MonthlyCreditsPage.tsx', [
        ['features.monthlyCredit', '返利账本直达时按功能开关显示关闭态'],
        ['功能已关闭', '返利账本关闭态提示'],
        ['terms.credit', '返利账本文案读取术语配置'],
        ['terms.customer', '返利账本客户文案读取术语配置'],
        ['terms.category', '返利账本类别文案读取术语配置']
      ]);
      requireAll('src/pages/OutboundPage.tsx', [
        ['features.monthlyCredit', '快速出库按功能开关裁剪返利额度'],
        ['terms.customer', '快速出库客户文案读取术语配置'],
        ['terms.region', '快速出库地区文案读取术语配置'],
        ['terms.product', '快速出库商品文案读取术语配置'],
        ['terms.category', '快速出库类别文案读取术语配置'],
        ['terms.credit', '快速出库返利额度文案读取术语配置'],
        ['monthlyCreditUses: features.monthlyCredit', '关闭返利额度后保存订单不提交抵扣明细']
      ]);
      requireAll('src/components/ProductPickerModal.tsx', [
        ['terms.product', '商品选择弹窗商品文案读取术语配置'],
        ['terms.customer', '商品选择弹窗客户文案读取术语配置'],
        ['terms.category', '商品选择弹窗类别文案读取术语配置']
      ]);
      requireAll('src/components/PrintPreview.tsx', [
        ['useAppStore', '打印预览读取运行时商户信息'],
        ['merchantName', '打印预览商户名变量'],
        ['terms.customer', '打印预览客户文案读取术语配置'],
        ['terms.product', '打印预览商品文案读取术语配置']
      ]);
      requireAll('src/pages/CustomerStatementPage.tsx', [
        ['terms.customer', '客户对账单文案读取术语配置']
      ]);
      requireAll('src/pages/DocumentsPage.tsx', [
        ['terms.customer', '单据档案客户文案读取术语配置'],
        ['terms.credit', '单据作废返利抵扣文案读取术语配置']
      ]);
      requireAll('src/pages/ReceivablesPage.tsx', [
        ['features.receivables', '欠款收款直达页按功能开关显示关闭态'],
        ['欠款收款功能已关闭', '欠款收款关闭态提示'],
        ['terms.customer', '欠款收款客户文案读取术语配置'],
        ['terms.region', '欠款收款地区文案读取术语配置']
      ]);
      requireAll('src/pages/InventoryReportPage.tsx', [
        ['terms.product', '进销存报表商品文案读取术语配置'],
        ['terms.category', '进销存报表类别文案读取术语配置']
      ]);
      requireAll('src/pages/ProductRankingPage.tsx', [
        ['features.productRanking', '商品经营排行直达页按功能开关显示关闭态'],
        ['经营排行功能已关闭', '商品经营排行关闭态提示'],
        ['terms.product', '商品经营排行商品文案读取术语配置'],
        ['terms.category', '商品经营排行类别文案读取术语配置']
      ]);
      requireAll('src/pages/CustomerAnalysisPage.tsx', [
        ['features.customerAnalysis', '客户经营分析直达页按功能开关显示关闭态'],
        ['经营分析功能已关闭', '客户经营分析关闭态提示'],
        ['terms.customer', '客户经营分析客户文案读取术语配置'],
        ['terms.region', '客户经营分析地区文案读取术语配置'],
        ['terms.product', '客户经营分析商品文案读取术语配置'],
        ['terms.category', '客户经营分析类别文案读取术语配置']
      ]);
      requireAll('src/pages/InventoryControlPage.tsx', [
        ['features.inventoryControl', '库存盘点直达页按功能开关显示关闭态'],
        ['库存盘点功能已关闭', '库存盘点关闭态提示'],
        ['terms.product', '库存盘点商品文案读取术语配置'],
        ['terms.category', '库存盘点类别文案读取术语配置']
      ]);
      requireAll('src/pages/SupplierLedgerPage.tsx', [
        ['features.supplierLedger', '供应商采购台账直达页按功能开关显示关闭态'],
        ['供应商采购台账功能已关闭', '供应商采购台账关闭态提示'],
        ['terms.product', '供应商采购台账商品文案读取术语配置'],
        ['terms.category', '供应商采购台账类别文案读取术语配置']
      ]);
    }
  },
  {
    name: '项目级功能单测覆盖完整',
    run() {
      requireAll('src-tauri/src/db.rs', [
        ['recalc_stock_balance_uses_weighted_average_and_outbound_quantity', '库存余额和加权均价重算测试']
      ]);
      requireAll('src-tauri/src/orders.rs', [
        ['list_orders_filters_by_date_customer_and_status', '订单列表筛选测试'],
        ['monthly_credit_lifecycle_filters_and_status_changes', '月费生成、查询、关闭、作废测试']
      ]);
      requireAll('src-tauri/src/commands.rs', [
        ['customer_product_rule_lifecycle_disables_old_active_rule_and_deletes_draft_rule', '客户计价规则生命周期测试'],
        ['save_settings_updates_known_keys_only', '系统设置保存测试']
      ]);
      requireAll('src-tauri/src/reports.rs', [
        ['list_documents_filters_by_customer_and_status', '单据档案筛选测试'],
        ['export_data_rejects_unknown_export_type', '导出类型校验测试']
      ]);
    }
  },
  {
    name: '项目级压力和并发测试覆盖完整',
    run() {
      requireAll('src-tauri/src/commands.rs', [
        ['concurrent_inbounds_keep_stock_balance_consistent', '并发入库库存一致性测试'],
        ['concurrent_payments_keep_customer_balance_consistent', '并发收款余额一致性测试']
      ]);
      requireAll('src-tauri/src/orders.rs', [
        ['high_volume_order_listing_stays_under_two_seconds', '万级订单列表查询测试']
      ]);
    }
  }
];

const failures = [];

for (const check of checks) {
  try {
    check.run();
    console.log(`OK ${check.name}`);
  } catch (error) {
    failures.push(`${check.name}: ${error instanceof Error ? error.message : String(error)}`);
  }
}

if (failures.length > 0) {
  console.error('\n健壮性检查失败：');
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}

console.log(`\n健壮性检查通过：${checks.length} 项`);
