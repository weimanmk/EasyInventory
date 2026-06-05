export type ApiResponse<T> = {
  success: boolean;
  data?: T;
  error?: {
    code: string;
    message: string;
    details?: unknown;
  };
};

export type ProductDto = {
  id: number;
  name: string;
  category: string;
  barcode?: string;
  defaultPrice: number;
  safetyStock: number;
  unit?: string;
  currentStock: number;
  avgCost: number;
  stockValue: number;
  isActive: boolean;
  remark?: string;
};

export type BatchUpdateResultDto = {
  affectedCount: number;
};

export type CustomerDto = {
  id: number;
  region?: string;
  name: string;
  address?: string;
  phone?: string;
  isActive: boolean;
  remark?: string;
};

export type InboundRecordDto = {
  id: number;
  inboundDate: string;
  productId: number;
  productName: string;
  category: string;
  supplierId?: number;
  supplierName?: string;
  quantity: number;
  unitCost: number;
  amount: number;
  remark?: string;
};

export type GiftPreviewDto = {
  productId: number;
  productName: string;
  quantity: number;
};

export type QuotePreviewDto = {
  productId: number;
  unitPrice: number;
  priceSource: 'manual' | 'customer_fixed_price' | 'default_price' | 'zero';
  amount: number;
  ruleId?: number;
  giftPreview?: GiftPreviewDto;
  directDiscountPreview?: { amount: number };
  monthlyCreditPreview?: { amount: number; category: string };
  message: string;
};

export type OrderLine = {
  key: string;
  productId: number;
  productName: string;
  category: string;
  barcode?: string;
  currentStock: number;
  quantity: number;
  unitPrice: number;
  amount: number;
  ruleMessage?: string;
  remark?: string;
  preview?: QuotePreviewDto;
  monthlyCreditUses?: MonthlyCreditUse[];
};

export type MonthlyCreditUse = {
  monthlyCreditId: number;
  amount: number;
};

export type OrderTotalsDto = {
  productSalesAmount: number;
  directDiscountAmount: number;
  monthlyCreditUsed: number;
  customerPayableAmount: number;
  brandSubsidyAmount: number;
  costAmount: number;
  giftCostAmount: number;
  profitAmount: number;
};

export type OrderDto = {
  id: number;
  orderNo: string;
  orderDate: string;
  customerId: number;
  customerName: string;
  customerAddress?: string;
  totals: OrderTotalsDto;
  remark?: string;
  documentPath?: string;
  printCount: number;
  status: string;
};

export type OrderItemDto = {
  id: number;
  lineType: string;
  productId?: number;
  productName?: string;
  category?: string;
  barcode?: string;
  quantity: number;
  unitPrice: number;
  amount: number;
  avgCost: number;
  costAmount: number;
  profitAmount: number;
  ruleId?: number;
  monthlyCreditId?: number;
  remark?: string;
  sortOrder: number;
};

export type OrderDetailDto = {
  order: OrderDto;
  items: OrderItemDto[];
};

export type CustomerProductRuleDto = {
  id: number;
  customerId: number;
  customerName: string;
  productId: number;
  productName: string;
  category: string;
  fixedPrice?: number;
  thresholdQuantity?: number;
  giftProductId?: number;
  giftProductName?: string;
  giftQuantity?: number;
  directDiscountAmount?: number;
  monthlyCreditAmount?: number;
  creditCategory?: string;
  isActive: boolean;
  remark?: string;
};

export type CustomerProductRuleImportRowDto = {
  rowNumber: number;
  customerName: string;
  productName: string;
  category?: string;
  fixedPrice?: number;
  thresholdQuantity?: number;
  giftProductName?: string;
  giftQuantity?: number;
  directDiscountAmount?: number;
  monthlyCreditAmount?: number;
  creditCategory?: string;
  remark?: string;
  action: string;
  status: string;
  message?: string;
};

