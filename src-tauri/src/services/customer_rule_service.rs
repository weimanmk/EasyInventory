use crate::models::{
    CustomerProductRuleDto, CustomerProductRuleImportPreviewDto,
    CustomerProductRuleImportResultDto, CustomerProductRuleImportRowDto, RuleFilterRequest,
    SaveCustomerProductRuleRequest,
};
use crate::repositories::customer_rule_repository;
use crate::services::audit_service::{record_audit, AuditEvent};
use crate::utils::money;
use calamine::{open_workbook_auto, Data, Reader};
use std::collections::HashMap;

pub fn list_customer_product_rules(
    conn: &rusqlite::Connection,
    filter: Option<RuleFilterRequest>,
) -> anyhow::Result<Vec<CustomerProductRuleDto>> {
    customer_rule_repository::list_customer_product_rules(
        conn,
        filter.unwrap_or_else(default_rule_filter),
    )
}

pub fn save_customer_product_rule(
    conn: &rusqlite::Connection,
    payload: SaveCustomerProductRuleRequest,
) -> anyhow::Result<i64> {
    validate_rule_payload(&payload)?;
    let id = customer_rule_repository::save_customer_product_rule(conn, payload)?;
    record_rule_audit(conn, "save", id, "客户商品规则已保存")?;
    Ok(id)
}

pub fn disable_customer_product_rule(conn: &rusqlite::Connection, id: i64) -> anyhow::Result<bool> {
    let disabled = customer_rule_repository::disable_customer_product_rule(conn, id)?;
    record_rule_audit(conn, "disable", id, "客户商品规则已停用")?;
    Ok(disabled)
}

pub fn delete_customer_product_rule(conn: &rusqlite::Connection, id: i64) -> anyhow::Result<bool> {
    let deleted = customer_rule_repository::delete_customer_product_rule(conn, id)?;
    record_rule_audit(conn, "delete", id, "客户商品规则已删除")?;
    Ok(deleted)
}

pub fn preview_customer_product_rule_import(
    conn: &rusqlite::Connection,
    file_path: &str,
) -> anyhow::Result<CustomerProductRuleImportPreviewDto> {
    let rows = parse_customer_product_rule_import_rows(conn, file_path)?;
    Ok(rule_import_preview(
        rows.into_iter().map(|row| row.dto).collect(),
    ))
}

pub fn import_customer_product_rules(
    conn: &rusqlite::Connection,
    file_path: &str,
) -> anyhow::Result<CustomerProductRuleImportResultDto> {
    let rows = parse_customer_product_rule_import_rows(conn, file_path)?;
    let mut output_rows = Vec::new();
    let mut imported_count = 0;
    for row in rows {
        let mut dto = row.dto;
        if let Some(payload) = row.payload {
            match save_customer_product_rule(conn, payload) {
                Ok(_) => {
                    imported_count += 1;
                    dto.status = "imported".to_string();
                    dto.message = Some("已导入".to_string());
                }
                Err(error) => {
                    dto.status = "error".to_string();
                    dto.message = Some(error.to_string());
                }
            }
        }
        output_rows.push(dto);
    }
    let create_count = output_rows
        .iter()
        .filter(|row| row.status == "imported" && row.action == "create")
        .count() as i64;
    let overwrite_count = output_rows
        .iter()
        .filter(|row| row.status == "imported" && row.action == "overwrite")
        .count() as i64;
    let error_count = output_rows
        .iter()
        .filter(|row| row.status == "error")
        .count() as i64;
    let skipped_count = output_rows
        .iter()
        .filter(|row| row.status == "skipped")
        .count() as i64;
    Ok(CustomerProductRuleImportResultDto {
        imported_count,
        create_count,
        overwrite_count,
        error_count,
        skipped_count,
        rows: output_rows,
    })
}

fn validate_rule_payload(payload: &SaveCustomerProductRuleRequest) -> anyhow::Result<()> {
    if payload.customer_id <= 0 || payload.product_id <= 0 {
        anyhow::bail!("客户和商品必填");
    }
    if payload.threshold_quantity.unwrap_or(1.0) <= 0.0 {
        anyhow::bail!("每满数量必须大于 0");
    }
    Ok(())
}

