use crate::db;
use crate::models::*;
use crate::utils::{money, normalize_user_file_path, now_text};
use anyhow::{anyhow, Context};
use calamine::{open_workbook_auto, Data, Reader};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;
use std::path::Path;

pub fn setup_status(conn: &Connection) -> anyhow::Result<SetupStatusDto> {
    Ok(SetupStatusDto {
        completed: setting_bool(conn, "setup_completed", false)?,
        merchant_name: setting_text(conn, "merchant_name", "我的商行")?,
        industry_template: setting_text(conn, "industry_template", "general_wholesale")?,
        product_count: count(conn, "products")?,
        customer_count: count(conn, "customers")?,
        order_count: count(conn, "orders")?,
    })
}

pub fn complete_setup(conn: &Connection, request: CompleteSetupRequest) -> anyhow::Result<bool> {
    save_merchant_profile(conn, request.merchant)?;
    if let Some(template_id) = request.industry_template {
        apply_industry_template(
            conn,
            ApplyIndustryTemplateRequest {
                template_id,
                overwrite_terms: Some(request.terms.is_none()),
                overwrite_features: Some(true),
            },
        )?;
    }
    if let Some(terms) = request.terms {
        save_term_settings(conn, terms)?;
    }
    if let Some(features) = request.features {
        save_feature_flags(conn, features)?;
    }
    if let Some(value) = request.default_print_template {
        db::set_setting(conn, "active_order_template", &value)?;
        db::set_setting(conn, "default_print_template", &value)?;
    }
    if let Some(value) = request.default_export_format {
        db::set_setting(conn, "default_export_format", &value)?;
    }
    if let Some(value) = request.default_printer {
        db::set_setting(conn, "default_printer", &value)?;
    }
    db::set_setting(conn, "setup_completed", "true")?;
    Ok(true)
}

pub fn merchant_profile(conn: &Connection) -> anyhow::Result<MerchantProfileDto> {
    Ok(MerchantProfileDto {
        name: setting_text(conn, "merchant_name", "我的商行")?,
        contact: setting_optional(conn, "merchant_contact")?,
        phone: setting_optional(conn, "merchant_phone")?,
        address: setting_optional(conn, "merchant_address")?,
        logo_path: setting_optional(conn, "merchant_logo_path")?,
        remark: setting_optional(conn, "merchant_remark")?,
    })
}

pub fn save_merchant_profile(
    conn: &Connection,
    profile: MerchantProfileDto,
) -> anyhow::Result<bool> {
    let name = clean_required(&profile.name, "商户名称")?;
    db::set_setting(conn, "merchant_name", &name)?;
    db::set_setting(conn, "template_store_name", &name)?;
    db::set_setting(
        conn,
        "merchant_contact",
        optional_value(profile.contact).as_deref().unwrap_or(""),
    )?;
    db::set_setting(
        conn,
        "merchant_phone",
        optional_value(profile.phone).as_deref().unwrap_or(""),
    )?;
    db::set_setting(
        conn,
        "merchant_address",
        optional_value(profile.address).as_deref().unwrap_or(""),
    )?;
    db::set_setting(
        conn,
        "merchant_logo_path",
        optional_value(profile.logo_path).as_deref().unwrap_or(""),
    )?;
    db::set_setting(
        conn,
        "merchant_remark",
        optional_value(profile.remark).as_deref().unwrap_or(""),
    )?;
    Ok(true)
}

pub fn term_settings(conn: &Connection) -> anyhow::Result<TermSettingsDto> {
    Ok(TermSettingsDto {
        customer: setting_text(conn, "term_customer", "客户")?,
        region: setting_text(conn, "term_region", "地区")?,
        product: setting_text(conn, "term_product", "商品")?,
        category: setting_text(conn, "term_category", "类别")?,
        rule: setting_text(conn, "term_rule", "价格规则")?,
        credit: setting_text(conn, "term_credit", "返利额度")?,
        guest_customer: setting_text(conn, "guest_customer_name", db::GUEST_CUSTOMER_NAME)?,
    })
}

pub fn save_term_settings(conn: &Connection, terms: TermSettingsDto) -> anyhow::Result<bool> {
    db::set_setting(
        conn,
        "term_customer",
        &clean_required(&terms.customer, "客户显示名")?,
    )?;
    db::set_setting(
        conn,
        "term_region",
        &clean_required(&terms.region, "地区显示名")?,
    )?;
    db::set_setting(
        conn,
        "term_product",
        &clean_required(&terms.product, "商品显示名")?,
    )?;
    db::set_setting(
        conn,
        "term_category",
        &clean_required(&terms.category, "类别显示名")?,
    )?;
    db::set_setting(
        conn,
        "term_rule",
        &clean_required(&terms.rule, "规则显示名")?,
    )?;
    db::set_setting(
        conn,
        "term_credit",
        &clean_required(&terms.credit, "额度显示名")?,
    )?;
    db::set_setting(
        conn,
        "guest_customer_name",
        &clean_required(&terms.guest_customer, "默认客户显示名")?,
    )?;
    db::ensure_guest_customer(conn)?;
    Ok(true)
}

pub fn feature_flags(conn: &Connection) -> anyhow::Result<FeatureFlagsDto> {
    Ok(FeatureFlagsDto {
        supplier_ledger: setting_bool(conn, "feature_supplier_ledger", true)?,
        customer_rules: setting_bool(conn, "feature_customer_rules", true)?,
        monthly_credit: setting_bool(conn, "feature_monthly_credit", true)?,
        receivables: setting_bool(conn, "feature_receivables", true)?,
        product_ranking: setting_bool(conn, "feature_product_ranking", true)?,
        customer_analysis: setting_bool(conn, "feature_customer_analysis", true)?,
        inventory_control: setting_bool(conn, "feature_inventory_control", true)?,
        diagnostics: setting_bool(conn, "feature_diagnostics", true)?,
    })
}

pub fn save_feature_flags(conn: &Connection, flags: FeatureFlagsDto) -> anyhow::Result<bool> {
    set_bool(conn, "feature_supplier_ledger", flags.supplier_ledger)?;
    set_bool(conn, "feature_customer_rules", flags.customer_rules)?;
    set_bool(conn, "feature_monthly_credit", flags.monthly_credit)?;
    set_bool(conn, "feature_receivables", flags.receivables)?;
    set_bool(conn, "feature_product_ranking", flags.product_ranking)?;
    set_bool(conn, "feature_customer_analysis", flags.customer_analysis)?;
    set_bool(conn, "feature_inventory_control", flags.inventory_control)?;
    set_bool(conn, "feature_diagnostics", flags.diagnostics)?;
    Ok(true)
}

