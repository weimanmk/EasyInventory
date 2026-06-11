import { chromium } from '@playwright/test';
import { mkdirSync } from 'node:fs';
import net from 'node:net';
import path from 'node:path';
import { spawn } from 'node:child_process';

const root = process.cwd();
const outputDir = path.join(root, 'docs', 'images');
const providedBaseUrl = process.env.EASYINVENTORY_SCREENSHOT_URL;
const port = providedBaseUrl ? null : await availablePort(4174, 4184);
const baseUrl = providedBaseUrl ?? `http://127.0.0.1:${port}`;

const routes = [
  ['#/', 'home_page.png', '首页'],
  ['#/outbound', 'outbound_page.png', '快速出库'],
  ['#/products', 'products_page.png', '商品库存'],
  ['#/customers', 'customers_page.png', '客户管理'],
  ['#/profit', 'profit_page.png', '利润统计'],
  ['#/settings', 'settings_page.png', '系统设置']
];

mkdirSync(outputDir, { recursive: true });

const preview = providedBaseUrl ? null : startPreview(port);

try {
  await waitForServer(baseUrl);

  const browser = await chromium.launch({ channel: 'chrome', headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1 });

  await installTauriMock(page);

  for (const [hash, fileName, heading] of routes) {
    await page.goto(`${baseUrl}/${hash}`, { waitUntil: 'networkidle' });
    await page.getByRole('heading', { name: heading }).waitFor({ timeout: 15_000 });
    await page.screenshot({ path: path.join(outputDir, fileName), fullPage: true });
    console.log(`OK docs/images/${fileName}`);
  }

  await browser.close();
} finally {
  if (preview && !preview.killed) {
    preview.kill();
  }
}

function startPreview(serverPort) {
  const child = spawn(
    process.execPath,
    [
      path.join(root, 'node_modules', 'vite', 'bin', 'vite.js'),
      'preview',
      '--host',
      '127.0.0.1',
      '--port',
      String(serverPort),
      '--strictPort'
    ],
    {
      cwd: root,
      stdio: ['ignore', 'pipe', 'pipe']
    }
  );
  child.stdout.on('data', (data) => process.stdout.write(data));
  child.stderr.on('data', (data) => process.stderr.write(data));
  return child;
}

async function waitForServer(url) {
  const deadline = Date.now() + 30_000;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(url);
      if (response.ok) {
        return;
      }
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 500));
    }
  }
  throw new Error(`Vite preview 启动超时：${url}`);
}

async function availablePort(start, end) {
  for (let candidate = start; candidate <= end; candidate += 1) {
    if (await isPortAvailable(candidate)) {
      return candidate;
    }
  }
  throw new Error(`没有可用端口：${start}-${end}`);
}

function isPortAvailable(candidate) {
  return new Promise((resolve) => {
    const server = net.createServer();
    server.once('error', () => resolve(false));
    server.once('listening', () => {
      server.close(() => resolve(true));
    });
    server.listen(candidate, '127.0.0.1');
  });
}