#[derive(Clone)]
struct ParsedRuleImportRow {
    dto: CustomerProductRuleImportRowDto,
    payload: Option<SaveCustomerProductRuleRequest>,
}

fn rule_import_preview(
    rows: Vec<CustomerProductRuleImportRowDto>,
) -> CustomerProductRuleImportPreviewDto {
    let valid_count = rows.iter().filter(|row| row.status == "valid").count() as i64;
    let create_count = rows
        .iter()
        .filter(|row| row.status == "valid" && row.action == "create")
        .count() as i64;
    let overwrite_count = rows
        .iter()
        .filter(|row| row.status == "valid" && row.action == "overwrite")
        .count() as i64;
    let error_count = rows.iter().filter(|row| row.status == "error").count() as i64;
    let skipped_count = rows.iter().filter(|row| row.status == "skipped").count() as i64;
    CustomerProductRuleImportPreviewDto {
        total_count: rows.len() as i64,
        valid_count,
        create_count,
        overwrite_count,
        error_count,
        skipped_count,
        rows,
    }
}

fn parse_customer_product_rule_import_rows(
    conn: &rusqlite::Connection,
    file_path: &str,
) -> anyhow::Result<Vec<ParsedRuleImportRow>> {
    let mut workbook = open_workbook_auto(file_path)?;
    let sheet_name = workbook
        .sheet_names()
        .iter()
        .find(|name| name.as_str() == "客户商品规则")
        .cloned()
        .or_else(|| workbook.sheet_names().first().cloned())
        .ok_or_else(|| anyhow::anyhow!("Excel 中没有可读取的工作表"))?;
    let range = workbook.worksheet_range(&sheet_name)?;
    let mut row_iter = range.rows();
    let header_row = row_iter
        .next()
        .ok_or_else(|| anyhow::anyhow!("客户商品规则导入表缺少表头"))?;
    let headers = rule_import_headers(header_row);
    let customer_col =
        required_rule_import_column(&headers, &["客户", "客户名称", "customer", "customername"])?;
    let product_col =
        required_rule_import_column(&headers, &["商品", "商品名称", "product", "productname"])?;

    let mut parsed_rows = Vec::new();
    for (index, row) in row_iter.enumerate() {
        let row_number = index as i64 + 2;
        parsed_rows.push(parse_rule_import_row(
            conn,
            row,
            row_number,
            &headers,
            customer_col,
            product_col,
        ));
    }
    Ok(parsed_rows)
}

fn parse_rule_import_row(
    conn: &rusqlite::Connection,
    row: &[Data],
    row_number: i64,
    headers: &HashMap<String, usize>,
    customer_col: usize,
    product_col: usize,
) -> ParsedRuleImportRow {
    let customer_name = rule_cell_text(row.get(customer_col));
    let product_name = rule_cell_text(row.get(product_col));
    let category = optional_text(rule_import_text(row, headers, &["类别", "category"]));
    let fixed_price = rule_import_number(row, headers, &["固定售价", "售价", "fixedprice"]);
    let threshold_quantity =
        rule_import_number(row, headers, &["每满数量", "满数量", "thresholdquantity"]);
    let gift_product_name = optional_text(rule_import_text(
        row,
        headers,
        &["赠品商品", "赠品", "giftproduct", "giftproductname"],
    ));
    let gift_quantity = rule_import_number(row, headers, &["赠品数量", "giftquantity"]);
    let direct_discount_amount =
        rule_import_number(row, headers, &["直接折现", "折现", "directdiscountamount"]);
    let monthly_credit_amount =
        rule_import_number(row, headers, &["生成月费", "月费", "monthlycreditamount"]);
    let credit_category = optional_text(rule_import_text(
        row,
        headers,
        &["月费可用类别", "可用类别", "creditcategory"],
    ));
    let remark = optional_text(rule_import_text(row, headers, &["备注", "remark"]));

    let mut dto = CustomerProductRuleImportRowDto {
        row_number,
        customer_name,
        product_name,
        category,
        fixed_price,
        threshold_quantity,
        gift_product_name,
        gift_quantity,
        direct_discount_amount,
        monthly_credit_amount,
        credit_category,
        remark,
        action: "skip".to_string(),
        status: "skipped".to_string(),
        message: Some("空行".to_string()),
    };
    if rule_import_row_is_empty(&dto) {
        return ParsedRuleImportRow { dto, payload: None };
    }

    match build_rule_import_payload(conn, &dto) {
        Ok((payload, action)) => {
            dto.status = "valid".to_string();
            dto.action = action;
            dto.message = None;
            ParsedRuleImportRow {
                dto,
                payload: Some(payload),
            }
        }
        Err(error) => {
            dto.status = "error".to_string();
            dto.action = "skip".to_string();
            dto.message = Some(error.to_string());
            ParsedRuleImportRow { dto, payload: None }
        }
    }
}