pub fn industry_templates() -> Vec<IndustryTemplateDto> {
    vec![
        IndustryTemplateDto {
            id: "general_wholesale".to_string(),
            name: "通用批发".to_string(),
            description: "适合多客户、多商品、按客户价格出库的批发经营。".to_string(),
            terms: terms(
                "客户",
                "地区",
                "商品",
                "类别",
                "价格规则",
                "返利额度",
                "散客",
            ),
            features: features([true, true, true, true, true, true, true, true]),
            order_template: "general".to_string(),
        },
        IndustryTemplateDto {
            id: "delivery_wholesale".to_string(),
            name: "配送批发".to_string(),
            description: "适合按线路或片区给门店配送的经营方式。".to_string(),
            terms: terms(
                "门店",
                "线路",
                "商品",
                "品类",
                "客户价规则",
                "返利额度",
                "临时客户",
            ),
            features: features([true, true, true, true, true, true, true, true]),
            order_template: "delivery".to_string(),
        },
        IndustryTemplateDto {
            id: "retail_outbound".to_string(),
            name: "零售出库".to_string(),
            description: "适合临时客户较多、价格规则较少的零售出库场景。".to_string(),
            terms: terms(
                "客户",
                "区域",
                "商品",
                "分类",
                "价格规则",
                "抵扣额度",
                "零售客户",
            ),
            features: features([false, false, false, true, true, false, true, true]),
            order_template: "simple".to_string(),
        },
        IndustryTemplateDto {
            id: "hardware_general".to_string(),
            name: "五金百货".to_string(),
            description: "适合商品多、分类多、条码和库存预警常用的经营方式。".to_string(),
            terms: terms(
                "客户",
                "区域",
                "货品",
                "分类",
                "客户价规则",
                "抵扣额度",
                "散客",
            ),
            features: features([true, true, false, true, true, true, true, true]),
            order_template: "general".to_string(),
        },
        IndustryTemplateDto {
            id: "blank".to_string(),
            name: "空白模板".to_string(),
            description: "只启用基础库存、客户、入库和出库，其他能力可手动开启。".to_string(),
            terms: terms(
                "客户",
                "地区",
                "商品",
                "类别",
                "价格规则",
                "抵扣额度",
                "散客",
            ),
            features: features([false, false, false, false, false, false, false, true]),
            order_template: "general".to_string(),
        },
    ]
}

pub fn apply_industry_template(
    conn: &Connection,
    request: ApplyIndustryTemplateRequest,
) -> anyhow::Result<IndustryTemplateDto> {
    let template = industry_templates()
        .into_iter()
        .find(|item| item.id == request.template_id)
        .ok_or_else(|| anyhow!("行业模板不存在：{}", request.template_id))?;
    db::set_setting(conn, "industry_template", &template.id)?;
    db::set_setting(conn, "active_order_template", &template.order_template)?;
    if request.overwrite_terms.unwrap_or(true) {
        save_term_settings(conn, template.terms.clone())?;
    }
    if request.overwrite_features.unwrap_or(true) {
        save_feature_flags(conn, template.features.clone())?;
    }
    Ok(template)
}

pub fn document_templates(conn: &Connection) -> anyhow::Result<Vec<DocumentTemplateDto>> {
    let active = setting_text(conn, "active_order_template", "general")?;
    Ok(vec![
        doc_template(
            "general",
            "通用出库单",
            "适合大多数批发和配送场景。",
            &active,
        ),
        doc_template(
            "delivery",
            "配送出库单",
            "适合按线路配送给门店的出库场景。",
            &active,
        ),
        doc_template(
            "kezhan_legacy",
            "科展兼容模板",
            "保留原 Excel 打印区域核心样式。",
            &active,
        ),
        doc_template(
            "simple",
            "简洁出库单",
            "弱化促销和补贴信息，适合零售出库。",
            &active,
        ),
        doc_template(
            "statement",
            "客户对账单",
            "用于客户期间对账和收款沟通。",
            &active,
        ),
    ])
}

pub fn apply_document_template(conn: &Connection, template_id: String) -> anyhow::Result<bool> {
    let exists = document_templates(conn)?
        .iter()
        .any(|item| item.id == template_id);
    if !exists {
        anyhow::bail!("单据模板不存在：{template_id}");
    }
    db::set_setting(conn, "active_order_template", &template_id)?;
    db::set_setting(conn, "default_print_template", &template_id)?;
    Ok(true)
}

pub fn preview_generic_import(
    conn: &Connection,
    request: GenericImportRequest,
) -> anyhow::Result<GenericImportPreviewDto> {
    let rows = parse_generic_import_rows(conn, &request)?;
    Ok(generic_preview(request.import_type, rows))
}

pub fn preview_generic_import_headers(
    request: GenericImportHeaderRequest,
) -> anyhow::Result<GenericImportHeadersDto> {
    let import_type = request.import_type.trim();
    ensure_supported_generic_import_type(import_type)?;
    let normalized_path = normalize_user_file_path(&request.file_path);
    let mut workbook = open_workbook_auto(&normalized_path)
        .with_context(|| format!("无法打开 Excel 文件：{normalized_path}"))?;
    let sheet_name = workbook
        .sheet_names()
        .first()
        .cloned()
        .ok_or_else(|| anyhow!("Excel 中没有可读取的工作表"))?;
    let range = workbook.worksheet_range(&sheet_name)?;
    let header_row = range
        .rows()
        .next()
        .ok_or_else(|| anyhow!("通用导入表缺少表头"))?;
    let headers = header_row
        .iter()
        .map(|cell| cell_text(Some(cell)))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    let fields = generic_import_fields(import_type)?;
    let suggested_mapping = suggest_field_mapping(&headers, &fields);

    Ok(GenericImportHeadersDto {
        import_type: import_type.to_string(),
        sheet_name,
        headers,
        fields,
        suggested_mapping,
    })
}

pub fn confirm_generic_import(
    conn: &Connection,
    request: GenericImportRequest,
) -> anyhow::Result<GenericImportResultDto> {
    let strategy = duplicate_strategy(request.duplicate_strategy.as_deref())?;
    let parsed = parse_generic_import_rows(conn, &request)?;
    let mut rows = Vec::new();
    let mut imported_count = 0;
    for mut row in parsed {
        if row.status != "valid" {
            rows.push(row);
            continue;
        }
        let result = match request.import_type.as_str() {
            "products" => import_product_row(conn, &row, strategy),
            "customers" => import_customer_row(conn, &row, strategy),
            "initial_stock" => import_initial_stock_row(conn, &row, strategy),
            _ => Err(anyhow!("不支持的通用导入类型：{}", request.import_type)),
        };
        match result {
            Ok(()) => {
                imported_count += 1;
                row.status = "imported".to_string();
                row.message = Some("已导入".to_string());
            }
            Err(error) => {
                row.status = "error".to_string();
                row.action = "skip".to_string();
                row.message = Some(error.to_string());
            }
        }
        rows.push(row);
    }
    let create_count = rows
        .iter()
        .filter(|row| {
            row.status == "imported" && matches!(row.action.as_str(), "create" | "append_suffix")
        })
        .count() as i64;
    let overwrite_count = rows
        .iter()
        .filter(|row| row.status == "imported" && row.action == "overwrite")
        .count() as i64;
    let error_count = rows.iter().filter(|row| row.status == "error").count() as i64;
    let skipped_count = rows.iter().filter(|row| row.status == "skipped").count() as i64;
    Ok(GenericImportResultDto {
        import_type: request.import_type,
        imported_count,
        create_count,
        overwrite_count,
        error_count,
        skipped_count,
        rows,
    })
}

