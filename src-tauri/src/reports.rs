use crate::app::AppState;
use crate::models::{
    CustomerAnalysisRequest, CustomerStatementDto, CustomerStatementRequest, ExportDataRequest,
    InventoryReportRequest, MonthlyCreditFilterRequest, ProductRankingRequest, ProfitFilterRequest,
};
use crate::services::customer_statement_service;
use crate::utils::{money, now_text, safe_file_name};
use anyhow::anyhow;
use rusqlite::{params, params_from_iter, types::Value, Connection};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use umya_spreadsheet::{
    self, writer::xlsx, Border, Coordinate, HorizontalAlignmentValues, OrientationValues,
    Selection, SheetView, SheetViews, Style, VerticalAlignmentValues,
};

const ORDER_TEMPLATE_LAST_COLUMN: u32 = 11;
const ORDER_TEMPLATE_LAST_ROW: u32 = 21;
const ORDER_TEMPLATE_DETAIL_FIRST_ROW: u32 = 6;
const ORDER_TEMPLATE_DETAIL_LAST_ROW: u32 = 20;
const ORDER_TEMPLATE_TOTAL_ROW: u32 = 21;

type ExportRows = Vec<Vec<String>>;
type ExportTable = (&'static str, Vec<&'static str>, ExportRows);

#[derive(Debug, Clone)]
struct OrderTemplateSettings {
    store_name: String,
    footer_text: Option<String>,
    show_barcode: bool,
    product_label: String,
    quantity_label: String,
    price_label: String,
    amount_label: String,
    remark_label: String,
    orientation: String,
    margin: f64,
}

impl Default for OrderTemplateSettings {
    fn default() -> Self {
        Self {
            store_name: "我的商行".to_string(),
            footer_text: None,
            show_barcode: true,
            product_label: "商品名称".to_string(),
            quantity_label: "数量".to_string(),
            price_label: "价格".to_string(),
            amount_label: "总价格".to_string(),
            remark_label: "备注".to_string(),
            orientation: "portrait".to_string(),
            margin: 0.0,
        }
    }
}

fn order_template_settings(conn: &Connection) -> anyhow::Result<OrderTemplateSettings> {
    let default = OrderTemplateSettings::default();
    Ok(OrderTemplateSettings {
        store_name: setting_text(conn, "template_store_name", &default.store_name)?,
        footer_text: setting_optional_text(conn, "template_footer_text")?,
        show_barcode: setting_bool(conn, "template_show_barcode", default.show_barcode)?,
        product_label: setting_text(conn, "template_product_label", &default.product_label)?,
        quantity_label: setting_text(conn, "template_quantity_label", &default.quantity_label)?,
        price_label: setting_text(conn, "template_price_label", &default.price_label)?,
        amount_label: setting_text(conn, "template_amount_label", &default.amount_label)?,
        remark_label: setting_text(conn, "template_remark_label", &default.remark_label)?,
        orientation: setting_text(conn, "template_orientation", &default.orientation)?,
        margin: setting_float(conn, "template_margin", default.margin)?.clamp(0.0, 2.0),
    })
}

fn setting_text(conn: &Connection, key: &str, default: &str) -> anyhow::Result<String> {
    Ok(crate::db::setting(conn, key)?
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_string()))
}

fn setting_optional_text(conn: &Connection, key: &str) -> anyhow::Result<Option<String>> {
    Ok(crate::db::setting(conn, key)?
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty()))
}

fn setting_bool(conn: &Connection, key: &str, default: bool) -> anyhow::Result<bool> {
    Ok(crate::db::setting(conn, key)?
        .map(|value| value == "true")
        .unwrap_or(default))
}

fn setting_float(conn: &Connection, key: &str, default: f64) -> anyhow::Result<f64> {
    Ok(crate::db::setting(conn, key)?
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(default))
}

pub fn export_order_document(state: &AppState, order_id: i64) -> anyhow::Result<String> {
    let conn = state.connection()?;
    let detail = crate::services::order_service::get_order_detail(&conn, order_id)?;
    let template = order_template_settings(&conn)?;
    let customer_folder = state
        .orders_dir()
        .join(safe_file_name(&detail.order.customer_name));
    std::fs::create_dir_all(&customer_folder)?;
    let file_path = customer_folder.join(format!(
        "{}_{}.xlsx",
        detail.order.order_no,
        safe_file_name(&detail.order.customer_name)
    ));
    write_order_workbook_with_template(&file_path, &detail, &template)?;
    let now = now_text();
    conn.execute(
        "INSERT INTO documents (order_id, order_no, customer_id, customer_name, file_path, file_type, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'xlsx', ?6)
         ON CONFLICT DO NOTHING",
        params![
            detail.order.id,
            detail.order.order_no,
            detail.order.customer_id,
            detail.order.customer_name,
            file_path.to_string_lossy().to_string(),
            now
        ],
    )?;
    conn.execute(
        "UPDATE orders SET document_path = ?1, updated_at = ?2 WHERE id = ?3",
        params![file_path.to_string_lossy().to_string(), now, order_id],
    )?;
    Ok(file_path.to_string_lossy().to_string())
}

pub fn export_order_pdf_document(state: &AppState, order_id: i64) -> anyhow::Result<String> {
    let conn = state.connection()?;
    let detail = crate::services::order_service::get_order_detail(&conn, order_id)?;
    let template = order_template_settings(&conn)?;
    let customer_folder = state
        .orders_dir()
        .join(safe_file_name(&detail.order.customer_name));
    std::fs::create_dir_all(&customer_folder)?;
    let file_path = customer_folder.join(format!(
        "{}_{}.pdf",
        detail.order.order_no,
        safe_file_name(&detail.order.customer_name)
    ));
    write_order_pdf(&file_path, &detail, &template)?;
    let now = now_text();
    conn.execute(
        "INSERT INTO documents (order_id, order_no, customer_id, customer_name, file_path, file_type, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, 'pdf', ?6)",
        params![
            detail.order.id,
            detail.order.order_no,
            detail.order.customer_id,
            detail.order.customer_name,
            file_path.to_string_lossy().to_string(),
            now
        ],
    )?;
    Ok(file_path.to_string_lossy().to_string())
}

pub fn export_customer_statement_pdf_document(
    state: &AppState,
    request: CustomerStatementRequest,
) -> anyhow::Result<String> {
    let conn = state.connection()?;
    let statement = customer_statement_service::customer_statement(&conn, request)?;
    std::fs::create_dir_all(state.exports_dir())?;
    let file_path = state.exports_dir().join(format!(
        "客户对账单_{}_{}_{}.pdf",
        safe_file_name(&statement.summary.customer_name),
        statement.summary.start_date,
        statement.summary.end_date
    ));
    write_customer_statement_pdf(&file_path, &statement)?;
    Ok(file_path.to_string_lossy().to_string())
}

pub fn export_data(state: &AppState, request: ExportDataRequest) -> anyhow::Result<String> {
    let conn = state.connection()?;
    let (title, headers, rows) = export_data_table(&conn, &request)?;
    let stamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
    let file_path = state
        .exports_dir()
        .join(format!("{}_{}.xlsx", safe_file_name(title), stamp));
    write_table_workbook(&file_path, title, &headers, &rows)?;
    Ok(file_path.to_string_lossy().to_string())
}

fn export_data_table(
    conn: &Connection,
    request: &ExportDataRequest,
) -> anyhow::Result<ExportTable> {
    match request.export_type.as_str() {
        "products" => export_products(conn, request),
        "customers" => export_customers(conn, request),
        "inbounds" => export_inbounds(conn, request),
        "inventory_report" => export_inventory_report(conn, request),
        "product_ranking" => export_product_ranking(conn, request),
        "customer_analysis" => export_customer_analysis(conn, request),
        "customer_statement" => export_customer_statement(conn, request),
        "monthly_credits" => export_monthly_credits(conn, request),
        "profits" => export_profits(conn, request),
        _ => Err(anyhow!("不支持的导出类型：{}", request.export_type)),
    }
}

pub fn list_system_printers() -> anyhow::Result<Vec<String>> {
    if cfg!(target_os = "windows") {
        let mut command = Command::new("powershell");
        command.args([
            "-NoProfile",
            "-Command",
            "Get-Printer | Select-Object -ExpandProperty Name",
        ]);
        #[cfg(target_os = "windows")]
        command.creation_flags(0x08000000);
        let output = command.output();
        if let Ok(output) = output {
            if output.status.success() {
                let text = String::from_utf8_lossy(&output.stdout);
                let printers = text
                    .lines()
                    .map(str::trim)
                    .filter(|line| !line.is_empty())
                    .map(ToString::to_string)
                    .collect::<Vec<_>>();
                if !printers.is_empty() {
                    return Ok(printers);
                }
            }
        }
    }
    Ok(Vec::new())
}

