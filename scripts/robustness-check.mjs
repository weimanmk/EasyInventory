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

function requireNotText(file, text, reason) {
  const content = read(file);
  if (content.includes(text)) {
    throw new Error(`${file} 不应包含：${reason}`);
  }
}

function requireAtMost(file, pattern, maxCount, reason) {
  const content = read(file);
  const matches = content.match(pattern) ?? [];
  if (matches.length > maxCount) {
    throw new Error(`${file} ${reason}，当前 ${matches.length}，上限 ${maxCount}`);
  }
}

const checks = [
  {
    name: '项目自动化测试入口完整',
    run() {
      requireAll('package.json', [
        ['"version": "1.3.2"', '前端包版本为 V1.3.2'],
        ['"e2e:flows": "node scripts/e2e-flow-check.mjs"', '前端核心流程验收脚本'],
        ['"e2e:browser": "npm run build && node scripts/browser-e2e-check.mjs"', '浏览器级前端 E2E 脚本'],
        ['"package:smoke": "node scripts/package-smoke-check.mjs"', '安装包产物 smoke 检查脚本'],
        ['"release:manifest": "node scripts/release-manifest.mjs"', 'Release 清单生成脚本'],
        ['"docs:screenshots": "npm run build && node scripts/capture-doc-screenshots.mjs"', 'README 截图生成脚本'],
        ['"release:verify": "npm run check && npm run tauri:build && npm run package:smoke && npm run release:manifest"', '发布验收脚本'],
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
      requireAll('scripts/release-manifest.mjs', [
        ['createHash', '生成 SHA256 校验值'],
        ['EasyInventory_${pkg.version}_release_manifest.md', '生成版本化 Release 清单'],
        ['EasyInventory_${pkg.version}_github_release_notes.md', '生成可复制到 GitHub Release 的说明'],
        ['## EasyInventory v${pkg.version}', 'GitHub Release 说明包含版本标题'],
        ['### 下载', 'GitHub Release 说明包含下载区'],
        ['### SHA256', 'GitHub Release 说明包含校验值'],
        ['### 升级说明', 'GitHub Release 说明包含升级说明'],
        ['### 已知问题', 'GitHub Release 说明包含已知问题']
      ]);
      requireAll('scripts/capture-doc-screenshots.mjs', [
        ['outbound_page.png', '快速出库截图'],
        ['products_page.png', '商品库存截图'],
        ['customers_page.png', '客户管理截图'],
        ['profit_page.png', '利润统计截图'],
        ['settings_page.png', '系统设置截图']
      ]);
      requireAll('docs/release-checklist.md', [
        ['GitHub Release 说明模板', 'Release 页面说明模板'],
        ['SHA256', 'Release 校验值说明'],
        ['升级说明', '升级说明'],
        ['已知问题', '已知问题']
      ]);
      requireAll('src-tauri/Cargo.toml', [['version = "1.3.2"', 'Rust 包版本为 V1.3.2']]);
      requireAll('src-tauri/tauri.conf.json', [['"version": "1.3.2"', 'Tauri 打包版本为 V1.3.2']]);
    }
  },
  {
    name: '产品化发布与安全配置完整',
    run() {
      requireAll('vite.config.ts', [
        ["minify: 'esbuild'", '生产构建开启 minify'],
        ['sourcemap: false', '生产构建关闭 sourcemap'],
        ['manualChunks', '生产构建配置 vendor 分包'],
        ["vendor-echarts", 'ECharts 独立 vendor 分包']
      ]);
      requireAll('src-tauri/tauri.conf.json', [
        ['"csp": "default-src', 'Tauri 配置明确 CSP'],
        ['script-src', 'CSP 限制脚本来源'],
        ['connect-src ipc:', 'CSP 允许 Tauri IPC 连接']
      ]);
      requireAll('.github/workflows/ci.yml', [
        ['npm ci', 'CI 安装依赖'],
        ['npm run typecheck', 'CI 类型检查'],
        ['npm run build', 'CI 前端构建'],
        ['npm run lint:rust', 'CI Rust clippy'],
        ['npm run test:rust', 'CI Rust 测试'],
        ['npx playwright install chrome', 'CI 安装浏览器 E2E 所需 Chrome'],
        ['npm run e2e:browser', 'CI 运行浏览器 E2E 主流程'],
        ['npm run audit:robustness', 'CI 健壮性审计']
      ]);
      requireAll('.github/ISSUE_TEMPLATE/bug_report.md', [['Diagnostics', 'Bug 模板包含诊断信息说明']]);
      requireAll('.github/pull_request_template.md', [['Verification', 'PR 模板包含验证清单']]);
      requireAll('README.md', [
        ['## 快速下载安装', 'README 提供下载说明'],
        ['https://github.com/weimanmk/EasyInventory/releases/tag/v1.3.2', 'README 提供当前版本 Release 下载入口'],
        ['## 10 分钟快速试用', 'README 提供快速试用路径'],
        ['## 数据安全边界', 'README 说明数据安全边界'],
        ['docs/images/outbound_page.png', 'README 展示快速出库截图'],
        ['docs/images/products_page.png', 'README 展示商品库存截图'],
        ['docs/images/customers_page.png', 'README 展示客户管理截图'],
        ['docs/images/profit_page.png', 'README 展示利润统计截图'],
        ['docs/images/settings_page.png', 'README 展示系统设置截图'],
        ['## 常见问题', 'README 提供 FAQ'],
        ['## 路线图', 'README 提供路线图'],
        ['## 贡献指南', 'README 提供贡献入口'],
        ['CONTRIBUTING.md', 'README 链接贡献指南'],
        ['npm run release:manifest', 'README 说明 Release 清单命令'],
        ['docs/release-checklist.md', 'README 链接 Release checklist'],
        ['EasyInventory_1.3.2_github_release_notes.md', 'README 说明 GitHub Release 正文文件'],
        ['诊断包会对常见敏感字段做默认脱敏', 'README 说明诊断包默认脱敏']
      ]);
      requireAll('SECURITY.md', [
        ['反馈问题前请脱敏', '安全说明提醒反馈前脱敏'],
        ['EasyInventory v1.3.2', '安全说明覆盖当前版本']
      ]);
      requireAll('CONTRIBUTING.md', [
        ['npm run check', '贡献指南说明完整检查命令'],
        ['commands / services / repositories / domain', '贡献指南说明后端分层约定'],
        ['src/api/catalog.ts', '贡献指南说明前端 API 分域'],
        ['诊断包请先脱敏', '贡献指南提醒诊断信息脱敏'],
        ['AGPLv3', '贡献指南说明许可证']
      ]);
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
    name: '前端入口、路由和菜单完成基础拆分',
    run() {
      requireAll('src/App.tsx', [
        ['AppShell', 'App 入口挂载应用壳'],
        ['<HashRouter>', 'App 入口保留路由 Provider'],
        ['<AntApp>', 'App 入口保留 Ant Design Provider']
      ]);
      requireNotText('src/App.tsx', '<Routes>', 'App 入口不应直接承载路由表');
      requireNotText('src/App.tsx', 'api.status()', 'App 入口不应直接承载启动加载');
      requireAll('src/app/AppShell.tsx', [
        ['useAppBootstrap()', '应用壳调用启动 hook'],
        ['buildMenuItems', '应用壳从统一配置生成菜单'],
        ['<AppRoutes />', '应用壳挂载独立路由组件'],
        ['merchant.name', '应用壳显示商户名称']
      ]);
      requireAll('src/app/routes.tsx', [
        ['export const appRoutes', '统一路由配置'],
        ['export function AppRoutes', '独立路由组件'],
        ['export function buildMenuItems', '菜单由路由配置生成'],
        ['features.monthlyCredit', '功能开关控制菜单项'],
        ['terms.customer', '菜单文案读取术语配置']
      ]);
      requireAll('src/app/useAppBootstrap.ts', [
        ['api.status()', '启动 hook 读取应用状态'],
        ['api.setupStatus()', '启动 hook 读取初始化状态'],
        ['api.merchantProfile()', '启动 hook 读取商户信息'],
        ['api.termSettings()', '启动 hook 读取术语配置'],
        ['api.featureFlags()', '启动 hook 读取功能开关']
      ]);
    }
  },
  {
    name: '前端页面路由支持懒加载代码拆分',
    run() {
      requireAll('src/app/routes.tsx', [
        ['lazy(', '页面组件使用 React lazy 懒加载'],
        ['<Suspense', '路由包裹 Suspense'],
        ['fallback=', '路由加载态'],
        ['import(\'../pages/ProfitPage\')', '利润页独立动态导入'],
        ['import(\'../pages/SettingsPage\')', '设置页独立动态导入']
      ]);
      requireNotText('src/app/routes.tsx', "import ProfitPage from '../pages/ProfitPage'", '利润页不应进入首屏静态 chunk');
      requireNotText('src/app/routes.tsx', "import SettingsPage from '../pages/SettingsPage'", '设置页不应进入首屏静态 chunk');
    }
  },
  {
    name: 'ECharts 按需加载避免整包进入图表 chunk',
    run() {
      requireAll('src/components/EChart.tsx', [
        ["from 'echarts/core'", '使用 ECharts core 按需构建'],
        ['BarChart', '注册柱状图'],
        ['LineChart', '注册折线图'],
        ['PieChart', '注册饼图'],
        ['echarts.use', '按需注册图表和组件']
      ]);
      requireNotText('src/components/EChart.tsx', "import * as echarts from 'echarts'", '不要整包导入 ECharts');
    }
  },
  {
    name: '前端 API 按业务域完成基础拆分',
    run() {
      requireAll('src/api/inventory.ts', [
        ['...catalogApi', '聚合基础资料 API'],
        ['...orderApi', '聚合订单和规则 API'],
        ['...reportApi', '聚合报表和单据 API'],
        ['...settingsApi', '聚合设置和通用导入 API'],
        ['...systemApi', '聚合系统状态、备份和诊断 API']
      ]);
      requireAll('src/api/catalog.ts', [
        ['export const catalogApi', '基础资料 API 模块'],
        ['list_products', '商品列表命令'],
        ['list_customers', '客户列表命令'],
        ['list_suppliers', '供应商列表命令']
      ]);
      requireAll('src/api/orders.ts', [
        ['export const orderApi', '订单 API 模块'],
        ['save_order', '订单保存命令'],
        ['list_customer_product_rules', '规则列表命令'],
        ['list_monthly_credits', '返利账本命令'],
        ['create_payment', '收款命令']
      ]);
      requireAll('src/api/reports.ts', [
        ['export const reportApi', '报表 API 模块'],
        ['get_profit_analytics', '利润统计命令'],
        ['get_product_ranking', '商品排行命令'],
        ['get_customer_analysis', '客户分析命令'],
        ['list_documents', '单据列表命令']
      ]);
      requireAll('src/api/settings.ts', [
        ['export const settingsApi', '设置 API 模块'],
        ['complete_setup', '初始化完成命令'],
        ['get_term_settings', '术语设置命令'],
        ['preview_generic_import', '通用导入预览命令'],
        ['list_document_templates', '单据模板命令']
      ]);
      requireAll('src/api/system.ts', [
        ['export const systemApi', '系统 API 模块'],
        ['get_app_status', '应用状态命令'],
        ['create_backup', '手动备份命令'],
        ['restore_backup', '恢复备份命令'],
        ['export_diagnostic_package', '诊断包命令']
      ]);
    }
  },
  {
    name: '设置页完成基础子模块拆分',
    run() {
      requireAll('src/pages/SettingsPage.tsx', [
        ['<LocalPathsCard', '设置页使用本地路径子组件'],
        ['<SetupGuideCard', '设置页使用初始化向导子组件'],
        ['<MerchantProfileCard', '设置页使用商户信息子组件'],
        ['<IndustryFeatureCard', '设置页使用行业和功能开关子组件'],
        ['<TermSettingsCard', '设置页使用术语配置子组件']
      ]);
      requireAll('src/pages/settings/LocalPathsCard.tsx', [
        ['export function LocalPathsCard', '本地路径子组件']
      ]);
      requireAll('src/pages/settings/SetupGuideCard.tsx', [
        ['export function SetupGuideCard', '初始化向导子组件'],
        ['重新打开初始化向导', '初始化向导入口文案']
      ]);
      requireAll('src/pages/settings/MerchantProfileCard.tsx', [
        ['export function MerchantProfileCard', '商户信息子组件'],
        ['Logo 路径', '商户 Logo 字段保留'],
        ['保存商户信息', '保存商户按钮保留']
      ]);
      requireAll('src/pages/settings/IndustryFeatureCard.tsx', [
        ['export function IndustryFeatureCard', '行业和功能开关子组件'],
        ['featureItems', '功能开关配置集中在子组件'],
        ['保存功能开关', '保存功能开关按钮保留']
      ]);
      requireAll('src/pages/settings/TermSettingsCard.tsx', [
        ['export function TermSettingsCard', '术语配置子组件'],
        ['保存术语配置', '保存术语按钮保留']
      ]);
    }
  },
  {
    name: '设置页单据、备份、诊断和审计区域完成拆分',
    run() {
      requireAll('src/pages/SettingsPage.tsx', [
        ['<DocumentTemplateSettingsCard', '设置页使用单据模板设置子组件'],
        ['<BackupRestoreCard', '设置页使用备份恢复子组件'],
        ['<DiagnosticsCard', '设置页使用诊断中心子组件'],
        ['<AuditLogCard', '设置页使用审计日志子组件']
      ]);
      requireAll('src/pages/settings/DocumentTemplateSettingsCard.tsx', [
        ['export function DocumentTemplateSettingsCard', '单据模板设置子组件'],
        ['系统与单据模板设置', '单据模板设置标题保留'],
        ['template-preview', '模板预览保留'],
        ['恢复默认模板', '恢复默认模板按钮保留']
      ]);
      requireAll('src/pages/settings/BackupRestoreCard.tsx', [
        ['export function BackupRestoreCard', '备份恢复子组件'],
        ['备份与恢复', '备份恢复标题保留'],
        ['立即备份', '立即备份按钮保留'],
        ['打开备份目录', '打开备份目录按钮保留']
      ]);
      requireAll('src/pages/settings/DiagnosticsCard.tsx', [
        ['export function DiagnosticsCard', '诊断中心子组件'],
        ['诊断中心', '诊断中心标题保留'],
        ['运行数据自检', '运行自检按钮保留'],
        ['最近日志', '最近日志列表保留']
      ]);
      requireAll('src/pages/settings/AuditLogCard.tsx', [
        ['export function AuditLogCard', '审计日志子组件'],
        ['审计日志', '审计日志标题保留'],
        ['targetLabel', '审计对象列保留']
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
      requireAll('src-tauri/src/commands/system_commands.rs', [
        ['pub fn write_client_log', '后端客户端日志命令']
      ]);
      requireAll('src-tauri/src/commands.rs', [
        ['logger::error("command"', '统一记录后端命令错误链']
      ]);
      requireAll('src/api/tauri.ts', [
        ['writeClientLog', '前端写日志工具'],
        ['sanitizeLogDetails', '前端日志 details 脱敏'],
        ['summarizeArgs', '前端日志参数摘要化'],
        ['命令返回失败', '记录 API 失败'],
        ['durationMs', '记录 API 耗时']
      ]);
      requireAll('src-tauri/src/logger.rs', [
        ['redact_sensitive_text', '后端日志统一脱敏函数'],
        ['redacts_common_sensitive_text', '后端日志脱敏测试']
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
    name: '高频列表查询使用参数化筛选',
    run() {
      requireAll('src-tauri/src/commands/catalog_commands.rs', [
        ['product_service::list_products', '商品列表命令委托 service 层'],
        ['product_service::create_product', '商品新增命令委托 service 层'],
        ['product_service::update_product', '商品编辑命令委托 service 层'],
        ['product_service::disable_product', '商品停用命令委托 service 层'],
        ['product_service::batch_update_products', '商品批量编辑命令委托 service 层'],
        ['product_service::find_product_by_barcode', '扫码查询命令委托 service 层'],
        ['customer_service::list_customers', '客户列表命令委托 service 层'],
        ['customer_service::create_customer', '客户新增命令委托 service 层'],
        ['customer_service::update_customer', '客户编辑命令委托 service 层'],
        ['customer_service::disable_customer', '客户停用命令委托 service 层'],
        ['customer_service::batch_update_customers', '客户批量编辑命令委托 service 层'],
        ['supplier_service::list_suppliers', '供应商列表命令委托 service 层'],
        ['supplier_service::create_supplier', '供应商新增命令委托 service 层'],
        ['supplier_service::update_supplier', '供应商编辑命令委托 service 层'],
        ['supplier_service::disable_supplier', '供应商停用命令委托 service 层'],
        ['supplier_service::batch_update_suppliers', '供应商批量编辑命令委托 service 层']
      ]);
      requireAll('src-tauri/src/commands/rule_account_commands.rs', [
        ['customer_rule_service::list_customer_product_rules', '客户商品规则列表命令委托 service 层'],
        ['customer_rule_service::save_customer_product_rule', '客户商品规则保存命令委托 service 层'],
        ['customer_rule_service::disable_customer_product_rule', '客户商品规则停用命令委托 service 层'],
        ['customer_rule_service::delete_customer_product_rule', '客户商品规则删除命令委托 service 层'],
        ['customer_account_service::list_customer_balances', '客户余额命令委托 service 层'],
        ['customer_account_service::list_payment_records', '收款列表命令委托 service 层'],
        ['customer_account_service::create_payment', '收款新增命令委托 service 层'],
        ['customer_account_service::void_payment', '收款作废命令委托 service 层']
      ]);
      requireAll('src-tauri/src/commands/inventory_commands.rs', [
        ['inventory_service::create_inbound', '入库保存命令委托 service 层'],
        ['inventory_service::list_inbound_records', '入库列表命令委托 service 层'],
        ['inventory_control_service::create_inventory_adjustment', '库存调整新增命令委托 service 层'],
        ['inventory_control_service::list_inventory_adjustments', '库存调整列表命令委托 service 层'],
        ['inventory_control_service::void_inventory_adjustment', '库存调整作废命令委托 service 层'],
        ['inventory_control_service::create_stocktake', '盘点新增命令委托 service 层'],
        ['inventory_control_service::list_stocktakes', '盘点列表命令委托 service 层'],
        ['inventory_control_service::void_stocktake', '盘点作废命令委托 service 层']
      ]);
      requireAll('src-tauri/src/commands/report_document_commands.rs', [
        ['diagnostics_service::run_data_self_check', '数据自检命令委托 service 层'],
        ['diagnostics_service::write_self_check_export', '数据自检导出委托 service 层'],
        ['diagnostics_service::diagnostic_summary', '诊断摘要命令委托 service 层'],
        ['diagnostics_service::export_diagnostic_package', '诊断包导出命令委托 service 层'],
        ['audit_service::list_audit_logs', '审计日志列表命令委托 service 层']
      ]);
      requireAll('src-tauri/src/commands/import_backup_commands.rs', [
        ['backup_service::list_backups', '备份列表命令委托 service 层'],
        ['backup_service::restore_backup', '恢复备份命令委托 service 层'],
        ['backup_service::finalize_restore', '恢复备份收尾命令委托 service 层']
      ]);
      requireAll('src-tauri/src/commands/settings_generalization_commands.rs', [
        ['settings_service::list_settings', '设置列表命令委托 service 层'],
        ['settings_service::save_settings', '设置保存命令委托 service 层']
      ]);
      requireAll('src-tauri/src/commands.rs', [
        ['list_product_and_customer_filters_treat_sql_fragments_as_text', 'SQL 片段按普通文本处理的安全测试']
      ]);
      requireNotText('src-tauri/src/commands.rs', 'fn list_product_records', '商品 SQL 查询不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn batch_update_products_record', '商品批量编辑 SQL 不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn list_customer_records', '客户 SQL 查询不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn create_customer_record(', '客户新增业务不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn update_customer_record(', '客户编辑业务不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn batch_update_customers_record(', '客户批量编辑 SQL 不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn save_customer_product_rule_record(', '客户商品规则保存业务不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn disable_customer_product_rule_record(', '客户商品规则停用业务不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn delete_customer_product_rule_record(', '客户商品规则删除业务不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn preview_customer_product_rule_import_record(', '客户商品规则导入预览不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn import_customer_product_rules_record(', '客户商品规则导入执行不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn parse_customer_product_rule_import_rows(', '客户商品规则 Excel 解析不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn lookup_import_product_id(', '客户商品规则导入商品查询不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn batch_update_suppliers_record(', '供应商批量编辑 SQL 不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn disable_supplier_record(', '供应商停用 SQL 不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn supplier_by_id(', '供应商查询 SQL 不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn map_supplier(', '供应商映射不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn create_inbound_record(', '入库事务不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn create_payment_record(', '收款新增业务不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn payment_by_id(', '收款详情 SQL 不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn customer_balances(', '客户余额 SQL 不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn map_payment_record(', '收款映射不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn map_customer_balance(', '客户余额映射不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn create_inventory_adjustment_record(', '库存调整新增业务不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn void_inventory_adjustment_record(', '库存调整作废业务不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn create_stocktake_record(', '盘点新增业务不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn void_stocktake_record(', '盘点作废业务不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn inventory_adjustments(', '库存调整列表 SQL 不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn stocktakes(', '盘点列表 SQL 不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn map_inventory_adjustment(', '库存调整映射不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn map_stocktake(', '盘点映射不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn save_settings_record(', '设置保存业务不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn run_data_self_check_record', '数据自检业务不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn write_self_check_export', '数据自检导出不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn diagnostic_summary(', '诊断摘要业务不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn export_diagnostic_package_record(', '诊断包导出不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn latest_log_lines(', '日志摘要读取不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn restore_backup_record(', '恢复备份业务不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'fn audit_logs(', '审计日志 SQL 不应留在 command 层');
      requireNotText('src-tauri/src/commands.rs', 'escape_sql', 'command 层不再依赖手工 SQL 转义');
      requireAll('src-tauri/src/lib.rs', [
        ['mod repositories;', '注册 repository 层'],
        ['mod services;', '注册 service 层']
      ]);
      requireAll('src-tauri/src/commands.rs', [
        ['mod catalog_commands;', 'commands 聚合入口注册商品/客户/供应商命令模块'],
        ['mod inventory_commands;', 'commands 聚合入口注册库存命令模块'],
        ['mod order_commands;', 'commands 聚合入口注册订单命令模块'],
        ['mod report_document_commands;', 'commands 聚合入口注册报表单据命令模块'],
        ['pub use catalog_commands::*;', 'commands 聚合入口导出商品/客户/供应商命令'],
        ['pub use order_commands::*;', 'commands 聚合入口导出订单命令']
      ]);
      requireAtMost('src-tauri/src/commands.rs', /#\[tauri::command\]/g, 0, '不应继续承载 Tauri command 具体实现');
      requireAll('src-tauri/src/commands/catalog_commands.rs', [
        ['product_service::list_products', '商品列表命令迁入 catalog_commands'],
        ['customer_service::create_customer', '客户新增命令迁入 catalog_commands'],
        ['supplier_service::batch_update_suppliers', '供应商批量编辑命令迁入 catalog_commands']
      ]);
      requireAll('src-tauri/src/commands/inventory_commands.rs', [
        ['inventory_service::create_inbound', '入库命令迁入 inventory_commands'],
        ['inventory_control_service::create_stocktake', '盘点命令迁入 inventory_commands']
      ]);
      requireAll('src-tauri/src/commands/order_commands.rs', [
        ['order_service::save_order', '订单保存命令迁入 order_commands'],
        ['order_service::void_order', '订单作废命令迁入 order_commands']
      ]);
      requireAll('src-tauri/src/commands/report_document_commands.rs', [
        ['profit_service::get_profit_analytics', '利润统计命令迁入 report_document_commands'],
        ['document_service::print_document', '单据打印命令迁入 report_document_commands'],
        ['diagnostics_service::export_diagnostic_package', '诊断导出命令迁入 report_document_commands']
      ]);
      requireAll('src-tauri/src/services/product_service.rs', [
        ['pub fn list_products', '商品 service 暴露列表业务入口'],
        ['default_product_filter', '商品默认筛选在 service 层处理'],
        ['商品名称和类别必填', '商品新增校验在 service 层处理'],
        ['product_repository::list_products', '商品 service 委托 repository 查询'],
        ['product_repository::batch_update_products', '商品 service 委托 repository 批量更新']
      ]);
      requireAll('src-tauri/src/repositories/product_repository.rs', [
        ['params_from_iter', '商品 repository 动态查询使用参数绑定'],
        ['types::Value', '商品 repository 动态参数使用 rusqlite Value'],
        ['pub fn list_products', '商品 repository 封装列表 SQL'],
        ['pub fn create_product', '商品 repository 封装新增 SQL'],
        ['pub fn update_product', '商品 repository 封装编辑 SQL'],
        ['pub fn disable_product', '商品 repository 封装停用 SQL'],
        ['pub fn batch_update_products', '商品 repository 封装批量编辑 SQL'],
        ['pub fn find_by_barcode', '商品 repository 封装扫码查询 SQL'],
        ["sql.push_str(\" AND p.category = ?\")", '商品类别筛选参数化'],
        ["sql.push_str(\" AND (p.name LIKE ? OR p.barcode LIKE ?)\")", '商品关键字筛选参数化']
      ]);
      requireAll('src-tauri/src/services/customer_service.rs', [
        ['pub fn list_customers', '客户 service 暴露列表业务入口'],
        ['default_customer_filter', '客户默认筛选在 service 层处理'],
        ['客户名称必填', '客户新增和编辑校验在 service 层处理'],
        ['db::guest_customer_name', '客户 service 读取默认客户名'],
        ['名称不能修改', '客户 service 禁止修改散客名称'],
        ['不能删除', '客户 service 禁止删除散客'],
        ['不能批量停用', '客户 service 禁止批量停用散客'],
        ['customer_repository::list_customers', '客户 service 委托 repository 查询'],
        ['customer_repository::batch_update_customers', '客户 service 委托 repository 批量更新']
      ]);
      requireAll('src-tauri/src/repositories/customer_repository.rs', [
        ['params_from_iter', '客户 repository 动态查询使用参数绑定'],
        ['types::Value', '客户 repository 动态参数使用 rusqlite Value'],
        ['pub fn list_customers', '客户 repository 封装列表 SQL'],
        ['pub fn create_customer', '客户 repository 封装新增 SQL'],
        ['pub fn update_customer', '客户 repository 封装编辑 SQL'],
        ['pub fn disable_customer', '客户 repository 封装停用 SQL'],
        ['pub fn batch_update_customers', '客户 repository 封装批量编辑 SQL'],
        ["sql.push_str(\" AND region = ?\")", '客户地区筛选参数化'],
        ["sql.push_str(\" AND (name LIKE ? OR address LIKE ?)\")", '客户关键字筛选参数化']
      ]);
      requireAll('src-tauri/src/services/customer_rule_service.rs', [
        ['pub fn list_customer_product_rules', '客户商品规则 service 暴露列表入口'],
        ['pub fn save_customer_product_rule', '客户商品规则 service 暴露保存入口'],
        ['pub fn disable_customer_product_rule', '客户商品规则 service 暴露停用入口'],
        ['pub fn delete_customer_product_rule', '客户商品规则 service 暴露删除入口'],
        ['pub fn preview_customer_product_rule_import', '客户商品规则 service 暴露导入预览入口'],
        ['pub fn import_customer_product_rules', '客户商品规则 service 暴露导入执行入口'],
        ['parse_customer_product_rule_import_rows', '客户商品规则 Excel 解析集中在 service 层'],
        ['open_workbook_auto', '客户商品规则 service 读取导入工作簿'],
        ['default_rule_filter', '客户商品规则默认筛选在 service 层处理'],
        ['客户和商品必填', '客户商品规则 service 校验主数据'],
        ['每满数量必须大于 0', '客户商品规则 service 校验买赠阈值'],
        ['record_rule_audit', '客户商品规则 service 写审计日志'],
        ['customer_rule_repository::list_customer_product_rules', '客户商品规则 service 委托 repository 查询']
      ]);
      requireAll('src-tauri/src/repositories/customer_rule_repository.rs', [
        ['params_from_iter', '客户商品规则 repository 动态查询使用参数绑定'],
        ['types::Value', '客户商品规则 repository 动态参数使用 rusqlite Value'],
        ['pub fn list_customer_product_rules', '客户商品规则 repository 封装列表 SQL'],
        ['pub fn save_customer_product_rule', '客户商品规则 repository 封装保存 SQL'],
        ['pub fn disable_customer_product_rule', '客户商品规则 repository 封装停用 SQL'],
        ['pub fn delete_customer_product_rule', '客户商品规则 repository 封装删除 SQL'],
        ['pub fn lookup_import_customer_id', '客户商品规则 repository 封装导入客户查询'],
        ['pub fn lookup_import_product_id', '客户商品规则 repository 封装导入商品查询'],
        ["sql.push_str(\" AND r.customer_id = ?\")", '客户商品规则客户筛选参数化'],
        ["sql.push_str(\" AND p.category = ?\")", '客户商品规则类别筛选参数化'],
        ["sql.push_str(\" AND (c.name LIKE ? OR p.name LIKE ?)\")", '客户商品规则关键字筛选参数化']
      ]);
      requireAll('src-tauri/src/services/supplier_service.rs', [
        ['pub fn list_suppliers', '供应商 service 暴露列表业务入口'],
        ['default_supplier_filter', '供应商默认筛选在 service 层处理'],
        ['供应商名称必填', '供应商新增和编辑校验在 service 层处理'],
        ['supplier_repository::list_suppliers', '供应商 service 委托 repository 查询'],
        ['supplier_repository::batch_update_suppliers', '供应商 service 委托 repository 批量更新']
      ]);
      requireAll('src-tauri/src/repositories/supplier_repository.rs', [
        ['params_from_iter', '供应商 repository 动态查询使用参数绑定'],
        ['types::Value', '供应商 repository 动态参数使用 rusqlite Value'],
        ['pub fn list_suppliers', '供应商 repository 封装列表 SQL'],
        ['pub fn create_supplier', '供应商 repository 封装新增 SQL'],
        ['pub fn update_supplier', '供应商 repository 封装编辑 SQL'],
        ['pub fn disable_supplier', '供应商 repository 封装停用 SQL'],
        ['pub fn batch_update_suppliers', '供应商 repository 封装批量编辑 SQL'],
        ["sql.push_str(\" AND (name LIKE ? OR contact LIKE ? OR phone LIKE ?)\")", '供应商关键字筛选参数化']
      ]);
      requireAll('src-tauri/src/services/inventory_service.rs', [
        ['pub fn create_inbound', '入库 service 暴露保存业务入口'],
        ['pub fn list_inbound_records', '入库 service 暴露列表业务入口'],
        ['default_inbound_filter', '入库默认筛选在 service 层处理'],
        ['供应商不存在或已停用', '入库 service 校验供应商状态'],
        ['db::recalc_stock_balance', '入库 service 事务内重算库存余额'],
        ['inventory_repository::active_supplier_name', '入库 service 通过 repository 查询供应商'],
        ['inventory_repository::list_inbound_records', '入库 service 委托 repository 查询列表']
      ]);
      requireAll('src-tauri/src/repositories/inventory_repository.rs', [
        ['params_from_iter', '入库 repository 动态查询使用参数绑定'],
        ['types::Value', '入库 repository 动态参数使用 rusqlite Value'],
        ['pub fn active_supplier_name', '入库 repository 封装供应商状态查询'],
        ['pub fn list_inbound_records', '入库 repository 封装列表 SQL'],
        ["sql.push_str(\" AND i.inbound_date >= ?\")", '入库开始日期筛选参数化'],
        ["sql.push_str(\" AND i.inbound_date <= ?\")", '入库结束日期筛选参数化'],
        ["sql.push_str(\" AND i.product_id = ?\")", '入库商品筛选参数化'],
        ["sql.push_str(\" AND p.category = ?\")", '入库类别筛选参数化']
      ]);
      requireAll('src-tauri/src/services/inventory_control_service.rs', [
        ['pub fn create_inventory_adjustment', '库存调整 service 暴露新增入口'],
        ['pub fn list_inventory_adjustments', '库存调整 service 暴露列表入口'],
        ['pub fn void_inventory_adjustment', '库存调整 service 暴露作废入口'],
        ['pub fn create_stocktake', '盘点 service 暴露新增入口'],
        ['pub fn list_stocktakes', '盘点 service 暴露列表入口'],
        ['pub fn void_stocktake', '盘点 service 暴露作废入口'],
        ['库存调整原因必填', '库存调整 service 校验原因'],
        ['盘点原因必填', '盘点 service 校验原因'],
        ['db::recalc_stock_balance', '库存控制 service 事务内重算库存余额'],
        ['record_audit', '库存控制 service 写审计日志'],
        ['inventory_control_repository::list_inventory_adjustments', '库存调整列表委托 repository'],
        ['inventory_control_repository::list_stocktakes', '盘点列表委托 repository']
      ]);
      requireAll('src-tauri/src/repositories/inventory_control_repository.rs', [
        ['params_from_iter', '库存控制 repository 动态查询使用参数绑定'],
        ['types::Value', '库存控制 repository 动态参数使用 rusqlite Value'],
        ['pub fn list_inventory_adjustments', '库存调整 repository 封装列表 SQL'],
        ['pub fn list_stocktakes', '盘点 repository 封装列表 SQL'],
        ['pub fn inventory_adjustment_by_id', '库存调整 repository 封装详情 SQL'],
        ['pub fn stocktake_by_id', '盘点 repository 封装详情 SQL'],
        ['CommonInventoryFilter', '库存类筛选条件集中封装在 repository 层'],
        ['sql.push_str(&format!(" AND {} >= ?", filter.date_column))', '库存类日期筛选参数化'],
        ["sql.push_str(\" AND category = ?\")", '库存类类别筛选参数化'],
        ["sql.push_str(\" AND status = ?\")", '库存类状态筛选参数化']
      ]);
      requireAll('src-tauri/src/services/backup_service.rs', [
        ['pub fn list_backups', '备份 service 暴露列表入口'],
        ['pub fn restore_backup', '备份 service 暴露恢复文件入口'],
        ['pub fn finalize_restore', '备份 service 暴露恢复后收尾入口'],
        ['db::restore_database_file', '备份 service 调用数据库恢复底层函数'],
        ['record_audit', '备份恢复写审计日志']
      ]);
      requireAll('src-tauri/src/repositories/backup_repository.rs', [
        ['pub fn list_backups', '备份 repository 封装列表 SQL'],
        ['pub fn successful_backup_path', '备份 repository 封装成功备份查询'],
        ['pub fn record_backup_event', '备份 repository 封装备份日志写入']
      ]);
      requireAll('src-tauri/src/services/audit_service.rs', [
        ['pub fn record_audit', '审计 service 暴露记录入口'],
        ['pub fn list_audit_logs', '审计 service 暴露列表入口'],
        ['audit_repository::list_audit_logs', '审计 service 委托 repository 查询']
      ]);
      requireAll('src-tauri/src/repositories/audit_repository.rs', [
        ['params_from_iter', '审计 repository 动态查询使用参数绑定'],
        ['types::Value', '审计 repository 动态参数使用 rusqlite Value'],
        ["sql.push_str(\" AND module = ?\")", '审计模块筛选参数化'],
        ["sql.push_str(\" AND action = ?\")", '审计动作筛选参数化'],
        ["sql.push_str(\" AND log_time >= ?\")", '审计开始日期筛选参数化']
      ]);
      requireAll('src-tauri/src/services/diagnostics_service.rs', [
        ['pub fn run_data_self_check', '数据自检 service 暴露自检入口'],
        ['pub fn write_self_check_export', '数据自检 service 暴露导出入口'],
        ['pub fn diagnostic_summary', '诊断 service 暴露摘要入口'],
        ['pub fn export_diagnostic_package', '诊断 service 暴露诊断包入口'],
        ['inventory_balance', '数据自检检查库存余额一致性'],
        ['order_totals', '数据自检检查订单汇总一致性'],
        ['monthly_credit_remaining', '数据自检检查月费余额一致性'],
        ['document_file_missing', '数据自检检查单据文件存在性']
      ]);
      requireAll('src-tauri/src/services/settings_service.rs', [
        ['pub fn list_settings', '设置 service 暴露列表入口'],
        ['pub fn save_settings', '设置 service 暴露保存入口'],
        ['settings_repository::list_settings', '设置列表委托 repository'],
        ['settings_repository::save_settings', '设置保存委托 repository']
      ]);
      requireAll('src-tauri/src/repositories/settings_repository.rs', [
        ['pub fn list_settings', '设置 repository 封装列表 SQL'],
        ['pub fn save_settings', '设置 repository 封装保存逻辑'],
        ['set_text_if_some', '设置文本字段按需更新'],
        ['set_bool_if_some', '设置布尔字段按需更新'],
        ['db::set_setting', '设置 repository 统一写 settings 表']
      ]);
      requireAll('src-tauri/src/services/customer_account_service.rs', [
        ['pub fn list_customer_balances', '客户账款 service 暴露余额入口'],
        ['pub fn list_payment_records', '客户账款 service 暴露收款列表入口'],
        ['pub fn create_payment', '客户账款 service 暴露收款新增入口'],
        ['pub fn void_payment', '客户账款 service 暴露收款作废入口'],
        ['default_customer_balance_filter', '客户余额默认筛选在 service 层处理'],
        ['default_payment_filter', '收款默认筛选在 service 层处理'],
        ['收款客户和金额不合法', '收款 service 校验金额和客户'],
        ['关联订单不存在或不属于该客户', '收款 service 校验订单归属'],
        ['customer_account_repository::list_customer_balances', '客户账款 service 委托 repository 查询余额'],
        ['customer_account_repository::create_payment', '客户账款 service 委托 repository 新增收款']
      ]);
      requireAll('src-tauri/src/repositories/customer_account_repository.rs', [
        ['params_from_iter', '客户账款 repository 动态查询使用参数绑定'],
        ['types::Value', '客户账款 repository 动态参数使用 rusqlite Value'],
        ['pub fn list_customer_balances', '客户账款 repository 封装余额 SQL'],
        ['pub fn list_payment_records', '客户账款 repository 封装收款列表 SQL'],
        ['pub fn active_customer_exists', '客户账款 repository 封装客户状态查询'],
        ['pub fn normal_order_belongs_to_customer', '客户账款 repository 封装订单归属查询'],
        ['pub fn create_payment', '客户账款 repository 封装收款新增 SQL'],
        ['pub fn void_payment', '客户账款 repository 封装收款作废 SQL'],
        ["sql.push_str(\" AND c.region = ?\")", '客户余额地区筛选参数化'],
        ["sql.push_str(\" AND (c.name LIKE ? OR c.address LIKE ?)\")", '客户余额关键字筛选参数化'],
        ["sql.push_str(\" AND p.customer_id = ?\")", '收款客户筛选参数化'],
        ["sql.push_str(\" AND p.payment_date >= ?\")", '收款开始日期筛选参数化'],
        ["sql.push_str(\" AND p.payment_date <= ?\")", '收款结束日期筛选参数化'],
        ["sql.push_str(\" AND p.status = ?\")", '收款状态筛选参数化']
      ]);
      requireAll('src-tauri/src/repositories/order_repository.rs', [
        ['params_from_iter', '订单动态筛选查询使用参数绑定'],
        ['types::Value', '订单动态筛选参数使用 rusqlite Value'],
        ['pub fn get_order_detail', '订单详情查询封装在 repository'],
        ['pub fn list_orders', '订单列表 SQL 封装在 repository'],
        ['pub fn list_monthly_credits', '月费列表 SQL 封装在 repository'],
        ['pub fn active_rule', '订单报价和保存规则查询封装在 repository'],
        ['pub fn next_order_no', '订单号生成 SQL 封装在 repository'],
        ['pub fn create_order_header', '订单保存订单头写入封装在 repository'],
        ['pub fn update_order_totals', '订单保存汇总回写封装在 repository'],
        ['pub fn create_order_item', '订单保存明细写入封装在 repository'],
        ['pub fn create_inventory_movement', '订单保存库存流水写入封装在 repository'],
        ['pub fn apply_monthly_credit_use', '订单保存月费抵扣更新封装在 repository'],
        ['pub fn create_monthly_credit', '订单保存生成月费封装在 repository'],
        ['pub fn order_movement_product_ids', '订单作废涉及商品查询封装在 repository'],
        ['pub fn order_credit_uses', '订单作废返利抵扣查询封装在 repository'],
        ['pub fn restore_monthly_credit_use', '订单作废返利抵扣回滚封装在 repository'],
        ['pub fn void_credits_generated_by_order', '订单作废生成返利作废封装在 repository'],
        ['pub fn delete_order_movements', '订单作废库存流水删除封装在 repository'],
        ['pub fn mark_order_voided', '订单作废状态更新封装在 repository'],
        ['pub fn mark_order_documents_voided', '订单作废单据同步封装在 repository'],
        ['sql.push_str(" AND order_no LIKE ?")', '订单号模糊筛选参数化'],
        ['sql.push_str(" AND status = ?")', '订单状态筛选参数化'],
        ['sql.push_str(" AND m.category = ?")', '额度类别筛选参数化'],
        ['sql.push_str(" AND m.available_month = ?")', '额度可用月份筛选参数化']
      ]);
      requireAll('src-tauri/src/services/order_service.rs', [
        ['pub fn preview_quote', '订单报价预览 service 入口'],
        ['pub fn save_order', '订单保存 service 入口'],
        ['pub fn void_order', '订单作废 service 入口'],
        ['pub fn list_orders_with_default_filter', '订单默认筛选在 service 层处理'],
        ['pub fn list_monthly_credits_with_default_filter', '月费默认筛选在 service 层处理'],
        ['transaction_with_behavior', '订单保存事务编排在 service 层'],
        ['order_repository::create_order_header', '订单保存 service 写订单头'],
        ['order_repository::apply_monthly_credit_use', '订单保存 service 更新月费抵扣'],
        ['order_repository::create_monthly_credit', '订单保存 service 生成月费'],
        ['order_repository::active_rule', '订单报价 service 委托 repository 查询规则'],
        ['choose_unit_price', '订单报价 service 复用 domain 价格优先级'],
        ['threshold_times', '订单报价 service 复用 domain 买赠阈值'],
        ['order_repository::restore_monthly_credit_use', '订单作废 service 回滚返利抵扣'],
        ['order_repository::delete_order_movements', '订单作废 service 删除库存流水'],
        ['db::recalc_stock_balance', '订单作废 service 重算库存余额'],
        ['order_repository::list_orders', '订单 service 委托 repository 查询'],
        ['order_repository::refresh_credit_statuses', '月费可用状态刷新由 service 编排']
      ]);
      requireAll('src-tauri/src/orders.rs', [
        ['list_orders_and_credits_treat_sql_fragments_as_text', '订单和额度 SQL 片段按普通文本处理的安全测试']
      ]);
      requireNotText('src-tauri/src/orders.rs', 'pub fn save_order', '订单保存入口应迁移到 order_service');
      requireNotText('src-tauri/src/orders.rs', 'fn next_order_no', '订单号生成应迁移到 order_repository');
      requireNotText('src-tauri/src/orders.rs', 'fn insert_order_item', '订单明细写入应迁移到 order_repository');
      requireNotText('src-tauri/src/orders.rs', 'fn insert_movement', '库存流水写入应迁移到 order_repository');
      requireNotText('src-tauri/src/orders.rs', 'pub fn preview_quote', '订单报价入口应迁移到 order_service');
      requireNotText('src-tauri/src/orders.rs', 'pub fn void_order', '订单作废入口应迁移到 order_service');
      requireAll('src-tauri/src/repositories/document_repository.rs', [
        ['params_from_iter', '单据档案 repository 动态查询使用参数绑定'],
        ['types::Value', '单据档案 repository 动态参数使用 rusqlite Value'],
        ['pub fn list_documents', '单据列表 SQL 封装在 repository'],
        ['pub fn document_by_id', '单据详情查询封装在 repository'],
        ['pub fn increment_print_count', '单据打印计数封装在 repository'],
        ['sql.push_str(" AND d.customer_id = ?")', '单据客户筛选参数化'],
        ['sql.push_str(" AND o.order_date >= ?")', '单据开始日期筛选参数化'],
        ['sql.push_str(" AND o.order_date <= ?")', '单据结束日期筛选参数化'],
        ['sql.push_str(" AND d.order_no LIKE ?")', '单据编号筛选参数化'],
        ['sql.push_str(" AND COALESCE(d.status, \'normal\') = ?")', '单据状态筛选参数化']
      ]);
      requireAll('src-tauri/src/services/document_service.rs', [
        ['pub fn list_documents_with_default_filter', '单据默认筛选在 service 层处理'],
        ['document_repository::list_documents', '单据 service 委托 repository 查询'],
        ['document_repository::document_by_id', '单据 service 委托 repository 读取详情'],
        ['document_repository::increment_print_count', '单据 service 编排打印计数更新']
      ]);
      requireAll('src-tauri/src/commands/report_document_commands.rs', [
        ['document_service::list_documents_with_default_filter', '单据列表命令调用 service'],
        ['document_service::open_document', '打开单据命令调用 service'],
        ['document_service::print_document', '打印单据命令调用 service'],
        ['analytics_service::product_ranking', '商品经营排行命令调用 service'],
        ['analytics_service::customer_analysis', '客户经营分析命令调用 service'],
        ['profit_service::daily_profit_summary', '每日利润命令调用 service'],
        ['profit_service::get_profit_analytics', '利润统计命令调用 service'],
        ['profit_service::list_profit_records_with_default_filter', '利润明细命令调用 service'],
        ['report_service::list_inventory_report_with_default_filter', '进销存报表命令调用 service'],
        ['report_service::supplier_purchase_ledger_with_default_filter', '供应商采购台账命令调用 service']
      ]);
      requireAll('src-tauri/src/commands/order_commands.rs', [
        ['order_service::preview_quote', '订单报价命令调用 service'],
        ['order_service::save_order', '订单保存命令调用 service'],
        ['order_service::void_order', '订单作废命令调用 service']
      ]);
      requireAll('src-tauri/src/repositories/report_repository.rs', [
        ['params_from_iter', '报表 repository 动态查询使用参数绑定'],
        ['types::Value', '报表 repository 动态参数使用 rusqlite Value'],
        ['pub fn list_inventory_report', '进销存报表 SQL 封装在 repository'],
        ['pub fn supplier_purchase_summaries', '供应商采购汇总 SQL 封装在 repository'],
        ['pub fn supplier_purchase_details', '供应商采购明细 SQL 封装在 repository'],
        ['pub fn supplier_purchase_monthly_trend', '供应商采购趋势 SQL 封装在 repository'],
        ['sql.push_str(" AND p.category = ?")', '进销存类别筛选参数化'],
        ['sql.push_str(" AND (p.name LIKE ? OR p.barcode LIKE ?)")', '进销存关键字筛选参数化']
      ]);
      requireAll('src-tauri/src/services/report_service.rs', [
        ['pub fn list_inventory_report_with_default_filter', '进销存默认筛选在 service 层处理'],
        ['pub fn supplier_purchase_ledger_with_default_filter', '供应商台账默认筛选在 service 层处理'],
        ['fn movement_date_filter', '库存流水日期筛选集中参数化'],
        ['report_repository::list_inventory_report', '进销存 service 委托 repository'],
        ['report_repository::supplier_purchase_summaries', '供应商汇总 service 委托 repository'],
        ['report_repository::supplier_purchase_details', '供应商明细 service 委托 repository'],
        ['report_repository::supplier_purchase_monthly_trend', '供应商趋势 service 委托 repository']
      ]);
      requireAll('src-tauri/src/repositories/analytics_repository.rs', [
        ['params_from_iter', '经营分析 repository 动态查询使用参数绑定'],
        ['types::Value', '经营分析 repository 动态参数使用 rusqlite Value'],
        ['pub fn product_ranking', '商品经营排行 SQL 封装在 repository'],
        ['pub fn customer_analysis_rows', '客户经营分析 SQL 封装在 repository'],
        ['pub fn customer_order_dates', '客户复购日期 SQL 封装在 repository'],
        ['pub fn customer_favorite_product_rows', '客户偏好商品 SQL 封装在 repository']
      ]);
      requireAll('src-tauri/src/services/analytics_service.rs', [
        ['pub fn product_ranking', '商品经营排行 service 入口'],
        ['pub fn customer_analysis', '客户经营分析 service 入口'],
        ['fn product_rank_expr', '商品排行排序字段白名单'],
        ['fn customer_rank_expr', '客户分析排序字段白名单'],
        ['analytics_repository::product_ranking', '商品排行 service 委托 repository'],
        ['analytics_repository::customer_analysis_rows', '客户分析 service 委托 repository'],
        ['analytics_repository::customer_order_dates', '复购间隔 service 委托 repository'],
        ['analytics_repository::customer_favorite_product_rows', '偏好商品 service 委托 repository']
      ]);
      requireAll('src-tauri/src/repositories/profit_repository.rs', [
        ['params_from_iter', '利润 repository 动态查询使用参数绑定'],
        ['types::Value', '利润 repository 动态参数使用 rusqlite Value'],
        ['pub fn daily_profit_summary', '每日利润 SQL 封装在 repository'],
        ['pub fn profit_analytics_summary', '利润汇总 SQL 封装在 repository'],
        ['pub fn profit_analytics_trend', '利润趋势 SQL 封装在 repository'],
        ['pub fn profit_analytics_category_breakdown', '利润类别拆分 SQL 封装在 repository'],
        ['pub fn profit_analytics_customer_breakdown', '利润客户拆分 SQL 封装在 repository'],
        ['pub fn order_has_category', '利润明细类别过滤 SQL 封装在 repository']
      ]);
      requireAll('src-tauri/src/services/profit_service.rs', [
        ['fn profit_order_filter_for_dates', '利润统计筛选条件集中参数化'],
        ['fn profit_comparison_range', '利润同比环比周期计算'],
        ['fn percent_change', '利润同比环比增长率计算'],
        ['comparison_period', '利润趋势对比字段赋值'],
        ['pub fn list_profit_records_with_default_filter', '利润明细默认筛选在 service 层处理'],
        ['profit_repository::profit_analytics_summary', '利润 service 委托 repository 汇总'],
        ['profit_repository::profit_analytics_trend', '利润 service 委托 repository 趋势'],
        ['profit_repository::order_has_category', '利润明细 service 委托 repository 类别过滤']
      ]);
      requireAll('src-tauri/src/reports.rs', [
        ['params_from_iter', '报表动态筛选查询使用参数绑定'],
        ['types::Value', '报表动态筛选参数使用 rusqlite Value'],
        ['fn append_date_filters', '导出日期筛选集中参数化'],
        ['sql.push_str(" AND p.category = ?")', '导出商品类别筛选参数化'],
        ['sql.push_str(" AND (p.name LIKE ? OR p.barcode LIKE ?)")', '导出商品关键字筛选参数化']
      ]);
      requireNotText('src-tauri/src/reports.rs', 'escape_sql', '报表层不再依赖手工 SQL 转义');
    }
  },
  {
    name: '利润类别筛选有回归测试',
    run() {
      requireAll('src-tauri/src/reports.rs', [
        ['list_profit_records_filters_by_order_item_category', '利润类别筛选测试'],
        ['profit_service::list_profit_records', '利润类别筛选测试调用 service']
      ]);
      requireAll('src-tauri/src/repositories/profit_repository.rs', [
        ['COUNT(*) > 0 FROM order_items', '通过订单明细类别过滤利润记录']
      ]);
    }
  },
  {
    name: '后端 domain 纯业务规则可测试',
    run() {
      requireAll('src-tauri/src/lib.rs', [['mod domain;', '注册 domain 模块']]);
      requireAll('src-tauri/src/domain/mod.rs', [['pub mod order_math;', '订单金额规则模块']]);
      requireAll('src-tauri/src/domain/order_math.rs', [
        ['pub fn choose_unit_price', '价格来源选择纯函数'],
        ['pub fn threshold_times', '买赠/折现触发倍数纯函数'],
        ['price_source_prefers_manual_then_customer_fixed_then_default', '价格优先级单测'],
        ['threshold_times_uses_full_multiples_only', '买赠倍数单测']
      ]);
      requireAll('src-tauri/src/services/order_service.rs', [
        ['use crate::domain::order_math', '订单流程使用 domain 规则'],
        ['choose_unit_price(', '报价使用价格来源规则'],
        ['threshold_times(', '保存和报价使用规则触发倍数']
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
      requireAll('src-tauri/src/services/customer_service.rs', [
        ['db::guest_customer_name', '按配置读取默认客户名'],
        ['名称不能修改', '禁止改名'],
        ['不能删除', '禁止删除']
      ]);
      requireAll('src-tauri/src/commands.rs', [
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
      requireAll('src/app/routes.tsx', [
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
        ['customer_account_service_tracks_payments_and_balances', '客户账款 service 收款和余额测试'],
        ['supplier_crud_helpers_disable_and_reload_supplier', '供应商停用测试'],
        ['supplier_service_defaults_to_active_suppliers_and_uses_text_filters', '供应商 service 默认筛选和参数化测试'],
        ['customer_rule_service_lists_rules_with_text_filters', '客户商品规则 service 筛选和参数化测试'],
        ['create_payment_rejects_invalid_order_customer_pair', '收款关联订单校验测试'],
        ['create_inbound_rejects_disabled_supplier', '入库供应商状态校验测试'],
        ['inventory_control_service_adjustment_lists_and_voids_with_audit', '库存控制 service 调整、查询、作废和审计测试']
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
      requireAll('src/app/routes.tsx', [
        ['InventoryControlPage', '库存盘点页面路由'],
        ['/inventory-control', '库存盘点菜单入口']
      ]);
      requireAll('src/pages/settings/BackupRestoreCard.tsx', [
        ['onRestoreBackup', '设置页恢复备份入口']
      ]);
      requireAll('src/pages/settings/AuditLogCard.tsx', [
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
        ['stocktake_records_difference_and_void_reverses_it', '盘点差异和作废反冲测试']
      ]);
      requireAll('src-tauri/src/commands/order_commands.rs', [
        ['module: "order"', '订单保存和作废写入审计日志']
      ]);
      requireAll('src-tauri/src/services/customer_rule_service.rs', [
        ['module: "rule"', '规则变更写入审计日志'],
        ['record_rule_audit', '规则审计集中在 service 层']
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
      requireAll('src-tauri/src/services/customer_statement_service.rs', [
        ['pub fn customer_statement', '客户对账单计算 service'],
        ['customer_statement_repository::customer_name', '客户对账单 service 查询客户名'],
        ['customer_statement_repository::opening_payable', '客户对账单 service 查询期初应收'],
        ['customer_statement_repository::opening_paid', '客户对账单 service 查询期初收款'],
        ['customer_statement_repository::period_discount_amount', '客户对账单 service 查询本期优惠'],
        ['customer_statement_repository::ledger_rows', '客户对账单 service 查询流水']
      ]);
      requireAll('src-tauri/src/repositories/customer_statement_repository.rs', [
        ['pub fn customer_name', '客户对账单 repository 封装客户查询'],
        ['pub fn opening_payable', '客户对账单 repository 封装期初应收 SQL'],
        ['pub fn opening_paid', '客户对账单 repository 封装期初收款 SQL'],
        ['pub fn period_discount_amount', '客户对账单 repository 封装本期优惠 SQL'],
        ['pub fn ledger_rows', '客户对账单 repository 封装流水 SQL'],
        ['params![customer_id, start_date, end_date]', '客户对账单流水查询参数化']
      ]);
      requireAll('src-tauri/src/reports.rs', [
        ['customer_statement_service::customer_statement', '客户对账单导出复用 service'],
        ['export_customer_statement', '客户对账单导出函数'],
        ['"customer_statement" => export_customer_statement', '客户对账单导出类型分发'],
        ['customer_statement_computes_opening_and_ignores_voided_records', '客户对账单金额滚动测试'],
        ['export_customer_statement_outputs_opening_rows_and_summary', '客户对账单导出结构测试']
      ]);
      requireAll('src-tauri/src/commands/rule_account_commands.rs', [
        ['pub fn get_customer_statement', '客户对账单命令'],
        ['customer_statement_service::customer_statement', '客户对账单命令委托 service']
      ]);
      requireAll('src-tauri/src/lib.rs', [
        ['get_customer_statement', '客户对账单命令注册']
      ]);
      requireAll('src/api/reports.ts', [
        ['customerStatement', '前端客户对账单 API'],
        ['get_customer_statement', '前端调用客户对账单命令']
      ]);
      requireAll('src/shared/types.ts', [
        ['CustomerStatementDto', '前端客户对账单类型'],
        ['periodDiscountAmount', '前端客户对账单本期优惠字段']
      ]);
      requireAll('src/app/routes.tsx', [
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
      requireAll('src-tauri/src/services/report_service.rs', [
        ['pub fn supplier_purchase_ledger', '供应商采购台账查询函数'],
        ['report_repository::supplier_purchase_summaries', '供应商采购台账汇总查询'],
        ['report_repository::supplier_purchase_monthly_trend', '供应商采购台账趋势查询']
      ]);
      requireAll('src-tauri/src/reports.rs', [
        ['supplier_purchase_ledger_summarizes_details_and_monthly_trend', '供应商采购台账汇总、明细和趋势测试']
      ]);
      requireAll('src-tauri/src/commands/report_document_commands.rs', [
        ['pub fn get_supplier_purchase_ledger', '供应商采购台账命令']
      ]);
      requireAll('src-tauri/src/lib.rs', [
        ['get_supplier_purchase_ledger', '供应商采购台账命令注册']
      ]);
      requireAll('src/api/catalog.ts', [
        ['supplierPurchaseLedger', '前端供应商采购台账 API'],
        ['get_supplier_purchase_ledger', '前端调用供应商采购台账命令']
      ]);
      requireAll('src/shared/types.ts', [
        ['SupplierPurchaseLedgerDto', '前端供应商采购台账类型'],
        ['monthlyTrend', '前端供应商采购月度趋势字段']
      ]);
      requireAll('src/app/routes.tsx', [
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
      requireAll('src-tauri/src/commands/catalog_commands.rs', [
        ['pub fn batch_update_products', '商品批量编辑命令'],
        ['pub fn batch_update_customers', '客户批量编辑命令'],
        ['pub fn batch_update_suppliers', '供应商批量编辑命令']
      ]);
      requireAll('src-tauri/src/commands.rs', [
        ['batch_update_products_updates_requested_fields_only', '商品批量编辑测试'],
        ['batch_update_customers_and_suppliers_update_requested_fields', '客户和供应商批量编辑测试']
      ]);
      requireAll('src-tauri/src/lib.rs', [
        ['batch_update_products', '商品批量编辑命令注册'],
        ['batch_update_customers', '客户批量编辑命令注册'],
        ['batch_update_suppliers', '供应商批量编辑命令注册']
      ]);
      requireAll('src/api/catalog.ts', [
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
      requireAll('src-tauri/src/commands/rule_account_commands.rs', [
        ['pub fn preview_customer_product_rule_import', '客户商品规则导入预览命令'],
        ['pub fn import_customer_product_rules', '客户商品规则确认导入命令']
      ]);
      requireAll('src-tauri/src/commands.rs', [
        ['customer_product_rule_import_previews_then_imports_valid_rows', '客户商品规则导入测试']
      ]);
      requireAll('src-tauri/src/services/customer_rule_service.rs', [
        ['parse_customer_product_rule_import_rows', '客户商品规则 Excel 解析'],
        ['pub fn preview_customer_product_rule_import', '客户商品规则导入预览 service'],
        ['pub fn import_customer_product_rules', '客户商品规则确认导入 service']
      ]);
      requireAll('src-tauri/src/lib.rs', [
        ['preview_customer_product_rule_import', '客户商品规则导入预览命令注册'],
        ['import_customer_product_rules', '客户商品规则确认导入命令注册']
      ]);
      requireAll('src/api/orders.ts', [
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
      requireAll('src-tauri/src/services/analytics_service.rs', [
        ['pub fn product_ranking', '商品经营排行查询函数'],
        ['product_rank_expr', '商品经营排行排序白名单'],
        ['gift_cost_amount', '商品经营排行赠品成本指标']
      ]);
      requireAll('src-tauri/src/reports.rs', [
        ['export_product_ranking', '商品经营排行导出函数'],
        ['"product_ranking" => export_product_ranking', '商品经营排行导出类型分发'],
        ['product_ranking_summarizes_sales_profit_and_gift_cost', '商品经营排行销量利润赠品成本测试']
      ]);
      requireAll('src-tauri/src/commands/report_document_commands.rs', [
        ['pub fn get_product_ranking', '商品经营排行命令']
      ]);
      requireAll('src-tauri/src/lib.rs', [
        ['get_product_ranking', '商品经营排行命令注册']
      ]);
      requireAll('src/api/reports.ts', [
        ['productRanking', '前端商品经营排行 API'],
        ['get_product_ranking', '前端调用商品经营排行命令']
      ]);
      requireAll('src/shared/types.ts', [
        ['ProductRankingRankBy', '前端商品经营排行指标类型'],
        ['ProductRankingRowDto', '前端商品经营排行结果类型']
      ]);
      requireAll('src/app/routes.tsx', [
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
      requireAll('src-tauri/src/services/analytics_service.rs', [
        ['pub fn customer_analysis', '客户经营分析查询函数'],
        ['customer_average_repurchase_days', '客户复购间隔计算'],
        ['customer_favorite_products', '客户偏好商品计算'],
        ['customer_rank_expr', '客户分析排序白名单']
      ]);
      requireAll('src-tauri/src/reports.rs', [
        ['export_customer_analysis', '客户经营分析导出函数'],
        ['"customer_analysis" => export_customer_analysis', '客户经营分析导出类型分发'],
        ['customer_analysis_ranks_sales_profit_balance_and_preferences', '客户经营分析销售利润欠款偏好测试']
      ]);
      requireAll('src-tauri/src/commands/report_document_commands.rs', [
        ['pub fn get_customer_analysis', '客户经营分析命令']
      ]);
      requireAll('src-tauri/src/lib.rs', [
        ['get_customer_analysis', '客户经营分析命令注册']
      ]);
      requireAll('src/api/reports.ts', [
        ['customerAnalysis', '前端客户经营分析 API'],
        ['get_customer_analysis', '前端调用客户经营分析命令']
      ]);
      requireAll('src/shared/types.ts', [
        ['CustomerAnalysisRankBy', '前端客户分析排行指标类型'],
        ['CustomerAnalysisRowDto', '前端客户分析结果类型']
      ]);
      requireAll('src/app/routes.tsx', [
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
      requireAll('src-tauri/src/services/profit_service.rs', [
        ['profit_comparison_range', '利润同比环比周期计算'],
        ['percent_change', '利润同比环比增长率计算'],
        ['comparison_period', '利润趋势对比字段赋值']
      ]);
      requireAll('src-tauri/src/reports.rs', [
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
      requireAll('src-tauri/src/commands/order_commands.rs', [
        ['pub fn export_order_pdf_document', '订单 PDF 导出命令'],
      ]);
      requireAll('src-tauri/src/commands/report_document_commands.rs', [
        ['pub fn export_document_pdf', '单据档案 PDF 导出命令']
      ]);
      requireAll('src-tauri/src/commands/settings_generalization_commands.rs', [
        ['settings_service::save_settings', '单据模板设置命令委托 service']
      ]);
      requireAll('src-tauri/src/models.rs', [
        ['template_store_name', '模板店名保存字段']
      ]);
      requireAll('src-tauri/src/lib.rs', [
        ['export_order_pdf_document', '订单 PDF 导出命令注册'],
        ['export_document_pdf', '单据档案 PDF 导出命令注册']
      ]);
      requireAll('src/api/orders.ts', [
        ['exportOrderPdf', '前端订单 PDF 导出 API']
      ]);
      requireAll('src/api/reports.ts', [
        ['exportDocumentPdf', '前端单据档案 PDF 导出 API']
      ]);
      requireAll('src/pages/settings/DocumentTemplateSettingsCard.tsx', [
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
      requireAll('src-tauri/src/commands/report_document_commands.rs', [
        ['pub fn run_data_self_check', '运行数据自检命令'],
        ['pub fn export_data_self_check', '导出数据自检命令'],
        ['pub fn get_diagnostic_summary', '诊断中心摘要命令'],
        ['pub fn export_diagnostic_package', '导出诊断包命令']
      ]);
      requireAll('src-tauri/src/commands.rs', [
        ['data_self_check_detects_core_data_inconsistencies', '数据自检异常测试']
      ]);
      requireAll('src-tauri/src/services/diagnostics_service.rs', [
        ['pub fn run_data_self_check', '数据自检核心函数'],
        ['pub fn write_self_check_export', '数据自检导出函数'],
        ['pub fn diagnostic_summary', '诊断摘要核心函数'],
        ['pub fn export_diagnostic_package', '诊断包导出核心函数']
      ]);
      requireAll('src-tauri/src/lib.rs', [
        ['run_data_self_check', '运行数据自检命令注册'],
        ['export_diagnostic_package', '导出诊断包命令注册']
      ]);
      requireAll('src/api/system.ts', [
        ['runDataSelfCheck', '前端运行数据自检 API'],
        ['exportDataSelfCheck', '前端导出数据自检 API'],
        ['diagnosticSummary', '前端诊断摘要 API'],
        ['exportDiagnosticPackage', '前端导出诊断包 API']
      ]);
      requireAll('src/pages/settings/DiagnosticsCard.tsx', [
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
      requireAll('src-tauri/src/commands/settings_generalization_commands.rs', [
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
      requireAll('src/app/useAppBootstrap.ts', [
        ['api.setupStatus()', '前端启动读取初始化状态'],
        ['api.merchantProfile()', '前端启动读取商户信息'],
        ['api.termSettings()', '前端启动读取术语配置'],
        ['api.featureFlags()', '前端启动读取功能开关']
      ]);
      requireAll('src/app/routes.tsx', [
        ['SetupPage', '初始化向导路由'],
        ['features.monthlyCredit', '功能开关控制月费菜单'],
        ['terms.customer', '菜单使用术语配置']
      ]);
      requireAll('src/app/AppShell.tsx', [
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
      requireAll('src/pages/settings/MerchantProfileCard.tsx', [
        ['商户信息', '设置页商户信息区'],
        ['Logo 路径', '设置页商户 Logo 路径配置'],
        ['name="remark"', '设置页商户备注配置']
      ]);
      requireAll('src/pages/settings/IndustryFeatureCard.tsx', [
        ['行业模板与功能开关', '设置页行业模板和功能开关区']
      ]);
      requireAll('src/pages/settings/TermSettingsCard.tsx', [
        ['业务术语', '设置页术语配置区']
      ]);
      requireAll('src/pages/SettingsPage.tsx', [
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
      requireAll('src/api/settings.ts', [
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
        ['recalc_stock_balance_uses_weighted_average_and_outbound_quantity', '库存余额和加权均价重算测试'],
        ['create_consistent_backup', 'SQLite 一致性备份函数'],
        ['consistent_backup_includes_wal_committed_data', 'WAL 备份一致性测试'],
        ['restore_database_file_creates_consistent_snapshot_in_wal_mode', 'WAL 恢复前快照测试']
      ]);
      requireAll('src-tauri/src/app.rs', [
        ['db::open_database_connection', '运行时连接复用统一数据库连接配置'],
        ['runtime_connection_uses_busy_timeout', '运行时连接 busy_timeout 测试'],
        ['runtime_connections_handle_concurrent_writes', '运行时连接并发写入测试']
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