pub fn export_generic_import_report(
    output_path: &Path,
    request: GenericImportReportRequest,
) -> anyhow::Result<String> {
    let mut book = umya_spreadsheet::new_file();
    let sheet = book
        .get_sheet_by_name_mut("Sheet1")
        .ok_or_else(|| anyhow!("无法创建导入报告工作表"))?;
    sheet.set_name("导入报告");
    let title = request.title.trim();
    let title = if title.is_empty() {
        "通用导入报告"
    } else {
        title
    };
    sheet.get_cell_mut("A1").set_value(title);
    let headers = [
        "行号",
        "动作",
        "状态",
        "说明",
        "名称",
        "类别",
        "地区",
        "条码",
        "默认售价",
        "安全库存",
        "单位",
        "地址",
        "电话",
        "数量",
        "单价/成本",
        "备注",
    ];
    for (index, header) in headers.iter().enumerate() {
        sheet
            .get_cell_mut(((index + 1) as u32, 2_u32))
            .set_value(*header);
    }
    for (row_index, row) in request.rows.iter().enumerate() {
        let excel_row = row_index as u32 + 3;
        let values = [
            row.row_number.to_string(),
            import_action_text(&row.action).to_string(),
            import_status_text(&row.status).to_string(),
            row.message.clone().unwrap_or_default(),
            row.name.clone().unwrap_or_default(),
            row.category.clone().unwrap_or_default(),
            row.region.clone().unwrap_or_default(),
            row.barcode.clone().unwrap_or_default(),
            row.default_price
                .map(|value| value.to_string())
                .unwrap_or_default(),
            row.safety_stock
                .map(|value| value.to_string())
                .unwrap_or_default(),
            row.unit.clone().unwrap_or_default(),
            row.address.clone().unwrap_or_default(),
            row.phone.clone().unwrap_or_default(),
            row.quantity
                .map(|value| value.to_string())
                .unwrap_or_default(),
            row.unit_price
                .map(|value| value.to_string())
                .unwrap_or_default(),
            row.remark.clone().unwrap_or_default(),
        ];
        for (column_index, value) in values.iter().enumerate() {
            sheet
                .get_cell_mut(((column_index + 1) as u32, excel_row))
                .set_value(value);
        }
    }
    umya_spreadsheet::writer::xlsx::write(&book, output_path)?;
    Ok(output_path.to_string_lossy().to_string())
}

pub fn export_generic_import_template(
    output_path: &Path,
    import_type: &str,
) -> anyhow::Result<String> {
    let spec = generic_template_spec(import_type)?;
    let mut book = umya_spreadsheet::new_file();
    let sheet = book
        .get_sheet_by_name_mut("Sheet1")
        .ok_or_else(|| anyhow!("无法创建导入模板工作表"))?;
    sheet.set_name(spec.sheet_name);
    for (index, header) in spec.headers.iter().enumerate() {
        sheet
            .get_cell_mut(((index + 1) as u32, 1_u32))
            .set_value(*header);
    }
    for (row_index, row) in spec.examples.iter().enumerate() {
        for (column_index, value) in row.iter().enumerate() {
            sheet
                .get_cell_mut(((column_index + 1) as u32, (row_index + 2) as u32))
                .set_value(*value);
        }
    }
    umya_spreadsheet::writer::xlsx::write(&book, output_path)?;
    Ok(output_path.to_string_lossy().to_string())
}

pub fn save_import_mapping(conn: &Connection, mapping: ImportMappingDto) -> anyhow::Result<bool> {
    if mapping.name.trim().is_empty() {
        anyhow::bail!("映射方案名称不能为空");
    }
    if mapping.import_type.trim().is_empty() {
        anyhow::bail!("映射导入类型不能为空");
    }
    let key = format!(
        "import_mapping_{}_{}",
        sanitize_key(&mapping.import_type),
        sanitize_key(&mapping.name)
    );
    db::set_setting(conn, &key, &serde_json::to_string(&mapping)?)?;
    Ok(true)
}

pub fn list_import_mappings(
    conn: &Connection,
    import_type: Option<String>,
) -> anyhow::Result<Vec<ImportMappingDto>> {
    let mut stmt = conn.prepare("SELECT value FROM settings WHERE key LIKE 'import_mapping_%'")?;
    let values = stmt
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    let mut mappings = Vec::new();
    for value in values {
        if let Ok(mapping) = serde_json::from_str::<ImportMappingDto>(&value) {
            if import_type
                .as_deref()
                .is_none_or(|kind| kind == mapping.import_type)
            {
                mappings.push(mapping);
            }
        }
    }
    mappings.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(mappings)
}

struct GenericTemplateSpec {
    sheet_name: &'static str,
    headers: Vec<&'static str>,
    examples: Vec<Vec<&'static str>>,
}

fn generic_template_spec(import_type: &str) -> anyhow::Result<GenericTemplateSpec> {
    match import_type {
        "products" => Ok(GenericTemplateSpec {
            sheet_name: "商品导入模板",
            headers: vec![
                "商品名称",
                "类别",
                "条码",
                "默认售价",
                "安全库存",
                "单位",
                "备注",
            ],
            examples: vec![
                vec![
                    "示例商品A",
                    "饮料",
                    "690000000001",
                    "12.5",
                    "10",
                    "件",
                    "示例行，可删除",
                ],
                vec!["示例商品B", "日化", "", "8", "5", "箱", ""],
            ],
        }),
        "customers" => Ok(GenericTemplateSpec {
            sheet_name: "客户导入模板",
            headers: vec!["客户名称", "地区", "地址", "电话", "备注"],
            examples: vec![
                vec![
                    "示例客户A",
                    "东线",
                    "示例地址1",
                    "13800000000",
                    "示例行，可删除",
                ],
                vec!["示例客户B", "西线", "示例地址2", "", ""],
            ],
        }),
        "initial_stock" => Ok(GenericTemplateSpec {
            sheet_name: "期初库存导入模板",
            headers: vec!["商品名称", "条码", "期初库存", "期初成本", "备注"],
            examples: vec![
                vec![
                    "示例商品A",
                    "690000000001",
                    "100",
                    "6.5",
                    "需先导入商品资料",
                ],
                vec!["示例商品B", "", "50", "3", ""],
            ],
        }),
        other => anyhow::bail!("不支持的通用导入模板类型：{other}"),
    }
}

fn parse_generic_import_rows(
    conn: &Connection,
    request: &GenericImportRequest,
) -> anyhow::Result<Vec<GenericImportRowDto>> {
    let import_type = request.import_type.trim();
    ensure_supported_generic_import_type(import_type)?;
    let normalized_path = normalize_user_file_path(&request.file_path);
    let mut workbook = open_workbook_auto(&normalized_path)
        .with_context(|| format!("无法打开 Excel 文件：{normalized_path}"))?;
    let sheet_name = workbook
        .sheet_names()
        .first()
        .cloned()
        .ok_or_else(|| anyhow!("Excel 中没有可读取的工作表"))?;
    let range = workbook.worksheet_range(&sheet_name)?;
    let mut rows = range.rows();
    let header_row = rows.next().ok_or_else(|| anyhow!("通用导入表缺少表头"))?;
    let headers = generic_headers(header_row, request.field_mapping.as_ref());
    let mut output = Vec::new();
    for (index, row) in rows.enumerate() {
        let row_number = index as i64 + 2;
        let parsed = match import_type {
            "products" => parse_product_import_row(conn, row, row_number, &headers, request),
            "customers" => parse_customer_import_row(conn, row, row_number, &headers, request),
            "initial_stock" => {
                parse_initial_stock_import_row(conn, row, row_number, &headers, request)
            }
            _ => unreachable!(),
        };
        output.push(parsed);
    }
    Ok(output)
}

