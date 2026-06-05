use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiResponse<T>
where
    T: Serialize,
{
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<ApiError>,
}

impl<T> ApiResponse<T>
where
    T: Serialize,
{
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn err(code: &str, message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(ApiError {
                code: code.to_string(),
                message: message.into(),
                details: None,
            }),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListProductsRequest {
    pub category: Option<String>,
    pub keyword: Option<String>,
    pub only_low_stock: Option<bool>,
    pub only_in_stock: Option<bool>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductDto {
    pub id: i64,
    pub name: String,
    pub category: String,
    pub barcode: Option<String>,
    pub default_price: f64,
    pub safety_stock: f64,
    pub unit: Option<String>,
    pub current_stock: f64,
    pub avg_cost: f64,
    pub stock_value: f64,
    pub is_active: bool,
    pub remark: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductPayload {
    pub name: String,
    pub category: String,
    pub barcode: Option<String>,
    pub default_price: Option<f64>,
    pub safety_stock: Option<f64>,
    pub unit: Option<String>,
    pub remark: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchUpdateProductsRequest {
    pub ids: Vec<i64>,
    pub category: Option<String>,
    pub safety_stock: Option<f64>,
    pub default_price: Option<f64>,
    pub unit: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchUpdateCustomersRequest {
    pub ids: Vec<i64>,
    pub region: Option<String>,
    pub remark: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchUpdateSuppliersRequest {
    pub ids: Vec<i64>,
    pub contact: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub remark: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchUpdateResultDto {
    pub affected_count: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListCustomersRequest {
    pub region: Option<String>,
    pub keyword: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerDto {
    pub id: i64,
    pub region: Option<String>,
    pub name: String,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub is_active: bool,
    pub remark: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerPayload {
    pub region: Option<String>,
    pub name: String,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub remark: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInboundRequest {
    pub inbound_date: String,
    pub product_id: i64,
    pub supplier_id: Option<i64>,
    pub quantity: f64,
    pub unit_cost: f64,
    pub remark: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInboundResponse {
    pub inbound_id: i64,
    pub product_id: i64,
    pub current_stock: f64,
    pub avg_cost: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListInboundRecordsRequest {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub product_id: Option<i64>,
    pub category: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InboundRecordDto {
    pub id: i64,
    pub inbound_date: String,
    pub product_id: i64,
    pub product_name: String,
    pub category: String,
    pub supplier_id: Option<i64>,
    pub supplier_name: Option<String>,
    pub quantity: f64,
    pub unit_cost: f64,
    pub amount: f64,
    pub remark: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewQuoteRequest {
    pub customer_id: i64,
    pub product_id: i64,
    pub quantity: f64,
    pub manual_price: Option<f64>,
    pub order_date: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GiftPreviewDto {
    pub product_id: i64,
    pub product_name: String,
    pub quantity: f64,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DiscountPreviewDto {
    pub amount: f64,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MonthlyCreditPreviewDto {
    pub amount: f64,
    pub category: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct QuotePreviewDto {
    pub product_id: i64,
    pub unit_price: f64,
    pub price_source: String,
    pub amount: f64,
    pub rule_id: Option<i64>,
    pub gift_preview: Option<GiftPreviewDto>,
    pub direct_discount_preview: Option<DiscountPreviewDto>,
    pub monthly_credit_preview: Option<MonthlyCreditPreviewDto>,
    pub message: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MonthlyCreditUseRequest {
    pub monthly_credit_id: i64,
    pub amount: f64,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SaveOrderItemRequest {
    pub product_id: i64,
    pub quantity: f64,
    pub unit_price: f64,
    pub remark: Option<String>,
    pub monthly_credit_uses: Option<Vec<MonthlyCreditUseRequest>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveOrderRequest {
    pub order_date: String,
    pub customer_id: i64,
    pub customer_address: Option<String>,
    pub remark: Option<String>,
    pub items: Vec<SaveOrderItemRequest>,
}

#[derive(Debug, Serialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OrderTotalsDto {
    pub product_sales_amount: f64,
    pub direct_discount_amount: f64,
    pub monthly_credit_used: f64,
    pub customer_payable_amount: f64,
    pub brand_subsidy_amount: f64,
    pub cost_amount: f64,
    pub gift_cost_amount: f64,
    pub profit_amount: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveOrderResponse {
    pub order_id: i64,
    pub order_no: String,
    pub document_path: String,
    pub totals: OrderTotalsDto,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListOrdersRequest {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub customer_id: Option<i64>,
    pub order_no: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OrderDto {
    pub id: i64,
    pub order_no: String,
    pub order_date: String,
    pub customer_id: i64,
    pub customer_name: String,
    pub customer_address: Option<String>,
    pub totals: OrderTotalsDto,
    pub remark: Option<String>,
    pub document_path: Option<String>,
    pub print_count: i64,
    pub status: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct OrderItemDto {
    pub id: i64,
    pub line_type: String,
    pub product_id: Option<i64>,
    pub product_name: Option<String>,
    pub category: Option<String>,
    pub barcode: Option<String>,
    pub quantity: f64,
    pub unit_price: f64,
    pub amount: f64,
    pub avg_cost: f64,
    pub cost_amount: f64,
    pub profit_amount: f64,
    pub rule_id: Option<i64>,
    pub monthly_credit_id: Option<i64>,
    pub remark: Option<String>,
    pub sort_order: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderDetailDto {
    pub order: OrderDto,
    pub items: Vec<OrderItemDto>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuleFilterRequest {
    pub customer_id: Option<i64>,
    pub product_id: Option<i64>,
    pub category: Option<String>,
    pub keyword: Option<String>,
    pub is_active: Option<bool>,
    pub rule_type: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SaveCustomerProductRuleRequest {
    pub id: Option<i64>,
    pub customer_id: i64,
    pub product_id: i64,
    pub fixed_price: Option<f64>,
    pub threshold_quantity: Option<f64>,
    pub gift_product_id: Option<i64>,
    pub gift_quantity: Option<f64>,
    pub direct_discount_amount: Option<f64>,
    pub monthly_credit_amount: Option<f64>,
    pub credit_category: Option<String>,
    pub is_active: bool,
    pub remark: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerProductRuleDto {
    pub id: i64,
    pub customer_id: i64,
    pub customer_name: String,
    pub product_id: i64,
    pub product_name: String,
    pub category: String,
    pub fixed_price: Option<f64>,
    pub threshold_quantity: Option<f64>,
    pub gift_product_id: Option<i64>,
    pub gift_product_name: Option<String>,
    pub gift_quantity: Option<f64>,
    pub direct_discount_amount: Option<f64>,
    pub monthly_credit_amount: Option<f64>,
    pub credit_category: Option<String>,
    pub is_active: bool,
    pub remark: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CustomerProductRuleImportRowDto {
    pub row_number: i64,
    pub customer_name: String,
    pub product_name: String,
    pub category: Option<String>,
    pub fixed_price: Option<f64>,
    pub threshold_quantity: Option<f64>,
    pub gift_product_name: Option<String>,
    pub gift_quantity: Option<f64>,
    pub direct_discount_amount: Option<f64>,
    pub monthly_credit_amount: Option<f64>,
    pub credit_category: Option<String>,
    pub remark: Option<String>,
    pub action: String,
    pub status: String,
    pub message: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerProductRuleImportPreviewDto {
    pub total_count: i64,
    pub valid_count: i64,
    pub create_count: i64,
    pub overwrite_count: i64,
    pub error_count: i64,
    pub skipped_count: i64,
    pub rows: Vec<CustomerProductRuleImportRowDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerProductRuleImportResultDto {
    pub imported_count: i64,
    pub create_count: i64,
    pub overwrite_count: i64,
    pub error_count: i64,
    pub skipped_count: i64,
    pub rows: Vec<CustomerProductRuleImportRowDto>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthlyCreditFilterRequest {
    pub customer_id: Option<i64>,
    pub category: Option<String>,
    pub status: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub available_month: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthlyCreditDto {
    pub id: i64,
    pub source_order_id: i64,
    pub source_order_no: String,
    pub customer_id: i64,
    pub customer_name: String,
    pub category: String,
    pub amount: f64,
    pub used_amount: f64,
    pub remaining_amount: f64,
    pub generated_date: String,
    pub available_month: String,
    pub status: String,
    pub remark: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyProfitSummary {
    pub date: String,
    pub order_count: i64,
    pub product_sales_amount: f64,
    pub customer_payable_amount: f64,
    pub direct_discount_amount: f64,
    pub monthly_credit_used: f64,
    pub brand_subsidy_amount: f64,
    pub cost_amount: f64,
    pub gift_cost_amount: f64,
    pub profit_amount: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfitAnalyticsRequest {
    pub period: String,
    pub start_date: String,
    pub end_date: String,
    pub customer_id: Option<i64>,
    pub category: Option<String>,
}

#[derive(Debug, Serialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProfitAnalyticsMetricDto {
    pub order_count: i64,
    pub product_sales_amount: f64,
    pub customer_payable_amount: f64,
    pub direct_discount_amount: f64,
    pub monthly_credit_used: f64,
    pub brand_subsidy_amount: f64,
    pub cost_amount: f64,
    pub gift_cost_amount: f64,
    pub profit_amount: f64,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProfitAnalyticsTrendPointDto {
    pub period: String,
    pub order_count: i64,
    pub product_sales_amount: f64,
    pub customer_payable_amount: f64,
    pub direct_discount_amount: f64,
    pub monthly_credit_used: f64,
    pub brand_subsidy_amount: f64,
    pub cost_amount: f64,
    pub gift_cost_amount: f64,
    pub profit_amount: f64,
    pub comparison_period: Option<String>,
    pub comparison_sales_amount: Option<f64>,
    pub comparison_profit_amount: Option<f64>,
    pub sales_change_amount: Option<f64>,
    pub sales_change_rate: Option<f64>,
    pub profit_change_amount: Option<f64>,
    pub profit_change_rate: Option<f64>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProfitBreakdownDto {
    pub name: String,
    pub order_count: i64,
    pub product_sales_amount: f64,
    pub customer_payable_amount: f64,
    pub cost_amount: f64,
    pub profit_amount: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfitAnalyticsResponse {
    pub summary: ProfitAnalyticsMetricDto,
    pub trend: Vec<ProfitAnalyticsTrendPointDto>,
    pub category_breakdown: Vec<ProfitBreakdownDto>,
    pub customer_breakdown: Vec<ProfitBreakdownDto>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductRankingRequest {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub category: Option<String>,
    pub rank_by: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProductRankingRowDto {
    pub product_id: i64,
    pub product_name: String,
    pub category: String,
    pub order_count: i64,
    pub sales_quantity: f64,
    pub sales_amount: f64,
    pub cost_amount: f64,
    pub profit_amount: f64,
    pub gift_quantity: f64,
    pub gift_cost_amount: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerAnalysisRequest {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub category: Option<String>,
    pub rank_by: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerAnalysisRowDto {
    pub customer_id: i64,
    pub customer_name: String,
    pub region: Option<String>,
    pub order_count: i64,
    pub sales_amount: f64,
    pub cost_amount: f64,
    pub profit_amount: f64,
    pub balance_amount: f64,
    pub recent_order_date: Option<String>,
    pub average_repurchase_days: Option<f64>,
    pub favorite_products: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerAnalysisDto {
    pub rows: Vec<CustomerAnalysisRowDto>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfitFilterRequest {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub customer_id: Option<i64>,
    pub category: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListSuppliersRequest {
    pub keyword: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupplierDto {
    pub id: i64,
    pub name: String,
    pub contact: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub is_active: bool,
    pub remark: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupplierPayload {
    pub name: String,
    pub contact: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub remark: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SupplierPurchaseLedgerRequest {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub supplier_id: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupplierPurchaseSummaryDto {
    pub supplier_id: Option<i64>,
    pub supplier_name: String,
    pub inbound_count: i64,
    pub inbound_amount: f64,
    pub recent_inbound_date: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupplierPurchaseTrendPointDto {
    pub period: String,
    pub inbound_count: i64,
    pub inbound_amount: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SupplierPurchaseLedgerDto {
    pub summaries: Vec<SupplierPurchaseSummaryDto>,
    pub details: Vec<InboundRecordDto>,
    pub monthly_trend: Vec<SupplierPurchaseTrendPointDto>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerBalanceFilterRequest {
    pub region: Option<String>,
    pub keyword: Option<String>,
    pub only_unpaid: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerBalanceDto {
    pub customer_id: i64,
    pub customer_name: String,
    pub region: Option<String>,
    pub total_payable: f64,
    pub total_paid: f64,
    pub balance: f64,
    pub last_order_date: Option<String>,
    pub last_payment_date: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentFilterRequest {
    pub customer_id: Option<i64>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreatePaymentRequest {
    pub payment_date: String,
    pub customer_id: i64,
    pub amount: f64,
    pub method: Option<String>,
    pub related_order_id: Option<i64>,
    pub remark: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentRecordDto {
    pub id: i64,
    pub payment_date: String,
    pub customer_id: i64,
    pub customer_name: String,
    pub amount: f64,
    pub method: Option<String>,
    pub related_order_id: Option<i64>,
    pub status: String,
    pub remark: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CustomerStatementRequest {
    pub customer_id: i64,
    pub start_date: String,
    pub end_date: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerStatementSummaryDto {
    pub customer_id: i64,
    pub customer_name: String,
    pub start_date: String,
    pub end_date: String,
    pub opening_balance: f64,
    pub period_payable: f64,
    pub period_paid: f64,
    pub period_discount_amount: f64,
    pub closing_balance: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerStatementRowDto {
    pub record_date: String,
    pub record_type: String,
    pub record_no: String,
    pub description: String,
    pub debit_amount: f64,
    pub credit_amount: f64,
    pub balance_after: f64,
    pub remark: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CustomerStatementDto {
    pub summary: CustomerStatementSummaryDto,
    pub rows: Vec<CustomerStatementRowDto>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryReportRequest {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub category: Option<String>,
    pub keyword: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryReportRowDto {
    pub product_id: i64,
    pub product_name: String,
    pub category: String,
    pub barcode: Option<String>,
    pub inbound_quantity: f64,
    pub inbound_amount: f64,
    pub outbound_quantity: f64,
    pub outbound_amount: f64,
    pub gift_quantity: f64,
    pub current_stock: f64,
    pub avg_cost: f64,
    pub stock_value: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentDto {
    pub id: i64,
    pub order_id: i64,
    pub order_no: String,
    pub customer_id: i64,
    pub customer_name: String,
    pub file_path: String,
    pub file_type: String,
    pub printed_at: Option<String>,
    pub print_count: i64,
    pub created_at: String,
    pub status: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentFilterRequest {
    pub customer_id: Option<i64>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub order_no: Option<String>,
    pub printed: Option<bool>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    pub product_count: i64,
    pub customer_count: i64,
    pub movement_count: i64,
    pub profit_count: i64,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupStatusDto {
    pub completed: bool,
    pub merchant_name: String,
    pub industry_template: String,
    pub product_count: i64,
    pub customer_count: i64,
    pub order_count: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MerchantProfileDto {
    pub name: String,
    pub contact: Option<String>,
    pub phone: Option<String>,
    pub address: Option<String>,
    pub logo_path: Option<String>,
    pub remark: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TermSettingsDto {
    pub customer: String,
    pub region: String,
    pub product: String,
    pub category: String,
    pub rule: String,
    pub credit: String,
    pub guest_customer: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct FeatureFlagsDto {
    pub supplier_ledger: bool,
    pub customer_rules: bool,
    pub monthly_credit: bool,
    pub receivables: bool,
    pub product_ranking: bool,
    pub customer_analysis: bool,
    pub inventory_control: bool,
    pub diagnostics: bool,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct IndustryTemplateDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub terms: TermSettingsDto,
    pub features: FeatureFlagsDto,
    pub order_template: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DocumentTemplateDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub template_type: String,
    pub is_default: bool,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CompleteSetupRequest {
    pub merchant: MerchantProfileDto,
    pub terms: Option<TermSettingsDto>,
    pub features: Option<FeatureFlagsDto>,
    pub industry_template: Option<String>,
    pub default_print_template: Option<String>,
    pub default_export_format: Option<String>,
    pub default_printer: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ApplyIndustryTemplateRequest {
    pub template_id: String,
    pub overwrite_terms: Option<bool>,
    pub overwrite_features: Option<bool>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GenericImportRequest {
    pub import_type: String,
    pub file_path: String,
    pub duplicate_strategy: Option<String>,
    pub field_mapping: Option<HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GenericImportHeaderRequest {
    pub import_type: String,
    pub file_path: String,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GenericImportReportRequest {
    pub title: String,
    pub rows: Vec<GenericImportRowDto>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ImportMappingDto {
    pub name: String,
    pub import_type: String,
    pub field_mapping: HashMap<String, String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GenericImportFieldDto {
    pub name: String,
    pub required: bool,
    pub aliases: Vec<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GenericImportHeadersDto {
    pub import_type: String,
    pub sheet_name: String,
    pub headers: Vec<String>,
    pub fields: Vec<GenericImportFieldDto>,
    pub suggested_mapping: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GenericImportRowDto {
    pub row_number: i64,
    pub action: String,
    pub status: String,
    pub message: Option<String>,
    pub name: Option<String>,
    pub category: Option<String>,
    pub region: Option<String>,
    pub barcode: Option<String>,
    pub default_price: Option<f64>,
    pub safety_stock: Option<f64>,
    pub unit: Option<String>,
    pub address: Option<String>,
    pub phone: Option<String>,
    pub quantity: Option<f64>,
    pub unit_price: Option<f64>,
    pub remark: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GenericImportPreviewDto {
    pub import_type: String,
    pub total_count: i64,
    pub valid_count: i64,
    pub create_count: i64,
    pub overwrite_count: i64,
    pub error_count: i64,
    pub skipped_count: i64,
    pub rows: Vec<GenericImportRowDto>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GenericImportResultDto {
    pub import_type: String,
    pub imported_count: i64,
    pub create_count: i64,
    pub overwrite_count: i64,
    pub error_count: i64,
    pub skipped_count: i64,
    pub rows: Vec<GenericImportRowDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupDto {
    pub id: i64,
    pub backup_path: String,
    pub backup_type: String,
    pub status: String,
    pub message: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreBackupResultDto {
    pub restored_backup_path: String,
    pub pre_restore_backup_path: String,
    pub message: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataSelfCheckIssueDto {
    pub check_code: String,
    pub severity: String,
    pub target_type: String,
    pub target_id: Option<i64>,
    pub target_label: String,
    pub message: String,
    pub details: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataSelfCheckDto {
    pub checked_at: String,
    pub issue_count: i64,
    pub inventory_checked: i64,
    pub orders_checked: i64,
    pub credits_checked: i64,
    pub documents_checked: i64,
    pub issues: Vec<DataSelfCheckIssueDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticSummaryDto {
    pub generated_at: String,
    pub database_path: String,
    pub logs_dir: String,
    pub backups_dir: String,
    pub exports_dir: String,
    pub version: String,
    pub database_size: i64,
    pub backup_count: i64,
    pub latest_backup_at: Option<String>,
    pub product_count: i64,
    pub customer_count: i64,
    pub order_count: i64,
    pub document_count: i64,
    pub setting_count: i64,
    pub latest_logs: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticPackageDto {
    pub file_path: String,
    pub message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateInventoryAdjustmentRequest {
    pub adjustment_date: String,
    pub product_id: i64,
    pub adjustment_type: String,
    pub quantity_delta: f64,
    pub reason: String,
    pub remark: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InventoryAdjustmentFilterRequest {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub product_id: Option<i64>,
    pub category: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryAdjustmentDto {
    pub id: i64,
    pub adjustment_date: String,
    pub product_id: i64,
    pub product_name: String,
    pub category: String,
    pub adjustment_type: String,
    pub quantity_delta: f64,
    pub unit_cost: f64,
    pub amount: f64,
    pub reason: String,
    pub remark: Option<String>,
    pub status: String,
    pub void_reason: Option<String>,
    pub voided_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateStocktakeRequest {
    pub stocktake_date: String,
    pub product_id: i64,
    pub actual_stock: f64,
    pub reason: String,
    pub remark: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StocktakeFilterRequest {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub product_id: Option<i64>,
    pub category: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StocktakeRecordDto {
    pub id: i64,
    pub stocktake_date: String,
    pub product_id: i64,
    pub product_name: String,
    pub category: String,
    pub system_stock: f64,
    pub actual_stock: f64,
    pub difference_quantity: f64,
    pub unit_cost: f64,
    pub difference_amount: f64,
    pub reason: String,
    pub remark: Option<String>,
    pub status: String,
    pub void_reason: Option<String>,
    pub voided_at: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoidRecordRequest {
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogFilterRequest {
    pub module: Option<String>,
    pub action: Option<String>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditLogDto {
    pub id: i64,
    pub log_time: String,
    pub module: String,
    pub action: String,
    pub target_type: Option<String>,
    pub target_id: Option<i64>,
    pub target_label: Option<String>,
    pub result: String,
    pub message: Option<String>,
    pub details: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingDto {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveSettingsRequest {
    pub daily_auto_backup: Option<bool>,
    pub default_print_template: Option<String>,
    pub default_export_format: Option<String>,
    pub default_printer: Option<String>,
    pub template_store_name: Option<String>,
    pub template_footer_text: Option<String>,
    pub template_show_barcode: Option<bool>,
    pub template_product_label: Option<String>,
    pub template_quantity_label: Option<String>,
    pub template_price_label: Option<String>,
    pub template_amount_label: Option<String>,
    pub template_remark_label: Option<String>,
    pub template_orientation: Option<String>,
    pub template_margin: Option<f64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportDataRequest {
    pub export_type: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub customer_id: Option<i64>,
    pub category: Option<String>,
    pub rank_by: Option<String>,
    pub status: Option<String>,
    pub keyword: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintRequest {
    pub printer_name: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PrintStatusDto {
    pub file_path: String,
    pub printer_name: Option<String>,
    pub message: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoidOrderRequest {
    pub reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientLogRequest {
    pub level: String,
    pub module: Option<String>,
    pub message: String,
    pub details: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppStatusDto {
    pub database_path: String,
    pub data_dir: String,
    pub backups_dir: String,
    pub orders_dir: String,
    pub exports_dir: String,
    pub logs_dir: String,
    pub version: String,
}
