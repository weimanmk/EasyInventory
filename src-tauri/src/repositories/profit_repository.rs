use crate::models::{
    DailyProfitSummary, ProfitAnalyticsMetricDto, ProfitAnalyticsTrendPointDto, ProfitBreakdownDto,
};
use rusqlite::{params_from_iter, types::Value};

#[derive(Debug, Clone)]
pub struct ProfitSqlFilter {
    pub where_sql: String,
    pub params: Vec<Value>,
}

pub fn daily_profit_summary(
    conn: &rusqlite::Connection,
    date: &str,
) -> anyhow::Result<DailyProfitSummary> {
    conn.query_row(
        "SELECT
           COUNT(*),
           COALESCE(SUM(product_sales_amount), 0),
           COALESCE(SUM(customer_payable_amount), 0),
           COALESCE(SUM(direct_discount_amount), 0),
           COALESCE(SUM(monthly_credit_used), 0),
           COALESCE(SUM(brand_subsidy_amount), 0),
           COALESCE(SUM(cost_amount), 0),
           COALESCE(SUM(gift_cost_amount), 0),
           COALESCE(SUM(profit_amount), 0)
         FROM orders
         WHERE order_date = ?1 AND status = 'normal'",
        [date],
        |row| {
            Ok(DailyProfitSummary {
                date: date.to_string(),
                order_count: row.get(0)?,
                product_sales_amount: row.get(1)?,
                customer_payable_amount: row.get(2)?,
                direct_discount_amount: row.get(3)?,
                monthly_credit_used: row.get(4)?,
                brand_subsidy_amount: row.get(5)?,
                cost_amount: row.get(6)?,
                gift_cost_amount: row.get(7)?,
                profit_amount: row.get(8)?,
            })
        },
    )
    .map_err(Into::into)
}

pub fn profit_analytics_summary(
    conn: &rusqlite::Connection,
    order_filter: &ProfitSqlFilter,
) -> anyhow::Result<ProfitAnalyticsMetricDto> {
    let sql = format!(
        "SELECT
           COUNT(*),
           COALESCE(SUM(product_sales_amount), 0),
           COALESCE(SUM(customer_payable_amount), 0),
           COALESCE(SUM(direct_discount_amount), 0),
           COALESCE(SUM(monthly_credit_used), 0),
           COALESCE(SUM(brand_subsidy_amount), 0),
           COALESCE(SUM(cost_amount), 0),
           COALESCE(SUM(gift_cost_amount), 0),
           COALESCE(SUM(profit_amount), 0)
         FROM orders o
         WHERE {}",
        order_filter.where_sql
    );
    conn.query_row(&sql, params_from_iter(order_filter.params.iter()), |row| {
        Ok(ProfitAnalyticsMetricDto {
            order_count: row.get(0)?,
            product_sales_amount: row.get(1)?,
            customer_payable_amount: row.get(2)?,
            direct_discount_amount: row.get(3)?,
            monthly_credit_used: row.get(4)?,
            brand_subsidy_amount: row.get(5)?,
            cost_amount: row.get(6)?,
            gift_cost_amount: row.get(7)?,
            profit_amount: row.get(8)?,
        })
    })
    .map_err(Into::into)
}