export type CustomerProductRuleImportPreviewDto = {
  totalCount: number;
  validCount: number;
  createCount: number;
  overwriteCount: number;
  errorCount: number;
  skippedCount: number;
  rows: CustomerProductRuleImportRowDto[];
};

export type CustomerProductRuleImportResultDto = {
  importedCount: number;
  createCount: number;
  overwriteCount: number;
  errorCount: number;
  skippedCount: number;
  rows: CustomerProductRuleImportRowDto[];
};

export type MonthlyCreditDto = {
  id: number;
  sourceOrderId: number;
  sourceOrderNo: string;
  customerId: number;
  customerName: string;
  category: string;
  amount: number;
  usedAmount: number;
  remainingAmount: number;
  generatedDate: string;
  availableMonth: string;
  status: string;
  remark?: string;
};

export type DailyProfitSummary = {
  date: string;
  orderCount: number;
  productSalesAmount: number;
  customerPayableAmount: number;
  directDiscountAmount: number;
  monthlyCreditUsed: number;
  brandSubsidyAmount: number;
  costAmount: number;
  giftCostAmount: number;
  profitAmount: number;
};

export type ProfitPeriod = 'day' | 'month' | 'year';

export type ProfitAnalyticsRequest = {
  period: ProfitPeriod;
  startDate: string;
  endDate: string;
  customerId?: number;
  category?: string;
};

export type ProfitAnalyticsMetricDto = {
  orderCount: number;
  productSalesAmount: number;
  customerPayableAmount: number;
  directDiscountAmount: number;
  monthlyCreditUsed: number;
  brandSubsidyAmount: number;
  costAmount: number;
  giftCostAmount: number;
  profitAmount: number;
};

export type ProfitAnalyticsTrendPointDto = ProfitAnalyticsMetricDto & {
  period: string;
  comparisonPeriod?: string;
  comparisonSalesAmount?: number;
  comparisonProfitAmount?: number;
  salesChangeAmount?: number;
  salesChangeRate?: number;
  profitChangeAmount?: number;
  profitChangeRate?: number;
};

export type ProfitBreakdownDto = {
  name: string;
  orderCount: number;
  productSalesAmount: number;
  customerPayableAmount: number;
  costAmount: number;
  profitAmount: number;
};

export type ProfitAnalyticsResponse = {
  summary: ProfitAnalyticsMetricDto;
  trend: ProfitAnalyticsTrendPointDto[];
  categoryBreakdown: ProfitBreakdownDto[];
  customerBreakdown: ProfitBreakdownDto[];
};

export type ProductRankingRankBy = 'sales_quantity' | 'sales_amount' | 'profit_amount' | 'gift_cost_amount';

export type ProductRankingRequest = {
  startDate?: string;
  endDate?: string;
  category?: string;
  rankBy?: ProductRankingRankBy;
  limit?: number;
};

export type ProductRankingRowDto = {
  productId: number;
  productName: string;
  category: string;
  orderCount: number;
  salesQuantity: number;
  salesAmount: number;
  costAmount: number;
  profitAmount: number;
  giftQuantity: number;
  giftCostAmount: number;
};

export type CustomerAnalysisRankBy = 'sales_amount' | 'profit_amount' | 'balance_amount';

export type CustomerAnalysisRequest = {
  startDate?: string;
  endDate?: string;
  category?: string;
  rankBy?: CustomerAnalysisRankBy;
  limit?: number;
};

export type CustomerAnalysisRowDto = {
  customerId: number;
  customerName: string;
  region?: string;
  orderCount: number;
  salesAmount: number;
  costAmount: number;
  profitAmount: number;
  balanceAmount: number;
  recentOrderDate?: string;
  averageRepurchaseDays?: number;
  favoriteProducts: string;
};

export type CustomerAnalysisDto = {
  rows: CustomerAnalysisRowDto[];
};

export type SupplierDto = {
  id: number;
  name: string;
  contact?: string;
  phone?: string;
  address?: string;
  isActive: boolean;
  remark?: string;
};

