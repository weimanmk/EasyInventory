use serde::{Deserialize, Serialize};

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

#[derive(Debug, Deserialize)]
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
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportDataRequest {
    pub export_type: String,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub customer_id: Option<i64>,
    pub category: Option<String>,
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