fn build_rule_import_payload(
    conn: &rusqlite::Connection,
    row: &CustomerProductRuleImportRowDto,
) -> anyhow::Result<(SaveCustomerProductRuleRequest, String)> {
    if row.customer_name.trim().is_empty() {
        anyhow::bail!("客户不能为空");
    }
    if row.product_name.trim().is_empty() {
        anyhow::bail!("商品不能为空");
    }
    let customer_id =
        customer_rule_repository::lookup_import_customer_id(conn, &row.customer_name)?;
    let product_id = customer_rule_repository::lookup_import_product_id(
        conn,
        &row.product_name,
        row.category.as_deref(),
    )?;
    validate_non_negative(row.fixed_price, "固定售价")?;
    validate_non_negative(row.direct_discount_amount, "直接折现")?;
    validate_non_negative(row.monthly_credit_amount, "生成月费")?;
    if let Some(threshold) = row.threshold_quantity {
        if threshold <= 0.0 {
            anyhow::bail!("每满数量必须大于 0");
        }
    }
    let gift_product_id = match row.gift_product_name.as_ref() {
        Some(name) => Some(customer_rule_repository::lookup_import_product_id(
            conn, name, None,
        )?),
        None => None,
    };
    if gift_product_id.is_some() {
        if row.threshold_quantity.is_none() {
            anyhow::bail!("买赠规则必须填写每满数量");
        }
        if row.gift_quantity.unwrap_or(0.0) <= 0.0 {
            anyhow::bail!("买赠规则必须填写赠品数量");
        }
    }
    if gift_product_id.is_none() && row.gift_quantity.is_some() {
        anyhow::bail!("填写赠品数量时必须填写赠品商品");
    }
    let has_rule = row.fixed_price.is_some()
        || gift_product_id.is_some()
        || row.direct_discount_amount.is_some()
        || row.monthly_credit_amount.is_some();
    if !has_rule {
        anyhow::bail!("未填写有效计价规则");
    }
    let existing = list_customer_product_rules(
        conn,
        Some(RuleFilterRequest {
            customer_id: Some(customer_id),
            product_id: Some(product_id),
            category: None,
            keyword: None,
            is_active: Some(true),
            rule_type: None,
        }),
    )?;
    let action = if existing.is_empty() {
        "create"
    } else {
        "overwrite"
    }
    .to_string();
    Ok((
        SaveCustomerProductRuleRequest {
            id: None,
            customer_id,
            product_id,
            fixed_price: row.fixed_price,
            threshold_quantity: row.threshold_quantity,
            gift_product_id,
            gift_quantity: row.gift_quantity,
            direct_discount_amount: row.direct_discount_amount,
            monthly_credit_amount: row.monthly_credit_amount,
            credit_category: row.credit_category.clone(),
            is_active: true,
            remark: row.remark.clone(),
        },
        action,
    ))
}