pub fn profit_analytics_trend(
    conn: &rusqlite::Connection,
    period_expr: &str,
    order_filter: &ProfitSqlFilter,
) -> anyhow::Result<Vec<ProfitAnalyticsTrendPointDto>> {
    let sql = format!(
        "SELECT
           {period_expr},
           COUNT(*),
           COALESCE(SUM(product_sales_amount), 0),
           COALESCE(SUM(customer_payable_amount), 0),
           COALESCE(SUM(direct_discount_amount), 0),
           COALESCE(SUM(monthly_credit_used), 0),
           COALESCE(SUM(brand_subsidy_amount), 0),
           COALESCE(SUM(cost_amount), 0),
           COALESCE(SUM(gift_cost_amount), 0),
           COALESCE(SUM(profit_amount), 0)
         FROM orders o
         WHERE {}
         GROUP BY 1
         ORDER BY 1",
        order_filter.where_sql
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(params_from_iter(order_filter.params.iter()), |row| {
            Ok(ProfitAnalyticsTrendPointDto {
                period: row.get(0)?,
                order_count: row.get(1)?,
                product_sales_amount: row.get(2)?,
                customer_payable_amount: row.get(3)?,
                direct_discount_amount: row.get(4)?,
                monthly_credit_used: row.get(5)?,
                brand_subsidy_amount: row.get(6)?,
                cost_amount: row.get(7)?,
                gift_cost_amount: row.get(8)?,
                profit_amount: row.get(9)?,
                comparison_period: None,
                comparison_sales_amount: None,
                comparison_profit_amount: None,
                sales_change_amount: None,
                sales_change_rate: None,
                profit_change_amount: None,
                profit_change_rate: None,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn profit_analytics_category_breakdown(
    conn: &rusqlite::Connection,
    order_filter: &ProfitSqlFilter,
    category: Option<&str>,
) -> anyhow::Result<Vec<ProfitBreakdownDto>> {
    let mut where_sql = format!(
        "{} AND oi.category IS NOT NULL AND TRIM(oi.category) <> ''",
        order_filter.where_sql
    );
    let mut sql_params = order_filter.params.clone();
    if let Some(category) = category {
        where_sql.push_str(" AND oi.category = ?");
        sql_params.push(Value::Text(category.to_string()));
    }
    let sql = format!(
        "SELECT
           oi.category,
           COUNT(DISTINCT o.id),
           COALESCE(SUM(oi.amount), 0),
           COALESCE(SUM(oi.amount), 0),
           COALESCE(SUM(oi.cost_amount), 0),
           COALESCE(SUM(oi.profit_amount), 0)
         FROM order_items oi
         JOIN orders o ON o.id = oi.order_id
         WHERE {where_sql}
         GROUP BY oi.category
         ORDER BY COALESCE(SUM(oi.profit_amount), 0) DESC,
                  COALESCE(SUM(oi.amount), 0) DESC,
                  oi.category
         LIMIT 20"
    );
    profit_breakdown_rows(conn, &sql, &sql_params)
}

pub fn profit_analytics_customer_breakdown(
    conn: &rusqlite::Connection,
    order_filter: &ProfitSqlFilter,
) -> anyhow::Result<Vec<ProfitBreakdownDto>> {
    let sql = format!(
        "SELECT
           o.customer_name,
           COUNT(*),
           COALESCE(SUM(o.product_sales_amount), 0),
           COALESCE(SUM(o.customer_payable_amount), 0),
           COALESCE(SUM(o.cost_amount), 0),
           COALESCE(SUM(o.profit_amount), 0)
         FROM orders o
         WHERE {}
         GROUP BY o.customer_id, o.customer_name
         ORDER BY COALESCE(SUM(o.profit_amount), 0) DESC,
                  COALESCE(SUM(o.product_sales_amount), 0) DESC,
                  o.customer_name
         LIMIT 20",
        order_filter.where_sql
    );
    profit_breakdown_rows(conn, &sql, &order_filter.params)
}

pub fn order_has_category(
    conn: &rusqlite::Connection,
    order_id: i64,
    category: &str,
) -> anyhow::Result<bool> {
    conn.query_row(
        "SELECT COUNT(*) > 0 FROM order_items
         WHERE order_id = ?1 AND category = ?2",
        rusqlite::params![order_id, category],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

fn profit_breakdown_rows(
    conn: &rusqlite::Connection,
    sql: &str,
    sql_params: &[Value],
) -> anyhow::Result<Vec<ProfitBreakdownDto>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt
        .query_map(params_from_iter(sql_params.iter()), |row| {
            Ok(ProfitBreakdownDto {
                name: row.get(0)?,
                order_count: row.get(1)?,
                product_sales_amount: row.get(2)?,
                customer_payable_amount: row.get(3)?,
                cost_amount: row.get(4)?,
                profit_amount: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}
