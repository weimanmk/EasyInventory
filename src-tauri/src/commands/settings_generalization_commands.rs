use super::{fail, ok};
use crate::app::AppState;
use crate::generalization;
use crate::logger;
use crate::models::*;
use crate::services::settings_service;
use crate::utils::safe_file_name;
use tauri::State;

#[tauri::command]
pub fn list_settings(state: State<AppState>) -> ApiResponse<Vec<SettingDto>> {
    let result = (|| {
        let conn = state.connection()?;
        settings_service::list_settings(&conn)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn save_settings(state: State<AppState>, payload: SaveSettingsRequest) -> ApiResponse<bool> {
    let result = (|| {
        let conn = state.connection()?;
        settings_service::save_settings(&conn, payload)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn get_setup_status(state: State<AppState>) -> ApiResponse<SetupStatusDto> {
    let result = (|| {
        let conn = state.connection()?;
        generalization::setup_status(&conn)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn complete_setup(state: State<AppState>, request: CompleteSetupRequest) -> ApiResponse<bool> {
    let result = (|| {
        let conn = state.connection()?;
        generalization::complete_setup(&conn, request)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn get_merchant_profile(state: State<AppState>) -> ApiResponse<MerchantProfileDto> {
    let result = (|| {
        let conn = state.connection()?;
        generalization::merchant_profile(&conn)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn save_merchant_profile(
    state: State<AppState>,
    profile: MerchantProfileDto,
) -> ApiResponse<bool> {
    let result = (|| {
        let conn = state.connection()?;
        generalization::save_merchant_profile(&conn, profile)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn get_term_settings(state: State<AppState>) -> ApiResponse<TermSettingsDto> {
    let result = (|| {
        let conn = state.connection()?;
        generalization::term_settings(&conn)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn save_term_settings(state: State<AppState>, terms: TermSettingsDto) -> ApiResponse<bool> {
    let result = (|| {
        let conn = state.connection()?;
        generalization::save_term_settings(&conn, terms)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn get_feature_flags(state: State<AppState>) -> ApiResponse<FeatureFlagsDto> {
    let result = (|| {
        let conn = state.connection()?;
        generalization::feature_flags(&conn)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn save_feature_flags(state: State<AppState>, flags: FeatureFlagsDto) -> ApiResponse<bool> {
    let result = (|| {
        let conn = state.connection()?;
        generalization::save_feature_flags(&conn, flags)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_industry_templates() -> ApiResponse<Vec<IndustryTemplateDto>> {
    ok(generalization::industry_templates())
}

#[tauri::command]
pub fn apply_industry_template(
    state: State<AppState>,
    request: ApplyIndustryTemplateRequest,
) -> ApiResponse<IndustryTemplateDto> {
    let result = (|| {
        let conn = state.connection()?;
        generalization::apply_industry_template(&conn, request)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_document_templates(state: State<AppState>) -> ApiResponse<Vec<DocumentTemplateDto>> {
    let result = (|| {
        let conn = state.connection()?;
        generalization::document_templates(&conn)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn apply_document_template(state: State<AppState>, template_id: String) -> ApiResponse<bool> {
    let result = (|| {
        let conn = state.connection()?;
        generalization::apply_document_template(&conn, template_id)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn preview_generic_import(
    state: State<AppState>,
    request: GenericImportRequest,
) -> ApiResponse<GenericImportPreviewDto> {
    let result = (|| {
        let conn = state.connection()?;
        generalization::preview_generic_import(&conn, request)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn preview_generic_import_headers(
    request: GenericImportHeaderRequest,
) -> ApiResponse<GenericImportHeadersDto> {
    let result = generalization::preview_generic_import_headers(request);
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn confirm_generic_import(
    state: State<AppState>,
    request: GenericImportRequest,
) -> ApiResponse<GenericImportResultDto> {
    let result = (|| {
        let conn = state.connection()?;
        generalization::confirm_generic_import(&conn, request)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn export_generic_import_report(
    state: State<AppState>,
    request: GenericImportReportRequest,
) -> ApiResponse<String> {
    let result = {
        let title = safe_file_name(if request.title.trim().is_empty() {
            "通用导入报告"
        } else {
            request.title.trim()
        });
        let path = state.exports_dir().join(format!(
            "{}_{}.xlsx",
            title,
            chrono::Local::now().format("%Y%m%d_%H%M%S")
        ));
        generalization::export_generic_import_report(&path, request)
    };
    if let Ok(path) = &result {
        logger::info("import", format!("通用导入报告已导出：{path}"));
    }
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn download_import_template(
    state: State<AppState>,
    import_type: String,
) -> ApiResponse<String> {
    let result = (|| {
        let title = match import_type.as_str() {
            "products" => "通用商品导入模板",
            "customers" => "通用客户导入模板",
            "initial_stock" => "通用期初库存导入模板",
            other => anyhow::bail!("不支持的通用导入模板类型：{other}"),
        };
        let path = state.exports_dir().join(format!(
            "{}_{}.xlsx",
            safe_file_name(title),
            chrono::Local::now().format("%Y%m%d_%H%M%S")
        ));
        generalization::export_generic_import_template(&path, &import_type)
    })();
    if let Ok(path) = &result {
        logger::info("import", format!("通用导入模板已导出：{path}"));
    }
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn save_import_mapping(state: State<AppState>, mapping: ImportMappingDto) -> ApiResponse<bool> {
    let result = (|| {
        let conn = state.connection()?;
        generalization::save_import_mapping(&conn, mapping)
    })();
    result.map(ok).unwrap_or_else(fail)
}

#[tauri::command]
pub fn list_import_mappings(
    state: State<AppState>,
    import_type: Option<String>,
) -> ApiResponse<Vec<ImportMappingDto>> {
    let result = (|| {
        let conn = state.connection()?;
        generalization::list_import_mappings(&conn, import_type)
    })();
    result.map(ok).unwrap_or_else(fail)
}
