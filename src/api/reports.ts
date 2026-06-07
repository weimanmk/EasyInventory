import type {
  CustomerAnalysisDto,
  CustomerAnalysisRequest,
  CustomerStatementDto,
  CustomerStatementRequest,
  DailyProfitSummary,
  DocumentDto,
  InventoryReportRowDto,
  OrderDto,
  PrintStatusDto,
  ProductRankingRequest,
  ProductRankingRowDto,
  ProfitAnalyticsRequest,
  ProfitAnalyticsResponse
} from '../shared/types';
import { callCommand } from './tauri';

export const reportApi = {
  customerStatement: (request: CustomerStatementRequest) =>
    callCommand<CustomerStatementDto>('get_customer_statement', { request }),
  exportCustomerStatementPdf: (request: CustomerStatementRequest) =>
    callCommand<string>('export_customer_statement_pdf', { request }),
  dailyProfit: (date: string) => callCommand<DailyProfitSummary>('get_daily_profit_summary', { date }),
  profitAnalytics: (request: ProfitAnalyticsRequest) =>
    callCommand<ProfitAnalyticsResponse>('get_profit_analytics', { request }),
  profitRecords: (filter?: Record<string, unknown>) => callCommand<OrderDto[]>('list_profit_records', { filter }),
  inventoryReport: (filter?: Record<string, unknown>) => callCommand<InventoryReportRowDto[]>('list_inventory_report', { filter }),
  productRanking: (request: ProductRankingRequest) =>
    callCommand<ProductRankingRowDto[]>('get_product_ranking', { request }),
  customerAnalysis: (request: CustomerAnalysisRequest) =>
    callCommand<CustomerAnalysisDto>('get_customer_analysis', { request }),
  documents: (filter?: Record<string, unknown>) => callCommand<DocumentDto[]>('list_documents', { filter }),
  openDocument: (documentId: number) => callCommand<string>('open_document', { documentId }),
  exportDocument: (orderId: number) => callCommand<string>('export_document', { orderId }),
  exportDocumentPdf: (orderId: number) => callCommand<string>('export_document_pdf', { orderId }),
  printDocument: (documentId: number, payload?: Record<string, unknown>) =>
    callCommand<PrintStatusDto>('print_document', { documentId, payload }),
  exportData: (payload: Record<string, unknown>) => callCommand<string>('export_data', { payload })
};
