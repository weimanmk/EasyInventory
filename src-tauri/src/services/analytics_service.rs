use crate::models::{
    CustomerAnalysisDto, CustomerAnalysisRequest, ProductRankingRequest, ProductRankingRowDto,
};
use crate::repositories::analytics_repository;
use crate::utils::{money, normalize_date};
use anyhow::anyhow;
use chrono::NaiveDate;
use rusqlite::types::Value;

pub fn product_ranking(
    conn: &rusqlite::Connection,
    request: ProductRankingRequest,
) -> anyhow::Result<Vec<ProductRankingRowDto>> {
    let date_range = normalized_date_range(
        request.start_date.as_deref(),
        request.end_date.as_deref(),
        "商品经营排行开始日期不能晚于结束日期",
    )?;
    let mut filters = vec![
        "o.status = 'normal'".to_string(),
        "oi.product_id IS NOT NULL".to_string(),
        "oi.line_type IN ('normal', 'gift')".to_string(),
    ];
    let mut sql_params: Vec<Value> = Vec::new();
    append_order_date_filters(&mut filters, &mut sql_params, &date_range);
    if let Some(category) = active_category(request.category.as_deref()) {
        filters.push("oi.category = ?".to_string());
        sql_params.push(Value::Text(category.to_string()));
    }

    let rank_expr = product_rank_expr(request.rank_by.as_deref())?;
    let limit = request.limit.unwrap_or(20).clamp(1, 100);
    analytics_repository::product_ranking(
        conn,
        &filters.join(" AND "),
        &sql_params,
        rank_expr,
        limit,
    )
}

pub fn customer_analysis(
    conn: &rusqlite::Connection,
    request: CustomerAnalysisRequest,
) -> anyhow::Result<CustomerAnalysisDto> {
    let date_range = normalized_date_range(
        request.start_date.as_deref(),
        request.end_date.as_deref(),
        "客户经营分析开始日期不能晚于结束日期",
    )?;
    let mut metric_filters = vec![
        "o.status = 'normal'".to_string(),
        "oi.product_id IS NOT NULL".to_string(),
        "oi.line_type IN ('normal', 'gift')".to_string(),
    ];
    let mut metric_params: Vec<Value> = Vec::new();
    append_order_date_filters(&mut metric_filters, &mut metric_params, &date_range);
    if let Some(category) = active_category(request.category.as_deref()) {
        metric_filters.push("oi.category = ?".to_string());
        metric_params.push(Value::Text(category.to_string()));
    }

    let mut order_balance_filters = vec!["status = 'normal'".to_string()];
    let mut order_balance_params: Vec<Value> = Vec::new();
    if let Some(end) = &date_range.end {
        order_balance_filters.push("order_date <= ?".to_string());
        order_balance_params.push(Value::Text(end.to_string()));
    }

    let mut payment_balance_filters = vec!["status = 'normal'".to_string()];
    let mut payment_balance_params: Vec<Value> = Vec::new();
    if let Some(end) = &date_range.end {
        payment_balance_filters.push("payment_date <= ?".to_string());
        payment_balance_params.push(Value::Text(end.to_string()));
    }

    let mut sql_params = Vec::new();
    sql_params.extend(metric_params);
    sql_params.extend(order_balance_params);
    sql_params.extend(payment_balance_params);
    let mut rows = analytics_repository::customer_analysis_rows(
        conn,
        &metric_filters.join(" AND "),
        &order_balance_filters.join(" AND "),
        &payment_balance_filters.join(" AND "),
        &sql_params,
        customer_rank_expr(request.rank_by.as_deref())?,
        request.limit.unwrap_or(20).clamp(1, 100),
    )?;

    for row in &mut rows {
        row.average_repurchase_days = customer_average_repurchase_days(
            conn,
            row.customer_id,
            date_range.start.as_deref(),
            date_range.end.as_deref(),
            request.category.as_deref(),
        )?;
        row.favorite_products = customer_favorite_products(
            conn,
            row.customer_id,
            date_range.start.as_deref(),
            date_range.end.as_deref(),
            request.category.as_deref(),
        )?;
    }

    Ok(CustomerAnalysisDto { rows })
}