fn parse_product_import_row(
    conn: &Connection,
    row: &[Data],
    row_number: i64,
    headers: &HashMap<String, usize>,
    request: &GenericImportRequest,
) -> GenericImportRowDto {
    let strategy = duplicate_strategy(request.duplicate_strategy.as_deref())
        .unwrap_or(DuplicateStrategy::Skip);
    let name = generic_text(
        row,
        headers,
        &["商品名称", "商品", "品名", "货品", "product_name", "name"],
    );
    let category = generic_text(row, headers, &["类别", "分类", "品类", "category"])
        .or_else(|| Some("其他".to_string()));
    let barcode = generic_text(row, headers, &["条码", "barcode"]);
    let default_price =
        generic_number(row, headers, &["默认售价", "售价", "价格", "default_price"]);
    let safety_stock = generic_number(row, headers, &["安全库存", "库存预警", "safety_stock"]);
    let unit = generic_text(row, headers, &["单位", "unit"]);
    let remark = generic_text(row, headers, &["备注", "remark"]);
    let mut dto = import_row(ImportRowInput {
        row_number,
        name,
        category,
        barcode,
        default_price,
        safety_stock,
        unit,
        remark,
        ..ImportRowInput::default()
    });
    if row_is_empty(&dto) {
        dto.status = "skipped".to_string();
        dto.message = Some("空行".to_string());
        return dto;
    }
    if dto.name.as_deref().unwrap_or("").trim().is_empty() {
        dto.status = "error".to_string();
        dto.message = Some("商品名称必填".to_string());
        return dto;
    }
    if dto.category.as_deref().unwrap_or("").trim().is_empty() {
        dto.category = Some("其他".to_string());
    }
    if dto.default_price.is_some_and(|value| value < 0.0) {
        dto.status = "error".to_string();
        dto.message = Some("默认售价不能小于 0".to_string());
        return dto;
    }
    let exists = product_exists(
        conn,
        dto.name.as_deref().unwrap_or(""),
        dto.barcode.as_deref(),
    )
    .unwrap_or(false);
    set_duplicate_action(&mut dto, exists, strategy);
    dto
}

fn parse_customer_import_row(
    conn: &Connection,
    row: &[Data],
    row_number: i64,
    headers: &HashMap<String, usize>,
    request: &GenericImportRequest,
) -> GenericImportRowDto {
    let strategy = duplicate_strategy(request.duplicate_strategy.as_deref())
        .unwrap_or(DuplicateStrategy::Skip);
    let name = generic_text(
        row,
        headers,
        &[
            "客户名称",
            "客户",
            "客户单位",
            "门店",
            "customer_name",
            "name",
        ],
    );
    let region = generic_text(row, headers, &["地区", "区域", "线路", "片区", "region"]);
    let address = generic_text(row, headers, &["地址", "address"]);
    let phone = generic_text(row, headers, &["电话", "联系电话", "phone"]);
    let remark = generic_text(row, headers, &["备注", "remark"]);
    let mut dto = import_row(ImportRowInput {
        row_number,
        name,
        region,
        address,
        phone,
        remark,
        ..ImportRowInput::default()
    });
    if row_is_empty(&dto) {
        dto.status = "skipped".to_string();
        dto.message = Some("空行".to_string());
        return dto;
    }
    if dto.name.as_deref().unwrap_or("").trim().is_empty() {
        dto.status = "error".to_string();
        dto.message = Some("客户名称必填".to_string());
        return dto;
    }
    let exists = customer_exists(conn, dto.name.as_deref().unwrap_or("")).unwrap_or(false);
    set_duplicate_action(&mut dto, exists, strategy);
    dto
}

fn parse_initial_stock_import_row(
    conn: &Connection,
    row: &[Data],
    row_number: i64,
    headers: &HashMap<String, usize>,
    request: &GenericImportRequest,
) -> GenericImportRowDto {
    let strategy = duplicate_strategy(request.duplicate_strategy.as_deref())
        .unwrap_or(DuplicateStrategy::Overwrite);
    let name = generic_text(
        row,
        headers,
        &["商品名称", "商品", "品名", "product_name", "name"],
    );
    let barcode = generic_text(row, headers, &["条码", "barcode"]);
    let quantity = generic_number(row, headers, &["期初库存", "库存", "数量", "quantity"]);
    let unit_price = generic_number(
        row,
        headers,
        &["期初成本", "成本", "进货价", "unit_price", "cost"],
    );
    let remark = generic_text(row, headers, &["备注", "remark"]);
    let mut dto = import_row(ImportRowInput {
        row_number,
        name,
        barcode,
        quantity,
        unit_price,
        remark,
        ..ImportRowInput::default()
    });
    if row_is_empty(&dto) {
        dto.status = "skipped".to_string();
        dto.message = Some("空行".to_string());
        return dto;
    }
    if dto.name.as_deref().unwrap_or("").trim().is_empty()
        && dto.barcode.as_deref().unwrap_or("").trim().is_empty()
    {
        dto.status = "error".to_string();
        dto.message = Some("商品名称或条码必填".to_string());
        return dto;
    }
    if dto.quantity.is_none() {
        dto.status = "error".to_string();
        dto.message = Some("期初库存必填".to_string());
        return dto;
    }
    let exists = lookup_product_for_stock(conn, dto.name.as_deref(), dto.barcode.as_deref())
        .unwrap_or(None)
        .is_some();
    if !exists {
        dto.status = "error".to_string();
        dto.message = Some("商品不存在，需先导入商品资料".to_string());
        return dto;
    }
    set_duplicate_action(&mut dto, exists, strategy);
    dto
}

fn import_product_row(
    conn: &Connection,
    row: &GenericImportRowDto,
    strategy: DuplicateStrategy,
) -> anyhow::Result<()> {
    let now = now_text();
    let name = clean_required(row.name.as_deref().unwrap_or(""), "商品名称")?;
    let category = row
        .category
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("其他");
    let existing_id = find_product_id(conn, &name, row.barcode.as_deref())?;
    match (existing_id, strategy) {
        (Some(_), DuplicateStrategy::Skip) => Ok(()),
        (Some(id), DuplicateStrategy::Overwrite) => {
            conn.execute(
                "UPDATE products
                 SET name = ?1, category = ?2, barcode = ?3, default_price = ?4,
                     safety_stock = ?5, unit = ?6, remark = ?7, is_active = 1, updated_at = ?8
                 WHERE id = ?9",
                params![
                    name,
                    category,
                    optional_text_param(row.barcode.as_deref()),
                    row.default_price.unwrap_or(0.0),
                    row.safety_stock.unwrap_or(0.0),
                    optional_text_param(row.unit.as_deref()),
                    optional_text_param(row.remark.as_deref()),
                    now,
                    id
                ],
            )?;
            Ok(())
        }
        (Some(_), DuplicateStrategy::AppendSuffix) => {
            let unique_name = unique_text_value(conn, "products", "name", &name)?;
            conn.execute(
                "INSERT INTO products
                 (name, category, barcode, default_price, safety_stock, unit, is_active, remark, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?8, ?8)",
                params![
                    unique_name,
                    category,
                    Option::<String>::None,
                    row.default_price.unwrap_or(0.0),
                    row.safety_stock.unwrap_or(0.0),
                    optional_text_param(row.unit.as_deref()),
                    optional_text_param(row.remark.as_deref()),
                    now
                ],
            )?;
            let product_id = conn.last_insert_rowid();
            conn.execute(
                "INSERT OR IGNORE INTO stock_balances (product_id, current_stock, avg_cost, stock_value, updated_at)
                 VALUES (?1, 0, 0, 0, ?2)",
                params![product_id, now],
            )?;
            Ok(())
        }
        (None, _) => {
            conn.execute(
                "INSERT INTO products
                 (name, category, barcode, default_price, safety_stock, unit, is_active, remark, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, 1, ?7, ?8, ?8)",
                params![
                    name,
                    category,
                    optional_text_param(row.barcode.as_deref()),
                    row.default_price.unwrap_or(0.0),
                    row.safety_stock.unwrap_or(0.0),
                    optional_text_param(row.unit.as_deref()),
                    optional_text_param(row.remark.as_deref()),
                    now
                ],
            )?;
            let product_id = conn.last_insert_rowid();
            conn.execute(
                "INSERT OR IGNORE INTO stock_balances (product_id, current_stock, avg_cost, stock_value, updated_at)
                 VALUES (?1, 0, 0, 0, ?2)",
                params![product_id, now],
            )?;
            Ok(())
        }
    }
}

