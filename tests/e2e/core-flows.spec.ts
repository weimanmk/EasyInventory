import { expect, test, type Locator, type Page } from '@playwright/test';

type CommandCall = { cmd: string; args: Record<string, unknown> };

test.beforeEach(async ({ page }) => {
  await installTauriMock(page);
});

test('初始化向导和设置页会完成通用化配置', async ({ page }) => {
  await page.goto('/#/', { waitUntil: 'commit' });
  await expect(page.getByRole('heading', { name: '首页' })).toBeVisible();

  await page.evaluate(() => {
    window.location.hash = '#/setup';
  });
  await expect(page.getByRole('heading', { name: '首次使用初始化' })).toBeVisible();
  const setupPage = page.locator('.setup-page');
  await fillFormInput(setupPage, '商户名称', '浏览器通用商行');
  await fillFormInput(setupPage, '联系人', '李四');
  await page.getByRole('button', { name: '下一步' }).click();

  await selectFormOption(page, setupPage, '选择行业模板', '配送批发');
  await page.getByRole('button', { name: '下一步' }).click();
  await expect(page.locator('.ant-card').filter({ hasText: '术语配置' }).locator('input[value="门店"]')).toBeVisible();
  await page.getByRole('button', { name: '下一步' }).click();
  await page.getByRole('button', { name: '下一步' }).click();
  await selectFormOption(page, setupPage, '初始化后的导入方式', '使用通用 Excel 模板导入');
  await page.getByRole('button', { name: '下载商品模板' }).click();
  await expectCommand(page, 'download_import_template');
  await page.getByRole('button', { name: '下一步' }).click();
  await page.getByRole('button', { name: '完成初始化' }).click();
  await expectCommand(page, 'complete_setup');
  await expect(page.getByRole('heading', { name: '首页' })).toBeVisible();

  await openRoute(page, '/#/settings', '系统设置');
  await fillFormInput(page.locator('.ant-card').filter({ hasText: '商户信息' }), '商户名称', '浏览器设置商行');
  await page.getByRole('button', { name: '保存商户信息' }).click();
  await expectCommand(page, 'save_merchant_profile');
  await fillFormInput(page.locator('.ant-card').filter({ hasText: '业务术语' }), '客户显示名', '门店');
  await page.getByRole('button', { name: '保存术语配置' }).click();
  await expectCommand(page, 'save_term_settings');
  await page.locator('.ant-table-row').filter({ hasText: '配送批发' }).getByRole('button', { name: /应\s*用/ }).click();
  await page.locator('.ant-modal-confirm-btns .ant-btn-primary').click();
  await expectCommand(page, 'apply_industry_template');
  const importCard = page.locator('.ant-card').filter({ hasText: '通用数据导入' });
  await fillFormInput(importCard, 'Excel 文件路径', 'C:/tmp/products.xlsx');
  await importCard.getByRole('button', { name: '读取表头' }).click();
  await expectCommand(page, 'preview_generic_import_headers');
  await expect(importCard.getByText('工作表：商品导入模板')).toBeVisible();
  await fillFormInput(importCard, '映射方案名', '浏览器映射');
  await importCard.getByRole('button', { name: '保存映射方案' }).click();
  await expectCommand(page, 'save_import_mapping');
});

test('新增客户流程会在浏览器中保存并刷新', async ({ page }) => {
  await openRoute(page, '/#/customers', '客户管理');

  await page.getByRole('button', { name: '新增客户' }).click();
  const drawer = page.locator('.ant-drawer').filter({ hasText: '新增客户' });
  await fillFormInput(drawer, '地区', '东区');
  await fillFormInput(drawer, '客户名称', '浏览器测试客户');
  await fillFormInput(drawer, '地址', '测试路 1 号');
  await drawer.getByRole('button', { name: /保\s*存/ }).click();

  await expectCommand(page, 'create_customer');
  await expectCommand(page, 'list_customers', 2);
  await expect(page.getByRole('cell', { name: '浏览器测试客户' })).toBeVisible();
});

test('新增商品流程会在浏览器中保存并刷新', async ({ page }) => {
  await openRoute(page, '/#/products', '商品库存');

  await page.getByRole('button', { name: '新增商品' }).click();
  const drawer = page.locator('.ant-drawer').filter({ hasText: '新增商品' });
  await fillFormInput(drawer, '商品名称', '浏览器测试商品');
  await fillFormInput(drawer, '类别', '饮料');
  await drawer.getByRole('button', { name: /保\s*存/ }).click();

  await expectCommand(page, 'create_product');
  await expectCommand(page, 'list_products', 2);
  await expect(page.getByRole('cell', { name: '浏览器测试商品' })).toBeVisible();
});

