use crate::domain::order_math::{choose_unit_price, threshold_times, PriceSource};
use crate::models::{
    DiscountPreviewDto, GiftPreviewDto, ListOrdersRequest, MonthlyCreditDto,
    MonthlyCreditFilterRequest, MonthlyCreditPreviewDto, MonthlyCreditUseRequest, OrderDetailDto,
    OrderDto, OrderTotalsDto, PreviewQuoteRequest, ProductDto, QuotePreviewDto, SaveOrderRequest,
    SaveOrderResponse,
};
use crate::repositories::order_repository;
use crate::utils::{money, next_month, normalize_date};
use anyhow::anyhow;
use rusqlite::{Connection, Transaction, TransactionBehavior};

pub fn get_order_detail(
    conn: &rusqlite::Connection,
    order_id: i64,
) -> anyhow::Result<OrderDetailDto> {
    order_repository::get_order_detail(conn, order_id)
}

pub fn list_orders(
    conn: &rusqlite::Connection,
    filter: ListOrdersRequest,
) -> anyhow::Result<Vec<OrderDto>> {
    order_repository::list_orders(conn, filter)
}

pub fn list_orders_with_default_filter(
    conn: &rusqlite::Connection,
    filter: Option<ListOrdersRequest>,
) -> anyhow::Result<Vec<OrderDto>> {
    order_repository::list_orders(conn, filter.unwrap_or_else(default_list_orders_filter))
}

pub fn preview_quote(
    conn: &rusqlite::Connection,
    request: PreviewQuoteRequest,
) -> anyhow::Result<QuotePreviewDto> {
    if request.quantity <= 0.0 {
        return Err(anyhow!("数量必须大于 0"));
    }
    let product = crate::db::product_by_id(conn, request.product_id)?;
    let rule = order_repository::active_rule(conn, request.customer_id, request.product_id)?;
    let price_choice = choose_unit_price(
        request.manual_price,
        rule.as_ref().and_then(|rule| rule.fixed_price),
        product.default_price,
    );
    let unit_price = price_choice.unit_price;

    let mut messages = Vec::new();
    messages.push(price_source_message(price_choice.source).to_string());

    let mut gift_preview = None;
    let mut discount_preview = None;
    let mut monthly_preview = None;
    let mut rule_id = None;

    if let Some(rule) = rule {
        rule_id = Some(rule.id);
        let times = threshold_times(request.quantity, rule.threshold_quantity);
        if times > 0.0 {
            if let Some(threshold) = rule.threshold_quantity {
                if let (Some(gift_product_id), Some(gift_quantity)) =
                    (rule.gift_product_id, rule.gift_quantity)
                {
                    if gift_quantity > 0.0 {
                        let gift = crate::db::product_by_id(conn, gift_product_id)?;
                        let quantity = money(times * gift_quantity);
                        gift_preview = Some(GiftPreviewDto {
                            product_id: gift_product_id,
                            product_name: gift.name,
                            quantity,
                        });
                        messages.push(format!("每满 {} 送 {}", threshold, quantity));
                    }
                }
                if let Some(discount) = rule.direct_discount_amount.filter(|value| *value > 0.0) {
                    let amount = money(times * discount);
                    discount_preview = Some(DiscountPreviewDto { amount });
                    messages.push(format!("本单折现 {}", amount));
                }
                if let Some(credit) = rule.monthly_credit_amount.filter(|value| *value > 0.0) {
                    let amount = money(times * credit);
                    let category = rule
                        .credit_category
                        .unwrap_or_else(|| product.category.clone());
                    monthly_preview = Some(MonthlyCreditPreviewDto { amount, category });
                    messages.push(format!(
                        "生成 {} 月费 {}",
                        next_month(&request.order_date),
                        amount
                    ));
                }
            }
        }
    }

    Ok(QuotePreviewDto {
        product_id: request.product_id,
        unit_price: money(unit_price),
        price_source: price_source_code(price_choice.source).to_string(),
        amount: money(request.quantity * unit_price),
        rule_id,
        gift_preview,
        direct_discount_preview: discount_preview,
        monthly_credit_preview: monthly_preview,
        message: messages.join("；"),
    })
}