fn export_products(conn: &Connection, request: &ExportDataRequest) -> anyhow::Result<ExportTable> {
    let mut sql = String::from(
        "SELECT p.name, p.category, COALESCE(p.barcode, ''), COALESCE(p.unit, ''),
                p.default_price, p.safety_stock, COALESCE(s.current_stock, 0),
                COALESCE(s.avg_cost, 0), COALESCE(s.stock_value, 0), p.is_active
         FROM products p
         LEFT JOIN stock_balances s ON s.product_id = p.id
         WHERE 1 = 1",
    );
    let mut sql_params: Vec<Value> = Vec::new();
    if let Some(category) = request.category.as_ref().filter(|value| !value.is_empty()) {
        sql.push_str(" AND p.category = ?");
        sql_params.push(Value::Text(category.to_string()));
    }
    if let Some(keyword) = request.keyword.as_ref().filter(|value| !value.is_empty()) {
        let keyword = format!("%{keyword}%");
        sql.push_str(" AND (p.name LIKE ? OR p.barcode LIKE ?)");
        sql_params.push(Value::Text(keyword.clone()));
        sql_params.push(Value::Text(keyword));
    }
    sql.push_str(" ORDER BY p.category, p.name");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(params_from_iter(sql_params.iter()), |row| {
            Ok(vec![
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                money(row.get::<_, f64>(4)?).to_string(),
                money(row.get::<_, f64>(5)?).to_string(),
                money(row.get::<_, f64>(6)?).to_string(),
                money(row.get::<_, f64>(7)?).to_string(),
                money(row.get::<_, f64>(8)?).to_string(),
                if row.get::<_, i64>(9)? == 1 {
                    "启用"
                } else {
                    "停用"
                }
                .to_string(),
            ])
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok((
        "商品资料",
        vec![
            "商品名称",
            "类别",
            "条码",
            "单位",
            "默认售价",
            "安全库存",
            "当前库存",
            "平均进货价",
            "库存价值",
            "状态",
        ],
        rows,
    ))
}

fn export_customers(conn: &Connection, request: &ExportDataRequest) -> anyhow::Result<ExportTable> {
    let mut sql = String::from(
        "SELECT COALESCE(region, ''), name, COALESCE(address, ''), COALESCE(phone, ''), is_active
         FROM customers WHERE 1 = 1",
    );
    let mut sql_params: Vec<Value> = Vec::new();
    if let Some(keyword) = request.keyword.as_ref().filter(|value| !value.is_empty()) {
        let keyword = format!("%{keyword}%");
        sql.push_str(" AND (name LIKE ? OR address LIKE ?)");
        sql_params.push(Value::Text(keyword.clone()));
        sql_params.push(Value::Text(keyword));
    }
    sql.push_str(" ORDER BY region, name");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(params_from_iter(sql_params.iter()), |row| {
            Ok(vec![
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                if row.get::<_, i64>(4)? == 1 {
                    "启用"
                } else {
                    "停用"
                }
                .to_string(),
            ])
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok((
        "客户资料",
        vec!["地区", "客户名称", "地址", "电话", "状态"],
        rows,
    ))
}

fn export_inbounds(conn: &Connection, request: &ExportDataRequest) -> anyhow::Result<ExportTable> {
    let mut sql = String::from(
        "SELECT i.inbound_date, p.category, p.name, i.quantity, i.unit_cost, i.amount, COALESCE(i.remark, '')
         FROM inbound_records i
         JOIN products p ON p.id = i.product_id
         WHERE 1 = 1",
    );
    let mut sql_params: Vec<Value> = Vec::new();
    append_date_filters(&mut sql, &mut sql_params, "i.inbound_date", request);
    if let Some(category) = request.category.as_ref().filter(|value| !value.is_empty()) {
        sql.push_str(" AND p.category = ?");
        sql_params.push(Value::Text(category.to_string()));
    }
    sql.push_str(" ORDER BY i.inbound_date DESC, i.id DESC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(params_from_iter(sql_params.iter()), |row| {
            Ok(vec![
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                money(row.get::<_, f64>(3)?).to_string(),
                money(row.get::<_, f64>(4)?).to_string(),
                money(row.get::<_, f64>(5)?).to_string(),
                row.get::<_, String>(6)?,
            ])
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok((
        "入库记录",
        vec!["日期", "类别", "商品", "数量", "进货价", "金额", "备注"],
        rows,
    ))
}

fn export_inventory_report(
    conn: &Connection,
    request: &ExportDataRequest,
) -> anyhow::Result<ExportTable> {
    let rows = crate::services::report_service::list_inventory_report(
        conn,
        InventoryReportRequest {
            start_date: request.start_date.clone(),
            end_date: request.end_date.clone(),
            category: request.category.clone(),
            keyword: request.keyword.clone(),
        },
    )?
    .into_iter()
    .map(|row| {
        vec![
            row.category,
            row.product_name,
            row.barcode.unwrap_or_default(),
            money(row.inbound_quantity).to_string(),
            money(row.inbound_amount).to_string(),
            money(row.outbound_quantity).to_string(),
            money(row.outbound_amount).to_string(),
            money(row.gift_quantity).to_string(),
            money(row.current_stock).to_string(),
            money(row.avg_cost).to_string(),
            money(row.stock_value).to_string(),
        ]
    })
    .collect::<Vec<_>>();
    Ok((
        "进销存报表",
        vec![
            "类别",
            "商品",
            "条码",
            "入库数量",
            "入库金额",
            "销售数量",
            "销售金额",
            "赠品数量",
            "当前库存",
            "平均成本",
            "库存价值",
        ],
        rows,
    ))
}

fn export_product_ranking(
    conn: &Connection,
    request: &ExportDataRequest,
) -> anyhow::Result<ExportTable> {
    let rows = crate::services::analytics_service::product_ranking(
        conn,
        ProductRankingRequest {
            start_date: request.start_date.clone(),
            end_date: request.end_date.clone(),
            category: request.category.clone(),
            rank_by: request.rank_by.clone(),
            limit: Some(100),
        },
    )?
    .into_iter()
    .map(|row| {
        vec![
            row.category,
            row.product_name,
            row.order_count.to_string(),
            money(row.sales_quantity).to_string(),
            money(row.sales_amount).to_string(),
            money(row.cost_amount).to_string(),
            money(row.profit_amount).to_string(),
            money(row.gift_quantity).to_string(),
            money(row.gift_cost_amount).to_string(),
        ]
    })
    .collect::<Vec<_>>();
    Ok((
        "商品经营排行",
        vec![
            "类别",
            "商品",
            "订单数",
            "销量",
            "销售额",
            "成本",
            "利润",
            "赠品数量",
            "赠品成本",
        ],
        rows,
    ))
}

fn export_customer_analysis(
    conn: &Connection,
    request: &ExportDataRequest,
) -> anyhow::Result<ExportTable> {
    let rows = crate::services::analytics_service::customer_analysis(
        conn,
        CustomerAnalysisRequest {
            start_date: request.start_date.clone(),
            end_date: request.end_date.clone(),
            category: request.category.clone(),
            rank_by: request.rank_by.clone(),
            limit: Some(100),
        },
    )?
    .rows
    .into_iter()
    .map(|row| {
        vec![
            row.region.unwrap_or_default(),
            row.customer_name,
            row.order_count.to_string(),
            money(row.sales_amount).to_string(),
            money(row.cost_amount).to_string(),
            money(row.profit_amount).to_string(),
            money(row.balance_amount).to_string(),
            row.recent_order_date.unwrap_or_default(),
            row.average_repurchase_days
                .map(|value| format!("{value:.2}"))
                .unwrap_or_default(),
            row.favorite_products,
        ]
    })
    .collect::<Vec<_>>();
    Ok((
        "客户经营分析",
        vec![
            "地区",
            "客户",
            "订单数",
            "销售额",
            "成本",
            "利润",
            "当前欠款",
            "最近购买日期",
            "平均复购间隔(天)",
            "偏好商品",
        ],
        rows,
    ))
}

fn export_customer_statement(
    conn: &Connection,
    request: &ExportDataRequest,
) -> anyhow::Result<ExportTable> {
    let customer_id = request
        .customer_id
        .ok_or_else(|| anyhow!("导出客户对账单必须选择客户"))?;
    let start_date = request
        .start_date
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("导出客户对账单必须选择开始日期"))?;
    let end_date = request
        .end_date
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("导出客户对账单必须选择结束日期"))?;
    let statement = customer_statement_service::customer_statement(
        conn,
        CustomerStatementRequest {
            customer_id,
            start_date,
            end_date,
        },
    )?;
    let summary = statement.summary;
    let mut rows = vec![vec![
        summary.start_date.clone(),
        "期初".to_string(),
        String::new(),
        "期初欠款".to_string(),
        String::new(),
        String::new(),
        money(summary.opening_balance).to_string(),
        summary.customer_name.clone(),
    ]];
    rows.extend(statement.rows.into_iter().map(|row| {
        vec![
            row.record_date,
            match row.record_type.as_str() {
                "order" => "出库",
                "payment" => "收款",
                _ => row.record_type.as_str(),
            }
            .to_string(),
            row.record_no,
            row.description,
            money(row.debit_amount).to_string(),
            money(row.credit_amount).to_string(),
            money(row.balance_after).to_string(),
            row.remark.unwrap_or_default(),
        ]
    }));
    if summary.period_discount_amount > 0.0 {
        rows.push(vec![
            summary.end_date.clone(),
            "优惠".to_string(),
            String::new(),
            "本期优惠".to_string(),
            format!("-{}", money(summary.period_discount_amount)),
            String::new(),
            String::new(),
            "折现和月费抵扣合计".to_string(),
        ]);
    }
    rows.push(vec![
        summary.end_date.clone(),
        "汇总".to_string(),
        String::new(),
        "本期应收/收款".to_string(),
        money(summary.period_payable).to_string(),
        money(summary.period_paid).to_string(),
        String::new(),
        String::new(),
    ]);
    rows.push(vec![
        summary.end_date,
        "期末".to_string(),
        String::new(),
        "期末余额".to_string(),
        String::new(),
        String::new(),
        money(summary.closing_balance).to_string(),
        String::new(),
    ]);
    Ok((
        "客户对账单",
        vec![
            "日期", "类型", "单号", "说明", "应收", "收款", "余额", "备注",
        ],
        rows,
    ))
}

fn export_monthly_credits(
    conn: &Connection,
    request: &ExportDataRequest,
) -> anyhow::Result<ExportTable> {
    let rows = crate::services::order_service::list_monthly_credits(
        conn,
        MonthlyCreditFilterRequest {
            customer_id: request.customer_id,
            category: request.category.clone(),
            status: request.status.clone(),
            start_date: request.start_date.clone(),
            end_date: request.end_date.clone(),
            available_month: None,
        },
    )?
    .into_iter()
    .map(|row| {
        vec![
            row.source_order_no,
            row.customer_name,
            row.category,
            money(row.amount).to_string(),
            money(row.used_amount).to_string(),
            money(row.remaining_amount).to_string(),
            row.generated_date,
            row.available_month,
            row.status,
            row.remark.unwrap_or_default(),
        ]
    })
    .collect::<Vec<_>>();
    Ok((
        "月费账本",
        vec![
            "来源订单",
            "客户",
            "类别",
            "生成金额",
            "已使用",
            "剩余",
            "生成日期",
            "可用月份",
            "状态",
            "备注",
        ],
        rows,
    ))
}

fn export_profits(conn: &Connection, request: &ExportDataRequest) -> anyhow::Result<ExportTable> {
    let rows = crate::services::profit_service::list_profit_records(
        conn,
        ProfitFilterRequest {
            start_date: request.start_date.clone(),
            end_date: request.end_date.clone(),
            customer_id: request.customer_id,
            category: request.category.clone(),
        },
    )?
    .into_iter()
    .map(|row| {
        vec![
            row.order_date,
            row.order_no,
            row.customer_name,
            money(row.totals.product_sales_amount).to_string(),
            money(row.totals.customer_payable_amount).to_string(),
            money(row.totals.direct_discount_amount).to_string(),
            money(row.totals.monthly_credit_used).to_string(),
            money(row.totals.cost_amount).to_string(),
            money(row.totals.profit_amount).to_string(),
        ]
    })
    .collect::<Vec<_>>();
    Ok((
        "利润报表",
        vec![
            "日期",
            "单号",
            "客户",
            "销售额",
            "实收",
            "折现",
            "月费抵扣",
            "成本",
            "利润",
        ],
        rows,
    ))
}

fn append_date_filters(
    sql: &mut String,
    sql_params: &mut Vec<Value>,
    column: &str,
    request: &ExportDataRequest,
) {
    if let Some(start) = request
        .start_date
        .as_ref()
        .filter(|value| !value.is_empty())
    {
        sql.push_str(&format!(" AND {column} >= ?"));
        sql_params.push(Value::Text(start.to_string()));
    }
    if let Some(end) = request.end_date.as_ref().filter(|value| !value.is_empty()) {
        sql.push_str(&format!(" AND {column} <= ?"));
        sql_params.push(Value::Text(end.to_string()));
    }
}

fn write_table_workbook(
    path: &PathBuf,
    title: &str,
    headers: &[&str],
    rows: &[Vec<String>],
) -> anyhow::Result<()> {
    let mut book = umya_spreadsheet::new_file();
    let sheet = book.get_sheet_by_name_mut("Sheet1").unwrap();
    sheet.set_name(title);
    for (index, header) in headers.iter().enumerate() {
        let address = cell_address((index + 1) as u32, 1);
        sheet.get_cell_mut(address).set_value(*header);
        let column = column_name((index + 1) as u32);
        sheet.get_column_dimension_mut(&column).set_width(18.0);
    }
    for (row_index, row) in rows.iter().enumerate() {
        for (column_index, value) in row.iter().enumerate() {
            let address = cell_address((column_index + 1) as u32, (row_index + 2) as u32);
            sheet.get_cell_mut(address).set_value(value);
        }
    }
    xlsx::write(&book, path)?;
    Ok(())
}

#[cfg(test)]
fn write_order_workbook(
    path: &PathBuf,
    detail: &crate::models::OrderDetailDto,
) -> anyhow::Result<()> {
    write_order_workbook_with_template(path, detail, &OrderTemplateSettings::default())
}

fn write_order_workbook_with_template(
    path: &PathBuf,
    detail: &crate::models::OrderDetailDto,
    template: &OrderTemplateSettings,
) -> anyhow::Result<()> {
    let mut book = umya_spreadsheet::new_file();
    let sheet = book.get_sheet_by_name_mut("Sheet1").unwrap();
    sheet.set_name("单据");

    apply_order_template_layout(sheet, template);
    write_order_template_values(sheet, detail, template);

    xlsx::write(&book, path)?;
    Ok(())
}

fn apply_order_template_layout(
    sheet: &mut umya_spreadsheet::Worksheet,
    template: &OrderTemplateSettings,
) {
    sheet.set_active_cell("A1");
    set_order_template_view(sheet);
    for column in 1..=ORDER_TEMPLATE_LAST_COLUMN {
        sheet
            .get_column_dimension_mut(&column_name(column))
            .set_width(13.0);
    }

    for range in order_template_merge_ranges() {
        sheet.add_merge_cells(range);
    }

    sheet
        .get_page_setup_mut()
        .set_orientation(if template.orientation == "landscape" {
            OrientationValues::Landscape
        } else {
            OrientationValues::Portrait
        })
        .set_paper_size(9)
        .set_fit_to_width(1)
        .set_fit_to_height(1)
        .set_horizontal_dpi(300)
        .set_vertical_dpi(300);
    sheet
        .get_page_margins_mut()
        .set_left(template.margin)
        .set_right(template.margin)
        .set_top(template.margin)
        .set_bottom(template.margin)
        .set_header(0.0)
        .set_footer(0.0);
    let _ = sheet.add_defined_name("_xlnm.Print_Area", "'单据'!$A$1:$K$21");

    for row in 1..=ORDER_TEMPLATE_LAST_ROW {
        for column in 1..=ORDER_TEMPLATE_LAST_COLUMN {
            let address = cell_address(column, row);
            sheet.set_style(address, template_cell_style(column, row));
        }
    }
}

fn write_order_template_values(
    sheet: &mut umya_spreadsheet::Worksheet,
    detail: &crate::models::OrderDetailDto,
    template: &OrderTemplateSettings,
) {
    sheet.get_cell_mut("A1").set_value(&template.store_name);
    sheet.get_cell_mut("A4").set_value("客户:");
    sheet
        .get_cell_mut("B4")
        .set_value(&detail.order.customer_name);
    sheet.get_cell_mut("D4").set_value("地址：");
    sheet
        .get_cell_mut("E4")
        .set_value(detail.order.customer_address.clone().unwrap_or_default());
    sheet.get_cell_mut("G4").set_value(&detail.order.order_no);
    sheet.get_cell_mut("I4").set_value(&detail.order.order_date);

    for (address, value) in [
        ("A5", "序号"),
        ("B5", if template.show_barcode { "条码" } else { "" }),
        ("D5", template.product_label.as_str()),
        ("E5", "单位"),
        ("F5", template.quantity_label.as_str()),
        ("G5", template.price_label.as_str()),
        ("H5", template.amount_label.as_str()),
        ("I5", template.remark_label.as_str()),
    ] {
        sheet.get_cell_mut(address).set_value(value);
    }

    let max_items = (ORDER_TEMPLATE_DETAIL_LAST_ROW - ORDER_TEMPLATE_DETAIL_FIRST_ROW + 1) as usize;
    for (index, item) in detail.items.iter().take(max_items).enumerate() {
        let row = ORDER_TEMPLATE_DETAIL_FIRST_ROW + index as u32;
        sheet
            .get_cell_mut(format!("A{row}"))
            .set_value_number((index + 1) as f64);
        sheet
            .get_cell_mut(format!("B{row}"))
            .set_value(if template.show_barcode {
                item.barcode.clone().unwrap_or_default()
            } else {
                String::new()
            });
        sheet.get_cell_mut(format!("D{row}")).set_value(
            item.product_name
                .clone()
                .unwrap_or_else(|| item.line_type.clone()),
        );
        sheet.get_cell_mut(format!("E{row}")).set_value("件");
        sheet
            .get_cell_mut(format!("F{row}"))
            .set_value_number(item.quantity);
        sheet
            .get_cell_mut(format!("G{row}"))
            .set_value_number(item.unit_price);
        sheet
            .get_cell_mut(format!("H{row}"))
            .set_value_number(item.amount);
        sheet
            .get_cell_mut(format!("I{row}"))
            .set_value(item.remark.clone().unwrap_or_default());
    }

    let quantity_total = detail
        .items
        .iter()
        .take(max_items)
        .map(|item| item.quantity)
        .sum::<f64>();
    let amount_total = detail
        .items
        .iter()
        .take(max_items)
        .map(|item| item.amount)
        .sum::<f64>();

    sheet
        .get_cell_mut(format!("A{ORDER_TEMPLATE_TOTAL_ROW}"))
        .set_value("总金额");
    sheet
        .get_cell_mut(format!("C{ORDER_TEMPLATE_TOTAL_ROW}"))
        .set_value_number(money(detail.order.totals.customer_payable_amount));
    sheet
        .get_cell_mut(format!("F{ORDER_TEMPLATE_TOTAL_ROW}"))
        .set_value_number(quantity_total);
    sheet
        .get_cell_mut(format!("G{ORDER_TEMPLATE_TOTAL_ROW}"))
        .set_value_number(amount_total);

    if let Some(remark) = detail
        .order
        .remark
        .as_ref()
        .or(template.footer_text.as_ref())
    {
        sheet
            .get_cell_mut(format!("I{ORDER_TEMPLATE_TOTAL_ROW}"))
            .set_value(remark);
    }
}

fn write_order_pdf(
    path: &PathBuf,
    detail: &crate::models::OrderDetailDto,
    template: &OrderTemplateSettings,
) -> anyhow::Result<()> {
    let mut lines = vec![
        (50.0, 800.0, 18.0, template.store_name.clone()),
        (
            50.0,
            770.0,
            11.0,
            format!(
                "客户：{}    地址：{}    单号：{}    日期：{}",
                detail.order.customer_name,
                detail.order.customer_address.clone().unwrap_or_default(),
                detail.order.order_no,
                detail.order.order_date
            ),
        ),
        (
            50.0,
            738.0,
            10.0,
            format!(
                "{}    {}    单位    {}    {}    {}    {}",
                if template.show_barcode { "条码" } else { "" },
                template.product_label,
                template.quantity_label,
                template.price_label,
                template.amount_label,
                template.remark_label
            ),
        ),
    ];
    let mut y = 716.0;
    for (index, item) in detail.items.iter().take(18).enumerate() {
        lines.push((
            50.0,
            y,
            10.0,
            format!(
                "{}. {}    {}    件    {}    {}    {}    {}",
                index + 1,
                if template.show_barcode {
                    item.barcode.clone().unwrap_or_default()
                } else {
                    String::new()
                },
                item.product_name
                    .clone()
                    .unwrap_or_else(|| item.line_type.clone()),
                money(item.quantity),
                money(item.unit_price),
                money(item.amount),
                item.remark.clone().unwrap_or_default()
            ),
        ));
        y -= 22.0;
    }
    lines.push((
        50.0,
        120.0,
        12.0,
        format!(
            "总金额：{}    数量：{}",
            money(detail.order.totals.customer_payable_amount),
            money(detail.items.iter().map(|item| item.quantity).sum::<f64>())
        ),
    ));
    if let Some(remark) = detail
        .order
        .remark
        .as_ref()
        .or(template.footer_text.as_ref())
    {
        lines.push((50.0, 96.0, 10.0, remark.clone()));
    }

    write_text_pdf(path, lines)
}

fn write_customer_statement_pdf(
    path: &PathBuf,
    statement: &CustomerStatementDto,
) -> anyhow::Result<()> {
    let summary = &statement.summary;
    let mut lines = vec![
        (50.0, 800.0, 18.0, "客户对账单".to_string()),
        (
            50.0,
            770.0,
            11.0,
            format!(
                "客户：{}    日期：{} 至 {}",
                summary.customer_name, summary.start_date, summary.end_date
            ),
        ),
        (
            50.0,
            742.0,
            10.0,
            format!(
                "期初欠款：{}    本期应收：{}    本期收款：{}    本期优惠：{}    期末余额：{}",
                money(summary.opening_balance),
                money(summary.period_payable),
                money(summary.period_paid),
                money(summary.period_discount_amount),
                money(summary.closing_balance)
            ),
        ),
        (
            50.0,
            708.0,
            10.0,
            "日期    类型    单号    说明    应收    收款    余额    备注".to_string(),
        ),
    ];
    let mut y = 686.0;
    for row in statement.rows.iter().take(22) {
        lines.push((
            50.0,
            y,
            9.0,
            format!(
                "{}    {}    {}    {}    {}    {}    {}    {}",
                row.record_date,
                match row.record_type.as_str() {
                    "order" => "出库",
                    "payment" => "收款",
                    _ => row.record_type.as_str(),
                },
                row.record_no,
                row.description,
                money(row.debit_amount),
                money(row.credit_amount),
                money(row.balance_after),
                row.remark.clone().unwrap_or_default()
            ),
        ));
        y -= 22.0;
    }
    if statement.rows.len() > 22 {
        lines.push((
            50.0,
            y,
            9.0,
            format!(
                "还有 {} 条明细未显示，请以 Excel 对账单为准",
                statement.rows.len() - 22
            ),
        ));
    }

    write_text_pdf(path, lines)
}

fn write_text_pdf(path: &PathBuf, lines: Vec<(f64, f64, f64, String)>) -> anyhow::Result<()> {
    let mut content = String::new();
    for (x, y, size, text) in lines {
        content.push_str(&format!(
            "BT /F1 {size} Tf {x} {y} Td <{}> Tj ET\n",
            pdf_utf16_hex(&text)
        ));
    }

    let objects = [
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        b"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 595 842] /Resources << /Font << /F1 4 0 R >> >> /Contents 6 0 R >>".to_vec(),
        b"<< /Type /Font /Subtype /Type0 /BaseFont /STSong-Light /Encoding /UniGB-UCS2-H /DescendantFonts [5 0 R] >>".to_vec(),
        b"<< /Type /Font /Subtype /CIDFontType0 /BaseFont /STSong-Light /CIDSystemInfo << /Registry (Adobe) /Ordering (GB1) /Supplement 5 >> >>".to_vec(),
        format!(
            "<< /Length {} >>\nstream\n{}endstream",
            content.len(),
            content
        )
        .into_bytes(),
    ];

    let mut pdf = Vec::new();
    pdf.extend_from_slice(b"%PDF-1.4\n");
    let mut offsets = vec![0usize];
    for (index, object) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        pdf.extend_from_slice(format!("{} 0 obj\n", index + 1).as_bytes());
        pdf.extend_from_slice(object);
        pdf.extend_from_slice(b"\nendobj\n");
    }
    let xref_start = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", offsets.len()).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_start}\n%%EOF\n",
            offsets.len()
        )
        .as_bytes(),
    );
    std::fs::write(path, pdf)?;
    Ok(())
}

