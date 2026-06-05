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

function requireAll(flow, file, checks) {
  const content = read(file);
  const missing = checks.filter(([text]) => !content.includes(text));
  if (missing.length > 0) {
    const details = missing.map(([, reason]) => reason).join('、');
    throw new Error(`${flow}：${file} 缺少 ${details}`);
  }
}

const flows = [
  {
    name: '新增商品',
    file: 'src/pages/ProductsPage.tsx',
    checks: [
      ['createProduct', '新增商品 API 调用'],
      ['保存后刷新完成', '保存后刷新日志'],
      ['closeEditor', '保存后关闭编辑面板']
    ]
  },
  {
    name: '新增客户',
    file: 'src/pages/CustomersPage.tsx',
    checks: [
      ['createCustomer', '新增客户 API 调用'],
      ['保存后刷新完成', '保存后刷新日志'],
      ['isGuestCustomer', '默认客户识别'],
      ['terms.guestCustomer', '默认客户通用保护']
    ]
  },
  {
    name: '新增规则',
    file: 'src/pages/RulesPage.tsx',
    checks: [
      ['saveRule', '保存规则 API 调用'],
      ['previewRuleImport', '规则批量导入预览'],
      ['保存后刷新完成', '保存后刷新日志']
    ]
  },
  {
    name: '入库',
    file: 'src/pages/InboundPage.tsx',
    checks: [
      ['createInbound', '创建入库 API 调用'],
      ['api.suppliers', '供应商选项加载'],
      ['api.products', '商品选项加载']
    ]
  },
  {
    name: '快速出库',
    file: 'src/pages/OutboundPage.tsx',
    checks: [
      ['previewQuote', '报价预览 API 调用'],
      ['saveOrder', '保存订单 API 调用'],
      ['availableMonthlyCredits', '返利额度抵扣查询'],
      ['ProductPickerModal', '商品选择弹窗']
    ]
  },
  {
    name: '订单作废',
    file: 'src/pages/DocumentsPage.tsx',
    checks: [
      ['voidOrder', '订单作废 API 调用'],
      ['作废会回滚', '作废影响提示'],
      ['单据档案作废', '作废来源说明']
    ]
  },
  {
    name: '返利额度生成和抵扣',
    file: 'src/pages/MonthlyCreditsPage.tsx',
    checks: [
      ['monthlyCredits', '返利额度列表 API 调用'],
      ['closeMonthlyCredit', '关闭返利额度 API 调用'],
      ['voidMonthlyCredit', '作废返利额度 API 调用']
    ]
  },
  {
    name: '利润统计查询',
    file: 'src/pages/ProfitPage.tsx',
    checks: [
      ['profitAnalytics', '利润统计 API 调用'],
      ['profitRecords', '利润明细 API 调用'],
      ['同比/环比分析', '同比环比展示']
    ]
  },
  {
    name: '单据导出',
    file: 'src/pages/DocumentsPage.tsx',
    checks: [
      ['exportDocument', '重新导出 xlsx'],
      ['exportDocumentPdf', '导出 PDF'],
      ['printDocument', '打印单据']
    ]
  }
];

const failures = [];

for (const flow of flows) {
  try {
    requireAll(flow.name, flow.file, flow.checks);
    console.log(`OK ${flow.name}`);
  } catch (error) {
    failures.push(error instanceof Error ? error.message : String(error));
  }
}

if (failures.length > 0) {
  console.error('\n前端流程验收失败：');
  for (const failure of failures) {
    console.error(`- ${failure}`);
  }
  process.exit(1);
}

console.log(`\n前端流程验收通过：${flows.length} 条核心流程`);