pub fn save_order(
    conn: &mut Connection,
    request: SaveOrderRequest,
) -> anyhow::Result<SaveOrderResponse> {
    if request.customer_id <= 0 {
        return Err(anyhow!("必须选择客户"));
    }
    if request.items.is_empty() {
        return Err(anyhow!("必须至少选择一条商品"));
    }
    for item in &request.items {
        if item.quantity <= 0.0 {
            return Err(anyhow!("商品数量必须大于 0"));
        }
        if item.unit_price < 0.0 {
            return Err(anyhow!("商品单价不能小于 0"));
        }
    }

    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let order_no = order_repository::next_order_no(&tx, &request.order_date)?;
    let customer = crate::db::customer_by_id(&tx, request.customer_id)?;
    let address = request
        .customer_address
        .clone()
        .or_else(|| customer.address.clone());
    let order_id = order_repository::create_order_header(
        &tx,
        order_repository::NewOrderHeader {
            order_no: &order_no,
            order_date: &request.order_date,
            customer_id: request.customer_id,
            customer_name: &customer.name,
            customer_address: address.as_deref(),
            remark: request.remark.as_deref(),
        },
    )?;

    let mut totals = OrderTotalsDto::default();
    let mut sort_order = 1_i64;
    for item in &request.items {
        let product = crate::db::product_by_id(&tx, item.product_id)?;
        let rule = order_repository::active_rule(&tx, request.customer_id, item.product_id)?;
        let amount = money(item.quantity * item.unit_price);
        let cost_amount = money(product.avg_cost * item.quantity);
        let profit = money(amount - cost_amount);
        totals.product_sales_amount = money(totals.product_sales_amount + amount);
        totals.cost_amount = money(totals.cost_amount + cost_amount);
        totals.profit_amount = money(totals.profit_amount + profit);

        order_repository::create_order_item(
            &tx,
            order_repository::NewOrderItem {
                order_id,
                line_type: "normal",
                product_id: Some(product.id),
                product_name: Some(&product.name),
                category: Some(&product.category),
                barcode: product.barcode.as_deref(),
                quantity: item.quantity,
                unit_price: item.unit_price,
                amount,
                avg_cost: product.avg_cost,
                cost_amount,
                profit_amount: profit,
                related_product_id: None,
                rule_id: rule.as_ref().map(|rule| rule.id),
                monthly_credit_id: None,
                remark: item.remark.as_deref(),
                sort_order,
            },
        )?;
        sort_order += 1;

        order_repository::create_inventory_movement(
            &tx,
            order_repository::NewMovement {
                date: &request.order_date,
                product_id: product.id,
                movement_type: "outbound",
                quantity: item.quantity,
                unit_price: item.unit_price,
                amount,
                order_id,
                order_no: &order_no,
                remark: "订单出库",
            },
        )?;
        crate::db::recalc_stock_balance(&tx, product.id)?;

        apply_credit_uses(
            &tx,
            order_id,
            &order_no,
            &mut sort_order,
            &mut totals,
            item.monthly_credit_uses.clone().unwrap_or_default(),
        )?;

        if let Some(rule) = rule {
            apply_rule_effects(
                &tx,
                &mut sort_order,
                &mut totals,
                RuleEffectInput {
                    request: &request,
                    order_id,
                    order_no: &order_no,
                    product: &product,
                    rule: &rule,
                    quantity: item.quantity,
                },
            )?;
        }
    }

    totals.customer_payable_amount = money(
        totals.product_sales_amount - totals.direct_discount_amount - totals.monthly_credit_used,
    );

    order_repository::update_order_totals(&tx, order_id, &totals)?;

    tx.commit()?;
    Ok(SaveOrderResponse {
        order_id,
        order_no,
        document_path: String::new(),
        totals,
    })
}

struct RuleEffectInput<'a> {
    request: &'a SaveOrderRequest,
    order_id: i64,
    order_no: &'a str,
    product: &'a ProductDto,
    rule: &'a order_repository::RuleRow,
    quantity: f64,
}

fn apply_credit_uses(
    tx: &Transaction<'_>,
    order_id: i64,
    order_no: &str,
    sort_order: &mut i64,
    totals: &mut OrderTotalsDto,
    uses: Vec<MonthlyCreditUseRequest>,
) -> anyhow::Result<()> {
    for credit_use in uses {
        if credit_use.amount <= 0.0 {
            continue;
        }
        let credit = order_repository::monthly_credit_by_id(tx, credit_use.monthly_credit_id)?;
        if credit.status != "available"
            || credit.remaining_amount + f64::EPSILON < credit_use.amount
        {
            return Err(anyhow!("月费额度不可用或余额不足"));
        }
        let used = money(credit.used_amount + credit_use.amount);
        let remaining = money(credit.remaining_amount - credit_use.amount);
        let status = if remaining <= 0.0 {
            "used_up"
        } else {
            "available"
        };
        order_repository::apply_monthly_credit_use(tx, credit.id, used, remaining, status)?;
        let remark = format!("使用月费 {}，来源 {}", credit_use.amount, order_no);
        order_repository::create_order_item(
            tx,
            order_repository::NewOrderItem {
                order_id,
                line_type: "credit",
                product_id: None,
                product_name: Some("月费抵扣"),
                category: Some(&credit.category),
                barcode: None,
                quantity: 1.0,
                unit_price: -credit_use.amount,
                amount: -credit_use.amount,
                avg_cost: 0.0,
                cost_amount: 0.0,
                profit_amount: 0.0,
                related_product_id: None,
                rule_id: None,
                monthly_credit_id: Some(credit.id),
                remark: Some(&remark),
                sort_order: *sort_order,
            },
        )?;
        *sort_order += 1;
        totals.monthly_credit_used = money(totals.monthly_credit_used + credit_use.amount);
    }
    Ok(())
}