fn pdf_utf16_hex(text: &str) -> String {
    text.encode_utf16()
        .map(|value| format!("{value:04X}"))
        .collect::<Vec<_>>()
        .join("")
}

fn template_cell_style(column: u32, row: u32) -> Style {
    let mut style = base_template_style();

    if row == 1 {
        style.get_font_mut().get_font_size_mut().set_val(20.0);
        style.get_font_mut().set_bold(true);
    }

    if row == 4 && (column == 1 || column == 4) {
        style
            .get_alignment_mut()
            .set_horizontal(HorizontalAlignmentValues::Right);
    }

    if (ORDER_TEMPLATE_DETAIL_FIRST_ROW..=ORDER_TEMPLATE_DETAIL_LAST_ROW).contains(&row)
        && (column == 7 || column == 8)
    {
        style
            .get_numbering_format_mut()
            .set_format_code("\\¥#,##0.00;[Red]\\¥\\-#,##0.00");
    }
    if row == ORDER_TEMPLATE_TOTAL_ROW && column == 3 {
        style
            .get_numbering_format_mut()
            .set_format_code("[DBNum2][$RMB]General;[Red][DBNum2][$RMB]General");
    }
    if row == ORDER_TEMPLATE_TOTAL_ROW && column == 7 {
        style
            .get_numbering_format_mut()
            .set_format_code("\\¥#,##0.00_);[Red]\\(\\¥#,##0.00\\)");
    }
    if row == 4 && column == 9 {
        style.get_numbering_format_mut().set_format_code("mm-dd-yy");
    }

    apply_template_borders(&mut style, column, row);
    style
}