export type SupplierPurchaseLedgerRequest = {
  startDate?: string;
  endDate?: string;
  supplierId?: number;
};

export type SupplierPurchaseSummaryDto = {
  supplierId?: number;
  supplierName: string;
  inboundCount: number;
  inboundAmount: number;
  recentInboundDate?: string;
};

export type SupplierPurchaseTrendPointDto = {
  period: string;
  inboundCount: number;
  inboundAmount: number;
};

export type SupplierPurchaseLedgerDto = {
  summaries: SupplierPurchaseSummaryDto[];
  details: InboundRecordDto[];
  monthlyTrend: SupplierPurchaseTrendPointDto[];
};

export type CustomerBalanceDto = {
  customerId: number;
  customerName: string;
  region?: string;
  totalPayable: number;
  totalPaid: number;
  balance: number;
  lastOrderDate?: string;
  lastPaymentDate?: string;
};

export type PaymentRecordDto = {
  id: number;
  paymentDate: string;
  customerId: number;
  customerName: string;
  amount: number;
  method?: string;
  relatedOrderId?: number;
  status: string;
  remark?: string;
  createdAt: string;
};

export type CustomerStatementRequest = {
  customerId: number;
  startDate: string;
  endDate: string;
};

export type CustomerStatementSummaryDto = {
  customerId: number;
  customerName: string;
  startDate: string;
  endDate: string;
  openingBalance: number;
  periodPayable: number;
  periodPaid: number;
  periodDiscountAmount: number;
  closingBalance: number;
};

export type CustomerStatementRowDto = {
  recordDate: string;
  recordType: string;
  recordNo: string;
  description: string;
  debitAmount: number;
  creditAmount: number;
  balanceAfter: number;
  remark?: string;
};

export type CustomerStatementDto = {
  summary: CustomerStatementSummaryDto;
  rows: CustomerStatementRowDto[];
};

export type InventoryReportRowDto = {
  productId: number;
  productName: string;
  category: string;
  barcode?: string;
  inboundQuantity: number;
  inboundAmount: number;
  outboundQuantity: number;
  outboundAmount: number;
  giftQuantity: number;
  currentStock: number;
  avgCost: number;
  stockValue: number;
};

export type DocumentDto = {
  id: number;
  orderId: number;
  orderNo: string;
  customerId: number;
  customerName: string;
  filePath: string;
  fileType: string;
  printedAt?: string;
  printCount: number;
  createdAt: string;
  status: string;
};

export type ImportResult = {
  productCount: number;
  customerCount: number;
  movementCount: number;
  profitCount: number;
  warnings: string[];
  errors: string[];
};

export type BackupDto = {
  id: number;
  backupPath: string;
  backupType: string;
  status: string;
  message?: string;
  createdAt: string;
};

export type RestoreBackupResultDto = {
  restoredBackupPath: string;
  preRestoreBackupPath: string;
  message: string;
};

export type DataSelfCheckIssueDto = {
  checkCode: string;
  severity: string;
  targetType: string;
  targetId?: number;
  targetLabel: string;
  message: string;
  details?: string;
};

export type DataSelfCheckDto = {
  checkedAt: string;
  issueCount: number;
  inventoryChecked: number;
  ordersChecked: number;
  creditsChecked: number;
  documentsChecked: number;
  issues: DataSelfCheckIssueDto[];
};

export type DiagnosticSummaryDto = {
  generatedAt: string;
  databasePath: string;
  logsDir: string;
  backupsDir: string;
  exportsDir: string;
  version: string;
  databaseSize: number;
  backupCount: number;
  latestBackupAt?: string;
  productCount: number;
  customerCount: number;
  orderCount: number;
  documentCount: number;
  settingCount: number;
  latestLogs: string[];
};

export type DiagnosticPackageDto = {
  filePath: string;
  message: string;
};

export type InventoryAdjustmentDto = {
  id: number;
  adjustmentDate: string;
  productId: number;
  productName: string;
  category: string;
  adjustmentType: string;
  quantityDelta: number;
  unitCost: number;
  amount: number;
  reason: string;
  remark?: string;
  status: string;
  voidReason?: string;
  voidedAt?: string;
  createdAt: string;
};