async function installTauriMock(targetPage) {
  await targetPage.addInitScript(() => {
    const totals = {
      productSalesAmount: 1280,
      directDiscountAmount: 30,
      monthlyCreditUsed: 50,
      customerPayableAmount: 1200,
      brandSubsidyAmount: 0,
      costAmount: 760,
      giftCostAmount: 20,
      profitAmount: 420
    };
    const products = [
      {
        id: 1,
        name: '经典汽水',
        category: '饮料',
        barcode: 'SKU-001',
        defaultPrice: 12,
        safetyStock: 10,
        unit: '箱',
        currentStock: 86,
        avgCost: 7,
        stockValue: 602,
        isActive: true,
        remark: '畅销'
      },
      {
        id: 2,
        name: '柠檬茶',
        category: '茶饮',
        barcode: 'SKU-002',
        defaultPrice: 15,
        safetyStock: 8,
        unit: '箱',
        currentStock: 42,
        avgCost: 9,
        stockValue: 378,
        isActive: true,
        remark: null
      }
    ];
    const customers = [
      { id: 1, region: '东区', name: '明湖便利店', address: '明湖路 18 号', phone: '13800000001', isActive: true, remark: null },
      { id: 2, region: '默认', name: '散客', address: null, phone: null, isActive: true, remark: '系统默认客户' }
    ];
    const order = {
      id: 1,
      orderNo: '20260601001',
      orderDate: '2026-06-01',
      customerId: 1,
      customerName: '明湖便利店',
      customerAddress: '明湖路 18 号',
      totals,
      remark: null,
      documentPath: 'C:/tmp/20260601001.xlsx',
      printCount: 1,
      status: 'normal'
    };
    const settings = {
      default_print_template: 'general',
      active_order_template: 'general',
      default_export_format: 'xlsx',
      default_printer: '默认打印机',
      template_store_name: '通用批发商行',
      template_show_barcode: 'true',
      template_product_label: '商品名称',
      template_quantity_label: '数量',
      template_price_label: '价格',
      template_amount_label: '总价格',
      template_remark_label: '备注',
      template_orientation: 'portrait',
      template_margin: '0'
    };
    const ok = (data) => ({ success: true, data });
    const importMappings = [];

    window.__TAURI_INTERNALS__ = {
      callbacks: {},
      transformCallback: () => 0,
      unregisterCallback: () => undefined,
      convertFileSrc: (filePath) => filePath,
      invoke: async (cmd, args = {}) => {
        switch (cmd) {
          case 'write_client_log':
            return ok(true);
          case 'get_app_status':
            return ok({
              version: '1.3.2',
              databasePath: 'C:/Users/User/AppData/Roaming/EasyInventory/data/inventory.db',
              dataDir: 'C:/Users/User/AppData/Roaming/EasyInventory',
              ordersDir: 'C:/Users/User/AppData/Roaming/EasyInventory/orders',
              exportsDir: 'C:/Users/User/AppData/Roaming/EasyInventory/exports',
              backupsDir: 'C:/Users/User/AppData/Roaming/EasyInventory/backups',
              logsDir: 'C:/Users/User/AppData/Roaming/EasyInventory/logs'
            });
          case 'get_setup_status':
            return ok({ completed: true, merchantName: '通用批发商行', industryTemplate: 'general_wholesale', productCount: products.length, customerCount: customers.length, orderCount: 1 });
          case 'get_merchant_profile':
            return ok({ name: '通用批发商行', contact: '张三', phone: '13800000000', address: '示例市仓储路 1 号', logoPath: null, remark: null });
          case 'get_term_settings':
            return ok({ customer: '客户', region: '地区', product: '商品', category: '类别', rule: '价格规则', credit: '返利额度', guestCustomer: '散客' });
          case 'get_feature_flags':
            return ok({ supplierLedger: true, customerRules: true, monthlyCredit: true, receivables: true, productRanking: true, customerAnalysis: true, inventoryControl: true, diagnostics: true });
          case 'list_products':
            return ok(products);
          case 'list_customers':
            return ok(customers);
          case 'list_suppliers':
            return ok([{ id: 1, name: '默认供应商', contact: '王五', phone: '13900000000', address: '供应商地址', isActive: true, remark: null }]);
          case 'list_orders':
          case 'list_profit_records':
            return ok([order]);
          case 'get_daily_profit_summary':
            return ok({ date: '2026-06-01', orderCount: 1, ...totals });
          case 'get_profit_analytics':
            return ok({
              summary: { orderCount: 8, ...totals },
              trend: [
                { period: '2026-05-30', orderCount: 3, productSalesAmount: 760, customerPayableAmount: 720, directDiscountAmount: 10, monthlyCreditUsed: 30, brandSubsidyAmount: 0, costAmount: 420, giftCostAmount: 10, profitAmount: 290 },
                { period: '2026-05-31', orderCount: 4, productSalesAmount: 980, customerPayableAmount: 930, directDiscountAmount: 20, monthlyCreditUsed: 30, brandSubsidyAmount: 0, costAmount: 590, giftCostAmount: 15, profitAmount: 325 },
                { period: '2026-06-01', orderCount: 8, ...totals }
              ],
              categoryBreakdown: [
                { name: '饮料', orderCount: 5, productSalesAmount: 820, customerPayableAmount: 780, costAmount: 480, profitAmount: 300 },
                { name: '茶饮', orderCount: 3, productSalesAmount: 460, customerPayableAmount: 420, costAmount: 280, profitAmount: 120 }
              ],
              customerBreakdown: [
                { name: '明湖便利店', orderCount: 5, productSalesAmount: 820, customerPayableAmount: 780, costAmount: 480, profitAmount: 300 },
                { name: '散客', orderCount: 3, productSalesAmount: 460, customerPayableAmount: 420, costAmount: 280, profitAmount: 120 }
              ]
            });
          case 'preview_quote':
            return ok({ productId: 1, unitPrice: 12, priceSource: 'default_price', amount: 12, ruleId: null, giftPreview: null, directDiscountPreview: null, monthlyCreditPreview: null, message: '默认售价' });
          case 'get_available_monthly_credits':
            return ok([]);
          case 'list_customer_product_rules':
            return ok([]);
          case 'list_backups':
            return ok([{ id: 1, backupPath: 'C:/tmp/inventory_20260601.db', backupType: 'manual', status: 'success', fileSize: 1024, createdAt: '2026-06-01 09:00:00', remark: null }]);
          case 'list_settings':
            return ok(Object.entries(settings).map(([key, value]) => ({ key, value, updatedAt: '2026-06-01 09:00:00' })));
          case 'list_printers':
            return ok(['默认打印机']);
          case 'list_audit_logs':
            return ok([{ id: 1, logTime: '2026-06-01 09:00:00', module: 'system', action: 'backup', targetType: 'database', targetId: null, targetLabel: '手动备份', result: 'success', message: '备份完成', details: null }]);
          case 'get_diagnostic_summary':
            return ok({ databasePath: 'inventory.db', databaseSize: 2048, version: '1.3.2', backupCount: 1, latestBackupAt: '2026-06-01 09:00:00', productCount: products.length, customerCount: customers.length, orderCount: 1, documentCount: 1, latestLogs: [] });
          case 'list_industry_templates':
            return ok([{ id: 'general_wholesale', name: '通用批发', description: '适合通用商贸批发场景。', terms: {}, features: {}, orderTemplate: 'general' }]);
          case 'list_document_templates':
            return ok([{ id: 'general', name: '通用出库单', description: '通用模板', templateType: 'order', isDefault: true }]);
          case 'list_import_mappings':
            return ok(importMappings);
          case 'list_monthly_credits':
            return ok([]);
          case 'list_inbound_records':
            return ok([]);
          case 'list_documents':
            return ok([]);
          case 'get_order':
            return ok({ order, items: [] });
          default:
            return ok(null);
        }
      }
    };
  });
}
