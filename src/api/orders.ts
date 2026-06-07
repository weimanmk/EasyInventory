import type {
  CustomerBalanceDto,
  CustomerProductRuleDto,
  CustomerProductRuleImportPreviewDto,
  CustomerProductRuleImportResultDto,
  InboundRecordDto,
  InventoryAdjustmentDto,
  MonthlyCreditDto,
  OrderDetailDto,
  OrderDto,
  PaymentRecordDto,
  PrintStatusDto,
  QuotePreviewDto,
  StocktakeRecordDto
} from '../shared/types';
import { callCommand } from './tauri';

export const orderApi = {
  createInbound: (payload: Record<string, unknown>) => callCommand('create_inbound', { payload }),
  inboundRecords: (filter?: Record<string, unknown>) => callCommand<InboundRecordDto[]>('list_inbound_records', { filter }),
  previewQuote: (payload: Record<string, unknown>) => callCommand<QuotePreviewDto>('preview_quote', { payload }),
  saveOrder: (payload: Record<string, unknown>) => callCommand('save_order', { payload }),
  order: (id: number) => callCommand<OrderDetailDto>('get_order', { id }),
  orders: (filter?: Record<string, unknown>) => callCommand<OrderDto[]>('list_orders', { filter }),
  exportOrder: (orderId: number) => callCommand<string>('export_order_document', { orderId }),
  exportOrderPdf: (orderId: number) => callCommand<string>('export_order_pdf_document', { orderId }),
  printOrder: (orderId: number) => callCommand<string>('print_order_document', { orderId }),
  printOrderWithOptions: (orderId: number, payload?: Record<string, unknown>) =>
    callCommand<PrintStatusDto>('print_order_document_with_options', { orderId, payload }),
  voidOrder: (id: number, payload?: Record<string, unknown>) => callCommand<OrderDto>('void_order', { id, payload }),
  rules: (filter?: Record<string, unknown>) => callCommand<CustomerProductRuleDto[]>('list_customer_product_rules', { filter }),
  saveRule: (payload: Record<string, unknown>) => callCommand<number>('save_customer_product_rule', { payload }),
  disableRule: (id: number) => callCommand<boolean>('disable_customer_product_rule', { id }),
  deleteRule: (id: number) => callCommand<boolean>('delete_customer_product_rule', { id }),
  previewRuleImport: (filePath: string) =>
    callCommand<CustomerProductRuleImportPreviewDto>('preview_customer_product_rule_import', { filePath }),
  importRules: (filePath: string) =>
    callCommand<CustomerProductRuleImportResultDto>('import_customer_product_rules', { filePath }),
  monthlyCredits: (filter?: Record<string, unknown>) => callCommand<MonthlyCreditDto[]>('list_monthly_credits', { filter }),
  availableMonthlyCredits: (customerId: number, category: string, orderDate: string) =>
    callCommand<MonthlyCreditDto[]>('get_available_monthly_credits', { customerId, category, orderDate }),
  closeMonthlyCredit: (id: number) => callCommand<boolean>('close_monthly_credit', { id }),
  voidMonthlyCredit: (id: number) => callCommand<boolean>('void_monthly_credit', { id }),
  customerBalances: (filter?: Record<string, unknown>) => callCommand<CustomerBalanceDto[]>('list_customer_balances', { filter }),
  paymentRecords: (filter?: Record<string, unknown>) => callCommand<PaymentRecordDto[]>('list_payment_records', { filter }),
  createPayment: (payload: Record<string, unknown>) => callCommand<PaymentRecordDto>('create_payment', { payload }),
  voidPayment: (id: number) => callCommand<PaymentRecordDto>('void_payment', { id }),
  createInventoryAdjustment: (payload: Record<string, unknown>) =>
    callCommand<InventoryAdjustmentDto>('create_inventory_adjustment', { payload }),
  inventoryAdjustments: (filter?: Record<string, unknown>) =>
    callCommand<InventoryAdjustmentDto[]>('list_inventory_adjustments', { filter }),
  voidInventoryAdjustment: (id: number, payload?: Record<string, unknown>) =>
    callCommand<InventoryAdjustmentDto>('void_inventory_adjustment', { id, payload }),
  createStocktake: (payload: Record<string, unknown>) => callCommand<StocktakeRecordDto>('create_stocktake', { payload }),
  stocktakes: (filter?: Record<string, unknown>) => callCommand<StocktakeRecordDto[]>('list_stocktakes', { filter }),
  voidStocktake: (id: number, payload?: Record<string, unknown>) =>
    callCommand<StocktakeRecordDto>('void_stocktake', { id, payload })
};