fn base_template_style() -> Style {
    let mut style = Style::default();
    style.get_font_mut().get_font_name_mut().set_val("宋体");
    style.get_font_mut().get_font_size_mut().set_val(11.0);
    style
        .get_alignment_mut()
        .set_horizontal(HorizontalAlignmentValues::Center);
    style
        .get_alignment_mut()
        .set_vertical(VerticalAlignmentValues::Center);
    style
}

fn apply_template_borders(style: &mut Style, column: u32, row: u32) {
    if (1..=3).contains(&row) {
        if row == 3 {
            set_bottom_border(style);
        }
        return;
    }

    if (4..=ORDER_TEMPLATE_TOTAL_ROW).contains(&row) && column <= ORDER_TEMPLATE_LAST_COLUMN {
        set_bottom_border(style);
        set_left_border(style);
        set_right_border(style);
        set_top_border(style);
    }
}

fn order_template_merge_ranges() -> Vec<String> {
    let mut ranges = vec![
        "A1:K3".to_string(),
        "E4:F4".to_string(),
        "G4:H4".to_string(),
        "I4:K4".to_string(),
    ];
    for row in 5..=ORDER_TEMPLATE_DETAIL_LAST_ROW {
        ranges.push(format!("B{row}:C{row}"));
        ranges.push(format!("I{row}:K{row}"));
    }
    ranges.extend([
        "A21:B21".to_string(),
        "C21:E21".to_string(),
        "G21:H21".to_string(),
        "I21:K21".to_string(),
    ]);
    ranges
}

fn set_order_template_view(sheet: &mut umya_spreadsheet::Worksheet) {
    let mut coordinate = Coordinate::default();
    coordinate.set_coordinate("A1");

    let mut selection = Selection::default();
    selection.set_active_cell(coordinate);
    selection.get_sequence_of_references_mut().set_sqref("A1");

    let mut view = SheetView::default();
    view.set_workbook_view_id(0)
        .set_top_left_cell("A1")
        .set_selection(selection);

    let mut views = SheetViews::default();
    views.add_sheet_view_list_mut(view);
    sheet.set_sheets_views(views);
}

fn set_left_border(style: &mut Style) {
    style
        .get_borders_mut()
        .get_left_mut()
        .set_border_style(Border::BORDER_THIN);
}

fn set_right_border(style: &mut Style) {
    style
        .get_borders_mut()
        .get_right_mut()
        .set_border_style(Border::BORDER_THIN);
}

fn set_top_border(style: &mut Style) {
    style
        .get_borders_mut()
        .get_top_mut()
        .set_border_style(Border::BORDER_THIN);
}

fn set_bottom_border(style: &mut Style) {
    style
        .get_borders_mut()
        .get_bottom_mut()
        .set_border_style(Border::BORDER_THIN);
}

fn cell_address(column: u32, row: u32) -> String {
    format!("{}{}", column_name(column), row)
}