fn apply_rule_effects(
    tx: &Transaction<'_>,
    sort_order: &mut i64,
    totals: &mut OrderTotalsDto,
    input: RuleEffectInput<'_>,
) -> anyhow::Result<()> {
    let request = input.request;
    let order_id = input.order_id;
    let order_no = input.order_no;
    let product = input.product;
    let rule = input.rule;
    let quantity = input.quantity;
    let times = threshold_times(quantity, rule.threshold_quantity);
    if times <= 0.0 {
        return Ok(());
    }

    if let (Some(gift_product_id), Some(gift_quantity)) = (rule.gift_product_id, rule.gift_quantity)
    {
        let total_gift_qty = money(times * gift_quantity);
        if total_gift_qty > 0.0 {
            let gift = crate::db::product_by_id(tx, gift_product_id)?;
            let gift_cost = money(gift.avg_cost * total_gift_qty);
            order_repository::create_order_item(
                tx,
                order_repository::NewOrderItem {
                    order_id,
                    line_type: "gift",
                    product_id: Some(gift.id),
                    product_name: Some(&gift.name),
                    category: Some(&gift.category),
                    barcode: gift.barcode.as_deref(),
                    quantity: total_gift_qty,
                    unit_price: 0.0,
                    amount: 0.0,
                    avg_cost: gift.avg_cost,
                    cost_amount: gift_cost,
                    profit_amount: -gift_cost,
                    related_product_id: Some(product.id),
                    rule_id: Some(rule.id),
                    monthly_credit_id: None,
                    remark: Some("赠品"),
                    sort_order: *sort_order,
                },
            )?;
            *sort_order += 1;
            order_repository::create_inventory_movement(
                tx,
                order_repository::NewMovement {
                    date: &request.order_date,
                    product_id: gift.id,
                    movement_type: "gift_outbound",
                    quantity: total_gift_qty,
                    unit_price: 0.0,
                    amount: 0.0,
                    order_id,
                    order_no,
                    remark: "买赠赠品出库",
                },
            )?;
            crate::db::recalc_stock_balance(tx, gift.id)?;
            totals.gift_cost_amount = money(totals.gift_cost_amount + gift_cost);
            totals.cost_amount = money(totals.cost_amount + gift_cost);
            totals.profit_amount = money(totals.profit_amount - gift_cost);
        }
    }

    if let Some(discount) = rule.direct_discount_amount.filter(|value| *value > 0.0) {
        let amount = money(times * discount);
        order_repository::create_order_item(
            tx,
            order_repository::NewOrderItem {
                order_id,
                line_type: "discount",
                product_id: None,
                product_name: Some("本单折现"),
                category: Some(&product.category),
                barcode: None,
                quantity: 1.0,
                unit_price: -amount,
                amount: -amount,
                avg_cost: 0.0,
                cost_amount: 0.0,
                profit_amount: -amount,
                related_product_id: Some(product.id),
                rule_id: Some(rule.id),
                monthly_credit_id: None,
                remark: Some("规则折现"),
                sort_order: *sort_order,
            },
        )?;
        *sort_order += 1;
        totals.direct_discount_amount = money(totals.direct_discount_amount + amount);
        totals.profit_amount = money(totals.profit_amount - amount);
    }

    if let Some(credit) = rule.monthly_credit_amount.filter(|value| *value > 0.0) {
        let amount = money(times * credit);
        let category = rule
            .credit_category
            .clone()
            .unwrap_or_else(|| product.category.clone());
        let available_month = next_month(&request.order_date);
        order_repository::create_monthly_credit(
            tx,
            order_repository::NewMonthlyCredit {
                order_id,
                order_no,
                customer_id: request.customer_id,
                category: &category,
                amount,
                generated_date: &request.order_date,
                available_month: &available_month,
                remark: "规则生成月费",
            },
        )?;
        totals.brand_subsidy_amount = money(totals.brand_subsidy_amount + amount);
    }

    Ok(())
}