export type StocktakeRecordDto = {
  id: number;
  stocktakeDate: string;
  productId: number;
  productName: string;
  category: string;
  systemStock: number;
  actualStock: number;
  differenceQuantity: number;
  unitCost: number;
  differenceAmount: number;
  reason: string;
  remark?: string;
  status: string;
  voidReason?: string;
  voidedAt?: string;
  createdAt: string;
};

export type AuditLogDto = {
  id: number;
  logTime: string;
  module: string;
  action: string;
  targetType?: string;
  targetId?: number;
  targetLabel?: string;
  result: string;
  message?: string;
  details?: string;
};

export type SettingDto = {
  key: string;
  value: string;
};

export type SetupStatusDto = {
  completed: boolean;
  merchantName: string;
  industryTemplate: string;
  productCount: number;
  customerCount: number;
  orderCount: number;
};

export type MerchantProfileDto = {
  name: string;
  contact?: string;
  phone?: string;
  address?: string;
  logoPath?: string;
  remark?: string;
};

export type TermSettingsDto = {
  customer: string;
  region: string;
  product: string;
  category: string;
  rule: string;
  credit: string;
  guestCustomer: string;
};

export type FeatureFlagsDto = {
  supplierLedger: boolean;
  customerRules: boolean;
  monthlyCredit: boolean;
  receivables: boolean;
  productRanking: boolean;
  customerAnalysis: boolean;
  inventoryControl: boolean;
  diagnostics: boolean;
};

export type IndustryTemplateDto = {
  id: string;
  name: string;
  description: string;
  terms: TermSettingsDto;
  features: FeatureFlagsDto;
  orderTemplate: string;
};

export type DocumentTemplateDto = {
  id: string;
  name: string;
  description: string;
  templateType: string;
  isDefault: boolean;
};

export type GenericImportRequest = {
  importType: 'products' | 'customers' | 'initial_stock';
  filePath: string;
  duplicateStrategy?: 'skip' | 'overwrite' | 'append_suffix';
  fieldMapping?: Record<string, string>;
};

export type GenericImportHeaderRequest = {
  importType: GenericImportRequest['importType'];
  filePath: string;
};

export type GenericImportFieldDto = {
  name: string;
  required: boolean;
  aliases: string[];
};

export type GenericImportHeadersDto = {
  importType: string;
  sheetName: string;
  headers: string[];
  fields: GenericImportFieldDto[];
  suggestedMapping: Record<string, string>;
};

export type GenericImportRowDto = {
  rowNumber: number;
  action: string;
  status: string;
  message?: string;
  name?: string;
  category?: string;
  region?: string;
  barcode?: string;
  defaultPrice?: number;
  safetyStock?: number;
  unit?: string;
  address?: string;
  phone?: string;
  quantity?: number;
  unitPrice?: number;
  remark?: string;
};

export type GenericImportPreviewDto = {
  importType: string;
  totalCount: number;
  validCount: number;
  createCount: number;
  overwriteCount: number;
  errorCount: number;
  skippedCount: number;
  rows: GenericImportRowDto[];
};

export type GenericImportResultDto = {
  importType: string;
  importedCount: number;
  createCount: number;
  overwriteCount: number;
  errorCount: number;
  skippedCount: number;
  rows: GenericImportRowDto[];
};

export type GenericImportReportRequest = {
  title: string;
  rows: GenericImportRowDto[];
};

export type ImportMappingDto = {
  name: string;
  importType: string;
  fieldMapping: Record<string, string>;
};

export type PrintStatusDto = {
  filePath: string;
  printerName?: string;
  message: string;
};

export type AppStatusDto = {
  databasePath: string;
  dataDir: string;
  backupsDir: string;
  ordersDir: string;
  exportsDir: string;
  logsDir: string;
  version: string;
};
