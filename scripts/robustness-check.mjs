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
        ['"version": "1.2.0"', '前端包版本为 V1.2'],
        ['"test": "npm run typecheck && npm run format:rust && npm run lint:rust && npm run test:rust && npm run audit:robustness"', 'npm test 聚合测试和健壮性检查'],
        ['"check": "npm run build && npm run format:rust && npm run lint:rust && npm run test:rust && npm run audit:robustness"', 'npm run check 聚合构建和质量门禁'],
        ['"lint:rust": "cd src-tauri && cargo clippy --all-targets -- -D warnings"', 'Rust clippy 严格检查']
      ]);
      requireAll('src-tauri/Cargo.toml', [['version = "1.2.0"', 'Rust 包版本为 V1.2']]);
      requireAll('src-tauri/tauri.conf.json', [['"version": "1.2.0"', 'Tauri 打包版本为 V1.2']]);
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
        ['ensure_guest_customer', '确保散客存在'],
        ['ensure_guest_customer_creates_active_fixed_customer', '散客创建测试'],
        ['ensure_guest_customer_reactivates_existing_guest_customer', '散客恢复测试']
      ]);
      requireAll('src-tauri/src/app.rs', [['db::ensure_guest_customer', '启动时补齐散客']]);
      requireAll('src-tauri/src/excel.rs', [['db::ensure_guest_customer', 'Excel 导入后补齐散客']]);
      requireAll('src-tauri/src/commands.rs', [
        ['散客是系统默认客户，名称不能修改', '禁止改名'],
        ['散客是系统默认客户，不能删除', '禁止删除']
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