fn validate_non_negative(value: Option<f64>, label: &str) -> anyhow::Result<()> {
    if value.is_some_and(|item| item < 0.0) {
        anyhow::bail!("{label}不能小于 0");
    }
    Ok(())
}

fn rule_import_row_is_empty(row: &CustomerProductRuleImportRowDto) -> bool {
    row.customer_name.trim().is_empty()
        && row.product_name.trim().is_empty()
        && row.category.as_deref().unwrap_or("").trim().is_empty()
        && row.fixed_price.is_none()
        && row.threshold_quantity.is_none()
        && row
            .gift_product_name
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
        && row.gift_quantity.is_none()
        && row.direct_discount_amount.is_none()
        && row.monthly_credit_amount.is_none()
        && row
            .credit_category
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
        && row.remark.as_deref().unwrap_or("").trim().is_empty()
}

fn rule_import_headers(row: &[Data]) -> HashMap<String, usize> {
    row.iter()
        .enumerate()
        .filter_map(|(index, cell)| {
            let key = normalize_rule_import_header(&rule_cell_text(Some(cell)));
            if key.is_empty() {
                None
            } else {
                Some((key, index))
            }
        })
        .collect()
}

fn required_rule_import_column(
    headers: &HashMap<String, usize>,
    names: &[&str],
) -> anyhow::Result<usize> {
    rule_import_column(headers, names).ok_or_else(|| anyhow::anyhow!("缺少必需表头：{}", names[0]))
}

fn rule_import_column(headers: &HashMap<String, usize>, names: &[&str]) -> Option<usize> {
    names
        .iter()
        .find_map(|name| headers.get(&normalize_rule_import_header(name)).copied())
}

fn rule_import_text(row: &[Data], headers: &HashMap<String, usize>, names: &[&str]) -> String {
    rule_import_column(headers, names)
        .map(|index| rule_cell_text(row.get(index)))
        .unwrap_or_default()
}

fn rule_import_number(
    row: &[Data],
    headers: &HashMap<String, usize>,
    names: &[&str],
) -> Option<f64> {
    rule_import_column(headers, names).and_then(|index| rule_cell_number(row.get(index)))
}

fn normalize_rule_import_header(value: &str) -> String {
    value.trim().replace([' ', '_'], "").to_ascii_lowercase()
}

fn optional_text(value: String) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn rule_cell_text(cell: Option<&Data>) -> String {
    match cell {
        Some(Data::String(value)) => value.trim().to_string(),
        Some(Data::Float(value)) => {
            if value.fract().abs() < f64::EPSILON {
                format!("{value:.0}")
            } else {
                value.to_string()
            }
        }
        Some(Data::Int(value)) => value.to_string(),
        Some(Data::Bool(value)) => value.to_string(),
        Some(Data::DateTime(value)) => value.to_string(),
        Some(Data::DateTimeIso(value)) => value.clone(),
        Some(Data::DurationIso(value)) => value.clone(),
        Some(Data::Error(value)) => format!("{value:?}"),
        Some(Data::Empty) | None => String::new(),
    }
}

fn rule_cell_number(cell: Option<&Data>) -> Option<f64> {
    match cell {
        Some(Data::Float(value)) => Some(money(*value)),
        Some(Data::Int(value)) => Some(*value as f64),
        Some(Data::String(value)) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                trimmed.parse::<f64>().ok().map(money)
            }
        }
        _ => None,
    }
}

fn default_rule_filter() -> RuleFilterRequest {
    RuleFilterRequest {
        customer_id: None,
        product_id: None,
        category: None,
        keyword: None,
        is_active: None,
        rule_type: None,
    }
}

fn record_rule_audit(
    conn: &rusqlite::Connection,
    action: &'static str,
    id: i64,
    message: &'static str,
) -> anyhow::Result<()> {
    record_audit(
        conn,
        AuditEvent {
            module: "rule",
            action,
            target_type: Some("customer_product_rules"),
            target_id: Some(id),
            target_label: Some("客户商品规则"),
            result: "success",
            message: Some(message),
            details: None,
        },
    )
}