fn import_customer_row(
    conn: &Connection,
    row: &GenericImportRowDto,
    strategy: DuplicateStrategy,
) -> anyhow::Result<()> {
    let now = now_text();
    let name = clean_required(row.name.as_deref().unwrap_or(""), "客户名称")?;
    let existing_id = find_customer_id(conn, &name)?;
    match (existing_id, strategy) {
        (Some(_), DuplicateStrategy::Skip) => Ok(()),
        (Some(id), DuplicateStrategy::Overwrite) => {
            conn.execute(
                "UPDATE customers
                 SET region = ?1, name = ?2, address = ?3, phone = ?4, remark = ?5,
                     is_active = 1, updated_at = ?6
                 WHERE id = ?7",
                params![
                    optional_text_param(row.region.as_deref()),
                    name,
                    optional_text_param(row.address.as_deref()),
                    optional_text_param(row.phone.as_deref()),
                    optional_text_param(row.remark.as_deref()),
                    now,
                    id
                ],
            )?;
            Ok(())
        }
        (Some(_), DuplicateStrategy::AppendSuffix) => {
            let unique_name = unique_text_value(conn, "customers", "name", &name)?;
            conn.execute(
                "INSERT INTO customers
                 (region, name, address, phone, is_active, remark, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, ?6)",
                params![
                    optional_text_param(row.region.as_deref()),
                    unique_name,
                    optional_text_param(row.address.as_deref()),
                    optional_text_param(row.phone.as_deref()),
                    optional_text_param(row.remark.as_deref()),
                    now
                ],
            )?;
            Ok(())
        }
        (None, _) => {
            conn.execute(
                "INSERT INTO customers
                 (region, name, address, phone, is_active, remark, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, 1, ?5, ?6, ?6)",
                params![
                    optional_text_param(row.region.as_deref()),
                    name,
                    optional_text_param(row.address.as_deref()),
                    optional_text_param(row.phone.as_deref()),
                    optional_text_param(row.remark.as_deref()),
                    now
                ],
            )?;
            Ok(())
        }
    }
}

fn import_initial_stock_row(
    conn: &Connection,
    row: &GenericImportRowDto,
    _strategy: DuplicateStrategy,
) -> anyhow::Result<()> {
    let product_id = lookup_product_for_stock(conn, row.name.as_deref(), row.barcode.as_deref())?
        .ok_or_else(|| anyhow!("商品不存在，需先导入商品资料"))?;
    let quantity = row.quantity.ok_or_else(|| anyhow!("期初库存必填"))?;
    let unit_price = row.unit_price.unwrap_or(0.0);
    let now = now_text();
    conn.execute(
        "INSERT INTO inventory_movements
         (movement_date, product_id, movement_type, quantity, unit_price, amount, source_type, remark, created_at)
         VALUES (?1, ?2, 'initial_stock', ?3, ?4, ?5, 'generic_import', ?6, ?7)",
        params![
            chrono::Local::now().format("%Y-%m-%d").to_string(),
            product_id,
            quantity,
            unit_price,
            money(quantity * unit_price),
            optional_text_param(row.remark.as_deref()).unwrap_or_else(|| "通用导入期初库存".to_string()),
            now
        ],
    )?;
    db::recalc_stock_balance(conn, product_id)?;
    Ok(())
}

fn generic_preview(import_type: String, rows: Vec<GenericImportRowDto>) -> GenericImportPreviewDto {
    GenericImportPreviewDto {
        import_type,
        total_count: rows.len() as i64,
        valid_count: rows.iter().filter(|row| row.status == "valid").count() as i64,
        create_count: rows
            .iter()
            .filter(|row| {
                row.status == "valid" && matches!(row.action.as_str(), "create" | "append_suffix")
            })
            .count() as i64,
        overwrite_count: rows
            .iter()
            .filter(|row| row.status == "valid" && row.action == "overwrite")
            .count() as i64,
        error_count: rows.iter().filter(|row| row.status == "error").count() as i64,
        skipped_count: rows.iter().filter(|row| row.status == "skipped").count() as i64,
        rows,
    }
}

#[derive(Clone, Copy)]
enum DuplicateStrategy {
    Skip,
    Overwrite,
    AppendSuffix,
}

fn duplicate_strategy(value: Option<&str>) -> anyhow::Result<DuplicateStrategy> {
    match value.unwrap_or("skip") {
        "skip" => Ok(DuplicateStrategy::Skip),
        "overwrite" => Ok(DuplicateStrategy::Overwrite),
        "append_suffix" => Ok(DuplicateStrategy::AppendSuffix),
        other => anyhow::bail!("不支持的重复数据处理策略：{other}"),
    }
}

fn set_duplicate_action(row: &mut GenericImportRowDto, exists: bool, strategy: DuplicateStrategy) {
    if exists {
        match strategy {
            DuplicateStrategy::Skip => {
                row.action = "skip".to_string();
                row.status = "skipped".to_string();
                row.message = Some("重复数据已跳过".to_string());
            }
            DuplicateStrategy::Overwrite => {
                row.action = "overwrite".to_string();
                row.status = "valid".to_string();
                row.message = None;
            }
            DuplicateStrategy::AppendSuffix => {
                row.action = "append_suffix".to_string();
                row.status = "valid".to_string();
                row.message = Some("重复数据将追加后缀新增".to_string());
            }
        }
    } else {
        row.action = "create".to_string();
        row.status = "valid".to_string();
        row.message = None;
    }
}

fn import_action_text(action: &str) -> &str {
    match action {
        "create" => "新增",
        "overwrite" => "覆盖",
        "append_suffix" => "追加后缀",
        "skip" => "跳过",
        _ => action,
    }
}

fn import_status_text(status: &str) -> &str {
    match status {
        "valid" => "有效",
        "imported" => "已导入",
        "error" => "异常",
        "skipped" => "跳过",
        _ => status,
    }
}

#[derive(Default)]
struct ImportRowInput {
    row_number: i64,
    name: Option<String>,
    category: Option<String>,
    region: Option<String>,
    barcode: Option<String>,
    default_price: Option<f64>,
    safety_stock: Option<f64>,
    unit: Option<String>,
    address: Option<String>,
    phone: Option<String>,
    quantity: Option<f64>,
    unit_price: Option<f64>,
    remark: Option<String>,
}

fn import_row(input: ImportRowInput) -> GenericImportRowDto {
    GenericImportRowDto {
        row_number: input.row_number,
        action: "skip".to_string(),
        status: "skipped".to_string(),
        message: Some("空行".to_string()),
        name: input.name,
        category: input.category,
        region: input.region,
        barcode: input.barcode,
        default_price: input.default_price,
        safety_stock: input.safety_stock,
        unit: input.unit,
        address: input.address,
        phone: input.phone,
        quantity: input.quantity,
        unit_price: input.unit_price,
        remark: input.remark,
    }
}

fn row_is_empty(row: &GenericImportRowDto) -> bool {
    [
        row.name.as_deref(),
        row.category.as_deref(),
        row.region.as_deref(),
        row.barcode.as_deref(),
        row.unit.as_deref(),
        row.address.as_deref(),
        row.phone.as_deref(),
        row.remark.as_deref(),
    ]
    .into_iter()
    .flatten()
    .all(|value| value.trim().is_empty())
        && row.default_price.is_none()
        && row.safety_stock.is_none()
        && row.quantity.is_none()
        && row.unit_price.is_none()
}

