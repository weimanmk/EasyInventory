use crate::models::{
    DailyProfitSummary, ListOrdersRequest, OrderDto, ProfitAnalyticsRequest,
    ProfitAnalyticsResponse, ProfitAnalyticsTrendPointDto, ProfitFilterRequest,
};
use crate::repositories::profit_repository::{self, ProfitSqlFilter};
use crate::utils::money;
use anyhow::{anyhow, Context};
use chrono::{Datelike, Duration, NaiveDate};
use rusqlite::types::Value;

pub fn daily_profit_summary(
    conn: &rusqlite::Connection,
    date: &str,
) -> anyhow::Result<DailyProfitSummary> {
    profit_repository::daily_profit_summary(conn, date)
}

pub fn get_profit_analytics(
    conn: &rusqlite::Connection,
    request: ProfitAnalyticsRequest,
) -> anyhow::Result<ProfitAnalyticsResponse> {
    if request.start_date.trim().is_empty() || request.end_date.trim().is_empty() {
        return Err(anyhow!("利润统计日期范围不能为空"));
    }
    let period_expr = profit_period_expression(&request.period)?;
    let order_filter = profit_order_filter(&request);
    let summary = profit_repository::profit_analytics_summary(conn, &order_filter)?;
    let trend = profit_analytics_trend(conn, &request, period_expr, &order_filter)?;
    let category_breakdown = profit_repository::profit_analytics_category_breakdown(
        conn,
        &order_filter,
        active_category(request.category.as_deref()),
    )?;
    let customer_breakdown =
        profit_repository::profit_analytics_customer_breakdown(conn, &order_filter)?;

    Ok(ProfitAnalyticsResponse {
        summary,
        trend,
        category_breakdown,
        customer_breakdown,
    })
}

pub fn list_profit_records(
    conn: &rusqlite::Connection,
    filter: ProfitFilterRequest,
) -> anyhow::Result<Vec<OrderDto>> {
    let category = filter.category.clone();
    let request = ListOrdersRequest {
        start_date: filter.start_date,
        end_date: filter.end_date,
        customer_id: filter.customer_id,
        order_no: None,
        status: Some("normal".to_string()),
    };
    let mut orders = crate::services::order_service::list_orders(conn, request)?;
    if let Some(category) = active_category(category.as_deref()) {
        let mut filtered = Vec::new();
        for order in orders {
            if profit_repository::order_has_category(conn, order.id, category)? {
                filtered.push(order);
            }
        }
        orders = filtered;
    }
    Ok(orders)
}

pub fn list_profit_records_with_default_filter(
    conn: &rusqlite::Connection,
    filter: Option<ProfitFilterRequest>,
) -> anyhow::Result<Vec<OrderDto>> {
    list_profit_records(conn, filter.unwrap_or_else(default_profit_filter))
}

fn profit_analytics_trend(
    conn: &rusqlite::Connection,
    request: &ProfitAnalyticsRequest,
    period_expr: &str,
    order_filter: &ProfitSqlFilter,
) -> anyhow::Result<Vec<ProfitAnalyticsTrendPointDto>> {
    let mut rows = profit_repository::profit_analytics_trend(conn, period_expr, order_filter)?;
    for row in &mut rows {
        let (comparison_period, start_date, end_date) =
            profit_comparison_range(&request.period, &row.period)?;
        let comparison_filter = profit_order_filter_for_dates(request, &start_date, &end_date);
        let comparison = profit_repository::profit_analytics_summary(conn, &comparison_filter)?;
        row.comparison_period = Some(comparison_period);
        row.comparison_sales_amount = Some(money(comparison.product_sales_amount));
        row.comparison_profit_amount = Some(money(comparison.profit_amount));
        row.sales_change_amount = Some(money(
            row.product_sales_amount - comparison.product_sales_amount,
        ));
        row.sales_change_rate =
            percent_change(row.product_sales_amount, comparison.product_sales_amount);
        row.profit_change_amount = Some(money(row.profit_amount - comparison.profit_amount));
        row.profit_change_rate = percent_change(row.profit_amount, comparison.profit_amount);
    }
    Ok(rows)
}