fn column_name(mut column: u32) -> String {
    let mut name = String::new();
    while column > 0 {
        let rem = ((column - 1) % 26) as u8;
        name.insert(0, (b'A' + rem) as char);
        column = (column - 1) / 26;
    }
    name
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::models::{
        CustomerAnalysisRequest, CustomerStatementRequest, CustomerStatementRowDto,
        CustomerStatementSummaryDto, OrderDetailDto, OrderDto, OrderItemDto, OrderTotalsDto,
        ProductRankingRequest, ProfitAnalyticsRequest, SupplierPurchaseLedgerRequest,
    };
    use std::time::{Duration, Instant};

    #[test]
    fn order_workbook_matches_source_print_template() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("order.xlsx");
        let detail = sample_order_detail();

        write_order_workbook(&path, &detail).unwrap();

        let book = umya_spreadsheet::reader::xlsx::read(&path).unwrap();
        let sheet = book.get_sheet_by_name("单据").unwrap();
        let merge_ranges = sheet
            .get_merge_cells()
            .iter()
            .map(|range| range.get_range())
            .collect::<Vec<_>>();

        assert_eq!(sheet.get_highest_column(), ORDER_TEMPLATE_LAST_COLUMN);
        assert_eq!(sheet.get_highest_row(), ORDER_TEMPLATE_LAST_ROW);
        assert!(merge_ranges.contains(&"A1:K3".to_string()));
        assert!(merge_ranges.contains(&"B6:C6".to_string()));
        assert!(merge_ranges.contains(&"C21:E21".to_string()));
        assert!(merge_ranges.contains(&"I21:K21".to_string()));
        let view = sheet
            .get_sheets_views()
            .get_sheet_view_list()
            .first()
            .expect("sheet view should be written");
        let selection = view
            .get_selection()
            .first()
            .expect("sheet selection should be written");
        assert_eq!(view.get_top_left_cell(), "A1");
        assert_eq!(
            selection
                .get_active_cell()
                .map(|cell| cell.get_coordinate()),
            Some("A1".to_string())
        );
        assert_eq!(selection.get_sequence_of_references().get_sqref(), "A1");
        assert_eq!(*sheet.get_page_setup().get_paper_size(), 9);
        assert_eq!(*sheet.get_page_setup().get_fit_to_width(), 1);
        assert_eq!(*sheet.get_page_setup().get_fit_to_height(), 1);
        assert_eq!(
            sheet
                .get_defined_names()
                .iter()
                .find(|name| name.get_name() == "_xlnm.Print_Area")
                .map(|name| name.get_address()),
            Some("'单据'!$A$1:$K$21".to_string())
        );
        assert!(matches!(
            sheet.get_page_setup().get_orientation(),
            OrientationValues::Portrait
        ));
        assert_eq!(sheet.get_value("A1"), "我的商行");
        assert_eq!(sheet.get_value("D5"), "商品名称");
        assert_eq!(sheet.get_value("A21"), "总金额");
    }

    #[test]
    fn order_template_settings_apply_to_workbook() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("custom-order.xlsx");
        let mut detail = sample_order_detail();
        detail.order.remark = None;
        let template = OrderTemplateSettings {
            store_name: "测试门店".to_string(),
            footer_text: Some("默认页脚".to_string()),
            show_barcode: false,
            product_label: "品名".to_string(),
            quantity_label: "件数".to_string(),
            price_label: "单价".to_string(),
            amount_label: "金额".to_string(),
            remark_label: "说明".to_string(),
            orientation: "landscape".to_string(),
            margin: 0.25,
        };

        write_order_workbook_with_template(&path, &detail, &template).unwrap();

        let book = umya_spreadsheet::reader::xlsx::read(&path).unwrap();
        let sheet = book.get_sheet_by_name("单据").unwrap();
        assert_eq!(sheet.get_value("A1"), "测试门店");
        assert_eq!(sheet.get_value("B5"), "");
        assert_eq!(sheet.get_value("D5"), "品名");
        assert_eq!(sheet.get_value("F5"), "件数");
        assert_eq!(sheet.get_value("G5"), "单价");
        assert_eq!(sheet.get_value("H5"), "金额");
        assert_eq!(sheet.get_value("I5"), "说明");
        assert_eq!(sheet.get_value("I21"), "默认页脚");
        assert!(matches!(
            sheet.get_page_setup().get_orientation(),
            OrientationValues::Landscape
        ));
    }

    #[test]
    fn order_pdf_document_writes_valid_pdf() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("order.pdf");
        let detail = sample_order_detail();

        write_order_pdf(&path, &detail, &OrderTemplateSettings::default()).unwrap();

        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.starts_with(b"%PDF-1.4"));
        assert!(bytes.ends_with(b"%%EOF\n"));
        assert!(bytes.windows(7).any(|window| window == b"/Type /"));
    }

    #[test]
    fn customer_statement_pdf_document_writes_valid_pdf() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("statement.pdf");
        let statement = CustomerStatementDto {
            summary: CustomerStatementSummaryDto {
                customer_id: 1,
                customer_name: "客户A".to_string(),
                start_date: "2026-06-01".to_string(),
                end_date: "2026-06-30".to_string(),
                opening_balance: 100.0,
                period_payable: 200.0,
                period_paid: 50.0,
                period_discount_amount: 10.0,
                closing_balance: 250.0,
            },
            rows: vec![
                CustomerStatementRowDto {
                    record_date: "2026-06-02".to_string(),
                    record_type: "order".to_string(),
                    record_no: "20260602001".to_string(),
                    description: "出库单".to_string(),
                    debit_amount: 200.0,
                    credit_amount: 0.0,
                    balance_after: 300.0,
                    remark: Some("本期出库".to_string()),
                },
                CustomerStatementRowDto {
                    record_date: "2026-06-03".to_string(),
                    record_type: "payment".to_string(),
                    record_no: "PAY000001".to_string(),
                    description: "现金".to_string(),
                    debit_amount: 0.0,
                    credit_amount: 50.0,
                    balance_after: 250.0,
                    remark: Some("客户付款".to_string()),
                },
            ],
        };

        write_customer_statement_pdf(&path, &statement).unwrap();

        let bytes = std::fs::read(&path).unwrap();
        let content = String::from_utf8_lossy(&bytes);
        assert!(bytes.starts_with(b"%PDF-1.4"));
        assert!(bytes.ends_with(b"%%EOF\n"));
        assert!(content.contains(&pdf_utf16_hex("客户对账单")));
        assert!(content.contains(&pdf_utf16_hex("客户A")));
        assert!(content.contains(&pdf_utf16_hex("期末余额：250")));
    }

    #[test]
    fn list_profit_records_filters_by_order_item_category() {
        let conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        insert_profit_order(&conn, "20260601001", "饮料");
        insert_profit_order(&conn, "20260601002", "零食");

        let rows = crate::services::profit_service::list_profit_records(
            &conn,
            ProfitFilterRequest {
                start_date: Some("2026-06-01".to_string()),
                end_date: Some("2026-06-01".to_string()),
                customer_id: None,
                category: Some("饮料".to_string()),
            },
        )
        .unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].order_no, "20260601001");
    }

    #[test]
    fn profit_analytics_groups_daily_monthly_and_yearly() {
        let conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        insert_profit_analytics_order(
            &conn,
            "20250531001",
            "2025-05-31",
            1,
            "客户A",
            &[("饮料", 12.0, 5.0, 7.0)],
            "normal",
        );
        insert_profit_analytics_order(
            &conn,
            "20250601001",
            "2025-06-01",
            1,
            "客户A",
            &[("饮料", 20.0, 12.0, 8.0)],
            "normal",
        );
        insert_profit_analytics_order(
            &conn,
            "20260601001",
            "2026-06-01",
            1,
            "客户A",
            &[("饮料", 10.0, 6.0, 4.0)],
            "normal",
        );
        insert_profit_analytics_order(
            &conn,
            "20260602001",
            "2026-06-02",
            2,
            "客户B",
            &[("零食", 30.0, 10.0, 20.0)],
            "normal",
        );
        insert_profit_analytics_order(
            &conn,
            "20260602002",
            "2026-06-02",
            1,
            "客户A",
            &[("饮料", 99.0, 20.0, 79.0)],
            "voided",
        );

        let daily = crate::services::profit_service::get_profit_analytics(
            &conn,
            ProfitAnalyticsRequest {
                period: "day".to_string(),
                start_date: "2026-06-01".to_string(),
                end_date: "2026-06-30".to_string(),
                customer_id: None,
                category: None,
            },
        )
        .unwrap();

        assert_eq!(daily.summary.order_count, 2);
        assert_eq!(daily.summary.product_sales_amount, 40.0);
        assert_eq!(daily.summary.cost_amount, 16.0);
        assert_eq!(daily.summary.profit_amount, 24.0);
        assert_eq!(daily.trend.len(), 2);
        assert_eq!(daily.trend[0].period, "2026-06-01");
        assert_eq!(daily.trend[0].profit_amount, 4.0);
        assert_eq!(
            daily.trend[0].comparison_period.as_deref(),
            Some("2026-05-31")
        );
        assert_eq!(daily.trend[0].profit_change_amount, Some(4.0));
        assert_eq!(daily.trend[0].profit_change_rate, None);
        assert_eq!(daily.trend[1].period, "2026-06-02");
        assert_eq!(daily.trend[1].profit_amount, 20.0);
        assert_eq!(
            daily.trend[1].comparison_period.as_deref(),
            Some("2026-06-01")
        );
        assert_eq!(daily.trend[1].sales_change_amount, Some(20.0));
        assert_eq!(daily.trend[1].sales_change_rate, Some(200.0));
        assert_eq!(daily.trend[1].profit_change_amount, Some(16.0));
        assert_eq!(daily.trend[1].profit_change_rate, Some(400.0));

        let monthly = crate::services::profit_service::get_profit_analytics(
            &conn,
            ProfitAnalyticsRequest {
                period: "month".to_string(),
                start_date: "2026-01-01".to_string(),
                end_date: "2026-12-31".to_string(),
                customer_id: None,
                category: None,
            },
        )
        .unwrap();
        assert_eq!(monthly.trend.len(), 1);
        assert_eq!(monthly.trend[0].period, "2026-06");
        assert_eq!(monthly.trend[0].profit_amount, 24.0);
        assert_eq!(
            monthly.trend[0].comparison_period.as_deref(),
            Some("2025-06")
        );
        assert_eq!(monthly.trend[0].profit_change_amount, Some(16.0));
        assert_eq!(monthly.trend[0].profit_change_rate, Some(200.0));

        let yearly = crate::services::profit_service::get_profit_analytics(
            &conn,
            ProfitAnalyticsRequest {
                period: "year".to_string(),
                start_date: "2025-01-01".to_string(),
                end_date: "2026-12-31".to_string(),
                customer_id: None,
                category: None,
            },
        )
        .unwrap();
        assert_eq!(yearly.trend.len(), 2);
        assert_eq!(yearly.trend[0].period, "2025");
        assert_eq!(yearly.trend[0].profit_amount, 15.0);
        assert_eq!(yearly.trend[1].period, "2026");
        assert_eq!(yearly.trend[1].profit_amount, 24.0);
        assert_eq!(yearly.trend[1].comparison_period.as_deref(), Some("2025"));
        assert_eq!(yearly.trend[1].profit_change_amount, Some(9.0));
        assert_eq!(yearly.trend[1].profit_change_rate, Some(60.0));
    }

    #[test]
    fn profit_analytics_breaks_down_categories_from_order_items() {
        let conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        insert_profit_analytics_order(
            &conn,
            "20260601001",
            "2026-06-01",
            1,
            "客户A",
            &[("饮料", 10.0, 6.0, 4.0), ("零食", 30.0, 10.0, 20.0)],
            "normal",
        );

        let analytics = crate::services::profit_service::get_profit_analytics(
            &conn,
            ProfitAnalyticsRequest {
                period: "day".to_string(),
                start_date: "2026-06-01".to_string(),
                end_date: "2026-06-30".to_string(),
                customer_id: None,
                category: None,
            },
        )
        .unwrap();

        assert_eq!(analytics.category_breakdown.len(), 2);
        assert_eq!(analytics.category_breakdown[0].name, "零食");
        assert_eq!(analytics.category_breakdown[0].product_sales_amount, 30.0);
        assert_eq!(analytics.category_breakdown[0].profit_amount, 20.0);
        assert_eq!(analytics.category_breakdown[1].name, "饮料");
        assert_eq!(analytics.category_breakdown[1].product_sales_amount, 10.0);
        assert_eq!(analytics.category_breakdown[1].profit_amount, 4.0);
    }

    #[test]
    fn profit_analytics_filters_by_customer_and_category() {
        let conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        insert_profit_analytics_order(
            &conn,
            "20260601001",
            "2026-06-01",
            1,
            "客户A",
            &[("饮料", 10.0, 6.0, 4.0)],
            "normal",
        );
        insert_profit_analytics_order(
            &conn,
            "20260601002",
            "2026-06-01",
            2,
            "客户B",
            &[("饮料", 20.0, 7.0, 13.0)],
            "normal",
        );
        insert_profit_analytics_order(
            &conn,
            "20260601003",
            "2026-06-01",
            1,
            "客户A",
            &[("零食", 30.0, 10.0, 20.0)],
            "normal",
        );

        let analytics = crate::services::profit_service::get_profit_analytics(
            &conn,
            ProfitAnalyticsRequest {
                period: "day".to_string(),
                start_date: "2026-06-01".to_string(),
                end_date: "2026-06-30".to_string(),
                customer_id: Some(1),
                category: Some("饮料".to_string()),
            },
        )
        .unwrap();

        assert_eq!(analytics.summary.order_count, 1);
        assert_eq!(analytics.summary.product_sales_amount, 10.0);
        assert_eq!(analytics.customer_breakdown.len(), 1);
        assert_eq!(analytics.customer_breakdown[0].name, "客户A");
        assert_eq!(analytics.customer_breakdown[0].profit_amount, 4.0);
    }

    #[test]
    fn product_ranking_summarizes_sales_profit_and_gift_cost() {
        let conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        let now = now_text();
        conn.execute(
            "INSERT INTO products
             (id, name, category, default_price, safety_stock, is_active, created_at, updated_at)
             VALUES
             (1, '可乐', '饮料', 4, 0, 1, ?1, ?1),
             (2, '赠品杯', '赠品', 0, 0, 1, ?1, ?1),
             (3, '薯片', '零食', 6, 0, 1, ?1, ?1)",
            params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO customers (id, region, name, is_active, created_at, updated_at)
             VALUES (1, '测试', '客户A', 1, ?1, ?1)",
            params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO orders
             (order_no, order_date, customer_id, customer_name, status, created_at, updated_at)
             VALUES
             ('20260601001', '2026-06-01', 1, '客户A', 'normal', ?1, ?1),
             ('20260602001', '2026-06-02', 1, '客户A', 'normal', ?1, ?1),
             ('20260603001', '2026-06-03', 1, '客户A', 'voided', ?1, ?1)",
            params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO order_items
             (order_id, line_type, product_id, product_name, category, quantity, unit_price,
              amount, avg_cost, cost_amount, profit_amount, sort_order)
             VALUES
             (1, 'normal', 1, '可乐', '饮料', 5, 4, 20, 2, 10, 10, 1),
             (1, 'gift', 2, '赠品杯', '赠品', 2, 0, 0, 3, 6, -6, 2),
             (2, 'normal', 1, '可乐', '饮料', 3, 5, 15, 2, 6, 9, 1),
             (2, 'normal', 3, '薯片', '零食', 4, 6, 24, 4, 16, 8, 2),
             (3, 'normal', 1, '可乐', '饮料', 100, 5, 500, 2, 200, 300, 1)",
            [],
        )
        .unwrap();

        let sales = crate::services::analytics_service::product_ranking(
            &conn,
            ProductRankingRequest {
                start_date: Some("2026-06-01".to_string()),
                end_date: Some("2026-06-30".to_string()),
                category: None,
                rank_by: Some("sales_quantity".to_string()),
                limit: Some(10),
            },
        )
        .unwrap();

        assert_eq!(sales[0].product_name, "可乐");
        assert_eq!(sales[0].sales_quantity, 8.0);
        assert_eq!(sales[0].sales_amount, 35.0);
        assert_eq!(sales[0].profit_amount, 19.0);
        assert_eq!(sales[0].order_count, 2);
        assert_eq!(sales[1].product_name, "薯片");
        assert_eq!(sales[2].product_name, "赠品杯");
        assert_eq!(sales[2].gift_quantity, 2.0);
        assert_eq!(sales[2].gift_cost_amount, 6.0);
        assert_eq!(sales[2].profit_amount, -6.0);

        let gift_cost = crate::services::analytics_service::product_ranking(
            &conn,
            ProductRankingRequest {
                start_date: Some("2026-06-01".to_string()),
                end_date: Some("2026-06-30".to_string()),
                category: Some("赠品".to_string()),
                rank_by: Some("gift_cost_amount".to_string()),
                limit: Some(10),
            },
        )
        .unwrap();

        assert_eq!(gift_cost.len(), 1);
        assert_eq!(gift_cost[0].product_name, "赠品杯");
        assert_eq!(gift_cost[0].gift_cost_amount, 6.0);
    }

    #[test]
    fn customer_analysis_ranks_sales_profit_balance_and_preferences() {
        let conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        let now = now_text();
        conn.execute(
            "INSERT INTO customers (id, region, name, is_active, created_at, updated_at)
             VALUES
             (1, '城东', '客户A', 1, ?1, ?1),
             (2, '城西', '客户B', 1, ?1, ?1),
             (3, '城南', '客户C', 1, ?1, ?1)",
            params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO products
             (id, name, category, default_price, safety_stock, is_active, created_at, updated_at)
             VALUES
             (1, '可乐', '饮料', 5, 0, 1, ?1, ?1),
             (2, '薯片', '零食', 6, 0, 1, ?1, ?1)",
            params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO orders
             (order_no, order_date, customer_id, customer_name, product_sales_amount,
              customer_payable_amount, cost_amount, profit_amount, status, created_at, updated_at)
             VALUES
             ('20260520001', '2026-05-20', 3, '客户C', 200, 200, 120, 80, 'normal', ?1, ?1),
             ('20260601001', '2026-06-01', 1, '客户A', 50, 50, 30, 20, 'normal', ?1, ?1),
             ('20260602001', '2026-06-02', 2, '客户B', 100, 100, 70, 30, 'normal', ?1, ?1),
             ('20260611001', '2026-06-11', 1, '客户A', 30, 30, 12, 18, 'normal', ?1, ?1),
             ('20260612001', '2026-06-12', 1, '客户A', 999, 999, 100, 899, 'voided', ?1, ?1)",
            params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO order_items
             (order_id, line_type, product_id, product_name, category, quantity, unit_price,
              amount, avg_cost, cost_amount, profit_amount, sort_order)
             VALUES
             (1, 'normal', 1, '可乐', '饮料', 20, 10, 200, 6, 120, 80, 1),
             (2, 'normal', 1, '可乐', '饮料', 5, 10, 50, 6, 30, 20, 1),
             (3, 'normal', 2, '薯片', '零食', 10, 10, 100, 7, 70, 30, 1),
             (4, 'normal', 1, '可乐', '饮料', 3, 10, 30, 4, 12, 18, 1),
             (5, 'normal', 1, '可乐', '饮料', 99, 10, 999, 1, 100, 899, 1)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO payment_records
             (payment_date, customer_id, amount, status, created_at, updated_at)
             VALUES
             ('2026-06-03', 1, 20, 'normal', ?1, ?1),
             ('2026-06-04', 2, 100, 'normal', ?1, ?1)",
            params![now],
        )
        .unwrap();

        let sales = crate::services::analytics_service::customer_analysis(
            &conn,
            CustomerAnalysisRequest {
                start_date: Some("2026-06-01".to_string()),
                end_date: Some("2026-06-30".to_string()),
                category: None,
                rank_by: Some("sales_amount".to_string()),
                limit: Some(10),
            },
        )
        .unwrap();

        assert_eq!(sales.rows[0].customer_name, "客户B");
        assert_eq!(sales.rows[0].sales_amount, 100.0);
        assert_eq!(sales.rows[1].customer_name, "客户A");
        assert_eq!(sales.rows[1].sales_amount, 80.0);
        assert_eq!(sales.rows[1].profit_amount, 38.0);
        assert_eq!(sales.rows[1].balance_amount, 60.0);
        assert_eq!(
            sales.rows[1].recent_order_date.as_deref(),
            Some("2026-06-11")
        );
        assert_eq!(sales.rows[1].average_repurchase_days, Some(10.0));
        assert!(sales.rows[1].favorite_products.contains("可乐"));

        let balance = crate::services::analytics_service::customer_analysis(
            &conn,
            CustomerAnalysisRequest {
                start_date: Some("2026-06-01".to_string()),
                end_date: Some("2026-06-30".to_string()),
                category: None,
                rank_by: Some("balance_amount".to_string()),
                limit: Some(10),
            },
        )
        .unwrap();

        assert_eq!(balance.rows[0].customer_name, "客户C");
        assert_eq!(balance.rows[0].balance_amount, 200.0);
    }

    #[test]
    fn customer_statement_computes_opening_and_ignores_voided_records() {
        let conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        let now = now_text();
        conn.execute(
            "INSERT INTO customers (region, name, is_active, created_at, updated_at)
             VALUES ('对账', '客户A', 1, ?1, ?1)",
            rusqlite::params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO orders
             (order_no, order_date, customer_id, customer_name, product_sales_amount,
              direct_discount_amount, monthly_credit_used, customer_payable_amount, status, created_at, updated_at)
             VALUES
             ('20260531001', '2026-05-31', 1, '客户A', 100, 0, 0, 100, 'normal', ?1, ?1),
             ('20260531002', '2026-05-31', 1, '客户A', 999, 0, 0, 999, 'voided', ?1, ?1),
             ('20260601001', '2026-06-01', 1, '客户A', 100, 30, 10, 60, 'normal', ?1, ?1)",
            rusqlite::params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO payment_records
             (payment_date, customer_id, amount, status, remark, created_at, updated_at)
             VALUES
             ('2026-05-31', 1, 40, 'normal', '期初前收款', ?1, ?1),
             ('2026-06-02', 1, 20, 'normal', '本期收款', ?1, ?1),
             ('2026-06-03', 1, 100, 'voided', '作废收款', ?1, ?1)",
            rusqlite::params![now],
        )
        .unwrap();

        let statement = crate::services::customer_statement_service::customer_statement(
            &conn,
            CustomerStatementRequest {
                customer_id: 1,
                start_date: "2026-06-01".to_string(),
                end_date: "2026-06-30".to_string(),
            },
        )
        .unwrap();

        assert_eq!(statement.summary.customer_name, "客户A");
        assert_eq!(statement.summary.opening_balance, 60.0);
        assert_eq!(statement.summary.period_payable, 60.0);
        assert_eq!(statement.summary.period_paid, 20.0);
        assert_eq!(statement.summary.period_discount_amount, 40.0);
        assert_eq!(statement.summary.closing_balance, 100.0);
        assert_eq!(statement.rows.len(), 2);
        assert_eq!(statement.rows[0].record_type, "order");
        assert_eq!(statement.rows[0].debit_amount, 60.0);
        assert_eq!(statement.rows[0].balance_after, 120.0);
        assert_eq!(statement.rows[1].record_type, "payment");
        assert_eq!(statement.rows[1].credit_amount, 20.0);
        assert_eq!(statement.rows[1].balance_after, 100.0);
    }

    #[test]
    fn export_customer_statement_outputs_opening_rows_and_summary() {
        let conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        let now = now_text();
        conn.execute(
            "INSERT INTO customers (region, name, is_active, created_at, updated_at)
             VALUES ('对账', '客户A', 1, ?1, ?1)",
            rusqlite::params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO orders
             (order_no, order_date, customer_id, customer_name, product_sales_amount,
              direct_discount_amount, monthly_credit_used, customer_payable_amount, status, created_at, updated_at)
             VALUES
             ('20260531001', '2026-05-31', 1, '客户A', 100, 0, 0, 100, 'normal', ?1, ?1),
             ('20260601001', '2026-06-01', 1, '客户A', 100, 30, 10, 60, 'normal', ?1, ?1)",
            rusqlite::params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO payment_records
             (payment_date, customer_id, amount, status, remark, created_at, updated_at)
             VALUES ('2026-06-02', 1, 20, 'normal', '本期收款', ?1, ?1)",
            rusqlite::params![now],
        )
        .unwrap();

        let (title, headers, rows) = export_data_table(
            &conn,
            &ExportDataRequest {
                export_type: "customer_statement".to_string(),
                start_date: Some("2026-06-01".to_string()),
                end_date: Some("2026-06-30".to_string()),
                customer_id: Some(1),
                category: None,
                rank_by: None,
                status: None,
                keyword: None,
            },
        )
        .unwrap();

        assert_eq!(title, "客户对账单");
        assert_eq!(headers[0], "日期");
        assert_eq!(rows[0][3], "期初欠款");
        assert_eq!(rows[0][6], "100");
        assert!(rows
            .iter()
            .any(|row| row[3] == "本期优惠" && row[4] == "-40"));
        assert!(rows
            .iter()
            .any(|row| row[3] == "期末余额" && row[6] == "140"));
    }

    #[test]
    fn list_documents_filters_by_customer_and_status() {
        let conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        let now = now_text();
        conn.execute(
            "INSERT INTO customers (region, name, is_active, created_at, updated_at)
             VALUES ('单据', '客户A', 1, ?1, ?1), ('单据', '客户B', 1, ?1, ?1)",
            rusqlite::params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO orders
             (order_no, order_date, customer_id, customer_name, customer_payable_amount, status, created_at, updated_at)
             VALUES
             ('20260601001', '2026-06-01', 1, '客户A', 10, 'normal', ?1, ?1),
             ('20260602001', '2026-06-02', 1, '客户A', 20, 'voided', ?1, ?1),
             ('20260603001', '2026-06-03', 2, '客户B', 30, 'normal', ?1, ?1)",
            rusqlite::params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO documents
             (order_id, order_no, customer_id, customer_name, file_path, file_type, print_count, status, created_at)
             VALUES
             (1, '20260601001', 1, '客户A', 'a.xlsx', 'xlsx', 0, 'normal', ?1),
             (2, '20260602001', 1, '客户A', 'b.xlsx', 'xlsx', 1, 'voided', ?1),
             (3, '20260603001', 2, '客户B', 'c.xlsx', 'xlsx', 0, 'normal', ?1)",
            rusqlite::params![now],
        )
        .unwrap();

        let rows = crate::services::document_service::list_documents(
            &conn,
            crate::models::DocumentFilterRequest {
                customer_id: Some(1),
                start_date: Some("2026-06-01".to_string()),
                end_date: Some("2026-06-30".to_string()),
                order_no: Some("202606".to_string()),
                printed: Some(false),
                status: Some("normal".to_string()),
            },
        )
        .unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].order_no, "20260601001");
        assert_eq!(rows[0].status, "normal");
    }

    #[test]
    fn export_data_rejects_unknown_export_type() {
        let conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();

        let result = export_data_table(
            &conn,
            &ExportDataRequest {
                export_type: "unknown".to_string(),
                start_date: None,
                end_date: None,
                customer_id: None,
                category: None,
                rank_by: None,
                status: None,
                keyword: None,
            },
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("不支持的导出类型"));
    }

    #[test]
    fn inventory_report_summarizes_purchase_sales_and_gifts() {
        let conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        let now = now_text();
        conn.execute(
            "INSERT INTO products (name, category, barcode, default_price, safety_stock, is_active, created_at, updated_at)
             VALUES ('报表商品', '报表类', 'RPT001', 10, 0, 1, ?1, ?1)",
            rusqlite::params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO inventory_movements
             (movement_date, product_id, movement_type, quantity, unit_price, amount, source_type, remark, created_at)
             VALUES
             ('2026-06-01', 1, 'inbound', 20, 5, 100, 'test', '入库', ?1),
             ('2026-06-02', 1, 'outbound', 6, 10, 60, 'test', '销售', ?1),
             ('2026-06-02', 1, 'gift_outbound', 2, 0, 0, 'test', '赠品', ?1)",
            rusqlite::params![now],
        )
        .unwrap();
        db::recalc_stock_balance(&conn, 1).unwrap();

        let rows = crate::services::report_service::list_inventory_report(
            &conn,
            InventoryReportRequest {
                start_date: Some("2026-06-01".to_string()),
                end_date: Some("2026-06-30".to_string()),
                category: Some("报表类".to_string()),
                keyword: Some("RPT001".to_string()),
            },
        )
        .unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].inbound_quantity, 20.0);
        assert_eq!(rows[0].inbound_amount, 100.0);
        assert_eq!(rows[0].outbound_quantity, 6.0);
        assert_eq!(rows[0].outbound_amount, 60.0);
        assert_eq!(rows[0].gift_quantity, 2.0);
        assert_eq!(rows[0].current_stock, 12.0);
    }

    #[test]
    fn supplier_purchase_ledger_summarizes_details_and_monthly_trend() {
        let conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        let now = now_text();
        conn.execute(
            "INSERT INTO products (name, category, default_price, safety_stock, is_active, created_at, updated_at)
             VALUES ('可乐', '饮料', 5, 0, 1, ?1, ?1),
                    ('薯片', '零食', 8, 0, 1, ?1, ?1)",
            params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO suppliers (name, is_active, created_at, updated_at)
             VALUES ('供货商A', 1, ?1, ?1),
                    ('供货商B', 1, ?1, ?1)",
            params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO inbound_records
             (inbound_date, product_id, supplier_id, supplier_name, quantity, unit_cost, amount, remark, created_at)
             VALUES
             ('2025-12-31', 1, 1, '供货商A', 1, 999, 999, '范围外', ?1),
             ('2026-01-05', 1, 1, '供货商A', 2, 10, 20, '一月饮料', ?1),
             ('2026-01-20', 2, 1, '供货商A', 3, 10, 30, '一月零食', ?1),
             ('2026-02-01', 1, 2, '供货商B', 7, 10, 70, '二月饮料', ?1)",
            params![now],
        )
        .unwrap();

        let ledger = crate::services::report_service::supplier_purchase_ledger(
            &conn,
            SupplierPurchaseLedgerRequest {
                start_date: Some("2026-01-01".to_string()),
                end_date: Some("2026-02-28".to_string()),
                supplier_id: None,
            },
        )
        .unwrap();

        assert_eq!(ledger.summaries.len(), 2);
        assert_eq!(ledger.summaries[0].supplier_name, "供货商B");
        assert_eq!(ledger.summaries[0].inbound_count, 1);
        assert_eq!(ledger.summaries[0].inbound_amount, 70.0);
        assert_eq!(ledger.summaries[1].supplier_name, "供货商A");
        assert_eq!(ledger.summaries[1].inbound_count, 2);
        assert_eq!(ledger.summaries[1].inbound_amount, 50.0);
        assert_eq!(
            ledger.summaries[1].recent_inbound_date.as_deref(),
            Some("2026-01-20")
        );
        assert_eq!(ledger.details.len(), 3);
        assert_eq!(ledger.monthly_trend.len(), 2);
        assert_eq!(ledger.monthly_trend[0].period, "2026-01");
        assert_eq!(ledger.monthly_trend[0].inbound_count, 2);
        assert_eq!(ledger.monthly_trend[0].inbound_amount, 50.0);
        assert_eq!(ledger.monthly_trend[1].period, "2026-02");
        assert_eq!(ledger.monthly_trend[1].inbound_amount, 70.0);

        let supplier_a = crate::services::report_service::supplier_purchase_ledger(
            &conn,
            SupplierPurchaseLedgerRequest {
                start_date: Some("2026-01-01".to_string()),
                end_date: Some("2026-02-28".to_string()),
                supplier_id: Some(1),
            },
        )
        .unwrap();

        assert_eq!(supplier_a.summaries.len(), 1);
        assert_eq!(supplier_a.details.len(), 2);
        assert_eq!(supplier_a.monthly_trend.len(), 1);
        assert_eq!(supplier_a.monthly_trend[0].period, "2026-01");
    }

    #[test]
    fn high_volume_inventory_report_stays_under_two_seconds() {
        let mut conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        let tx = conn.transaction().unwrap();
        let now = now_text();
        for index in 0..10_000 {
            tx.execute(
                "INSERT INTO products (name, category, barcode, default_price, safety_stock, is_active, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 10, 0, 1, ?4, ?4)",
                rusqlite::params![
                    format!("报表商品{index:05}"),
                    format!("类别{}", index % 20),
                    format!("RPT{index:05}"),
                    now
                ],
            )
            .unwrap();
            let product_id = tx.last_insert_rowid();
            tx.execute(
                "INSERT INTO stock_balances (product_id, current_stock, avg_cost, stock_value, updated_at)
                 VALUES (?1, 8, 5, 40, ?2)",
                rusqlite::params![product_id, now],
            )
            .unwrap();
            tx.execute(
                "INSERT INTO inventory_movements
                 (movement_date, product_id, movement_type, quantity, unit_price, amount, source_type, remark, created_at)
                 VALUES
                 ('2026-06-01', ?1, 'inbound', 10, 5, 50, 'test', '入库', ?2),
                 ('2026-06-02', ?1, 'outbound', 2, 10, 20, 'test', '出库', ?2),
                 ('2026-06-02', ?1, 'gift_outbound', 1, 0, 0, 'test', '赠品', ?2)",
                rusqlite::params![product_id, now],
            )
            .unwrap();
        }
        tx.commit().unwrap();

        let started = Instant::now();
        let rows = crate::services::report_service::list_inventory_report(
            &conn,
            InventoryReportRequest {
                start_date: Some("2026-06-01".to_string()),
                end_date: Some("2026-06-30".to_string()),
                category: None,
                keyword: None,
            },
        )
        .unwrap();
        let elapsed = started.elapsed();

        assert_eq!(rows.len(), 10_000);
        assert_eq!(rows[0].inbound_quantity, 10.0);
        assert_eq!(rows[0].outbound_quantity, 2.0);
        assert_eq!(rows[0].gift_quantity, 1.0);
        assert!(
            elapsed < Duration::from_secs(2),
            "万级进销存报表查询耗时 {:?}",
            elapsed
        );
    }

    fn insert_profit_order(conn: &Connection, order_no: &str, category: &str) {
        let now = now_text();
        conn.execute(
            "INSERT OR IGNORE INTO customers (id, region, name, is_active, created_at, updated_at)
             VALUES (1, '测试', '测试客户', 1, ?1, ?1)",
            rusqlite::params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO products (name, category, default_price, safety_stock, is_active, created_at, updated_at)
             VALUES (?1, ?2, 10, 0, 1, ?3, ?3)",
            rusqlite::params![format!("{category}商品"), category, now],
        )
        .unwrap();
        let product_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO orders
             (order_no, order_date, customer_id, customer_name, product_sales_amount,
              customer_payable_amount, profit_amount, status, created_at, updated_at)
             VALUES (?1, '2026-06-01', 1, '测试客户', 10, 10, 4, 'normal', ?2, ?2)",
            rusqlite::params![order_no, now],
        )
        .unwrap();
        let order_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO order_items
             (order_id, line_type, product_id, product_name, category, quantity, unit_price,
              amount, avg_cost, cost_amount, profit_amount, sort_order)
             VALUES (?1, 'normal', ?2, ?3, ?4, 1, 10, 10, 6, 6, 4, 1)",
            rusqlite::params![order_id, product_id, format!("{category}商品"), category],
        )
        .unwrap();
    }

    fn insert_profit_analytics_order(
        conn: &Connection,
        order_no: &str,
        order_date: &str,
        customer_id: i64,
        customer_name: &str,
        items: &[(&str, f64, f64, f64)],
        status: &str,
    ) {
        let now = now_text();
        conn.execute(
            "INSERT OR IGNORE INTO customers (id, region, name, is_active, created_at, updated_at)
             VALUES (?1, '统计', ?2, 1, ?3, ?3)",
            rusqlite::params![customer_id, customer_name, now],
        )
        .unwrap();
        let sales_amount = items.iter().map(|item| item.1).sum::<f64>();
        let cost_amount = items.iter().map(|item| item.2).sum::<f64>();
        let profit_amount = items.iter().map(|item| item.3).sum::<f64>();
        conn.execute(
            "INSERT INTO orders
             (order_no, order_date, customer_id, customer_name, product_sales_amount,
              customer_payable_amount, cost_amount, profit_amount, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5, ?6, ?7, ?8, ?9, ?9)",
            rusqlite::params![
                order_no,
                order_date,
                customer_id,
                customer_name,
                sales_amount,
                cost_amount,
                profit_amount,
                status,
                now
            ],
        )
        .unwrap();
        let order_id = conn.last_insert_rowid();
        for (index, (category, amount, cost, profit)) in items.iter().enumerate() {
            conn.execute(
                "INSERT INTO products (name, category, default_price, safety_stock, is_active, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 0, 1, ?4, ?4)",
                rusqlite::params![format!("{category}商品{order_no}{index}"), category, amount, now],
            )
            .unwrap();
            let product_id = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO order_items
                 (order_id, line_type, product_id, product_name, category, quantity, unit_price,
                  amount, avg_cost, cost_amount, profit_amount, sort_order)
                 VALUES (?1, 'normal', ?2, ?3, ?4, 1, ?5, ?5, ?6, ?6, ?7, ?8)",
                rusqlite::params![
                    order_id,
                    product_id,
                    format!("{category}商品{order_no}{index}"),
                    category,
                    amount,
                    cost,
                    profit,
                    index as i64 + 1
                ],
            )
            .unwrap();
        }
    }

    fn sample_order_detail() -> OrderDetailDto {
        OrderDetailDto {
            order: OrderDto {
                id: 1,
                order_no: "20260530001".to_string(),
                order_date: "2026-05-30".to_string(),
                customer_id: 1,
                customer_name: "测试客户".to_string(),
                customer_address: Some("测试地址".to_string()),
                totals: OrderTotalsDto {
                    product_sales_amount: 10.0,
                    customer_payable_amount: 10.0,
                    ..OrderTotalsDto::default()
                },
                remark: Some("测试备注".to_string()),
                document_path: None,
                print_count: 0,
                status: "normal".to_string(),
            },
            items: vec![OrderItemDto {
                id: 1,
                line_type: "normal".to_string(),
                product_id: Some(1),
                product_name: Some("测试商品".to_string()),
                category: Some("测试类别".to_string()),
                barcode: Some("123456".to_string()),
                quantity: 2.0,
                unit_price: 5.0,
                amount: 10.0,
                avg_cost: 3.0,
                cost_amount: 6.0,
                profit_amount: 4.0,
                rule_id: None,
                monthly_credit_id: None,
                remark: Some("行备注".to_string()),
                sort_order: 1,
            }],
        }
    }
}