fn generic_headers(
    header_row: &[Data],
    field_mapping: Option<&HashMap<String, String>>,
) -> HashMap<String, usize> {
    let mut headers = HashMap::new();
    for (index, cell) in header_row.iter().enumerate() {
        let raw = cell_text(Some(cell));
        let normalized = normalize_key(&raw);
        if !normalized.is_empty() {
            headers.insert(normalized.clone(), index);
        }
        if let Some(mapping) = field_mapping {
            for (system_field, excel_header) in mapping {
                if normalize_key(excel_header) == normalized {
                    headers.insert(normalize_key(system_field), index);
                }
            }
        }
    }
    headers
}

fn ensure_supported_generic_import_type(import_type: &str) -> anyhow::Result<()> {
    if !matches!(import_type, "products" | "customers" | "initial_stock") {
        anyhow::bail!("不支持的通用导入类型：{import_type}");
    }
    Ok(())
}

fn generic_import_fields(import_type: &str) -> anyhow::Result<Vec<GenericImportFieldDto>> {
    match import_type {
        "products" => Ok(vec![
            import_field(
                "商品名称",
                true,
                &["商品", "品名", "货品", "product_name", "name"],
            ),
            import_field("类别", false, &["分类", "品类", "分组", "category"]),
            import_field("条码", false, &["barcode"]),
            import_field("默认售价", false, &["售价", "价格", "default_price"]),
            import_field("安全库存", false, &["库存预警", "safety_stock"]),
            import_field("单位", false, &["unit"]),
            import_field("备注", false, &["remark"]),
        ]),
        "customers" => Ok(vec![
            import_field(
                "客户名称",
                true,
                &["客户", "客户单位", "门店", "customer_name", "name"],
            ),
            import_field("地区", false, &["区域", "线路", "片区", "region"]),
            import_field("地址", false, &["address"]),
            import_field("电话", false, &["联系电话", "phone"]),
            import_field("备注", false, &["remark"]),
        ]),
        "initial_stock" => Ok(vec![
            import_field("商品名称", false, &["商品", "品名", "product_name", "name"]),
            import_field("条码", false, &["barcode"]),
            import_field("期初库存", true, &["库存", "数量", "quantity"]),
            import_field("期初成本", false, &["成本", "进货价", "unit_price", "cost"]),
            import_field("备注", false, &["remark"]),
        ]),
        _ => anyhow::bail!("不支持的通用导入类型：{import_type}"),
    }
}

fn import_field(name: &str, required: bool, aliases: &[&str]) -> GenericImportFieldDto {
    GenericImportFieldDto {
        name: name.to_string(),
        required,
        aliases: aliases.iter().map(|item| item.to_string()).collect(),
    }
}

fn suggest_field_mapping(
    headers: &[String],
    fields: &[GenericImportFieldDto],
) -> HashMap<String, String> {
    let normalized_headers = headers
        .iter()
        .map(|header| (normalize_key(header), header.clone()))
        .collect::<Vec<_>>();
    let mut mapping = HashMap::new();
    for field in fields {
        let mut candidates = vec![field.name.as_str()];
        candidates.extend(field.aliases.iter().map(String::as_str));
        if let Some((_, header)) = candidates.iter().find_map(|candidate| {
            let normalized = normalize_key(candidate);
            normalized_headers
                .iter()
                .find(|(header_key, _)| *header_key == normalized)
        }) {
            mapping.insert(field.name.clone(), header.clone());
        }
    }
    mapping
}

fn generic_text(row: &[Data], headers: &HashMap<String, usize>, names: &[&str]) -> Option<String> {
    names
        .iter()
        .find_map(|name| headers.get(&normalize_key(name)))
        .and_then(|index| row.get(*index))
        .map(|cell| cell_text(Some(cell)))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn generic_number(row: &[Data], headers: &HashMap<String, usize>, names: &[&str]) -> Option<f64> {
    names
        .iter()
        .find_map(|name| headers.get(&normalize_key(name)))
        .and_then(|index| row.get(*index))
        .and_then(|cell| cell_number(Some(cell)))
}

fn product_exists(conn: &Connection, name: &str, barcode: Option<&str>) -> anyhow::Result<bool> {
    Ok(find_product_id(conn, name, barcode)?.is_some())
}

fn customer_exists(conn: &Connection, name: &str) -> anyhow::Result<bool> {
    Ok(find_customer_id(conn, name)?.is_some())
}

fn find_product_id(
    conn: &Connection,
    name: &str,
    barcode: Option<&str>,
) -> anyhow::Result<Option<i64>> {
    if let Some(barcode) = barcode.filter(|value| !value.trim().is_empty()) {
        let id = conn
            .query_row(
                "SELECT id FROM products WHERE barcode = ?1 ORDER BY is_active DESC, id LIMIT 1",
                [barcode.trim()],
                |row| row.get(0),
            )
            .optional()?;
        if id.is_some() {
            return Ok(id);
        }
    }
    conn.query_row(
        "SELECT id FROM products WHERE name = ?1 ORDER BY is_active DESC, id LIMIT 1",
        [name.trim()],
        |row| row.get(0),
    )
    .optional()
    .map_err(Into::into)
}

fn find_customer_id(conn: &Connection, name: &str) -> anyhow::Result<Option<i64>> {
    conn.query_row(
        "SELECT id FROM customers WHERE name = ?1 ORDER BY is_active DESC, id LIMIT 1",
        [name.trim()],
        |row| row.get(0),
    )
    .optional()
    .map_err(Into::into)
}

fn unique_text_value(
    conn: &Connection,
    table: &str,
    column: &str,
    base_value: &str,
) -> anyhow::Result<String> {
    let (table, column) = match (table, column) {
        ("products", "name") => ("products", "name"),
        ("customers", "name") => ("customers", "name"),
        _ => anyhow::bail!("不支持的唯一文本字段：{table}.{column}"),
    };
    let base_value = clean_required(base_value, "名称")?;
    for index in 1..=9999 {
        let candidate = format!("{base_value}_{index}");
        let sql = format!("SELECT COUNT(*) FROM {table} WHERE {column} = ?1");
        let count: i64 = conn.query_row(&sql, [&candidate], |row| row.get(0))?;
        if count == 0 {
            return Ok(candidate);
        }
    }
    anyhow::bail!("无法为重复名称生成唯一后缀：{base_value}")
}

fn lookup_product_for_stock(
    conn: &Connection,
    name: Option<&str>,
    barcode: Option<&str>,
) -> anyhow::Result<Option<i64>> {
    if let Some(barcode) = barcode.filter(|value| !value.trim().is_empty()) {
        let id = conn
            .query_row(
                "SELECT id FROM products WHERE barcode = ?1 AND is_active = 1 ORDER BY id LIMIT 1",
                [barcode.trim()],
                |row| row.get(0),
            )
            .optional()?;
        if id.is_some() {
            return Ok(id);
        }
    }
    if let Some(name) = name.filter(|value| !value.trim().is_empty()) {
        return conn
            .query_row(
                "SELECT id FROM products WHERE name = ?1 AND is_active = 1 ORDER BY id LIMIT 1",
                [name.trim()],
                |row| row.get(0),
            )
            .optional()
            .map_err(Into::into);
    }
    Ok(None)
}

fn doc_template(id: &str, name: &str, description: &str, active: &str) -> DocumentTemplateDto {
    DocumentTemplateDto {
        id: id.to_string(),
        name: name.to_string(),
        description: description.to_string(),
        template_type: if id == "statement" {
            "statement"
        } else {
            "order"
        }
        .to_string(),
        is_default: id == active,
    }
}

fn terms(
    customer: &str,
    region: &str,
    product: &str,
    category: &str,
    rule: &str,
    credit: &str,
    guest_customer: &str,
) -> TermSettingsDto {
    TermSettingsDto {
        customer: customer.to_string(),
        region: region.to_string(),
        product: product.to_string(),
        category: category.to_string(),
        rule: rule.to_string(),
        credit: credit.to_string(),
        guest_customer: guest_customer.to_string(),
    }
}

fn features(values: [bool; 8]) -> FeatureFlagsDto {
    let [supplier_ledger, customer_rules, monthly_credit, receivables, product_ranking, customer_analysis, inventory_control, diagnostics] =
        values;
    FeatureFlagsDto {
        supplier_ledger,
        customer_rules,
        monthly_credit,
        receivables,
        product_ranking,
        customer_analysis,
        inventory_control,
        diagnostics,
    }
}

fn count(conn: &Connection, table: &str) -> anyhow::Result<i64> {
    let sql = format!("SELECT COUNT(*) FROM {table}");
    conn.query_row(&sql, [], |row| row.get(0))
        .map_err(Into::into)
}

fn setting_text(conn: &Connection, key: &str, default: &str) -> anyhow::Result<String> {
    Ok(db::setting(conn, key)?
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| default.to_string()))
}

