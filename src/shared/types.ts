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

export type SupplierDto = {
  id: number;
  name: string;
  contact?: string;
  phone?: string;
  address?: string;
  isActive: boolean;
  remark?: string;
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

export type SettingDto = {
  key: string;
  value: string;
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