fn profit_period_expression(period: &str) -> anyhow::Result<&'static str> {
    match period {
        "day" => Ok("o.order_date"),
        "month" => Ok("substr(o.order_date, 1, 7)"),
        "year" => Ok("substr(o.order_date, 1, 4)"),
        _ => Err(anyhow!("不支持的利润统计周期: {period}")),
    }
}

fn profit_order_filter(request: &ProfitAnalyticsRequest) -> ProfitSqlFilter {
    profit_order_filter_for_dates(request, &request.start_date, &request.end_date)
}

fn profit_order_filter_for_dates(
    request: &ProfitAnalyticsRequest,
    start_date: &str,
    end_date: &str,
) -> ProfitSqlFilter {
    let mut conditions = vec![
        "o.status = 'normal'".to_string(),
        "o.order_date >= ?".to_string(),
        "o.order_date <= ?".to_string(),
    ];
    let mut sql_params = vec![
        Value::Text(start_date.to_string()),
        Value::Text(end_date.to_string()),
    ];
    if let Some(customer_id) = request.customer_id {
        conditions.push("o.customer_id = ?".to_string());
        sql_params.push(Value::Integer(customer_id));
    }
    if let Some(category) = active_category(request.category.as_deref()) {
        conditions.push(
            "EXISTS (
               SELECT 1 FROM order_items oi_filter
               WHERE oi_filter.order_id = o.id AND oi_filter.category = ?
             )"
            .to_string(),
        );
        sql_params.push(Value::Text(category.to_string()));
    }
    ProfitSqlFilter {
        where_sql: conditions.join(" AND "),
        params: sql_params,
    }
}

fn active_category(category: Option<&str>) -> Option<&str> {
    category
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "全部")
}

fn profit_comparison_range(
    period_type: &str,
    period: &str,
) -> anyhow::Result<(String, String, String)> {
    match period_type {
        "day" => {
            let current = NaiveDate::parse_from_str(period, "%Y-%m-%d")?;
            let previous = current - Duration::days(1);
            let value = previous.format("%Y-%m-%d").to_string();
            Ok((value.clone(), value.clone(), value))
        }
        "month" => {
            let current = NaiveDate::parse_from_str(&format!("{period}-01"), "%Y-%m-%d")?;
            let previous_year = current.year() - 1;
            let month = current.month();
            let start = NaiveDate::from_ymd_opt(previous_year, month, 1)
                .ok_or_else(|| anyhow!("利润统计月份无效: {period}"))?;
            let end = month_end(previous_year, month)?;
            Ok((
                start.format("%Y-%m").to_string(),
                start.format("%Y-%m-%d").to_string(),
                end.format("%Y-%m-%d").to_string(),
            ))
        }
        "year" => {
            let year = period
                .parse::<i32>()
                .with_context(|| format!("利润统计年份无效: {period}"))?
                - 1;
            Ok((
                year.to_string(),
                format!("{year}-01-01"),
                format!("{year}-12-31"),
            ))
        }
        _ => Err(anyhow!("不支持的利润统计周期: {period_type}")),
    }
}

fn month_end(year: i32, month: u32) -> anyhow::Result<NaiveDate> {
    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    let next_start = NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .ok_or_else(|| anyhow!("利润统计月份无效: {year}-{month:02}"))?;
    Ok(next_start - Duration::days(1))
}

fn percent_change(current: f64, previous: f64) -> Option<f64> {
    if previous.abs() < 0.0001 {
        None
    } else {
        Some(money((current - previous) / previous * 100.0))
    }
}

fn default_profit_filter() -> ProfitFilterRequest {
    ProfitFilterRequest {
        start_date: None,
        end_date: None,
        customer_id: None,
        category: None,
    }
}