fn setting_optional(conn: &Connection, key: &str) -> anyhow::Result<Option<String>> {
    Ok(db::setting(conn, key)?
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty()))
}

fn setting_bool(conn: &Connection, key: &str, default: bool) -> anyhow::Result<bool> {
    Ok(db::setting(conn, key)?
        .map(|value| value == "true")
        .unwrap_or(default))
}

fn set_bool(conn: &Connection, key: &str, value: bool) -> anyhow::Result<()> {
    db::set_setting(conn, key, if value { "true" } else { "false" })
}

fn clean_required(value: &str, label: &str) -> anyhow::Result<String> {
    let value = value.trim();
    if value.is_empty() {
        anyhow::bail!("{label}不能为空");
    }
    Ok(value.to_string())
}

fn optional_value(value: Option<String>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn optional_text_param(value: Option<&str>) -> Option<String> {
    value
        .map(|item| item.trim().to_string())
        .filter(|item| !item.is_empty())
}

fn sanitize_key(value: &str) -> String {
    normalize_key(value)
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
        .collect::<String>()
}

fn normalize_key(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .replace([' ', '_', '-', '/', '\\'], "")
}

fn cell_text(cell: Option<&Data>) -> String {
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
        _ => String::new(),
    }
}

fn cell_number(cell: Option<&Data>) -> Option<f64> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn memory_conn() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        db::init_schema(&conn).unwrap();
        db::seed_settings(&conn).unwrap();
        conn
    }

    #[test]
    fn setup_profile_terms_and_template_are_configurable() {
        let conn = memory_conn();

        assert!(!setup_status(&conn).unwrap().completed);
        save_merchant_profile(
            &conn,
            MerchantProfileDto {
                name: "通用测试商行".to_string(),
                contact: Some("张三".to_string()),
                phone: Some("13800000000".to_string()),
                address: Some("测试地址".to_string()),
                logo_path: None,
                remark: Some("通用商户备注".to_string()),
            },
        )
        .unwrap();
        save_term_settings(
            &conn,
            terms(
                "门店",
                "线路",
                "货品",
                "品类",
                "客户价规则",
                "返利额度",
                "临时客户",
            ),
        )
        .unwrap();
        let applied = apply_industry_template(
            &conn,
            ApplyIndustryTemplateRequest {
                template_id: "retail_outbound".to_string(),
                overwrite_terms: Some(false),
                overwrite_features: Some(true),
            },
        )
        .unwrap();
        apply_document_template(&conn, "simple".to_string()).unwrap();

        let profile = merchant_profile(&conn).unwrap();
        let terms = term_settings(&conn).unwrap();
        let flags = feature_flags(&conn).unwrap();
        let templates = document_templates(&conn).unwrap();

        assert_eq!(profile.name, "通用测试商行");
        assert_eq!(profile.remark.as_deref(), Some("通用商户备注"));
        assert_eq!(
            db::setting(&conn, "template_store_name")
                .unwrap()
                .as_deref(),
            Some("通用测试商行")
        );
        assert_eq!(terms.customer, "门店");
        assert_eq!(terms.guest_customer, "临时客户");
        assert_eq!(applied.id, "retail_outbound");
        assert!(!flags.monthly_credit);
        assert!(templates
            .iter()
            .any(|item| item.id == "simple" && item.is_default));
    }

    #[test]
    fn complete_setup_marks_database_ready() {
        let conn = memory_conn();

        complete_setup(
            &conn,
            CompleteSetupRequest {
                merchant: MerchantProfileDto {
                    name: "首次设置商行".to_string(),
                    contact: None,
                    phone: None,
                    address: None,
                    logo_path: None,
                    remark: None,
                },
                terms: None,
                features: None,
                industry_template: Some("delivery_wholesale".to_string()),
                default_print_template: Some("delivery".to_string()),
                default_export_format: Some("xlsx".to_string()),
                default_printer: None,
            },
        )
        .unwrap();

        let status = setup_status(&conn).unwrap();
        let terms = term_settings(&conn).unwrap();
        let templates = document_templates(&conn).unwrap();
        assert!(status.completed);
        assert_eq!(status.merchant_name, "首次设置商行");
        assert_eq!(status.industry_template, "delivery_wholesale");
        assert_eq!(terms.customer, "门店");
        assert_eq!(terms.region, "线路");
        assert!(templates
            .iter()
            .any(|item| item.id == "delivery" && item.is_default));
    }

    #[test]
    fn generic_product_import_previews_and_confirms_without_clearing_orders() {
        let conn = memory_conn();
        seed_order_marker(&conn);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.xlsx");
        write_generic_workbook(
            &path,
            &[
                "商品名称",
                "类别",
                "条码",
                "默认售价",
                "安全库存",
                "单位",
                "备注",
            ],
            &[
                &["测试商品A", "饮料", "A001", "12.5", "3", "件", "新增"],
                &["测试商品A", "饮料", "A001", "13", "5", "件", "重复"],
                &["", "饮料", "", "", "", "", "缺名称"],
            ],
        );

        let request = GenericImportRequest {
            import_type: "products".to_string(),
            file_path: path.to_string_lossy().to_string(),
            duplicate_strategy: Some("overwrite".to_string()),
            field_mapping: None,
        };
        let preview = preview_generic_import(&conn, request.clone()).unwrap();
        let result = confirm_generic_import(&conn, request).unwrap();
        let order_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM orders", [], |row| row.get(0))
            .unwrap();
        let product = crate::db::product_by_id(&conn, 1).unwrap();

        assert_eq!(preview.create_count, 2);
        assert_eq!(preview.error_count, 1);
        assert_eq!(result.imported_count, 2);
        assert_eq!(order_count, 1);
        assert_eq!(product.name, "测试商品A");
        assert_eq!(product.default_price, 13.0);
        assert_eq!(product.safety_stock, 5.0);
    }

    #[test]
    fn generic_customer_and_initial_stock_import_are_confirmed_separately() {
        let conn = memory_conn();
        let dir = tempfile::tempdir().unwrap();
        let customer_path = dir.path().join("customers.xlsx");
        write_generic_workbook(
            &customer_path,
            &["客户名称", "地区", "地址", "电话", "备注"],
            &[&["客户A", "东线", "地址A", "10086", "新增客户"]],
        );
        confirm_generic_import(
            &conn,
            GenericImportRequest {
                import_type: "customers".to_string(),
                file_path: customer_path.to_string_lossy().to_string(),
                duplicate_strategy: Some("skip".to_string()),
                field_mapping: None,
            },
        )
        .unwrap();
        confirm_generic_import(
            &conn,
            GenericImportRequest {
                import_type: "products".to_string(),
                file_path: product_workbook(&dir).to_string_lossy().to_string(),
                duplicate_strategy: Some("skip".to_string()),
                field_mapping: None,
            },
        )
        .unwrap();
        let stock_path = dir.path().join("stock.xlsx");
        write_generic_workbook(
            &stock_path,
            &["商品名称", "期初库存", "期初成本", "备注"],
            &[&["库存商品", "8", "2.5", "期初"]],
        );
        let result = confirm_generic_import(
            &conn,
            GenericImportRequest {
                import_type: "initial_stock".to_string(),
                file_path: stock_path.to_string_lossy().to_string(),
                duplicate_strategy: Some("overwrite".to_string()),
                field_mapping: None,
            },
        )
        .unwrap();
        let customer_name: String = conn
            .query_row(
                "SELECT name FROM customers WHERE name = '客户A'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let product = crate::db::product_by_id(&conn, 1).unwrap();

        assert_eq!(customer_name, "客户A");
        assert_eq!(result.imported_count, 1);
        assert_eq!(product.current_stock, 8.0);
        assert_eq!(product.avg_cost, 2.5);
    }

    #[test]
    fn generic_import_supports_saved_field_mapping() {
        let conn = memory_conn();
        let mut mapping = HashMap::new();
        mapping.insert("商品名称".to_string(), "品名".to_string());
        mapping.insert("类别".to_string(), "分组".to_string());
        save_import_mapping(
            &conn,
            ImportMappingDto {
                name: "供应商模板".to_string(),
                import_type: "products".to_string(),
                field_mapping: mapping.clone(),
            },
        )
        .unwrap();
        let mappings = list_import_mappings(&conn, Some("products".to_string())).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mapped.xlsx");
        write_generic_workbook(&path, &["品名", "分组"], &[&["映射商品", "映射分类"]]);

        let preview = preview_generic_import(
            &conn,
            GenericImportRequest {
                import_type: "products".to_string(),
                file_path: path.to_string_lossy().to_string(),
                duplicate_strategy: Some("skip".to_string()),
                field_mapping: Some(mapping),
            },
        )
        .unwrap();

        assert_eq!(mappings.len(), 1);
        assert_eq!(preview.valid_count, 1);
        assert_eq!(preview.rows[0].name.as_deref(), Some("映射商品"));
        assert_eq!(preview.rows[0].category.as_deref(), Some("映射分类"));
    }

    #[test]
    fn generic_import_header_preview_suggests_visual_mapping() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("headers.xlsx");
        write_generic_workbook(
            &path,
            &["品名", "分组", "售价", "备注"],
            &[&["表头商品", "表头分类", "16", "测试"]],
        );

        let preview = preview_generic_import_headers(GenericImportHeaderRequest {
            import_type: "products".to_string(),
            file_path: format!("\u{202A}\"{}\"\u{202C}", path.to_string_lossy()),
        })
        .unwrap();

        assert_eq!(preview.sheet_name, "Sheet1");
        assert_eq!(preview.headers, vec!["品名", "分组", "售价", "备注"]);
        assert!(preview
            .fields
            .iter()
            .any(|field| field.name == "商品名称" && field.required));
        assert_eq!(
            preview
                .suggested_mapping
                .get("商品名称")
                .map(String::as_str),
            Some("品名")
        );
        assert_eq!(
            preview.suggested_mapping.get("类别").map(String::as_str),
            Some("分组")
        );
        assert_eq!(
            preview
                .suggested_mapping
                .get("默认售价")
                .map(String::as_str),
            Some("售价")
        );
    }

    #[test]
    fn generic_import_append_suffix_creates_unique_rows_and_report() {
        let conn = memory_conn();
        let dir = tempfile::tempdir().unwrap();
        let product_path = dir.path().join("append-products.xlsx");
        write_generic_workbook(
            &product_path,
            &["商品名称", "类别", "条码", "默认售价"],
            &[
                &["重复商品", "饮料", "DUP001", "12"],
                &["重复商品", "饮料", "DUP001", "13"],
            ],
        );

        let result = confirm_generic_import(
            &conn,
            GenericImportRequest {
                import_type: "products".to_string(),
                file_path: product_path.to_string_lossy().to_string(),
                duplicate_strategy: Some("append_suffix".to_string()),
                field_mapping: None,
            },
        )
        .unwrap();
        let original: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM products WHERE name = '重复商品'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let suffixed: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM products WHERE name = '重复商品_1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let suffixed_barcode: Option<String> = conn
            .query_row(
                "SELECT barcode FROM products WHERE name = '重复商品_1'",
                [],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .unwrap()
            .flatten();

        let report_path = dir.path().join("import-report.xlsx");
        let report = export_generic_import_report(
            &report_path,
            GenericImportReportRequest {
                title: "导入报告测试".to_string(),
                rows: result.rows,
            },
        )
        .unwrap();

        assert_eq!(original, 1);
        assert_eq!(suffixed, 1);
        assert_eq!(suffixed_barcode, None);
        assert!(report.ends_with("import-report.xlsx"));
        assert!(report_path.exists());
    }

    #[test]
    fn generic_import_template_exports_all_supported_types() {
        let dir = tempfile::tempdir().unwrap();
        for (import_type, first_header) in [
            ("products", "商品名称"),
            ("customers", "客户名称"),
            ("initial_stock", "商品名称"),
        ] {
            let path = dir.path().join(format!("{import_type}.xlsx"));
            let exported = export_generic_import_template(&path, import_type).unwrap();
            let mut workbook = open_workbook_auto(&path).unwrap();
            let sheet_name = workbook.sheet_names().first().cloned().unwrap();
            let range = workbook.worksheet_range(&sheet_name).unwrap();
            let header = range.rows().next().unwrap();

            assert_eq!(exported, path.to_string_lossy().to_string());
            assert_eq!(cell_text(header.first()), first_header);
            assert!(range.height() >= 2);
        }
    }

    fn seed_order_marker(conn: &Connection) {
        let now = now_text();
        conn.execute(
            "INSERT INTO customers (id, region, name, is_active, created_at, updated_at)
             VALUES (1, '测试', '订单客户', 1, ?1, ?1)",
            params![now],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO orders
             (order_no, order_date, customer_id, customer_name, customer_payable_amount, status, created_at, updated_at)
             VALUES ('20260604001', '2026-06-04', 1, '订单客户', 1, 'normal', ?1, ?1)",
            params![now],
        )
        .unwrap();
    }

    fn product_workbook(dir: &tempfile::TempDir) -> std::path::PathBuf {
        let path = dir.path().join("stock-product.xlsx");
        write_generic_workbook(
            &path,
            &["商品名称", "类别", "默认售价"],
            &[&["库存商品", "期初", "10"]],
        );
        path
    }

    fn write_generic_workbook(path: &std::path::Path, headers: &[&str], rows: &[&[&str]]) {
        let mut book = umya_spreadsheet::new_file();
        let sheet = book.get_sheet_by_name_mut("Sheet1").unwrap();
        for (index, header) in headers.iter().enumerate() {
            sheet
                .get_cell_mut(cell_address((index + 1) as u32, 1))
                .set_value(*header);
        }
        for (row_index, row) in rows.iter().enumerate() {
            for (col_index, value) in row.iter().enumerate() {
                sheet
                    .get_cell_mut(cell_address((col_index + 1) as u32, (row_index + 2) as u32))
                    .set_value(*value);
            }
        }
        umya_spreadsheet::writer::xlsx::write(&book, path).unwrap();
    }

    fn cell_address(column: u32, row: u32) -> String {
        const NAMES: [&str; 12] = ["A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L"];
        format!("{}{}", NAMES[(column - 1) as usize], row)
    }
}