test('新增规则流程会选择客户和商品并保存', async ({ page }) => {
  await openRoute(page, '/#/rules', '价格规则');

  await page.getByRole('button', { name: '新增价格规则' }).click();
  const drawer = page.locator('.ant-drawer').filter({ hasText: '新增价格规则' });
  await selectFormOption(page, drawer, '客户', '测试客户');
  await selectFormOption(page, drawer, '商品', '饮料 / 测试商品');
  await fillFormInput(drawer, '固定售价', '9');
  await drawer.getByRole('button', { name: '保存价格规则' }).click();

  await expectCommand(page, 'save_customer_product_rule');
  await expectCommand(page, 'list_customer_product_rules', 2);
});

test('入库流程会提交商品、供应商、数量和进货价', async ({ page }) => {
  await openRoute(page, '/#/inbound', '入库');

  const form = page.locator('.ant-card').filter({ hasText: '入库表单' });
  await selectFormOption(page, form, '商品', '测试商品');
  await selectFormOption(page, form, '供应商', '默认供应商');
  await fillFormInput(form, '数量', '3');
  await fillFormInput(form, '进货价', '4');
  await form.getByRole('button', { name: '保存入库' }).click();

  await expectCommand(page, 'create_inbound');
  await expectCommand(page, 'list_inbound_records', 2);
});

test('快速出库流程会选择商品、使用返利额度并保存订单', async ({ page }) => {
  await openRoute(page, '/#/outbound', '快速出库');

  const outboundForm = page.locator('form').first();
  await selectFormOption(page, outboundForm, '客户', '测试客户');
  await page.getByRole('button', { name: '选择商品' }).click();
  await page.locator('.product-card').filter({ hasText: '测试商品' }).click();
  await page.getByRole('button', { name: '加入' }).click();
  await page.keyboard.press('Escape');

  await page.locator('.ant-table-row').first().getByRole('button', { name: /选择/ }).click();
  await expectCommand(page, 'get_available_monthly_credits');
  const creditModal = page.locator('.ant-modal').filter({ hasText: '选择返利额度抵扣' });
  await creditModal.locator('input').last().fill('5');
  await creditModal.locator('.ant-modal-footer .ant-btn-primary').click();

  await page.getByRole('button', { name: '保存并导出' }).click();
  await expectCommand(page, 'save_order');
});

test('单据档案流程会预览、导出 PDF、重新导出并作废订单', async ({ page }) => {
  await openRoute(page, '/#/documents', '单据档案');

  const row = page.locator('.ant-table-row').first();
  await row.getByRole('button', { name: /预\s*览/ }).click();
  await expectCommand(page, 'get_order');
  await page.locator('.ant-drawer').filter({ hasText: '单据预览' }).getByRole('button', { name: '关闭' }).click();
  await row.getByRole('button', { name: '导出 PDF' }).click();
  await expectCommand(page, 'export_document_pdf');
  await row.getByRole('button', { name: '重新导出' }).click();
  await expectCommand(page, 'export_document');
  await row.getByRole('button', { name: /作\s*废/ }).click();
  await page.locator('.ant-modal-confirm-btns .ant-btn-primary').click();
  await expectCommand(page, 'void_order');
});

test('返利额度账本流程会查询并关闭返利额度', async ({ page }) => {
  await openRoute(page, '/#/credits', '返利额度账本');

  await page.getByRole('button', { name: /查\s*询/ }).click();
  await expectCommand(page, 'list_monthly_credits', 2);
  await page.locator('.ant-table-row').first().getByRole('button', { name: /关\s*闭/ }).click();
  await page.locator('.ant-modal-confirm-btns .ant-btn-primary').click();
  await expectCommand(page, 'close_monthly_credit');
});

test('利润统计流程会执行统计查询和明细查询', async ({ page }) => {
  await openRoute(page, '/#/profit', '利润统计');

  await page.getByRole('button', { name: /查\s*询/ }).click();

  await expectCommand(page, 'get_profit_analytics');
  await expectCommand(page, 'list_profit_records');
  await expect(page.getByText('同比/环比分析')).toBeVisible();
});