pub fn list_monthly_credits(
    conn: &rusqlite::Connection,
    filter: MonthlyCreditFilterRequest,
) -> anyhow::Result<Vec<MonthlyCreditDto>> {
    order_repository::refresh_credit_statuses(conn)?;
    order_repository::list_monthly_credits(conn, filter)
}

pub fn list_monthly_credits_with_default_filter(
    conn: &rusqlite::Connection,
    filter: Option<MonthlyCreditFilterRequest>,
) -> anyhow::Result<Vec<MonthlyCreditDto>> {
    list_monthly_credits(conn, filter.unwrap_or_else(default_monthly_credit_filter))
}

pub fn void_order(
    conn: &mut Connection,
    order_id: i64,
    reason: Option<String>,
) -> anyhow::Result<OrderDto> {
    let tx = conn.transaction()?;
    let order = order_repository::order_by_id(&tx, order_id)?;
    if order.status == "voided" {
        return Err(anyhow!("订单已作废"));
    }

    let product_ids = order_repository::order_movement_product_ids(&tx, order_id)?;
    let credit_uses = order_repository::order_credit_uses(&tx, order_id)?;
    for credit_use in credit_uses {
        order_repository::restore_monthly_credit_use(
            &tx,
            credit_use.monthly_credit_id,
            credit_use.amount,
        )?;
    }
    order_repository::void_credits_generated_by_order(&tx, order_id)?;
    order_repository::delete_order_movements(&tx, order_id)?;

    for product_id in product_ids {
        crate::db::recalc_stock_balance(&tx, product_id)?;
    }

    let remark = voided_order_remark(
        order.remark.as_deref(),
        reason.as_deref().filter(|value| !value.trim().is_empty()),
    );
    order_repository::mark_order_voided(&tx, order_id, remark.as_deref())?;
    order_repository::mark_order_documents_voided(&tx, order_id)?;

    let updated = order_repository::order_by_id(&tx, order_id)?;
    tx.commit()?;
    Ok(updated)
}

pub fn available_monthly_credits(
    conn: &rusqlite::Connection,
    customer_id: i64,
    category: String,
    order_date: String,
) -> anyhow::Result<Vec<MonthlyCreditDto>> {
    order_repository::refresh_credit_statuses(conn)?;
    let order_month = normalize_date(&order_date)[..7].to_string();
    let filter = MonthlyCreditFilterRequest {
        customer_id: Some(customer_id),
        category: Some(category),
        status: Some("available".to_string()),
        start_date: None,
        end_date: None,
        available_month: None,
    };
    Ok(order_repository::list_monthly_credits(conn, filter)?
        .into_iter()
        .filter(|credit| credit.remaining_amount > 0.0 && credit.available_month <= order_month)
        .collect())
}

pub fn close_or_void_credit(
    conn: &rusqlite::Connection,
    id: i64,
    status: &str,
) -> anyhow::Result<()> {
    if !matches!(status, "closed" | "voided") {
        return Err(anyhow!("不支持的月费状态"));
    }
    order_repository::update_monthly_credit_status(conn, id, status)
}

fn voided_order_remark(existing: Option<&str>, reason: Option<&str>) -> Option<String> {
    match (existing, reason) {
        (Some(existing), Some(reason)) => Some(format!("{existing}；作废原因：{reason}")),
        (None, Some(reason)) => Some(format!("作废原因：{reason}")),
        (existing, None) => existing.map(str::to_string),
    }
}

fn price_source_code(source: PriceSource) -> &'static str {
    match source {
        PriceSource::Manual => "manual",
        PriceSource::CustomerFixedPrice => "customer_fixed_price",
        PriceSource::DefaultPrice => "default_price",
        PriceSource::Zero => "zero",
    }
}

fn price_source_message(source: PriceSource) -> &'static str {
    match source {
        PriceSource::Manual => "手动价",
        PriceSource::CustomerFixedPrice => "客户固定价",
        PriceSource::DefaultPrice => "默认售价",
        PriceSource::Zero => "价格为 0",
    }
}

fn default_list_orders_filter() -> ListOrdersRequest {
    ListOrdersRequest {
        start_date: None,
        end_date: None,
        customer_id: None,
        order_no: None,
        status: Some("normal".to_string()),
    }
}

fn default_monthly_credit_filter() -> MonthlyCreditFilterRequest {
    MonthlyCreditFilterRequest {
        customer_id: None,
        category: None,
        status: None,
        start_date: None,
        end_date: None,
        available_month: None,
    }
}
