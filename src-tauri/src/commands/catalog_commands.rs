use super::{fail, ok};
use crate::app::AppState;
use crate::logger;
use crate::models::*;
use crate::services::{customer_service, product_service, supplier_service};
use tauri::State;

#[tauri::command]
pub fn list_products(
    state: State<AppState>,
    filter: Option<ListProductsRequest>,
) -> ApiResponse<Vec<ProductDto>> {
    let result = (|| {
        let conn = state.connection()?;
        logger::info(
            "product",
            format!(
                "list_products filter_keys={}",
                product_filter_keys(filter.as_ref()).join(",")
            ),
        );
        product_service::list_products(&conn, filter)
    })();
    if let Ok(items) = &result {
        logger::info(
            "product",
            format!("list_products result count={}", items.len()),
        );
    }
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn create_product(state: State<AppState>, payload: ProductPayload) -> ApiResponse<ProductDto> {
    logger::info("product", "create_product start");
    let result = (|| {
        let conn = state.connection()?;
        product_service::create_product(&conn, payload)
    })();
    if let Ok(product) = &result {
        logger::info(
            "product",
            format!(
                "create_product success id={} category={}",
                product.id, product.category
            ),
        );
    }
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn update_product(
    state: State<AppState>,
    id: i64,
    payload: ProductPayload,
) -> ApiResponse<ProductDto> {
    logger::info("product", format!("update_product start id={id}"));
    let result = (|| {
        let conn = state.connection()?;
        product_service::update_product(&conn, id, payload)
    })();
    if let Ok(product) = &result {
        logger::info(
            "product",
            format!(
                "update_product success id={} category={}",
                product.id, product.category
            ),
        );
    }
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn disable_product(state: State<AppState>, id: i64) -> ApiResponse<bool> {
    let result = (|| {
        let conn = state.connection()?;
        product_service::disable_product(&conn, id)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn batch_update_products(
    state: State<AppState>,
    payload: BatchUpdateProductsRequest,
) -> ApiResponse<BatchUpdateResultDto> {
    let result = (|| {
        let conn = state.connection()?;
        product_service::batch_update_products(&conn, payload)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn find_product_by_barcode(
    state: State<AppState>,
    barcode: String,
) -> ApiResponse<Option<ProductDto>> {
    let result = (|| {
        let conn = state.connection()?;
        product_service::find_product_by_barcode(&conn, &barcode)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_customers(
    state: State<AppState>,
    filter: Option<ListCustomersRequest>,
) -> ApiResponse<Vec<CustomerDto>> {
    let result = (|| {
        let conn = state.connection()?;
        logger::info(
            "customer",
            format!(
                "list_customers filter_keys={}",
                customer_filter_keys(filter.as_ref()).join(",")
            ),
        );
        customer_service::list_customers(&conn, filter)
    })();
    if let Ok(items) = &result {
        logger::info(
            "customer",
            format!("list_customers result count={}", items.len()),
        );
    }
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn create_customer(
    state: State<AppState>,
    payload: CustomerPayload,
) -> ApiResponse<CustomerDto> {
    logger::info("customer", "create_customer start");
    let result = (|| {
        let conn = state.connection()?;
        customer_service::create_customer(&conn, payload)
    })();
    if let Ok(customer) = &result {
        logger::info(
            "customer",
            format!(
                "create_customer success id={} has_region={}",
                customer.id,
                customer.region.is_some()
            ),
        );
    }
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn update_customer(
    state: State<AppState>,
    id: i64,
    payload: CustomerPayload,
) -> ApiResponse<CustomerDto> {
    logger::info("customer", format!("update_customer start id={id}"));
    let result = (|| {
        let conn = state.connection()?;
        customer_service::update_customer(&conn, id, payload)
    })();
    if let Ok(customer) = &result {
        logger::info(
            "customer",
            format!(
                "update_customer success id={} has_region={}",
                customer.id,
                customer.region.is_some()
            ),
        );
    }
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn disable_customer(state: State<AppState>, id: i64) -> ApiResponse<bool> {
    let result = (|| {
        let conn = state.connection()?;
        customer_service::disable_customer(&conn, id)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn batch_update_customers(
    state: State<AppState>,
    payload: BatchUpdateCustomersRequest,
) -> ApiResponse<BatchUpdateResultDto> {
    let result = (|| {
        let conn = state.connection()?;
        customer_service::batch_update_customers(&conn, payload)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_suppliers(
    state: State<AppState>,
    filter: Option<ListSuppliersRequest>,
) -> ApiResponse<Vec<SupplierDto>> {
    let result = (|| {
        let conn = state.connection()?;
        supplier_service::list_suppliers(&conn, filter)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn create_supplier(
    state: State<AppState>,
    payload: SupplierPayload,
) -> ApiResponse<SupplierDto> {
    let result = (|| {
        let conn = state.connection()?;
        supplier_service::create_supplier(&conn, payload)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn update_supplier(
    state: State<AppState>,
    id: i64,
    payload: SupplierPayload,
) -> ApiResponse<SupplierDto> {
    let result = (|| {
        let conn = state.connection()?;
        supplier_service::update_supplier(&conn, id, payload)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn disable_supplier(state: State<AppState>, id: i64) -> ApiResponse<bool> {
    let result = (|| {
        let conn = state.connection()?;
        supplier_service::disable_supplier(&conn, id)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn batch_update_suppliers(
    state: State<AppState>,
    payload: BatchUpdateSuppliersRequest,
) -> ApiResponse<BatchUpdateResultDto> {
    let result = (|| {
        let conn = state.connection()?;
        supplier_service::batch_update_suppliers(&conn, payload)
    })();
    result.map(ok).unwrap_or_else(fail)
}

fn product_filter_keys(filter: Option<&ListProductsRequest>) -> Vec<&'static str> {
    let Some(filter) = filter else {
        return Vec::new();
    };
    let mut keys = Vec::new();
    if filter.category.is_some() {
        keys.push("category");
    }
    if filter.keyword.is_some() {
        keys.push("keyword");
    }
    if filter.only_low_stock.is_some() {
        keys.push("onlyLowStock");
    }
    if filter.only_in_stock.is_some() {
        keys.push("onlyInStock");
    }
    if filter.is_active.is_some() {
        keys.push("isActive");
    }
    keys
}

fn customer_filter_keys(filter: Option<&ListCustomersRequest>) -> Vec<&'static str> {
    let Some(filter) = filter else {
        return Vec::new();
    };
    let mut keys = Vec::new();
    if filter.region.is_some() {
        keys.push("region");
    }
    if filter.keyword.is_some() {
        keys.push("keyword");
    }
    if filter.is_active.is_some() {
        keys.push("isActive");
    }
    keys
}