test('客户对账单和数据导出流程会导出 Excel 与 PDF', async ({ page }) => {
  await openRoute(page, '/#/customer-statement', '客户对账单');

  await page.getByRole('button', { name: /查\s*询/ }).click();
  await expectCommand(page, 'get_customer_statement');
  await page.getByRole('button', { name: '导出 Excel' }).click();
  await expectCommand(page, 'export_data');
  await page.getByRole('button', { name: '导出 PDF' }).click();
  await expectCommand(page, 'export_customer_statement_pdf');
});

async function openRoute(page: Page, route: string, heading: string) {
  await page.goto('/#/', { waitUntil: 'commit' });
  await expect(page.getByRole('heading', { name: '首页' })).toBeVisible();
  if (route !== '/#/') {
    await page.evaluate((nextRoute) => {
      window.location.hash = nextRoute.replace(/^\/#/, '#');
    }, route);
  }
  await expect(page.getByRole('heading', { name: heading })).toBeVisible();
}

async function fillFormInput(container: Locator, label: string, value: string) {
  await container
    .locator('.ant-form-item')
    .filter({ hasText: label })
    .locator('input, textarea')
    .first()
    .fill(value);
}

async function selectFormOption(page: Page, container: Locator, label: string, option: string) {
  await container
    .locator('.ant-form-item')
    .filter({ hasText: label })
    .locator('.ant-select-selector')
    .first()
    .click();
  await page
    .locator('.ant-select-dropdown:not(.ant-select-dropdown-hidden)')
    .getByText(option, { exact: true })
    .click();
}

async function expectCommand(page: Page, cmd: string, minCount = 1) {
  await expect
    .poll(async () => page.evaluate(([name]) => {
      const calls = (window as unknown as { __EASY_E2E_CALLS__?: CommandCall[] }).__EASY_E2E_CALLS__ ?? [];
      return calls.filter((call) => call.cmd === name).length;
    }, [cmd]))
    .toBeGreaterThanOrEqual(minCount);
}

async function installTauriMock(page: Page) {
  await page.addInitScript(() => {
    const calls: CommandCall[] = [];
    const totals = {
      productSalesAmount: 12,
      directDiscountAmount: 0,
      monthlyCreditUsed: 5,
      customerPayableAmount: 7,
      brandSubsidyAmount: 0,
      costAmount: 4,
      giftCostAmount: 0,
      profitAmount: 3
    };
    const products = [
      {
        id: 1,
        name: '测试商品',
        category: '饮料',
        barcode: 'E2E001',
        defaultPrice: 12,
        safetyStock: 1,
        unit: '件',
        currentStock: 20,
        avgCost: 4,
        stockValue: 80,
        isActive: true,
        remark: null
      }
    ];
    const customers = [
      { id: 1, region: '东区', name: '测试客户', address: '测试路 1 号', phone: null, isActive: true, remark: null },
      { id: 2, region: '默认', name: '散客', address: null, phone: null, isActive: true, remark: null }
    ];
    let setupStatus = {
      completed: true,
      merchantName: '通用测试商行',
      industryTemplate: 'general_wholesale',
      productCount: products.length,
      customerCount: customers.length,
      orderCount: 1
    };
    let merchantProfile = {
      name: '通用测试商行',
      contact: '测试联系人',
      phone: '13800000000',
      address: '测试地址',
      logoPath: null,
      remark: null
    };
    let termSettings = {
      customer: '客户',
      region: '地区',
      product: '商品',
      category: '类别',
      rule: '价格规则',
      credit: '返利额度',
      guestCustomer: '散客'
    };
    let featureFlags = {
      supplierLedger: true,
      customerRules: true,
      monthlyCredit: true,
      receivables: true,
      productRanking: true,
      customerAnalysis: true,
      inventoryControl: true,
      diagnostics: true
    };
    const industryTemplates = [
      {
        id: 'general_wholesale',
        name: '通用批发',
        description: '适合多客户、多商品、按客户价格出库的批发经营。',
        terms: termSettings,
        features: featureFlags,
        orderTemplate: 'general'
      },
      {
        id: 'delivery_wholesale',
        name: '配送批发',
        description: '适合按线路或片区给门店配送的经营方式。',
        terms: {
          customer: '门店',
          region: '线路',
          product: '商品',
          category: '品类',
          rule: '客户价规则',
          credit: '返利额度',
          guestCustomer: '临时客户'
        },
        features: featureFlags,
        orderTemplate: 'delivery'
      }
    ];
    const settings: Record<string, string> = {
      default_print_template: 'general',
      active_order_template: 'general',
      default_export_format: 'xlsx',
      template_store_name: '通用测试商行',
      template_show_barcode: 'true',
      template_product_label: '商品名称',
      template_quantity_label: '数量',
      template_price_label: '价格',
      template_amount_label: '总价格',
      template_remark_label: '备注',
      template_orientation: 'portrait',
      template_margin: '0'
    };
    let importMappings: Record<string, unknown>[] = [];
    const suppliers = [
      { id: 1, name: '默认供应商', contact: '张三', phone: null, address: null, isActive: true, remark: null }
    ];
    const rules: Record<string, unknown>[] = [];
    const inboundRecords: Record<string, unknown>[] = [
      {
        id: 1,
        inboundDate: '2026-06-01',
        productId: 1,
        productName: '测试商品',
        category: '饮料',
        supplierId: 1,
        supplierName: '默认供应商',
        quantity: 5,
        unitCost: 4,
        amount: 20,
        remark: null
      }
    ];
    const credits = [
      {
        id: 1,
        sourceOrderId: 1,
        sourceOrderNo: '20260601001',
        customerId: 1,
        customerName: '测试客户',
        category: '饮料',
        amount: 30,
        usedAmount: 0,
        remainingAmount: 30,
        generatedDate: '2026-06-01',
        availableMonth: '2026-06',
        status: 'available',
        remark: null
      }
    ];
    const orders = [
      {
        id: 1,
        orderNo: '20260601001',
        orderDate: '2026-06-01',
        customerId: 1,
        customerName: '测试客户',
        customerAddress: '测试路 1 号',
        totals,
        remark: null,
        documentPath: 'C:/tmp/20260601001.xlsx',
        printCount: 0,
        status: 'normal'
      }
    ];
    const documents = [
      {
        id: 1,
        orderId: 1,
        orderNo: '20260601001',
        customerId: 1,
        customerName: '测试客户',
        filePath: 'C:/tmp/20260601001.xlsx',
        fileType: 'xlsx',
        printedAt: null,
        printCount: 0,
        createdAt: '2026-06-01 09:00:00',
        status: 'normal'
      }
    ];
    const orderItems = [
      {
        id: 1,
        lineType: 'normal',
        productId: 1,
        productName: '测试商品',
        category: '饮料',
        barcode: 'E2E001',
        quantity: 1,
        unitPrice: 12,
        amount: 12,
        avgCost: 4,
        costAmount: 4,
        profitAmount: 8,
        ruleId: null,
        monthlyCreditId: null,
        remark: null,
        sortOrder: 1
      }
    ];

    function ok(data: unknown) {
      return { success: true, data };
    }

    function statement() {
      return {
        summary: {
          customerId: 1,
          customerName: '测试客户',
          startDate: '2026-06-01',
          endDate: '2026-06-30',
          openingBalance: 100,
          periodPayable: 12,
          periodPaid: 5,
          periodDiscountAmount: 0,
          closingBalance: 107
        },
        rows: [
          {
            recordDate: '2026-06-01',
            recordType: 'order',
            recordNo: '20260601001',
            description: '出库单',
            debitAmount: 12,
            creditAmount: 0,
            balanceAfter: 112,
            remark: null
          }
        ]
      };
    }

    function detail() {
      return { order: orders[0], items: orderItems };
    }

    (window as unknown as { __EASY_E2E_CALLS__: CommandCall[] }).__EASY_E2E_CALLS__ = calls;
    (window as unknown as { __TAURI_INTERNALS__: unknown }).__TAURI_INTERNALS__ = {
      callbacks: {},
      transformCallback: () => 0,
      unregisterCallback: () => undefined,
      convertFileSrc: (filePath: string) => filePath,
      invoke: async (cmd: string, args: Record<string, unknown> = {}) => {
        calls.push({ cmd, args });
        switch (cmd) {
          case 'write_client_log':
            return ok(true);
          case 'get_app_status':
            return ok({
              version: '1.3.0',
              databasePath: 'C:/tmp/inventory.db',
              dataDir: 'C:/tmp',
              ordersDir: 'C:/tmp/orders',
              exportsDir: 'C:/tmp/exports',
              backupsDir: 'C:/tmp/backups',
              logsDir: 'C:/tmp/logs'
            });
          case 'get_setup_status':
            return ok({ ...setupStatus, productCount: products.length, customerCount: customers.length, orderCount: orders.length });
          case 'complete_setup': {
            const request = args.request as Record<string, unknown>;
            merchantProfile = { ...merchantProfile, ...(request.merchant as typeof merchantProfile) };
            termSettings = { ...termSettings, ...(request.terms as typeof termSettings) };
            featureFlags = { ...featureFlags, ...(request.features as typeof featureFlags) };
            setupStatus = {
              ...setupStatus,
              completed: true,
              merchantName: merchantProfile.name,
              industryTemplate: String(request.industryTemplate ?? setupStatus.industryTemplate)
            };
            return ok(true);
          }
          case 'get_merchant_profile':
            return ok(merchantProfile);
          case 'save_merchant_profile':
            merchantProfile = { ...merchantProfile, ...(args.profile as typeof merchantProfile) };
            setupStatus = { ...setupStatus, merchantName: merchantProfile.name };
            settings.template_store_name = merchantProfile.name;
            return ok(true);
          case 'get_term_settings':
            return ok(termSettings);
          case 'save_term_settings':
            termSettings = { ...termSettings, ...(args.terms as typeof termSettings) };
            return ok(true);
          case 'get_feature_flags':
            return ok(featureFlags);
          case 'save_feature_flags':
            featureFlags = { ...featureFlags, ...(args.flags as typeof featureFlags) };
            return ok(true);
          case 'list_industry_templates':
            return ok(industryTemplates);
          case 'apply_industry_template': {
            const request = args.request as Record<string, unknown>;
            const template = industryTemplates.find((item) => item.id === request.templateId) ?? industryTemplates[0];
            termSettings = { ...template.terms };
            featureFlags = { ...template.features };
            setupStatus = { ...setupStatus, industryTemplate: template.id };
            settings.default_print_template = template.orderTemplate;
            return ok(template);
          }
          case 'list_document_templates':
            return ok([
              { id: 'general', name: '通用出库单', description: '通用模板', templateType: 'order', isDefault: true },
              { id: 'delivery', name: '配送出库单', description: '配送模板', templateType: 'order', isDefault: false },
              { id: 'simple', name: '简洁出库单', description: '简洁模板', templateType: 'order', isDefault: false }
            ]);
          case 'list_import_mappings':
            return ok(importMappings);
          case 'save_import_mapping': {
            const mapping = args.mapping as Record<string, unknown>;
            importMappings = [mapping];
            return ok(true);
          }
          case 'list_settings':
            return ok(Object.entries(settings).map(([key, value]) => ({ key, value, updatedAt: '2026-06-01 09:00:00' })));
          case 'save_settings': {
            const payload = args.payload as Record<string, unknown>;
            for (const [key, value] of Object.entries(payload)) {
              settings[key] = String(value ?? '');
            }
            return ok(true);
          }
          case 'list_backups':
            return ok([]);
          case 'list_audit_logs':
            return ok([]);
          case 'get_diagnostic_summary':
            return ok({
              databasePath: 'C:/tmp/inventory.db',
              databaseSize: 1024,
              version: '1.3.0',
              backupCount: 0,
              latestBackupAt: null,
              productCount: products.length,
              customerCount: customers.length,
              orderCount: orders.length,
              documentCount: documents.length,
              latestLogs: []
            });
          case 'preview_generic_import_headers':
            return ok({
              sheetName: '商品导入模板',
              headers: ['商品名称', '类别', '默认售价'],
              fields: [
                { name: '商品名称', required: true, aliases: ['商品', '品名'] },
                { name: '类别', required: false, aliases: ['分类', '品类'] },
                { name: '默认售价', required: false, aliases: ['售价', '价格'] }
              ],
              suggestedMapping: { 商品名称: '商品名称', 类别: '类别', 默认售价: '默认售价' }
            });
          case 'preview_generic_import':
            return ok({
              totalCount: 1,
              validCount: 1,
              createCount: 1,
              overwriteCount: 0,
              skippedCount: 0,
              errorCount: 0,
              rows: [
                {
                  rowNumber: 2,
                  action: 'create',
                  status: 'valid',
                  message: null,
                  name: '导入商品',
                  category: '饮料',
                  defaultPrice: 12
                }
              ]
            });
          case 'confirm_generic_import':
            return ok({
              importedCount: 1,
              createCount: 1,
              overwriteCount: 0,
              skippedCount: 0,
              errorCount: 0,
              rows: []
            });
          case 'export_generic_import_report':
            return ok('C:/tmp/exports/generic_import_report.xlsx');
          case 'download_import_template':
            return ok('C:/tmp/exports/generic_import_template.xlsx');
          case 'list_products':
            return ok(products);
          case 'create_product': {
            const payload = args.payload as Record<string, unknown>;
            const product = {
              id: products.length + 1,
              name: String(payload.name),
              category: String(payload.category),
              barcode: payload.barcode ?? null,
              defaultPrice: Number(payload.defaultPrice ?? 0),
              safetyStock: Number(payload.safetyStock ?? 0),
              unit: payload.unit ?? '件',
              currentStock: 0,
              avgCost: 0,
              stockValue: 0,
              isActive: true,
              remark: payload.remark ?? null
            };
            products.push(product);
            return ok(product);
          }
          case 'list_customers':
            return ok(customers);
          case 'create_customer': {
            const payload = args.payload as Record<string, unknown>;
            const customer = {
              id: customers.length + 1,
              region: payload.region ?? null,
              name: String(payload.name),
              address: payload.address ?? null,
              phone: payload.phone ?? null,
              isActive: true,
              remark: payload.remark ?? null
            };
            customers.push(customer);
            return ok(customer);
          }
          case 'list_suppliers':
            return ok(suppliers);
          case 'list_customer_product_rules':
            return ok(rules);
          case 'save_customer_product_rule':
            rules.push({
              id: rules.length + 1,
              customerId: 1,
              customerName: '测试客户',
              productId: 1,
              productName: '测试商品',
              category: '饮料',
              fixedPrice: 9,
              thresholdQuantity: null,
              giftProductId: null,
              giftProductName: null,
              giftQuantity: null,
              directDiscountAmount: null,
              monthlyCreditAmount: null,
              creditCategory: null,
              isActive: true,
              remark: null
            });
            return ok(rules.length);
          case 'list_inbound_records':
            return ok(inboundRecords);
          case 'create_inbound':
            inboundRecords.push({
              id: inboundRecords.length + 1,
              inboundDate: '2026-06-03',
              productId: 1,
              productName: '测试商品',
              category: '饮料',
              supplierId: 1,
              supplierName: '默认供应商',
              quantity: 3,
              unitCost: 4,
              amount: 12,
              remark: null
            });
            return ok(inboundRecords[inboundRecords.length - 1]);
          case 'preview_quote':
            return ok({
              productId: 1,
              unitPrice: 12,
              priceSource: 'default',
              amount: 12,
              ruleId: null,
              giftPreview: null,
              directDiscountPreview: null,
              monthlyCreditPreview: { amount: 3, category: '饮料' },
              message: '测试报价'
            });
          case 'find_product_by_barcode':
            return ok(products[0]);
          case 'get_available_monthly_credits':
            return ok(credits);
          case 'save_order':
            return ok({ orderId: 1, orderNo: '20260601001', documentPath: 'C:/tmp/20260601001.xlsx', totals });
          case 'get_order':
            return ok(detail());
          case 'list_orders':
            return ok(orders);
          case 'void_order':
            orders[0].status = 'voided';
            documents[0].status = 'voided';
            return ok(orders[0]);
          case 'list_monthly_credits':
            return ok(credits);
          case 'close_monthly_credit':
            credits[0].status = 'closed';
            return ok(true);
          case 'void_monthly_credit':
            credits[0].status = 'voided';
            return ok(true);
          case 'get_profit_analytics':
            return ok({
              summary: totals,
              trend: [{ period: '2026-06-01', orderCount: 1, ...totals }],
              categoryBreakdown: [{ name: '饮料', orderCount: 1, ...totals }],
              customerBreakdown: [{ name: '测试客户', orderCount: 1, ...totals }]
            });
          case 'get_daily_profit_summary':
            return ok({ date: '2026-06-03', orderCount: 1, ...totals });
          case 'list_profit_records':
            return ok(orders);
          case 'list_documents':
            return ok(documents);
          case 'open_document':
            return ok('C:/tmp/20260601001.xlsx');
          case 'export_document':
            return ok('C:/tmp/20260601001.xlsx');
          case 'export_document_pdf':
            return ok('C:/tmp/20260601001.pdf');
          case 'print_document':
            return ok({ filePath: 'C:/tmp/20260601001.xlsx', printerName: null, message: '已发送打印' });
          case 'list_printers':
            return ok(['测试打印机']);
          case 'get_customer_statement':
            return ok(statement());
          case 'export_customer_statement_pdf':
            return ok('C:/tmp/customer_statement.pdf');
          case 'export_data':
            return ok('C:/tmp/export.xlsx');
          default:
            return ok(null);
        }
      }
    };
  });
}