fn customer_average_repurchase_days(
    conn: &rusqlite::Connection,
    customer_id: i64,
    start_date: Option<&str>,
    end_date: Option<&str>,
    category: Option<&str>,
) -> anyhow::Result<Option<f64>> {
    let mut filters = vec![
        "o.customer_id = ?".to_string(),
        "o.status = 'normal'".to_string(),
    ];
    let mut sql_params = vec![Value::Integer(customer_id)];
    append_optional_order_dates(&mut filters, &mut sql_params, start_date, end_date);
    if let Some(category) = active_category(category) {
        filters.push(
            "EXISTS (
               SELECT 1 FROM order_items oi_filter
               WHERE oi_filter.order_id = o.id AND oi_filter.category = ?
             )"
            .to_string(),
        );
        sql_params.push(Value::Text(category.to_string()));
    }
    let dates =
        analytics_repository::customer_order_dates(conn, &filters.join(" AND "), &sql_params)?;
    if dates.len() < 2 {
        return Ok(None);
    }

    let parsed = dates
        .iter()
        .map(|date| NaiveDate::parse_from_str(date, "%Y-%m-%d"))
        .collect::<Result<Vec<_>, _>>()?;
    let total_days = parsed
        .windows(2)
        .map(|window| (window[1] - window[0]).num_days())
        .sum::<i64>();
    Ok(Some(money(total_days as f64 / (parsed.len() - 1) as f64)))
}

fn customer_favorite_products(
    conn: &rusqlite::Connection,
    customer_id: i64,
    start_date: Option<&str>,
    end_date: Option<&str>,
    category: Option<&str>,
) -> anyhow::Result<String> {
    let mut filters = vec![
        "o.customer_id = ?".to_string(),
        "o.status = 'normal'".to_string(),
        "oi.line_type = 'normal'".to_string(),
        "oi.product_id IS NOT NULL".to_string(),
    ];
    let mut sql_params = vec![Value::Integer(customer_id)];
    append_optional_order_dates(&mut filters, &mut sql_params, start_date, end_date);
    if let Some(category) = active_category(category) {
        filters.push("oi.category = ?".to_string());
        sql_params.push(Value::Text(category.to_string()));
    }
    let products = analytics_repository::customer_favorite_product_rows(
        conn,
        &filters.join(" AND "),
        &sql_params,
    )?;
    Ok(products
        .into_iter()
        .map(|(name, quantity)| format!("{name}({})", compact_quantity(quantity)))
        .collect::<Vec<_>>()
        .join("、"))
}

#[derive(Debug)]
struct DateRange {
    start: Option<String>,
    end: Option<String>,
}

fn normalized_date_range(
    start_date: Option<&str>,
    end_date: Option<&str>,
    invalid_message: &str,
) -> anyhow::Result<DateRange> {
    let start = start_date
        .filter(|value| !value.trim().is_empty())
        .map(normalize_date);
    let end = end_date
        .filter(|value| !value.trim().is_empty())
        .map(normalize_date);
    if let (Some(start), Some(end)) = (&start, &end) {
        if start > end {
            return Err(anyhow!(invalid_message.to_string()));
        }
    }
    Ok(DateRange { start, end })
}

fn append_order_date_filters(
    filters: &mut Vec<String>,
    sql_params: &mut Vec<Value>,
    date_range: &DateRange,
) {
    append_optional_order_dates(
        filters,
        sql_params,
        date_range.start.as_deref(),
        date_range.end.as_deref(),
    );
}

fn append_optional_order_dates(
    filters: &mut Vec<String>,
    sql_params: &mut Vec<Value>,
    start_date: Option<&str>,
    end_date: Option<&str>,
) {
    if let Some(start) = start_date {
        filters.push("o.order_date >= ?".to_string());
        sql_params.push(Value::Text(start.to_string()));
    }
    if let Some(end) = end_date {
        filters.push("o.order_date <= ?".to_string());
        sql_params.push(Value::Text(end.to_string()));
    }
}

fn active_category(category: Option<&str>) -> Option<&str> {
    category
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "全部")
}

fn product_rank_expr(rank_by: Option<&str>) -> anyhow::Result<&'static str> {
    match rank_by.unwrap_or("profit_amount") {
        "sales_quantity" => Ok("sales_quantity"),
        "sales_amount" => Ok("sales_amount"),
        "profit_amount" => Ok("profit_amount"),
        "gift_cost_amount" => Ok("gift_cost_amount"),
        other => Err(anyhow!("不支持的商品排行指标: {other}")),
    }
}

fn customer_rank_expr(rank_by: Option<&str>) -> anyhow::Result<&'static str> {
    match rank_by.unwrap_or("profit_amount") {
        "sales_amount" => Ok("sales_amount"),
        "profit_amount" => Ok("profit_amount"),
        "balance_amount" => Ok("balance_amount"),
        other => Err(anyhow!("不支持的客户分析排行指标: {other}")),
    }
}

fn compact_quantity(value: f64) -> String {
    let rounded = money(value);
    if (rounded.fract()).abs() < 0.0001 {
        format!("{}", rounded as i64)
    } else {
        format!("{rounded:.2}")
    }
}
